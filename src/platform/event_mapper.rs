//=========================================================================
// Platform Event Mapper
//
// Converts Winit input events into engine-level `RawInputEvent` types,
// providing a clean separation between OS-specific input and the engine’s
// internal representation.
//
// Responsibilities:
// - Map keyboard and mouse input from Winit to engine enums
// - Provide safe fallbacks for unsupported or unknown inputs
// - Serve as the translation layer between Platform and InputSystem
//
//=========================================================================

//=== External Crates =====================================================
use winit::event::{WindowEvent, KeyEvent, ElementState, MouseButton as WinitMouseButton};
use winit::keyboard::{PhysicalKey, KeyCode as WinitKeyCode};

//=== Internal Modules ====================================================
use crate::core::input::event::{RawInputEvent, KeyCode, MouseButton};

//=========================================================================
// Key Conversion
//
// Maps `WinitKeyCode` values to the engine’s internal `KeyCode` enum.
// Unsupported codes are mapped to `KeyCode::Unidentified`.
//=========================================================================

impl From<WinitKeyCode> for KeyCode {
    fn from(code: WinitKeyCode) -> Self {
        use WinitKeyCode::*;
        match code {
            //--- Numeric keys ---------------------------------------------
            Digit0 => KeyCode::Digit0, Digit1 => KeyCode::Digit1,
            Digit2 => KeyCode::Digit2, Digit3 => KeyCode::Digit3,
            Digit4 => KeyCode::Digit4, Digit5 => KeyCode::Digit5,
            Digit6 => KeyCode::Digit6, Digit7 => KeyCode::Digit7,
            Digit8 => KeyCode::Digit8, Digit9 => KeyCode::Digit9,

            //--- Alphabetic keys ------------------------------------------
            KeyA => KeyCode::KeyA, KeyB => KeyCode::KeyB, KeyC => KeyCode::KeyC,
            KeyD => KeyCode::KeyD, KeyE => KeyCode::KeyE, KeyF => KeyCode::KeyF,
            KeyG => KeyCode::KeyG, KeyH => KeyCode::KeyH, KeyI => KeyCode::KeyI,
            KeyJ => KeyCode::KeyJ, KeyK => KeyCode::KeyK, KeyL => KeyCode::KeyL,
            KeyM => KeyCode::KeyM, KeyN => KeyCode::KeyN, KeyO => KeyCode::KeyO,
            KeyP => KeyCode::KeyP, KeyQ => KeyCode::KeyQ, KeyR => KeyCode::KeyR,
            KeyS => KeyCode::KeyS, KeyT => KeyCode::KeyT, KeyU => KeyCode::KeyU,
            KeyV => KeyCode::KeyV, KeyW => KeyCode::KeyW, KeyX => KeyCode::KeyX,
            KeyY => KeyCode::KeyY, KeyZ => KeyCode::KeyZ,

            //--- Arrow keys -----------------------------------------------
            ArrowUp => KeyCode::ArrowUp,
            ArrowDown => KeyCode::ArrowDown,
            ArrowLeft => KeyCode::ArrowLeft,
            ArrowRight => KeyCode::ArrowRight,

            //--- Fallback -------------------------------------------------
            _ => KeyCode::Unidentified,
        }
    }
}

//=========================================================================
// Mouse Conversion
//
// Maps Winit mouse buttons to the engine’s internal `MouseButton` enum.
//=========================================================================

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
// Full Event Conversion
//
// Converts Winit `WindowEvent`s into engine-level `RawInputEvent`s.
//
// Behavior:
// - KeyboardInput → KeyDown / KeyUp
// - MouseInput → MouseButtonDown / MouseButtonUp
// - CursorMoved → MouseMoved
// - Others → Unidentified
//=========================================================================

impl From<WindowEvent> for RawInputEvent {
    fn from(win_event: WindowEvent) -> Self {
        match win_event {
            //--- Keyboard Input ------------------------------------------
            WindowEvent::KeyboardInput {
                event:
                KeyEvent {
                    physical_key,
                    state,
                    ..
                },
                ..
            } => {
                let key = match physical_key {
                    PhysicalKey::Code(code) => KeyCode::from(code),
                    _ => KeyCode::Unidentified,
                };

                match state {
                    ElementState::Pressed => RawInputEvent::KeyDown(key),
                    ElementState::Released => RawInputEvent::KeyUp(key),
                }
            }

            //--- Mouse Button Input --------------------------------------
            WindowEvent::MouseInput { state, button, .. } => {
                let btn = MouseButton::from(button);
                match state {
                    ElementState::Pressed => RawInputEvent::MouseButtonDown(btn),
                    ElementState::Released => RawInputEvent::MouseButtonUp(btn),
                }
            }

            //--- Mouse Movement ------------------------------------------
            WindowEvent::CursorMoved { position, .. } => RawInputEvent::MouseMoved {
                x: position.x as f32,
                y: position.y as f32,
            },

            //--- Unhandled / Irrelevant Events ----------------------------
            _ => RawInputEvent::Unidentified,
        }
    }
}
