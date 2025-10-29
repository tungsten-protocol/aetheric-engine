//=========================================================================
// Input Processor
//
// Converts platform-specific Winit events into engine InputEvents.
//
// Data Flow:
// ```text
//  Winit Event Loop
//       ↓
//  ┌────────────────────────────────┐
//  │  InputProcessor                │
//  │  ┌──────────────────────────┐  │
//  │  │ current_modifiers        │  │ ← Updated on ModifiersChanged
//  │  └──────────────────────────┘  │
//  │           ↓                    │
//  │  ┌──────────────────────────┐  │
//  │  │ Conversion Methods       │  │
//  │  ├─ process_key_event()     │  │ → Option<InputEvent>
//  │  ├─ process_mouse_button()  │  │ → InputEvent
//  │  └─ process_mouse_move()    │  │ → InputEvent
//  │           ↓                    │
//  └───────────┼────────────────────┘
//              ↓
//       InputEvent (engine type)
//              ↓
//         InputBuffer
//
// Modifier State Management:
//   1. ModifiersChanged event arrives
//   2. update_modifiers() stores new state
//   3. All subsequent events include this state
//   4. Modifiers "stick" until next change
//
// Example:
//   Winit: ModifiersChanged{CTRL}
//   → processor.update_modifiers(CTRL)
//   Winit: KeyboardInput{KeyS}
//   → processor.process_key_event()
//   → Result: KeyDown{key: S, modifiers: CTRL}
// ```
//
// Key Design Decisions:
// - **Stateful modifiers**: Winit sends separate ModifiersChanged events,
//   so we cache state rather than querying per-event
// - **Unidentified filtering**: Returns None for unmapped keys (F13-F24,
//   exotic keyboards) to reduce noise in the input system
// - **Pure functions**: All methods are deterministic given inputs + state,
//   enabling comprehensive unit tests without Winit event loop
// - **Platform normalization**: Winit handles OS differences (macOS Cmd → Ctrl),
//   so conversions are straightforward
//
// Responsibilities:
// - Convert Winit types → engine types (KeyCode, MouseButton, Modifiers)
// - Apply current modifier state to all key/button events
// - Filter out unrecognized keys (returns None)
// - Normalize platform differences via Winit abstractions
//
//=========================================================================

//=== External Crates =====================================================

use winit::{
    event::ElementState,
    keyboard::{KeyCode as WinitKeyCode, ModifiersState, PhysicalKey},
    event::{KeyEvent, MouseButton as WinitMouseButton},
};
use winit::keyboard::KeyCode::{ArrowDown, ArrowLeft, ArrowRight, ArrowUp, Backspace, Delete, Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9, Enter, Escape, KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI, KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ, Space, Tab};
//=== Internal Imports ====================================================

use crate::core::input::event::{KeyCode, Modifiers, MouseButton, InputEvent};

//=== InputProcessor ======================================================

/// Converts platform-specific Winit events into engine InputEvents.
///
/// This processor maintains stateful modifier tracking (Shift, Ctrl, Alt)
/// and applies the current modifier state to all events.
///
/// # Modifier State Management
///
/// Winit sends a separate `ModifiersChanged` event when modifier keys are
/// pressed/released. The processor caches this state and applies it to
/// subsequent key/mouse events automatically:
///
/// ```ignore
/// // Platform sends: ModifiersChanged{CTRL}
/// processor.update_modifiers(ModifiersState::CONTROL);
///
/// // Platform sends: KeyboardInput{KeyS}
/// let event = processor.process_key_event(key_s);
/// // Result: KeyDown{key: S, modifiers: CTRL}
/// ```
///
/// This "sticky modifiers" approach is necessary because Winit doesn't
/// include modifier state in every event. The state persists until
/// explicitly updated by another `ModifiersChanged` event.
///
/// # Key Filtering
///
/// Unrecognized keys (F13-F24, exotic keyboards, etc.) are filtered:
/// - [`process_key_event`](Self::process_key_event) returns `None` for unmapped keys
/// - Reduces noise in the input system
/// - Game code doesn't need to handle unknown keys
///
/// Mapped keys include: A-Z, 0-9, arrows, Space, Enter, Escape, Tab, Backspace, Delete
///
/// # Thread Safety
///
/// This type is NOT Send/Sync (contains no synchronization). It's used
/// exclusively on the platform thread (main thread on macOS/iOS).
///
/// # Examples
///
/// Basic usage in platform layer:
/// ```ignore
/// use winit::keyboard::ModifiersState;
///
/// let mut processor = InputProcessor::new();
///
/// // Update modifiers from Winit event
/// processor.update_modifiers(ModifiersState::CONTROL);
///
/// // Process key event
/// if let Some(event) = processor.process_key_event(&key_event) {
///     buffer.push_discrete(event);
/// }
///
/// // Process mouse button
/// let event = processor.process_mouse_button(button, state);
/// buffer.push_discrete(event);
///
/// // Process mouse movement
/// let event = processor.process_mouse_move(x, y);
/// buffer.push_continuous(event);
/// ```
pub(crate) struct InputProcessor {
    /// Current modifier key state (updated on ModifiersChanged).
    ///
    /// Applied to all subsequent events until next update.
    /// Default: `Modifiers::NONE` (all flags false).
    current_modifiers: Modifiers,
}

impl InputProcessor {
    //--- Construction -----------------------------------------------------

    /// Creates a new processor with no modifiers active.
    ///
    /// Initial state: `Modifiers::NONE` (shift=false, ctrl=false, alt=false).
    pub(crate) fn new() -> Self {
        Self {
            current_modifiers: Modifiers::NONE,
        }
    }

    //--- Modifier State Management ----------------------------------------

    /// Updates the cached modifier state from Winit.
    ///
    /// Called when `ModifiersChanged` event is received. The new state will
    /// be applied to all subsequent key/mouse events until next update.
    ///
    /// # Platform Behavior
    ///
    /// - **Windows/Linux**: Shift, Ctrl, Alt map directly
    /// - **macOS**: Command → Ctrl, Option → Alt
    ///
    /// Winit handles platform normalization, so this method just stores
    /// the provided state.
    ///
    /// # Arguments
    ///
    /// - `modifiers_state`: Current state from Winit's ModifiersChanged event
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use winit::keyboard::ModifiersState;
    ///
    /// let mut processor = InputProcessor::new();
    ///
    /// // User presses Ctrl
    /// processor.update_modifiers(ModifiersState::CONTROL);
    ///
    /// // All subsequent events will have Ctrl applied
    /// assert!(processor.current_modifiers().ctrl);
    /// ```
    pub(crate) fn update_modifiers(&mut self, modifiers_state: ModifiersState) {
        self.current_modifiers = Modifiers::from(modifiers_state);
    }

    /// Returns the current modifier state.
    ///
    /// Reflects the most recent `ModifiersChanged` event received from Winit.
    /// Used for queries or debugging.
    pub(crate) fn current_modifiers(&self) -> Modifiers {
        self.current_modifiers
    }

    //--- Event Processing -------------------------------------------------

    /// Converts a Winit `KeyEvent` to an engine `InputEvent`.
    ///
    /// Returns `None` if:
    /// - The key is not a physical key code (e.g., Unidentified)
    /// - The key code is not mapped (e.g., F13-F24, exotic keys)
    ///
    /// Mapped keys include:
    /// - Digits: 0-9
    /// - Letters: A-Z (physical location)
    /// - Arrows: Up, Down, Left, Right
    /// - Special: Space, Enter, Escape, Tab, Backspace, Delete
    ///
    /// # Current Modifiers
    ///
    /// The event will automatically include the current modifier state
    /// (from the most recent [`update_modifiers`](Self::update_modifiers) call).
    ///
    /// # Arguments
    ///
    /// - `key_event`: Keyboard event from Winit
    ///
    /// # Returns
    ///
    /// - `Some(InputEvent)`: Mapped key with current modifiers applied
    /// - `None`: Unrecognized or unmapped key (filtered)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Recognized key → Some
    /// let event = processor.process_key_event(&key_event_a);
    /// assert!(event.is_some());
    ///
    /// // Unrecognized key (F13) → None
    /// let event = processor.process_key_event(&key_event_f13);
    /// assert!(event.is_none());
    /// ```
    pub(crate) fn process_key_event(&self, key_event: &KeyEvent) -> Option<InputEvent> {
        let key_code = match key_event.physical_key {
            PhysicalKey::Code(code) => KeyCode::from(code),
            _ => return None, // Not a physical key (e.g., Dead, Unidentified)
        };

        if matches!(key_code, KeyCode::Unidentified) {
            return None; // Unmapped key (filtered)
        }

        Some(self.create_key_input_event(key_code, key_event.state))
    }

    /// Converts a Winit mouse button event to an engine `InputEvent`.
    ///
    /// Always returns a valid event (no filtering). Current modifiers are
    /// applied automatically, enabling modifier+click bindings (Ctrl+Click).
    ///
    /// # Supported Buttons
    ///
    /// - Left, Right, Middle: Mapped directly
    /// - Back, Forward, Other: Mapped to `MouseButton::Other`
    ///
    /// # Arguments
    ///
    /// - `button`: Mouse button from Winit
    /// - `state`: Pressed or Released
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use winit::event::{MouseButton, ElementState};
    ///
    /// // Simple click
    /// let event = processor.process_mouse_button(
    ///     MouseButton::Left,
    ///     ElementState::Pressed
    /// );
    /// // Result: MouseButtonDown{Left, modifiers: NONE}
    ///
    /// // With modifiers
    /// processor.update_modifiers(ModifiersState::CONTROL);
    /// let event = processor.process_mouse_button(
    ///     MouseButton::Left,
    ///     ElementState::Pressed
    /// );
    /// // Result: MouseButtonDown{Left, modifiers: CTRL}
    /// ```
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

    /// Creates a mouse move event.
    ///
    /// Mouse movement does NOT include modifiers (not semantically meaningful
    /// for cursor position). For modifier-sensitive mouse input, use
    /// [`process_mouse_button`](Self::process_mouse_button) with click events.
    ///
    /// # Coordinate System
    ///
    /// Coordinates are in screen space:
    /// - Origin: Top-left corner (0, 0)
    /// - X-axis: Increases rightward
    /// - Y-axis: Increases downward
    /// - Units: Physical pixels (may differ from logical pixels on HiDPI)
    ///
    /// # Arguments
    ///
    /// - `x`: Screen X coordinate (pixels)
    /// - `y`: Screen Y coordinate (pixels)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let event = processor.process_mouse_move(100.5, 200.3);
    /// // Result: MouseMoved{x: 100.5, y: 200.3}
    /// ```
    pub(crate) fn process_mouse_move(&self, x: f32, y: f32) -> InputEvent {
        InputEvent::MouseMoved { x, y }
    }

    //--- Internal Helpers -------------------------------------------------

    /// Creates a key input event with current modifiers applied.
    ///
    /// Helper to avoid duplication between [`process_key_event`](Self::process_key_event)
    /// and test code. Not public API.
    ///
    /// # Arguments
    ///
    /// - `key`: Mapped engine key code
    /// - `state`: Pressed or Released
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

/// Converts Winit's `ModifiersState` to engine `Modifiers`.
///
/// Maps platform-specific modifier keys to engine representation:
/// - **Shift**: Left or Right Shift
/// - **Ctrl**: Left or Right Control (Command on macOS)
/// - **Alt**: Left or Right Alt (Option on macOS)
///
/// # Platform Normalization
///
/// Winit handles OS differences automatically:
/// - macOS Command → `ModifiersState::CONTROL`
/// - macOS Option → `ModifiersState::ALT`
///
/// This means the conversion is straightforward and games don't need
/// platform-specific logic for common shortcuts like Ctrl+S.
///
/// # Examples
///
/// ```ignore
/// use winit::keyboard::ModifiersState;
///
/// let state = ModifiersState::CONTROL | ModifiersState::SHIFT;
/// let mods = Modifiers::from(state);
///
/// assert!(mods.ctrl);
/// assert!(mods.shift);
/// assert!(!mods.alt);
/// ```
impl From<ModifiersState> for Modifiers {
    fn from(state: ModifiersState) -> Self {
        Self {
            shift: state.shift_key(),
            ctrl: state.control_key(),
            alt: state.alt_key(),
        }
    }
}

/// Converts Winit's physical key codes to engine key codes.
///
/// Maps commonly-used keys (alphanumeric, arrows, special keys) to engine
/// representation. Unmapped keys return `KeyCode::Unidentified` and are
/// filtered by [`InputProcessor::process_key_event`].
///
/// # Coverage
///
/// **Mapped keys** (60 total):
/// - **Digits**: 0-9 (number row)
/// - **Letters**: A-Z (physical location, not character)
/// - **Arrows**: Up, Down, Left, Right
/// - **Special**: Space, Enter, Escape, Tab, Backspace, Delete
///
/// **Unmapped keys** (return `Unidentified`):
/// - Function keys F13-F24
/// - Numpad keys (Numpad0-9, NumpadAdd, etc.)
/// - Media keys (AudioVolumeUp, MediaPlayPause, etc.)
/// - Exotic keys (IntlBackslash, Lang1-5, etc.)
///
/// # Physical vs Logical
///
/// This conversion uses **physical key codes**, meaning:
/// - `KeyA` is always the same physical key (QWERTY "A" position)
/// - AZERTY keyboards have "Q" character at `KeyA` position
/// - Games should use physical keys for movement (WASD)
/// - Use `KeyEvent::text` for text input (character-based)
///
/// # Extensibility
///
/// Games needing unmapped keys can:
/// 1. Fork and extend this conversion
/// 2. Use raw Winit events directly
/// 3. Request additional mappings upstream
///
/// # Examples
///
/// ```ignore
/// use winit::keyboard::KeyCode as WinitKeyCode;
///
/// // Mapped key
/// assert_eq!(KeyCode::from(WinitKeyCode::KeyA), KeyCode::KeyA);
///
/// // Unmapped key
/// assert_eq!(KeyCode::from(WinitKeyCode::F13), KeyCode::Unidentified);
/// ```
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

/// Converts Winit's mouse button types to engine button types.
///
/// # Mapping
///
/// - `Left` → `MouseButton::Left`
/// - `Right` → `MouseButton::Right`
/// - `Middle` → `MouseButton::Middle`
/// - `Back`, `Forward`, `Other` → `MouseButton::Other`
///
/// # Rationale
///
/// Only the three primary buttons (Left, Right, Middle) are mapped directly
/// since they're universally available. Side buttons (Back/Forward) vary
/// widely by hardware and driver, so they're grouped as `Other`.
///
/// Games needing precise side button control can extend this mapping or
/// use raw Winit events.
///
/// # Examples
///
/// ```ignore
/// use winit::event::MouseButton as WinitMouseButton;
///
/// // Primary buttons
/// assert_eq!(MouseButton::from(WinitMouseButton::Left), MouseButton::Left);
/// assert_eq!(MouseButton::from(WinitMouseButton::Middle), MouseButton::Middle);
///
/// // Side buttons → Other
/// assert_eq!(MouseButton::from(WinitMouseButton::Back), MouseButton::Other);
/// ```
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