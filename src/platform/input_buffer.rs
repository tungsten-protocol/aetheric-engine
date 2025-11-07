//=========================================================================
// Input Buffer
//=========================================================================
//
// Per-frame input buffer with discrete and continuous event storage.
//
// Architecture:
//   Discrete: Vec (order-preserved, consecutive dedup)
//   Continuous: HashSet (coalesced, latest-wins)
//
// Discrete handles keys/buttons, continuous handles mouse movement.
//
//=========================================================================

//=== External Dependencies ===============================================

use std::collections::HashSet;
use std::mem;

//=== Internal Dependencies ===============================================

use crate::core::input::event::InputEvent;

//=== InputBuffer =========================================================

/// Per-frame input buffer with order-preserving discrete storage and coalescing continuous storage.
/// Discrete: Vec with consecutive deduplication. Continuous: HashSet with latest-wins replacement.
pub(super) struct InputBuffer {
    discrete: Vec<InputEvent>,
    continuous: HashSet<InputEvent>,
}

impl InputBuffer {
    /// Creates buffer with initial capacity (128 discrete, 1 continuous).
    pub(super) fn new() -> Self {
        Self {
            discrete: Vec::with_capacity(128),
            // Continuous buffer only holds MouseMoved (max size = 1)
            continuous: HashSet::with_capacity(1),
        }
    }

    /// Adds a continuous event (replaces previous via hash-by-discriminant).
    pub(super) fn push_continuous(&mut self, event: InputEvent) {
        self.continuous.replace(event);
    }

    /// Adds a discrete event (ignores consecutive duplicates only).
    pub(super) fn push_discrete(&mut self, event: InputEvent) {
        if self.discrete.last() != Some(&event) {
            self.discrete.push(event);
        }
    }

    /// Drains all events, preserving capacity. Returns None if empty.
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

    /// Returns true if both buffers are empty.
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