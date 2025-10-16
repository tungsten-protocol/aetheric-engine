//=========================================================================
// System Event Types
//
// Defines the internal representation of low-level input events.
// This module abstracts away platform-specific input (e.g. Winit) into
// a unified, engine-friendly format used by the input subsystem.
//
// Responsibilities:
// - Represent keyboard and mouse inputs in a stable, portable way
// - Provide event categorization (RawInputEventKind)
// - Allow deduplication and normalization across frames
//
//=========================================================================

use std::hash::{Hash, Hasher};

//=== MouseButton Enum ====================================================
// Represents a physical mouse button.
// Used to identify which button triggered an event.
//
// This abstraction allows the engine to stay independent of
// the underlying platform or library (e.g., Winit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other
}

//=== KeyCode Enum ========================================================
// Represents a physical keyboard key in a simplified,
// cross-platform form.
//
// Only the most common alphanumeric and directional keys
// are included for now — additional codes can be added
// as needed by the input mapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode{
    //--- Numeric keys -----------------------------------------------------
    Digit0, Digit1, Digit2, Digit3, Digit4,
    Digit5, Digit6, Digit7, Digit8, Digit9,

    //--- Alphabetic keys --------------------------------------------------
    KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI,
    KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ, KeyR,
    KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,

    //--- Arrow keys -------------------------------------------------------
    ArrowDown, ArrowLeft, ArrowRight, ArrowUp,

    //--- Fallback ---------------------------------------------------------
    // Used for keys not mapped explicitly by the input layer.
    Unidentified
}

//=== RawInputEvent Enum ========================================================
// Represents a concrete input event as normalized by the platform layer.
//
// Each variant carries the relevant data payload — for example,
// mouse coordinates for `MouseMoved`, or a `KeyCode` for `KeyDown`.
#[derive(Debug, Clone)]
pub enum RawInputEvent {
    KeyDown(KeyCode),
    KeyUp(KeyCode),
    MouseButtonDown(MouseButton),
    MouseButtonUp(MouseButton),
    MouseMoved { x: f32, y: f32 },
    Unidentified
}

//=========================================================================
// Equality and Hashing
//
// Events are compared and hashed *by type*, not by payload.
// This allows deduplication of repeated discrete inputs and
// coalescing of continuous events regardless of their data.
//=========================================================================
impl PartialEq for RawInputEvent {
    fn eq(&self, other: &Self) -> bool {
        use RawInputEvent::*;
        match (self, other) {
            (KeyDown(a), KeyDown(b)) => a == b,
            (KeyUp(a), KeyUp(b)) => a == b,
            (MouseButtonDown(a), MouseButtonDown(b)) => a == b,
            (MouseButtonUp(a), MouseButtonUp(b)) => a == b,
            (MouseMoved { .. }, MouseMoved { .. }) => true,
            (Unidentified, Unidentified) => true,
            _ => false,
        }
    }
}
impl Eq for RawInputEvent {}

impl Hash for RawInputEvent {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash only the event *type*, not the data payload
        std::mem::discriminant(self).hash(state);
    }
}

//=========================================================================
// Unit Tests
//=========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    //--- Utility: compute hash -------------------------------------------
    fn hash_of<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    //=====================================================================
    // Equality Tests
    //=====================================================================

    #[test]
    fn equality_same_type_same_data() {
        let a = RawInputEvent::KeyDown(KeyCode::KeyA);
        let b = RawInputEvent::KeyDown(KeyCode::KeyA);
        assert_eq!(a, b, "Two identical KeyDown(KeyA) events should be equal");
    }

    #[test]
    fn equality_same_type_different_data() {
        let a = RawInputEvent::MouseMoved { x: 10.0, y: 10.0 };
        let b = RawInputEvent::MouseMoved { x: 200.0, y: 300.0 };
        assert_eq!(a, b, "MouseMoved events should be equal regardless of coordinates");
    }

    #[test]
    fn equality_different_type() {
        let a = RawInputEvent::KeyDown(KeyCode::KeyA);
        let b = RawInputEvent::KeyUp(KeyCode::KeyA);
        assert_ne!(a, b, "KeyDown(KeyA) and KeyUp(KeyA) must not be equal");
    }

    #[test]
    fn equality_mouse_button_same_button() {
        let a = RawInputEvent::MouseButtonDown(MouseButton::Left);
        let b = RawInputEvent::MouseButtonDown(MouseButton::Left);
        assert_eq!(a, b, "Two identical MouseButtonDown(Left) should be equal");
    }

    #[test]
    fn equality_mouse_button_different_button() {
        let a = RawInputEvent::MouseButtonDown(MouseButton::Left);
        let b = RawInputEvent::MouseButtonDown(MouseButton::Right);
        assert_ne!(a, b, "MouseButtonDown(Left) and MouseButtonDown(Right) should differ");
    }

    #[test]
    fn equality_unidentified() {
        let a = RawInputEvent::Unidentified;
        let b = RawInputEvent::Unidentified;
        assert_eq!(a, b, "Unidentified events should always be equal");
    }

    //=====================================================================
    // Hashing Tests
    //=====================================================================

    #[test]
    fn hash_same_type_same_hash() {
        let a = RawInputEvent::KeyDown(KeyCode::KeyA);
        let b = RawInputEvent::KeyDown(KeyCode::KeyB);
        assert_eq!(
            hash_of(&a),
            hash_of(&b),
            "All KeyDown events should have identical hash (type-based)"
        );
    }

    #[test]
    fn hash_different_type_different_hash() {
        let a = RawInputEvent::KeyDown(KeyCode::KeyA);
        let b = RawInputEvent::KeyUp(KeyCode::KeyA);
        assert_ne!(
            hash_of(&a),
            hash_of(&b),
            "Different event types must yield different hashes"
        );
    }

    #[test]
    fn hash_mousemove_stability() {
        let a = RawInputEvent::MouseMoved { x: 1.0, y: 2.0 };
        let b = RawInputEvent::MouseMoved { x: 300.0, y: 400.0 };
        assert_eq!(
            hash_of(&a),
            hash_of(&b),
            "MouseMoved events should produce identical hashes regardless of coordinates"
        );
    }
}