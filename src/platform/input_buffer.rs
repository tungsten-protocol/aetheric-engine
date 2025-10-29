//=========================================================================
// Input Buffer
//
// Aggregates input events per frame into discrete and continuous categories.
//
// Data Structure:
// ```text
// InputBuffer
// ├── discrete: Vec<InputEvent>
// │   ├─ KeyDown{A}
// │   ├─ KeyDown{B}     ← Order preserved
// │   ├─ KeyDown{A}     ← Non-consecutive duplicate OK
// │   └─ MouseButtonDown{Left}
// │
// └── continuous: HashSet<InputEvent>
//     └─ MouseMoved{x:100, y:200}  ← Only latest (size = 0 or 1)
//
// Deduplication Strategy:
// - Discrete: Last element check (O(1) for consecutive)
// - Continuous: HashSet::replace (O(1) via hash-by-discriminant)
//
// HashSet Trick:
//   MouseMoved{x:10, y:10}.hash() == MouseMoved{x:99, y:88}.hash()
//   ↓
//   .replace() automatically keeps latest coordinates
// ```
//
// Memory Management:
// - Initial capacity: 128 discrete, 1 continuous
// - Capacity preserved across drain() calls (avoids realloc)
// - Typical frame: 5-10 discrete, 0-1 continuous
//
//=========================================================================

//=== Internal Imports ====================================================

use std::collections::HashSet;
use std::mem;
use crate::core::input::event::InputEvent;

//=== InputBuffer =========================================================

/// Temporary storage for input events within a single frame.
///
/// Events are categorized into two buffers based on their semantics:
///
/// # Discrete Events
///
/// Stored in a `Vec` to preserve order (critical for correct action mapping).
/// Consecutive duplicates are filtered to avoid redundant processing:
///
/// ```ignore
/// push_discrete(KeyDown{A})
/// push_discrete(KeyDown{A})  // Ignored (duplicate)
/// push_discrete(KeyDown{B})
/// push_discrete(KeyDown{A})  // Allowed (non-consecutive)
/// ```
///
/// # Continuous Events
///
/// Stored in a `HashSet` that exploits hash-by-discriminant semantics.
/// Only the **latest** event of each type is kept:
///
/// ```ignore
/// push_continuous(MouseMoved{x:10, y:10})
/// push_continuous(MouseMoved{x:20, y:30})  // Replaces previous
/// // Result: Only MouseMoved{x:20, y:30} remains
/// ```
///
/// This works because `InputEvent` hashes by discriminant (event type),
/// not payload. See `InputEvent::hash()` implementation for details.
///
/// # Memory Management
///
/// - Initial capacities: 128 discrete, 1 continuous
/// - Capacities preserved across [`drain()`](Self::drain) calls
/// - Avoids per-frame allocations in steady state
///
/// # Visibility
///
/// This type is `pub(super)` - visible only within the platform module.
/// It's an internal optimization detail, not part of the public API.
pub(super) struct InputBuffer {
    /// Discrete events (keys, buttons) in insertion order.
    ///
    /// Consecutive duplicates are filtered on insertion.
    /// Typical capacity: 128 elements (resizable).
    discrete: Vec<InputEvent>,

    /// Continuous events (mouse position) with automatic coalescing.
    ///
    /// HashSet::replace keeps only latest event per discriminant.
    /// Typical size: 0-1 elements (MouseMoved or empty).
    continuous: HashSet<InputEvent>,
}

impl InputBuffer {
    //--- Construction -----------------------------------------------------

    /// Creates buffer with capacity tuned for typical gameplay.
    ///
    /// Initial capacities:
    /// - Discrete: 128 events (handles burst input)
    /// - Continuous: 1 event (only MouseMoved in practice)
    ///
    /// These values were empirically tuned for 60 FPS gameplay with
    /// typical keyboard/mouse input patterns.
    pub(super) fn new() -> Self {
        Self {
            discrete: Vec::with_capacity(128),
            // Continuous buffer only holds MouseMoved (max size = 1)
            continuous: HashSet::with_capacity(1),
        }
    }

    //--- Insertion --------------------------------------------------------

    /// Adds a continuous event, replacing any previous event of same type.
    ///
    /// Works via hash-by-discriminant: all `MouseMoved` events hash identically,
    /// so `HashSet::replace()` keeps only the most recent coordinates.
    ///
    /// # Performance
    ///
    /// - Time: O(1) average case (hash table)
    /// - Space: O(1) - set size never exceeds 1 for MouseMoved
    ///
    /// # Examples
    ///
    /// ```ignore
    /// buffer.push_continuous(MouseMoved{x: 10, y: 10});
    /// buffer.push_continuous(MouseMoved{x: 20, y: 30});
    /// // Buffer now contains only MouseMoved{x: 20, y: 30}
    /// ```
    pub(super) fn push_continuous(&mut self, event: InputEvent) {
        self.continuous.replace(event);
    }

    /// Adds a discrete event, ignoring consecutive duplicates.
    ///
    /// Only checks the **last** element for duplication (O(1) check).
    /// Non-consecutive duplicates are allowed to preserve game semantics
    /// (e.g., rapid key tapping).
    ///
    /// # Performance
    ///
    /// - Time: O(1) for duplicate check, O(1) amortized for push
    /// - Space: O(1) per event
    ///
    /// # Examples
    ///
    /// ```ignore
    /// buffer.push_discrete(KeyDown{A});
    /// buffer.push_discrete(KeyDown{A});  // Ignored
    /// buffer.push_discrete(KeyDown{B});
    /// buffer.push_discrete(KeyDown{A});  // Allowed
    /// // Buffer: [KeyDown{A}, KeyDown{B}, KeyDown{A}]
    /// ```
    pub(super) fn push_discrete(&mut self, event: InputEvent) {
        if self.discrete.last() != Some(&event) {
            self.discrete.push(event);
        }
    }

    //--- Draining ---------------------------------------------------------

    /// Drains all events, returning `None` if buffer was empty.
    ///
    /// Returns two separate vectors:
    /// 1. Discrete events (in insertion order)
    /// 2. Continuous events (unordered - typically 0 or 1 element)
    ///
    /// # Capacity Preservation
    ///
    /// Both internal buffers retain their capacity after draining:
    /// - `discrete`: Uses `mem::take` + manual realloc with same capacity
    /// - `continuous`: Drains into vec, then recreates HashSet with same capacity
    ///
    /// This avoids reallocation on every frame (60+ times/second).
    ///
    /// # Performance
    ///
    /// - Time: O(n) where n = total events
    /// - Space: O(n) for returned vectors, O(1) for internal buffers
    ///
    /// # Returns
    ///
    /// - `Some((discrete, continuous))` if any events were buffered
    /// - `None` if both buffers were empty (optimization for caller to skip send)
    pub(super) fn drain(&mut self) -> Option<(Vec<InputEvent>, Vec<InputEvent>)> {
        if self.is_empty() {
            return None;
        }

        // Capture capacities before draining
        let discrete_cap = self.discrete.capacity();
        let continuous_cap = self.continuous.capacity();

        // Move discrete vec (O(1) - just pointer swap)
        let discrete = mem::take(&mut self.discrete);

        // Drain continuous into vec (O(n) but n is typically 1)
        let continuous: Vec<_> = self.continuous.drain().collect();

        // Restore with original capacities (avoids realloc next frame)
        self.discrete = Vec::with_capacity(discrete_cap);
        self.continuous = HashSet::with_capacity(continuous_cap);

        Some((discrete, continuous))
    }

    //--- Queries ----------------------------------------------------------

    /// Returns `true` if both buffers are empty.
    ///
    /// Used by `drain()` to optimize out empty sends.
    pub(super) fn is_empty(&self) -> bool {
        self.discrete.is_empty() && self.continuous.is_empty()
    }
}

//=========================================================================
// Unit Tests
//=========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::input::event::{KeyCode, Modifiers, MouseButton};

    //--- Test Helpers -----------------------------------------------------

    fn key_down(key: KeyCode) -> InputEvent {
        InputEvent::KeyDown {
            key,
            modifiers: Modifiers::NONE,
        }
    }

    fn mouse_move(x: f32, y: f32) -> InputEvent {
        InputEvent::MouseMoved { x, y }
    }

    fn mouse_down(btn: MouseButton) -> InputEvent {
        InputEvent::MouseButtonDown {
            button: btn,
            modifiers: Modifiers::NONE,
        }
    }

    //=====================================================================
    // Construction Tests
    //=====================================================================

    #[test]
    fn new_buffer_is_empty() {
        let buffer = InputBuffer::new();
        assert!(buffer.is_empty());
        assert_eq!(buffer.discrete.len(), 0);
        assert_eq!(buffer.continuous.len(), 0);
    }

    #[test]
    fn new_buffer_has_preallocated_capacity() {
        let buffer = InputBuffer::new();
        assert!(buffer.discrete.capacity() >= 128);
        assert!(buffer.continuous.capacity() >= 1);
    }

    //=====================================================================
    // Discrete Event Tests
    //=====================================================================

    #[test]
    fn discrete_deduplicates_consecutive() {
        let mut buffer = InputBuffer::new();
        buffer.push_discrete(key_down(KeyCode::KeyA));
        buffer.push_discrete(key_down(KeyCode::KeyA));
        buffer.push_discrete(key_down(KeyCode::KeyB));

        assert_eq!(buffer.discrete.len(), 2);
    }

    #[test]
    fn discrete_allows_nonconsecutive_duplicates() {
        let mut buffer = InputBuffer::new();
        buffer.push_discrete(key_down(KeyCode::KeyA));
        buffer.push_discrete(key_down(KeyCode::KeyB));
        buffer.push_discrete(key_down(KeyCode::KeyA));

        assert_eq!(buffer.discrete.len(), 3);
    }

    #[test]
    fn discrete_preserves_insertion_order() {
        let mut buffer = InputBuffer::new();
        buffer.push_discrete(key_down(KeyCode::KeyA));
        buffer.push_discrete(key_down(KeyCode::KeyB));
        buffer.push_discrete(key_down(KeyCode::KeyC));

        let (discrete, _) = buffer.drain().unwrap();

        // Verify order
        match (&discrete[0], &discrete[1], &discrete[2]) {
            (
                InputEvent::KeyDown { key: KeyCode::KeyA, .. },
                InputEvent::KeyDown { key: KeyCode::KeyB, .. },
                InputEvent::KeyDown { key: KeyCode::KeyC, .. },
            ) => {},
            _ => panic!("Order not preserved"),
        }
    }

    #[test]
    fn discrete_different_types_no_dedup() {
        let mut buffer = InputBuffer::new();
        buffer.push_discrete(key_down(KeyCode::KeyA));
        buffer.push_discrete(mouse_down(MouseButton::Left));
        buffer.push_discrete(key_down(KeyCode::KeyA));

        assert_eq!(buffer.discrete.len(), 3);
    }

    //=====================================================================
    // Continuous Event Tests
    //=====================================================================

    #[test]
    fn continuous_keeps_only_latest() {
        let mut buffer = InputBuffer::new();
        buffer.push_continuous(mouse_move(10.0, 10.0));
        buffer.push_continuous(mouse_move(20.0, 30.0));

        assert_eq!(buffer.continuous.len(), 1);

        let event = buffer.continuous.iter().next().unwrap();
        match event {
            InputEvent::MouseMoved { x, y } => {
                assert_eq!((*x, *y), (20.0, 30.0));
            }
            _ => panic!("Expected MouseMoved"),
        }
    }

    #[test]
    fn continuous_size_remains_one() {
        let mut buffer = InputBuffer::new();

        for i in 0..100 {
            buffer.push_continuous(mouse_move(i as f32, i as f32));
        }

        assert_eq!(buffer.continuous.len(), 1, "Size should always be 1");
    }

    //=====================================================================
    // Mixed Event Tests
    //=====================================================================

    #[test]
    fn mixed_events_stored_independently() {
        let mut buffer = InputBuffer::new();

        buffer.push_discrete(key_down(KeyCode::KeyA));
        buffer.push_continuous(mouse_move(10.0, 20.0));
        buffer.push_discrete(key_down(KeyCode::KeyB));
        buffer.push_continuous(mouse_move(30.0, 40.0));

        assert_eq!(buffer.discrete.len(), 2);
        assert_eq!(buffer.continuous.len(), 1);
    }

    //=====================================================================
    // Drain Tests
    //=====================================================================

    #[test]
    fn drain_returns_both_categories() {
        let mut buffer = InputBuffer::new();
        buffer.push_discrete(key_down(KeyCode::KeyA));
        buffer.push_continuous(mouse_move(5.0, 5.0));

        let (discrete, continuous) = buffer.drain().unwrap();

        assert_eq!(discrete.len(), 1);
        assert_eq!(continuous.len(), 1);
        assert!(buffer.is_empty());
    }

    #[test]
    fn drain_empty_returns_none() {
        let mut buffer = InputBuffer::new();
        assert!(buffer.drain().is_none());
    }

    #[test]
    fn drain_only_discrete_returns_some() {
        let mut buffer = InputBuffer::new();
        buffer.push_discrete(key_down(KeyCode::KeyA));

        let result = buffer.drain();
        assert!(result.is_some());

        let (discrete, continuous) = result.unwrap();
        assert_eq!(discrete.len(), 1);
        assert_eq!(continuous.len(), 0);
    }

    #[test]
    fn drain_only_continuous_returns_some() {
        let mut buffer = InputBuffer::new();
        buffer.push_continuous(mouse_move(10.0, 20.0));

        let result = buffer.drain();
        assert!(result.is_some());

        let (discrete, continuous) = result.unwrap();
        assert_eq!(discrete.len(), 0);
        assert_eq!(continuous.len(), 1);
    }

    #[test]
    fn multiple_drains() {
        let mut buffer = InputBuffer::new();

        // First batch
        buffer.push_discrete(key_down(KeyCode::KeyA));
        let (d1, _) = buffer.drain().unwrap();
        assert_eq!(d1.len(), 1);

        // Second batch
        buffer.push_discrete(key_down(KeyCode::KeyB));
        let (d2, _) = buffer.drain().unwrap();
        assert_eq!(d2.len(), 1);

        // Third drain on empty
        assert!(buffer.drain().is_none());
    }

    //=====================================================================
    // Capacity Preservation Tests
    //=====================================================================

    #[test]
    fn drain_preserves_discrete_capacity() {
        let mut buffer = InputBuffer::new();

        for _ in 0..200 {
            buffer.push_discrete(key_down(KeyCode::KeyA));
        }

        let cap_before = buffer.discrete.capacity();
        buffer.drain();

        assert_eq!(buffer.discrete.capacity(), cap_before);
    }

    #[test]
    fn drain_preserves_continuous_capacity() {
        let mut buffer = InputBuffer::new();

        buffer.push_continuous(mouse_move(1.0, 1.0));

        // Manually grow capacity (simulating future growth)
        buffer.continuous.reserve(32);
        let cap_before = buffer.continuous.capacity();

        buffer.drain();

        assert_eq!(
            buffer.continuous.capacity(),
            cap_before,
            "Capacity should be preserved"
        );
    }

    //=====================================================================
    // is_empty Tests
    //=====================================================================

    #[test]
    fn is_empty_on_new_buffer() {
        let buffer = InputBuffer::new();
        assert!(buffer.is_empty());
    }

    #[test]
    fn is_empty_false_after_discrete() {
        let mut buffer = InputBuffer::new();
        buffer.push_discrete(key_down(KeyCode::KeyA));
        assert!(!buffer.is_empty());
    }

    #[test]
    fn is_empty_false_after_continuous() {
        let mut buffer = InputBuffer::new();
        buffer.push_continuous(mouse_move(10.0, 20.0));
        assert!(!buffer.is_empty());
    }

    #[test]
    fn is_empty_true_after_drain() {
        let mut buffer = InputBuffer::new();
        buffer.push_discrete(key_down(KeyCode::KeyA));
        buffer.drain();
        assert!(buffer.is_empty());
    }
}