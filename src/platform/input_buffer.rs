//=========================================================================
// Input Buffer
//
// Collects and normalizes raw input events (keyboard, mouse, etc.)
// into two categories: discrete and continuous. Acts as a transient
// event aggregator between the Platform and the InputSystem.
//
// Responsibilities:
// - Store incoming platform events per frame
// - Deduplicate repeated discrete inputs (e.g., KeyDown)
// - Coalesce continuous inputs (e.g., MouseMoved)
// - Provide unified access to collected events via `drain()`
//
// Notes:
// The InputBuffer exists only for the current frame and is reset
// after being drained by the engine’s core systems.
//=========================================================================

//=== Standard Library Imports ============================================
use std::collections::HashSet;

//=== Internal Modules ====================================================
use crate::core::input::event::RawInputEvent;

//=== InputBuffer Struct ==================================================
//
// Represents the transient event store for one frame of input.
//
// Internally maintains:
// - `discrete`: unique, one-shot inputs (e.g., KeyDown)
// - `continuous`: last-known state of continuous inputs (e.g., MouseMoved)
//
pub struct InputBuffer {
    discrete: Vec<RawInputEvent>,
    continuous: HashSet<RawInputEvent>,
}

impl InputBuffer {
    //--- Construction -----------------------------------------------------
    //
    // Creates a new input buffer with preallocated capacity to minimize
    // reallocations under typical gameplay conditions.
    //
    pub fn new() -> Self {
        const DISCRETE_BASE: usize = 128;
        const CONTINUOUS_BASE: usize = 16;

        Self {
            discrete: Vec::with_capacity(DISCRETE_BASE),
            continuous: HashSet::with_capacity(CONTINUOUS_BASE),
        }
    }

    //--- Continuous Event Handling ---------------------------------------
    //
    // Inserts or replaces a continuous input (e.g., mouse movement).
    // The latest event always replaces any previous one of the same type.
    //
    pub fn push_continuous(&mut self, event: RawInputEvent) {
        self.continuous.replace(event);
    }

    //--- Discrete Event Handling -----------------------------------------
    //
    // Appends a discrete input (e.g., key press, button click).
    // Duplicate consecutive events are ignored to prevent flooding.
    //
    pub fn push_discrete(&mut self, event: RawInputEvent) {
        if self.discrete.last() != Some(&event) {
            self.discrete.push(event);
        }
    }

    //--- Drain ------------------------------------------------------------
    //
    // Returns all collected events for this frame and clears the buffer.
    // Combines both discrete and continuous events into a single vector.
    //
    pub fn drain(&mut self) -> Vec<RawInputEvent> {
        let mut events = std::mem::take(&mut self.discrete);
        events.extend(self.continuous.drain());
        self.continuous.clear();
        events
    }

    //--- Utilities --------------------------------------------------------
    pub fn clear(&mut self) {
        self.discrete.clear();
        self.continuous.clear();
    }

    pub fn len(&self) -> usize {
        self.discrete.len() + self.continuous.len()
    }

    pub fn is_empty(&self) -> bool {
        self.discrete.is_empty() && self.continuous.is_empty()
    }
}

//=========================================================================
// Unit Tests
//=========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::input::event::KeyCode;

    fn key_down(code: KeyCode) -> RawInputEvent {
        RawInputEvent::KeyDown(code)
    }

    fn mouse_move(x: f32, y: f32) -> RawInputEvent {
        RawInputEvent::MouseMoved { x, y }
    }

    #[test]
    fn test_discrete_deduplication() {
        let mut buffer = InputBuffer::new();
        buffer.push_discrete(key_down(KeyCode::KeyA));
        buffer.push_discrete(key_down(KeyCode::KeyA));
        buffer.push_discrete(key_down(KeyCode::KeyB));
        assert_eq!(buffer.discrete.len(), 2, "Duplicates should be ignored");
    }

    #[test]
    fn test_continuous_overwrite() {
        let mut buffer = InputBuffer::new();

        buffer.push_continuous(mouse_move(10.0, 10.0));
        buffer.push_continuous(mouse_move(20.0, 30.0));

        assert_eq!(
            buffer.continuous.len(),
            1,
            "Continuous buffer should keep only the latest event"
        );

        let event = buffer.continuous.iter().next().unwrap();
        if let RawInputEvent::MouseMoved { x, y } = event {
            assert_eq!((*x, *y), (20.0, 30.0));
        } else {
            panic!("Expected MouseMoved event, found {:?}", event);
        }
    }

    #[test]
    fn test_drain_clears_buffer() {
        let mut buffer = InputBuffer::new();
        buffer.push_discrete(key_down(KeyCode::KeyA));
        buffer.push_continuous(mouse_move(5.0, 5.0));

        let events = buffer.drain();
        assert_eq!(events.len(), 2);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_clear_behavior() {
        let mut buffer = InputBuffer::new();
        buffer.push_discrete(key_down(KeyCode::KeyA));
        buffer.push_continuous(mouse_move(1.0, 2.0));
        buffer.clear();
        assert!(buffer.is_empty());
    }

    //--- Memory Retention -------------------------------------------------
    //
    // Ensures that `clear()` does not deallocate underlying storage,
    // preserving buffer capacity for reuse across frames.
    //
    #[test]
    fn test_clear_does_not_deallocate() {
        let mut buffer = InputBuffer::new();

        // Fill buffer to trigger allocation growth
        for _ in 0..256 {
            buffer.push_discrete(key_down(KeyCode::Unidentified));
        }
        for i in 0..32 {
            buffer.push_continuous(mouse_move(i as f32, i as f32));
        }

        let vec_cap_before = buffer.discrete.capacity();
        let map_cap_before = buffer.continuous.capacity();

        buffer.clear();

        assert_eq!(buffer.discrete.len(), 0);
        assert_eq!(buffer.continuous.len(), 0);
        assert_eq!(buffer.discrete.capacity(), vec_cap_before);
        assert_eq!(buffer.continuous.capacity(), map_cap_before);
    }
}
