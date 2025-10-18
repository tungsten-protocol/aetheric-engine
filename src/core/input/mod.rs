//=========================================================================
// Input System
//
// High-level interface for input handling within the engine.
// Wraps and manages the internal `InputState`, providing per-frame updates
// and query methods for gameplay, simulation, and UI layers.
//
// Responsibilities:
// - Aggregate and process batches of raw input events
// - Update the persistent `InputState` each frame
// - Expose high-level, read-only queries (keyboard, mouse, etc.)
//
// Notes:
// This system is owned and updated by the CoreSystemsOrchestrator.
// It provides a stable abstraction over low-level input event handling.
//
//=========================================================================

//=== Submodules ==========================================================
pub mod event;
mod input_state;

//=== Internal Imports ====================================================
use input_state::InputState;
use event::RawInputEvent;
use crate::core::input::event::{KeyCode, MouseButton};

//=== External Crates =====================================================
use log::info;

//=== InputSystem =========================================================
//
// Owns the engine's global input state and provides access to it.
// This is the public-facing API for querying user input.
//
pub struct InputSystem {
    input_state: InputState,
}

impl InputSystem {
    //--- Construction -----------------------------------------------------
    pub fn new() -> Self {
        Self {
            input_state: InputState::new(),
        }
    }

    //--- update() ---------------------------------------------------------
    //
    // Consumes all input batches received during the current frame,
    // updates the underlying `InputState`.
    //
    pub fn update(&mut self, input_batches: &mut Vec<Vec<RawInputEvent>>) {
        for batch in input_batches.drain(..) {
            self.input_state.digest_input_buffer(&batch);
        }

        if self.input_state.has_changed {
            info!("Input updated: {:?}", self.input_state);
            self.input_state.reset_changed();
        }
    }

    //--- Query Methods ----------------------------------------------------
    //
    // High-level API for accessing input state from gameplay or UI code.
    //

    /// Returns `true` if the specified key is currently pressed.
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.input_state
            .discrete
            .contains(&crate::core::input::input_state::DiscreteInput::Key(key))
    }

    /// Returns `true` if the specified mouse button is currently pressed.
    pub fn is_button_pressed(&self, btn: MouseButton) -> bool {
        self.input_state
            .discrete
            .contains(&crate::core::input::input_state::DiscreteInput::Button(btn))
    }

    /// Returns the current mouse position as `(x, y)`.
    pub fn mouse_position(&self) -> (f32, f32) {
        self.input_state.mouse
    }

    /// Returns whether the input state changed during the last update.
    pub fn has_changed(&self) -> bool {
        self.input_state.has_changed
    }
}

//=========================================================================
// Unit Tests
//=========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::input::event::{RawInputEvent, KeyCode, MouseButton};

    //--- Test Helpers -----------------------------------------------------
    fn key_down(key: KeyCode) -> RawInputEvent {
        RawInputEvent::KeyDown(key)
    }
    fn key_up(key: KeyCode) -> RawInputEvent {
        RawInputEvent::KeyUp(key)
    }
    fn mouse_down(btn: MouseButton) -> RawInputEvent {
        RawInputEvent::MouseButtonDown(btn)
    }
    fn mouse_up(btn: MouseButton) -> RawInputEvent {
        RawInputEvent::MouseButtonUp(btn)
    }
    fn mouse_move(x: f32, y: f32) -> RawInputEvent {
        RawInputEvent::MouseMoved { x, y }
    }

    //--- Tests ------------------------------------------------------------

    #[test]
    fn key_press_and_release_updates_state() {
        let mut system = InputSystem::new();

        let mut batches = vec![vec![key_down(KeyCode::KeyA)]];
        system.update(&mut batches);
        assert!(system.has_changed());
        assert!(system.is_key_pressed(KeyCode::KeyA));

        let mut batches = vec![vec![key_up(KeyCode::KeyA)]];
        system.update(&mut batches);
        assert!(system.has_changed());
        assert!(!system.is_key_pressed(KeyCode::KeyA));
    }

    #[test]
    fn mouse_button_press_and_release_updates_state() {
        let mut system = InputSystem::new();

        let mut batches = vec![vec![mouse_down(MouseButton::Left)]];
        system.update(&mut batches);
        assert!(system.has_changed());
        assert!(system.is_button_pressed(MouseButton::Left));

        let mut batches = vec![vec![mouse_up(MouseButton::Left)]];
        system.update(&mut batches);
        assert!(system.has_changed());
        assert!(!system.is_button_pressed(MouseButton::Left));
    }

    #[test]
    fn mouse_movement_updates_position() {
        let mut system = InputSystem::new();

        let mut batches = vec![vec![mouse_move(100.0, 200.0)]];
        system.update(&mut batches);
        assert!(system.has_changed());
        assert_eq!(system.mouse_position(), (100.0, 200.0));
    }
}
