//=========================================================================
// Input Buffer
//
// Collects and normalizes raw input events (keyboard, mouse, etc.) into
// two categories: discrete and continuous. Provides frame-level access
// to the event stream for the engine's input system.
//
// Responsibilities:
// - Store incoming platform events (via Platform subsystem)
// - Deduplicate repeated discrete inputs (KeyDown, KeyUp, etc.)
// - Coalesce continuous events (mouse movement, analog input, etc.)
// - Expose unified frame event list through `drain()`
//
//=========================================================================

use std::collections::HashSet;
use crate::core::input::event::RawInputEvent;
pub struct InputBuffer {
    discrete: Vec<RawInputEvent>,
    continuous: HashSet<RawInputEvent>,
}

//=== InputBuffer Struct ==================================================
//
// Represents the transient input buffer for a single frame.
//
// Internally maintains:
// - `discrete`: a vector of unique, one-shot events (e.g., KeyDown)
// - `continuous`: a hashmap of continuous inputs (e.g., MouseMoved)
//
impl InputBuffer {

    //--- Constructor ------------------------------------------------------
    //
    // Creates a new input buffer with pre-allocated space for common usage.
    // Capacity is chosen to avoid reallocations under normal gameplay.
    //
    pub fn new() -> Self {
        Self {
            discrete: Vec::with_capacity(1024),
            continuous: HashSet::with_capacity(64),
        }
    }

    //--- Continuous Event Handling ---------------------------------------
    //
    // Inserts or replaces a continuous event (like mouse motion).
    // The latest state always overrides the previous one for the same kind.
    //
    pub fn push_continuous(&mut self, event: RawInputEvent) {
        self.continuous.replace(event);
    }

    //--- Discrete Event Handling -----------------------------------------
    //
    // Pushes discrete (one-shot) events like key presses or button clicks.
    // Duplicate consecutive events are ignored to prevent flooding.
    //
    pub fn push_discrete(&mut self, event: RawInputEvent) {
        if self.discrete.is_empty() || self.discrete.last().unwrap() != &event {
            self.discrete.push(event);
        }
    }

    //--- Drain ------------------------------------------------------------
    //
    // Returns all collected events for this frame and clears the buffer.
    // Combines both discrete and continuous events into a single vector.
    //
    pub fn drain(&mut self) -> Vec<RawInputEvent> {
        let mut events =  std::mem::take(&mut self.discrete);
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
    use crate::core::input::event::{KeyCode};

    fn make_key_down(code: KeyCode) -> RawInputEvent {
        RawInputEvent::KeyDown(code)
    }

    fn make_mouse_move(x: f32, y: f32) -> RawInputEvent {
        RawInputEvent::MouseMoved { x, y }
    }

    #[test]
    fn test_discrete_deduplication() {
        let mut buffer = InputBuffer::new();
        buffer.push_discrete(make_key_down(KeyCode::KeyA));
        buffer.push_discrete(make_key_down(KeyCode::KeyA));
        buffer.push_discrete(make_key_down(KeyCode::KeyB));
        assert_eq!(buffer.discrete.len(), 2);
    }

    #[test]
    fn test_continuous_overwrite() {
        let mut buffer = InputBuffer::new();
        
        buffer.push_continuous(make_mouse_move(10.0, 10.0));
        buffer.push_continuous(make_mouse_move(20.0, 30.0));
        assert_eq!(buffer.continuous.len(), 1, "Continuous buffer should only keep latest event");

        // Verify coordinates of the stored event
        let event = buffer.continuous.iter().next().unwrap();
        if let RawInputEvent::MouseMoved { x, y } = event {
            assert_eq!((*x, *y), (20.0, 30.0), "MouseMoved should reflect last input");
        } else {
            panic!("Expected MouseMoved event, found {:?}", event);
        }
    }

    #[test]
    fn test_drain_clears_buffer() {
        let mut buffer = InputBuffer::new();
        buffer.push_discrete(make_key_down(KeyCode::KeyA));
        buffer.push_continuous(make_mouse_move(5.0, 5.0));

        let events = buffer.drain();
        assert_eq!(events.len(), 2);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_clear_behavior() {
        let mut buffer = InputBuffer::new();
        buffer.push_discrete(make_key_down(KeyCode::KeyA));
        buffer.push_continuous(make_mouse_move(1.0, 2.0));
        buffer.clear();
        assert!(buffer.is_empty());
    }

    //--- Memory Retention Test ------------------------------------------
    //
    // Ensures that calling `clear()` does not deallocate the underlying
    // memory of either `Vec` or `HashMap`. This allows for efficient
    // reuse of the buffer across multiple frames without reallocations.
    //
    #[test]
    fn test_clear_does_not_deallocate() {
        let mut buffer = InputBuffer::new();

        // Fill with some data to trigger internal allocation
        for i in 0..2048 {
            buffer.push_discrete(make_key_down(KeyCode::Unidentified));
        }

        for i in 0..128 {
            buffer.push_continuous(make_mouse_move(i as f32, i as f32));
        }

        // Capture capacities before clearing
        let vec_capacity_before = buffer.discrete.capacity();
        let map_capacity_before = buffer.continuous.capacity();

        // Clear the buffer
        buffer.clear();

        // After clearing, lengths must be zero
        assert_eq!(buffer.discrete.len(), 0);
        assert_eq!(buffer.continuous.len(), 0);

        // But capacities must remain unchanged
        assert_eq!(buffer.discrete.capacity(), vec_capacity_before);
        assert_eq!(buffer.continuous.capacity(), map_capacity_before);
    }
}