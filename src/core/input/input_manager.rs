//=========================================================================
// Input Manager
//
// Ingest normalized `RawInputEvent` data from the platform or input into persistent states.
//
// Responsibilities:
// - Maintain discrete input states (pressed / released via presence in set)
// - Track continuous inputs (mouse position, analog movement, etc.)
// - Detect per-frame state changes for efficient updates
// - Provide high-level queries (`is_key_pressed`, `is_button_pressed`, etc.)
//
// The `InputManager` is designed to live at the engine layer, consuming
// sanitized events from the `Platform` subsystem.
// It exposes read-only query methods for gameplay, GUI, and other subsystems.
//
//=========================================================================

use std::collections::HashSet;
use log::warn;
use std::fmt;
use crate::core::input::event::{KeyCode, MouseButton, RawInputEvent};

//=== DiscreteInput ========================================================
//
// Represents a binary input element such as a keyboard key or mouse button.
// Presence in the `discrete` set indicates a pressed state.
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DiscreteInput {
    Key(KeyCode),
    Button(MouseButton),
}

//=== InputStates ==========================================================
//
// Internal structure holding all input state for the current frame.
//
// Fields:
// - `discrete`: set of active (pressed) discrete inputs
// - `mouse`: absolute mouse position (in pixels or normalized coords)
// - `has_changed`: true if any input state changed this frame
//
struct InputStates {
    discrete: HashSet<DiscreteInput>,
    mouse: (f32, f32),
    has_changed: bool,
}

impl InputStates {
    //--- Constructor ------------------------------------------------------
    //
    // Initializes an empty input state container.
    //
    pub fn new() -> Self {
        Self {
            discrete: HashSet::new(),
            mouse: (0.0, 0.0),
            has_changed: false,
        }
    }

    //--- press_discrete() -------------------------------------------------
    //
    // Marks a discrete input as pressed.
    // Returns `true` if the input was newly added to the set.
    //
    fn press_discrete(&mut self, input: DiscreteInput) -> bool {
        self.discrete.insert(input)
    }

    //--- release_discrete() ----------------------------------------------
    //
    // Marks a discrete input as released.
    // Returns `true` if the input was previously in the set.
    //
    fn release_discrete(&mut self, input: DiscreteInput) -> bool {
        self.discrete.remove(&input)
    }
}

//=== InputManager =========================================================
//
// Engine-level input interface.
// Wraps `InputStates` and updates it each frame based on `RawInputEvent`s.
//
pub struct InputManager {
    input_states: InputStates,
}

impl InputManager {
    //--- Constructor ------------------------------------------------------
    //
    // Creates a new, empty `InputManager`.
    //
    pub fn new() -> Self {
        InputManager {
            input_states: InputStates::new(),
        }
    }

    //--- digest_input_buffer() -------------------------------------------
    //
    // Consumes all `RawInputEvent`s for the current frame and updates
    // the internal input state accordingly.
    //
    // Discrete inputs (keys, buttons) are deduplicated by the `HashSet`,
    // which only records state changes. Continuous inputs such as
    // mouse movement always flag a change, as their values are variable
    // by definition.
    //
    pub fn digest_input_buffer(&mut self, event_buffer: Vec<RawInputEvent>) {
        if event_buffer.is_empty() {
            self.input_states.has_changed = false;
            return;
        }

        let mut changed = false;

        for event in &event_buffer {
            match *event {
                //--- Discrete Inputs -------------------------------------
                RawInputEvent::KeyDown(key) => {
                    changed |= self.input_states.press_discrete(DiscreteInput::Key(key));
                }
                RawInputEvent::KeyUp(key) => {
                    changed |= self.input_states.release_discrete(DiscreteInput::Key(key));
                }
                RawInputEvent::MouseButtonDown(btn) => {
                    changed |= self.input_states.press_discrete(DiscreteInput::Button(btn));
                }
                RawInputEvent::MouseButtonUp(btn) => {
                    changed |= self.input_states.release_discrete(DiscreteInput::Button(btn));
                }

                //--- Continuous Inputs -----------------------------------
                RawInputEvent::MouseMoved { x, y } => {
                    self.input_states.mouse = (x, y);
                    changed = true; // continuous inputs always imply change
                }

                _ => warn!("Unhandled input event: {:?}", event),
            }

            self.input_states.has_changed = changed;
        }
    }

    //--- Query Methods ----------------------------------------------------
    //
    // Provides high-level accessors for game and UI logic.
    //
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.input_states.discrete.contains(&DiscreteInput::Key(key))
    }

    pub fn is_button_pressed(&self, btn: MouseButton) -> bool {
        self.input_states.discrete.contains(&DiscreteInput::Button(btn))
    }

    pub fn mouse_position(&self) -> (f32, f32) {
        self.input_states.mouse
    }

    pub fn has_changed(&self) -> bool {
        self.input_states.has_changed
    }
}

//=== Debug Trait ==========================================================
//
// Custom `Debug` implementation that prints only relevant state information.
//
// Example output:
//
// ```text
// InputManager {
//     mouse: (420.0, 255.0),
//     has_changed: true,
//     pressed: ["Key(W)", "Button(Left)"]
// }
// ```
//
impl fmt::Debug for InputManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pressed: Vec<_> = self
            .input_states
            .discrete
            .iter()
            .map(|k| format!("{:?}", k))
            .collect();

        f.debug_struct("InputManager")
            .field("mouse", &self.input_states.mouse)
            .field("has_changed", &self.input_states.has_changed)
            .field("pressed", &pressed)
            .finish()
    }
}

//=========================================================================
// Unit Tests
//=========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::input::event::{RawInputEvent, KeyCode, MouseButton};

    fn mk_key_down(key: KeyCode) -> RawInputEvent {
        RawInputEvent::KeyDown(key)
    }

    fn mk_key_up(key: KeyCode) -> RawInputEvent {
        RawInputEvent::KeyUp(key)
    }

    fn mk_mouse_btn_down(btn: MouseButton) -> RawInputEvent {
        RawInputEvent::MouseButtonDown(btn)
    }

    fn mk_mouse_btn_up(btn: MouseButton) -> RawInputEvent {
        RawInputEvent::MouseButtonUp(btn)
    }

    fn mk_mouse_move(x: f32, y: f32) -> RawInputEvent {
        RawInputEvent::MouseMoved { x, y }
    }

    #[test]
    fn test_press_and_release_key() {
        let mut im = InputManager::new();

        assert!(!im.has_changed());

        im.digest_input_buffer(vec![mk_key_down(KeyCode::KeyA)]);
        assert!(im.has_changed());
        assert!(im.is_key_pressed(KeyCode::KeyA));

        im.digest_input_buffer(vec![]);
        assert!(!im.has_changed());
        assert!(im.is_key_pressed(KeyCode::KeyA));

        im.digest_input_buffer(vec![mk_key_up(KeyCode::KeyA)]);
        assert!(im.has_changed());
        assert!(!im.is_key_pressed(KeyCode::KeyA));
    }

    #[test]
    fn test_press_same_key_again_no_change() {
        let mut im = InputManager::new();

        im.digest_input_buffer(vec![mk_key_down(KeyCode::KeyA)]);
        assert!(im.has_changed());

        im.digest_input_buffer(vec![mk_key_down(KeyCode::KeyA)]);
        assert!(!im.has_changed());
        assert!(im.is_key_pressed(KeyCode::KeyA));
    }

    #[test]
    fn test_mouse_move_always_changes() {
        let mut im = InputManager::new();

        // Muovo il mouse
        im.digest_input_buffer(vec![mk_mouse_move(100.0, 200.0)]);
        assert!(im.has_changed());
        assert_eq!(im.mouse_position(), (100.0, 200.0));

        // Muovo ancora a coordinate diverse
        im.digest_input_buffer(vec![mk_mouse_move(150.0, 250.0)]);
        assert!(im.has_changed());
        assert_eq!(im.mouse_position(), (150.0, 250.0));
    }

    #[test]
    fn test_button_press_and_release() {
        let mut im = InputManager::new();

        im.digest_input_buffer(vec![mk_mouse_btn_down(MouseButton::Left)]);
        assert!(im.has_changed());
        assert!(im.is_button_pressed(MouseButton::Left));

        im.digest_input_buffer(vec![mk_mouse_btn_up(MouseButton::Left)]);
        assert!(im.has_changed());
        assert!(!im.is_button_pressed(MouseButton::Left));
    }
}
