//=========================================================================
// Platform Event Mapper
//
// Converts Winit input events to engine-level `RawInputEvent` types.
// Provides a clean separation between OS-specific input and the
// engine’s internal event representation.
//
// Responsibilities:
// - Translate keyboard and mouse events
// - Ignore unsupported or irrelevant Winit events
// - Provide fallbacks (`Unidentified`) for unmapped inputs
//
//=========================================================================

use winit::event::{WindowEvent,KeyEvent, ElementState, MouseButton as WinitMouseButton};
use winit::keyboard::PhysicalKey;
use winit::keyboard::KeyCode as WinitKeyCode;

use crate::core::input::event::{RawInputEvent, KeyCode, MouseButton};

//=== Key Conversion ======================================================
//
// Maps `WinitKeyCode` values to the engine’s internal `KeyCode` enum.
// Only a subset of codes is supported; all others map to `Other(u16)`.
//

impl From<WinitKeyCode> for KeyCode {
    fn from(code: WinitKeyCode) -> Self {
        use WinitKeyCode::*;
        match code {
            //--- Numeric keys -----------------------------------------------------
            Digit0 => KeyCode::Digit0, Digit1 => KeyCode::Digit1,
            Digit2 => KeyCode::Digit2, Digit3 => KeyCode::Digit3,
            Digit4 => KeyCode::Digit4, Digit5 => KeyCode::Digit5,
            Digit6 => KeyCode::Digit6, Digit7 => KeyCode::Digit7,
            Digit8 => KeyCode::Digit8, Digit9 => KeyCode::Digit9,

            //--- Alphabetic keys --------------------------------------------------
            KeyA => KeyCode::KeyA, KeyB => KeyCode::KeyB, KeyC => KeyCode::KeyC,
            KeyD => KeyCode::KeyD, KeyE => KeyCode::KeyE, KeyF => KeyCode::KeyF,
            KeyG => KeyCode::KeyG, KeyH => KeyCode::KeyH, KeyI => KeyCode::KeyI,
            KeyJ => KeyCode::KeyJ, KeyK => KeyCode::KeyK, KeyL => KeyCode::KeyL,
            KeyM => KeyCode::KeyM, KeyN => KeyCode::KeyN, KeyO => KeyCode::KeyO,
            KeyP => KeyCode::KeyP, KeyQ => KeyCode::KeyQ, KeyR => KeyCode::KeyR,
            KeyS => KeyCode::KeyS, KeyT => KeyCode::KeyT, KeyU => KeyCode::KeyU,
            KeyV => KeyCode::KeyV, KeyW => KeyCode::KeyW, KeyX => KeyCode::KeyX,
            KeyY => KeyCode::KeyY, KeyZ => KeyCode::KeyZ,

            //--- Arrow keys -------------------------------------------------------
            ArrowDown => KeyCode::ArrowDown, ArrowLeft => KeyCode::ArrowLeft,
            ArrowRight => KeyCode::ArrowRight, ArrowUp => KeyCode::ArrowUp,

            //--- Fallback ---------------------------------------------------------
            _ => KeyCode::Unidentified
        }
    }
}

//=== Mouse Conversion ====================================================
//
// Maps Winit mouse button identifiers to internal mouse button types.
//

impl From<WinitMouseButton> for MouseButton {
    fn from(button: WinitMouseButton) -> Self {
        match button {
            WinitMouseButton::Left => MouseButton::Left,
            WinitMouseButton::Right => MouseButton::Right,
            WinitMouseButton::Middle => MouseButton::Middle,
            _ => MouseButton::Other
        }
    }
}

//=== Full Event Conversion ===============================================
//
// Converts full `WindowEvent` objects into `RawInputEvent`s. Unsupported
// events are converted into `RawInputEvent::Unidentified`.
//
// Notes:
// - `KeyboardInput` is translated into `KeyDown`/`KeyUp`.
// - `MouseInput` becomes `MouseButtonDown`/`MouseButtonUp`.
// - `CursorMoved` maps to `MouseMoved`.
// - Other Winit events are ignored for now.
//

impl From<WindowEvent> for RawInputEvent {
    fn from(win_event: WindowEvent) -> Self {
        match win_event {
            //--- Keyboard Input ------------------------------------------
            WindowEvent::KeyboardInput { event: KeyEvent {
                    physical_key,
                    state,
                    ..
                }, .. } => {

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
            WindowEvent::CursorMoved { position, .. } => {
                RawInputEvent::MouseMoved {
                    x: position.x as f32,
                    y: position.y as f32,
                }
            }

            //--- Unhandled Events ----------------------------------------
            _ => RawInputEvent::Unidentified,
        }
    }
}