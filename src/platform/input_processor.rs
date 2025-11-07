//=========================================================================
// Input Processor
//=========================================================================
//
// Converts platform-specific Winit events into engine InputEvents.
//
// Architecture:
//   Winit Events → InputProcessor → InputEvent (engine type) → InputBuffer
//
// Stateful modifier tracking: Caches modifier state from ModifiersChanged
// events and applies to all subsequent key/mouse events. Unmapped keys
// (F13-F24, exotic keyboards) are filtered (returns None).
//
//=========================================================================

//=== External Dependencies ===============================================

use winit::{
    event::ElementState,
    event::{KeyEvent, MouseButton as WinitMouseButton},
    keyboard::{KeyCode as WinitKeyCode, ModifiersState, PhysicalKey},
};

//=== Internal Dependencies ===============================================

use crate::core::input::event::{InputEvent, KeyCode, Modifiers, MouseButton};

//=== InputProcessor ======================================================

/// Converts Winit events to engine InputEvents with stateful modifier tracking.
///
/// Filters unmapped keys and applies cached modifier state to all events.
pub(crate) struct InputProcessor {
    current_modifiers: Modifiers,
}

impl InputProcessor {
    //--- Construction -----------------------------------------------------

    pub(crate) fn new() -> Self {
        Self {
            current_modifiers: Modifiers::NONE,
        }
    }

    //--- Modifier State Management ----------------------------------------

    /// Updates cached modifier state (applied to subsequent events).
    pub(crate) fn update_modifiers(&mut self, modifiers_state: ModifiersState) {
        self.current_modifiers = Modifiers::from(modifiers_state);
    }

    pub(crate) fn current_modifiers(&self) -> Modifiers {
        self.current_modifiers
    }

    //--- Event Processing -------------------------------------------------

    /// Converts Winit KeyEvent to InputEvent (filters unmapped keys).
    pub(crate) fn process_key_event(&self, key_event: &KeyEvent) -> Option<InputEvent> {
        let key_code = match key_event.physical_key {
            PhysicalKey::Code(code) => KeyCode::from(code),
            _ => return None,
        };

        if matches!(key_code, KeyCode::Unidentified) {
            return None;
        }

        Some(self.create_key_input_event(key_code, key_event.state))
    }

    /// Converts Winit mouse button event to InputEvent (with modifiers).
    pub(crate) fn process_mouse_button(
        &self,
        button: WinitMouseButton,
        state: ElementState,
    ) -> InputEvent {
        let mouse_button = MouseButton::from(button);

        match state {
            ElementState::Pressed => InputEvent::MouseButtonDown {
                button: mouse_button,
                modifiers: self.current_modifiers,
            },
            ElementState::Released => InputEvent::MouseButtonUp {
                button: mouse_button,
                modifiers: self.current_modifiers,
            },
        }
    }

    /// Creates a mouse move event (screen space, no modifiers).
    pub(crate) fn process_mouse_move(&self, x: f32, y: f32) -> InputEvent {
        InputEvent::MouseMoved { x, y }
    }

    //--- Internal Helpers -------------------------------------------------

    fn create_key_input_event(&self, key: KeyCode, state: ElementState) -> InputEvent {
        match state {
            ElementState::Pressed => InputEvent::KeyDown {
                key,
                modifiers: self.current_modifiers,
            },
            ElementState::Released => InputEvent::KeyUp {
                key,
                modifiers: self.current_modifiers,
            },
        }
    }
}

//=========================================================================
// Winit Conversions
//=========================================================================

/// Converts Winit ModifiersState to engine Modifiers.
///
/// Winit normalizes platform keys (macOS Cmd → Ctrl, Option → Alt).
impl From<ModifiersState> for Modifiers {
    fn from(state: ModifiersState) -> Self {
        Self {
            shift: state.shift_key(),
            ctrl: state.control_key(),
            alt: state.alt_key(),
        }
    }
}

/// Converts Winit physical key codes to engine key codes.
///
/// Maps A-Z, 0-9, arrows, and common special keys. Unmapped keys (F13-F24,
/// numpad, media keys) return `KeyCode::Unidentified`.
impl From<WinitKeyCode> for KeyCode {
    fn from(code: WinitKeyCode) -> Self {
        use WinitKeyCode::*;
        match code {
            //--- Digits -------------------------------------------------------

            Digit0 => KeyCode::Digit0,
            Digit1 => KeyCode::Digit1,
            Digit2 => KeyCode::Digit2,
            Digit3 => KeyCode::Digit3,
            Digit4 => KeyCode::Digit4,
            Digit5 => KeyCode::Digit5,
            Digit6 => KeyCode::Digit6,
            Digit7 => KeyCode::Digit7,
            Digit8 => KeyCode::Digit8,
            Digit9 => KeyCode::Digit9,

            //--- Letters ------------------------------------------------------

            KeyA => KeyCode::KeyA,
            KeyB => KeyCode::KeyB,
            KeyC => KeyCode::KeyC,
            KeyD => KeyCode::KeyD,
            KeyE => KeyCode::KeyE,
            KeyF => KeyCode::KeyF,
            KeyG => KeyCode::KeyG,
            KeyH => KeyCode::KeyH,
            KeyI => KeyCode::KeyI,
            KeyJ => KeyCode::KeyJ,
            KeyK => KeyCode::KeyK,
            KeyL => KeyCode::KeyL,
            KeyM => KeyCode::KeyM,
            KeyN => KeyCode::KeyN,
            KeyO => KeyCode::KeyO,
            KeyP => KeyCode::KeyP,
            KeyQ => KeyCode::KeyQ,
            KeyR => KeyCode::KeyR,
            KeyS => KeyCode::KeyS,
            KeyT => KeyCode::KeyT,
            KeyU => KeyCode::KeyU,
            KeyV => KeyCode::KeyV,
            KeyW => KeyCode::KeyW,
            KeyX => KeyCode::KeyX,
            KeyY => KeyCode::KeyY,
            KeyZ => KeyCode::KeyZ,

            //--- Arrows -------------------------------------------------------

            ArrowUp => KeyCode::ArrowUp,
            ArrowDown => KeyCode::ArrowDown,
            ArrowLeft => KeyCode::ArrowLeft,
            ArrowRight => KeyCode::ArrowRight,

            //--- Special ------------------------------------------------------

            Space => KeyCode::Space,
            Enter => KeyCode::Enter,
            Escape => KeyCode::Escape,
            Tab => KeyCode::Tab,
            Backspace => KeyCode::Backspace,
            Delete => KeyCode::Delete,

            //--- Unmapped (return Unidentified) -------------------------------

            _ => KeyCode::Unidentified,
        }
    }
}

/// Converts Winit mouse buttons to engine buttons.
///
/// Left/Right/Middle mapped directly; Back/Forward/Other → Other.
impl From<WinitMouseButton> for MouseButton {
    fn from(button: WinitMouseButton) -> Self {
        match button {
            WinitMouseButton::Left => MouseButton::Left,
            WinitMouseButton::Right => MouseButton::Right,
            WinitMouseButton::Middle => MouseButton::Middle,
            _ => MouseButton::Other,
        }
    }
}

//=========================================================================
// Tests
//=========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::KeyCode as WinitKeyCode;

    fn make_modifiers(shift: bool, ctrl: bool, alt: bool) -> ModifiersState {
        let mut state = ModifiersState::empty();
        if shift { state.insert(ModifiersState::SHIFT); }
        if ctrl { state.insert(ModifiersState::CONTROL); }
        if alt { state.insert(ModifiersState::ALT); }
        state
    }

    #[test]
    fn starts_with_no_modifiers() {
        let processor = InputProcessor::new();
        let mods = processor.current_modifiers();
        assert!(!mods.shift && !mods.ctrl && !mods.alt);
    }

    #[test]
    fn update_modifiers_works() {
        let mut processor = InputProcessor::new();
        processor.update_modifiers(make_modifiers(true, false, true));

        let mods = processor.current_modifiers();
        assert!(mods.shift && !mods.ctrl && mods.alt);
    }

    #[test]
    fn create_key_down_event_with_modifiers() {
        let mut processor = InputProcessor::new();
        processor.update_modifiers(make_modifiers(false, true, false));

        let event = processor.create_key_input_event(
            KeyCode::KeyS,
            ElementState::Pressed,
        );

        match event {
            InputEvent::KeyDown { key, modifiers } => {
                assert_eq!(key, KeyCode::KeyS);
                assert!(modifiers.ctrl);
                assert!(!modifiers.shift);
            }
            _ => panic!("Expected KeyDown"),
        }
    }

    #[test]
    fn create_key_up_event_with_modifiers() {
        let mut processor = InputProcessor::new();
        processor.update_modifiers(make_modifiers(true, true, false));

        let event = processor.create_key_input_event(
            KeyCode::KeyA,
            ElementState::Released,
        );

        match event {
            InputEvent::KeyUp { key, modifiers } => {
                assert_eq!(key, KeyCode::KeyA);
                assert!(modifiers.shift);
                assert!(modifiers.ctrl);
            }
            _ => panic!("Expected KeyUp"),
        }
    }

    #[test]
    fn keycode_conversion_filters_unidentified() {
        // Test conversion directly
        let unidentified = KeyCode::from(WinitKeyCode::F13);
        assert!(matches!(unidentified, KeyCode::Unidentified));
    }

    #[test]
    fn mouse_button_has_modifiers() {
        let mut processor = InputProcessor::new();
        processor.update_modifiers(make_modifiers(false, false, true));

        let event = processor.process_mouse_button(
            WinitMouseButton::Left,
            ElementState::Pressed,
        );

        match event {
            InputEvent::MouseButtonDown { button, modifiers } => {
                assert_eq!(button, MouseButton::Left);
                assert!(modifiers.alt);
            }
            _ => panic!("Expected MouseButtonDown"),
        }
    }

    #[test]
    fn mouse_move_correct() {
        let processor = InputProcessor::new();
        let event = processor.process_mouse_move(123.5, 456.7);

        match event {
            InputEvent::MouseMoved { x, y } => {
                assert_eq!(x, 123.5);
                assert_eq!(y, 456.7);
            }
            _ => panic!("Expected MouseMoved"),
        }
    }

    #[test]
    fn modifiers_persist_across_events() {
        let mut processor = InputProcessor::new();
        processor.update_modifiers(make_modifiers(true, false, false));

        let event1 = processor.process_mouse_button(
            WinitMouseButton::Left,
            ElementState::Pressed,
        );

        let event2 = processor.create_key_input_event(
            KeyCode::Space,
            ElementState::Pressed,
        );

        // Both should have Shift
        match event1 {
            InputEvent::MouseButtonDown { modifiers, .. } => {
                assert!(modifiers.shift)
            }
            _ => panic!(),
        }
        match event2 {
            InputEvent::KeyDown { modifiers, .. } => {
                assert!(modifiers.shift)
            }
            _ => panic!(),
        }
    }

    #[test]
    fn keycode_conversion_alphabetic() {
        assert_eq!(KeyCode::from(WinitKeyCode::KeyA), KeyCode::KeyA);
        assert_eq!(KeyCode::from(WinitKeyCode::KeyZ), KeyCode::KeyZ);
    }

    #[test]
    fn keycode_conversion_special() {
        assert_eq!(KeyCode::from(WinitKeyCode::Space), KeyCode::Space);
        assert_eq!(KeyCode::from(WinitKeyCode::Enter), KeyCode::Enter);
    }

    #[test]
    fn mouse_button_conversion() {
        assert_eq!(MouseButton::from(WinitMouseButton::Left), MouseButton::Left);
        assert_eq!(MouseButton::from(WinitMouseButton::Right), MouseButton::Right);
        assert_eq!(MouseButton::from(WinitMouseButton::Middle), MouseButton::Middle);
    }
}