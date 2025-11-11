//=========================================================================
// Input Event Types
//=========================================================================
//
// Low-level input event types with platform-agnostic representation.
//
// Hash-stable semantics: MouseMoved events hash/compare by discriminant only
// (coordinates ignored for coalescing). Modifiers must match exactly in
// bindings (Ctrl+S ≠ Ctrl+Shift+S).
//
//=========================================================================

//=== External Dependencies ===============================================

use std::hash::{Hash, Hasher};

//=== MouseButton =========================================================

/// Physical mouse button identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Primary button (typically left).
    Left,

    /// Secondary button (typically right).
    Right,

    /// Middle button (wheel click).
    Middle,

    /// Any other button beyond the standard three.
    ///
    /// Includes side buttons (button 4/5), thumb buttons, macro keys, etc.
    /// Note: Not all platforms expose these buttons consistently.
    Other
}

//=== KeyCode =============================================================

/// Physical keyboard key identifier based on key position, not character output.
///
/// Keys are identified by physical location, not the character they produce.
/// For example, `KeyCode::KeyW` is always the same physical key, even on
/// non-QWERTY layouts (AZERTY, Dvorak, etc.). This ensures consistent game
/// controls across different keyboard layouts.
///
/// # Why Location-Based?
///
/// - **Game controls**: "WASD" movement works the same on all layouts
/// - **Modifier keys**: Shift+W produces "W", not "w" or other characters
/// - **Cross-platform**: Platform layer normalizes key codes
///
/// For text input (chat, names, etc.), you'll need character events (future API).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    //--- Numeric Keys -----------------------------------------------------

    /// Number row: 0-9
    Digit0, Digit1, Digit2, Digit3, Digit4,
    Digit5, Digit6, Digit7, Digit8, Digit9,

    //--- Alphabetic Keys --------------------------------------------------

    /// Letter keys: A-Z (physical location, not character)
    KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI,
    KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ, KeyR,
    KeyS, KeyT, KeyU, KeyV, KeyW, KeyX, KeyY, KeyZ,

    //--- Arrow Keys -------------------------------------------------------

    /// Directional navigation keys
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,

    //--- Special Keys -----------------------------------------------------

    /// Spacebar
    Space,

    /// Return/Enter key
    Enter,

    /// Escape key
    Escape,

    /// Tab key
    Tab,

    /// Backspace key
    Backspace,

    /// Delete key
    Delete,

    /// Fallback for unmapped keys.
    Unidentified
}

//=== InputEvent ==========================================================

/// Low-level input event from the platform layer.
/// MouseMoved events hash/compare by discriminant only (coordinates ignored for coalescing).
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// Key pressed down.
    KeyDown {
        key: KeyCode,
        modifiers: Modifiers,
    },

    /// Key released.
    KeyUp {
        key: KeyCode,
        modifiers: Modifiers,
    },

    /// Mouse button pressed.
    MouseButtonDown {
        button: MouseButton,
        modifiers: Modifiers,
    },

    /// Mouse button released.
    MouseButtonUp {
        button: MouseButton,
        modifiers: Modifiers,
    },

    /// Mouse cursor moved (screen space, pixels, top-left origin).
    MouseMoved { x: f32, y: f32 },

    /// Unrecognized event (silently ignored).
    Unidentified
}

//--- Implementation ------------------------------------------------------

impl InputEvent {
    /// Returns a new event with updated modifiers (internal utility).
    pub(crate) fn with_modifiers(mut self, modifiers: Modifiers) -> Self {
        match &mut self {
            Self::KeyDown { modifiers: m, .. }
            | Self::KeyUp { modifiers: m, .. }
            | Self::MouseButtonDown { modifiers: m, .. }
            | Self::MouseButtonUp { modifiers: m, .. } => {
                *m = modifiers;
            }
            _ => {}
        }
        self
    }
}

//--- Trait Implementations -----------------------------------------------

/// Equality by discriminant + payload. MouseMoved always equal (coordinates ignored).
impl PartialEq for InputEvent {
    fn eq(&self, other: &Self) -> bool {
        use InputEvent::*;
        match (self, other) {
            (KeyDown { key: a, modifiers: ma }, KeyDown { key: b, modifiers: mb }) => {
                a == b && ma == mb
            }
            (KeyUp { key: a, modifiers: ma }, KeyUp { key: b, modifiers: mb }) => {
                a == b && ma == mb
            }
            (
                MouseButtonDown { button: a, modifiers: ma },
                MouseButtonDown { button: b, modifiers: mb }
            ) => {
                a == b && ma == mb
            }
            (
                MouseButtonUp { button: a, modifiers: ma },
                MouseButtonUp { button: b, modifiers: mb }
            ) => {
                a == b && ma == mb
            }
            // MouseMoved: coordinates ignored, always equal
            (MouseMoved { .. }, MouseMoved { .. }) => true,
            (Unidentified, Unidentified) => true,
            _ => false,
        }
    }
}

impl Eq for InputEvent {}

/// Hashes by discriminant + payload. MouseMoved coordinates not hashed (consistent with equality).
impl Hash for InputEvent {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the discriminant first (event type)
        std::mem::discriminant(self).hash(state);

        // Hash payload based on variant
        match self {
            Self::KeyDown { key, modifiers } | Self::KeyUp { key, modifiers } => {
                key.hash(state);
                modifiers.hash(state);
            }
            Self::MouseButtonDown { button, modifiers }
            | Self::MouseButtonUp { button, modifiers } => {
                button.hash(state);
                modifiers.hash(state);
            }
            // MouseMoved and Unidentified: only discriminant matters
            _ => {}
        }
    }
}

//=== Modifiers ===========================================================

/// Modifier key state for Shift, Ctrl, and Alt.
///
/// Does not distinguish left/right variants (e.g., Left Shift = Right Shift).
/// Modifiers must match exactly in bindings: `Ctrl+S` ≠ `Ctrl+Shift+S`.
///
/// # Exact Matching Behavior
///
/// When binding keys with modifiers using [`InputSystem::bind_key_with_mods`](crate::core::InputSystem::bind_key_with_mods),
/// the modifiers must match exactly. Pressing additional modifiers will not
/// trigger the action.
///
/// # Example
///
/// ```ignore
/// use aetheric_engine::prelude::*;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// enum GameAction { Save, SaveAs }
/// impl Action for GameAction {}
///
/// let mut input = InputSystem::<GameAction>::default();
///
/// // Bind Ctrl+S (save) and Ctrl+Shift+S (save as)
/// input.bind_key_with_mods(
///     KeyCode::KeyS,
///     Modifiers::CTRL,
///     GameAction::Save,
///     InputContext::Primary
/// );
/// input.bind_key_with_mods(
///     KeyCode::KeyS,
///     Modifiers::SHIFT_CTRL,
///     GameAction::SaveAs,
///     InputContext::Primary
/// );
///
/// // Pressing Ctrl+S triggers only Save (not SaveAs)
/// // Pressing Ctrl+Shift+S triggers only SaveAs (not Save)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

//--- Modifier Constants --------------------------------------------------

impl Modifiers {
    /// No modifiers held.
    pub const NONE: Self = Self {
        shift: false,
        ctrl: false,
        alt: false,
    };

    /// Shift only.
    pub const SHIFT: Self = Self {
        shift: true,
        ctrl: false,
        alt: false,
    };

    /// Ctrl only.
    pub const CTRL: Self = Self {
        shift: false,
        ctrl: true,
        alt: false,
    };

    /// Alt only.
    pub const ALT: Self = Self {
        shift: false,
        ctrl: false,
        alt: true,
    };

    /// Shift + Ctrl.
    pub const SHIFT_CTRL: Self = Self {
        shift: true,
        ctrl: true,
        alt: false,
    };

    /// Shift + Alt.
    pub const SHIFT_ALT: Self = Self {
        shift: true,
        ctrl: false,
        alt: true,
    };

    /// Ctrl + Alt.
    pub const CTRL_ALT: Self = Self {
        shift: false,
        ctrl: true,
        alt: true,
    };

    /// All modifiers held (Shift + Ctrl + Alt).
    pub const ALL: Self = Self {
        shift: true,
        ctrl: true,
        alt: true,
    };
}

//--- Trait Implementations -----------------------------------------------

impl Default for Modifiers {
    /// Defaults to no modifiers held.
    fn default() -> Self {
        Self::NONE
    }
}

//=========================================================================
// Unit Tests
//=========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;

    //--- Test Helpers -----------------------------------------------------

    fn hash_of<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    fn key_down(key: KeyCode) -> InputEvent {
        InputEvent::KeyDown {
            key,
            modifiers: Modifiers::NONE
        }
    }

    fn key_up(key: KeyCode) -> InputEvent {
        InputEvent::KeyUp {
            key,
            modifiers: Modifiers::NONE
        }
    }

    fn mouse_down(btn: MouseButton) -> InputEvent {
        InputEvent::MouseButtonDown {
            button: btn,
            modifiers: Modifiers::NONE
        }
    }

    fn mouse_up(btn: MouseButton) -> InputEvent {
        InputEvent::MouseButtonUp {
            button: btn,
            modifiers: Modifiers::NONE
        }
    }

    //=====================================================================
    // Equality Tests
    //=====================================================================

    /// Same key+modifiers should be equal.
    #[test]
    fn equality_same_type_same_data() {
        let a = key_down(KeyCode::KeyA);
        let b = key_down(KeyCode::KeyA);
        assert_eq!(a, b);
    }

    /// MouseMoved ignores coordinates (always equal).
    #[test]
    fn equality_mousemoved_ignores_coordinates() {
        let a = InputEvent::MouseMoved { x: 10.0, y: 10.0 };
        let b = InputEvent::MouseMoved { x: 200.0, y: 300.0 };
        assert_eq!(a, b);
    }

    /// Different event types are not equal.
    #[test]
    fn equality_different_discriminant() {
        let a = key_down(KeyCode::KeyA);
        let b = key_up(KeyCode::KeyA);
        assert_ne!(a, b);
    }

    /// Same key, different modifiers are not equal.
    #[test]
    fn equality_same_key_different_modifiers() {
        let a = InputEvent::KeyDown { key: KeyCode::KeyA, modifiers: Modifiers::NONE };
        let b = InputEvent::KeyDown { key: KeyCode::KeyA, modifiers: Modifiers::CTRL };
        assert_ne!(a, b);
    }

    /// Different keys, same modifiers are not equal.
    #[test]
    fn equality_different_key_same_modifiers() {
        let a = key_down(KeyCode::KeyA);
        let b = key_down(KeyCode::KeyB);
        assert_ne!(a, b);
    }

    /// Different mouse buttons are not equal.
    #[test]
    fn equality_different_mouse_buttons() {
        let a = mouse_down(MouseButton::Left);
        let b = mouse_down(MouseButton::Right);
        assert_ne!(a, b);
    }

    /// Same button, different modifiers are not equal.
    #[test]
    fn equality_mouse_button_different_modifiers() {
        let a = InputEvent::MouseButtonDown {
            button: MouseButton::Left,
            modifiers: Modifiers::NONE
        };
        let b = InputEvent::MouseButtonDown {
            button: MouseButton::Left,
            modifiers: Modifiers::CTRL
        };
        assert_ne!(a, b);
    }

    /// Unidentified events are always equal.
    #[test]
    fn equality_unidentified() {
        let a = InputEvent::Unidentified;
        let b = InputEvent::Unidentified;
        assert_eq!(a, b);
    }

    /// KeyDown and MouseButtonDown are different despite similar structure.
    #[test]
    fn equality_different_event_families() {
        let key = InputEvent::KeyDown {
            key: KeyCode::KeyA,
            modifiers: Modifiers::NONE
        };
        let mouse = InputEvent::MouseButtonDown {
            button: MouseButton::Left,
            modifiers: Modifiers::NONE
        };
        assert_ne!(key, mouse);
    }

    //=====================================================================
    // Hashing Tests
    //=====================================================================

    /// Different keys produce different hashes.
    #[test]
    fn hash_different_keys() {
        let a = key_down(KeyCode::KeyA);
        let b = key_down(KeyCode::KeyB);
        assert_ne!(hash_of(&a), hash_of(&b));
    }

    /// Different modifiers produce different hashes.
    #[test]
    fn hash_different_modifiers() {
        let a = InputEvent::KeyDown { key: KeyCode::KeyA, modifiers: Modifiers::NONE };
        let b = InputEvent::KeyDown { key: KeyCode::KeyA, modifiers: Modifiers::CTRL };
        assert_ne!(hash_of(&a), hash_of(&b));
    }

    /// Different event types produce different hashes.
    #[test]
    fn hash_different_discriminants() {
        let a = key_down(KeyCode::KeyA);
        let b = key_up(KeyCode::KeyA);
        assert_ne!(hash_of(&a), hash_of(&b));
    }

    /// MouseMoved hashes are identical regardless of coordinates.
    #[test]
    fn hash_mousemoved_stable() {
        let a = InputEvent::MouseMoved { x: 1.0, y: 2.0 };
        let b = InputEvent::MouseMoved { x: 300.0, y: 400.0 };
        assert_eq!(hash_of(&a), hash_of(&b));
    }

    /// Same event produces same hash (determinism).
    #[test]
    fn hash_deterministic() {
        let a = key_down(KeyCode::Space);
        let b = key_down(KeyCode::Space);
        assert_eq!(hash_of(&a), hash_of(&b));
    }

    /// Different mouse buttons produce different hashes.
    #[test]
    fn hash_different_mouse_buttons() {
        let a = mouse_down(MouseButton::Left);
        let b = mouse_down(MouseButton::Right);
        assert_ne!(hash_of(&a), hash_of(&b));
    }

    /// Unidentified events hash consistently.
    #[test]
    fn hash_unidentified_stable() {
        let a = InputEvent::Unidentified;
        let b = InputEvent::Unidentified;
        assert_eq!(hash_of(&a), hash_of(&b));
    }

    //=====================================================================
    // Hash-Equality Contract Tests
    //=====================================================================

    /// Verifies hash-equality contract: a == b → hash(a) == hash(b).
    #[test]
    fn hash_equality_contract_keys() {
        let a = key_down(KeyCode::Space);
        let b = key_down(KeyCode::Space);

        assert_eq!(a, b);
        assert_eq!(hash_of(&a), hash_of(&b));
    }

    /// Verifies contract for MouseMoved.
    #[test]
    fn hash_equality_contract_mousemoved() {
        let a = InputEvent::MouseMoved { x: 10.0, y: 20.0 };
        let b = InputEvent::MouseMoved { x: 999.0, y: 888.0 };

        assert_eq!(a, b); // Equal despite different coords
        assert_eq!(hash_of(&a), hash_of(&b)); // Hashes must match
    }

    //=====================================================================
    // with_modifiers Tests
    //=====================================================================

    /// with_modifiers updates modifiers on KeyDown.
    #[test]
    fn with_modifiers_key_down() {
        let event = InputEvent::KeyDown {
            key: KeyCode::KeyA,
            modifiers: Modifiers::NONE
        };

        let updated = event.with_modifiers(Modifiers::CTRL);

        match updated {
            InputEvent::KeyDown { key, modifiers } => {
                assert_eq!(key, KeyCode::KeyA);
                assert_eq!(modifiers, Modifiers::CTRL);
            }
            _ => panic!("Wrong event type"),
        }
    }

    /// with_modifiers updates modifiers on KeyUp.
    #[test]
    fn with_modifiers_key_up() {
        let event = key_up(KeyCode::KeyB);
        let updated = event.with_modifiers(Modifiers::SHIFT);

        match updated {
            InputEvent::KeyUp { key, modifiers } => {
                assert_eq!(key, KeyCode::KeyB);
                assert_eq!(modifiers, Modifiers::SHIFT);
            }
            _ => panic!("Wrong event type"),
        }
    }

    /// with_modifiers updates modifiers on MouseButtonDown.
    #[test]
    fn with_modifiers_mouse_down() {
        let event = mouse_down(MouseButton::Left);
        let updated = event.with_modifiers(Modifiers::ALT);

        match updated {
            InputEvent::MouseButtonDown { button, modifiers } => {
                assert_eq!(button, MouseButton::Left);
                assert_eq!(modifiers, Modifiers::ALT);
            }
            _ => panic!("Wrong event type"),
        }
    }

    /// with_modifiers is no-op on MouseMoved.
    #[test]
    fn with_modifiers_ignores_mouse_moved() {
        let event = InputEvent::MouseMoved { x: 10.0, y: 20.0 };
        let original = event.clone();
        let updated = event.with_modifiers(Modifiers::CTRL);

        assert_eq!(updated, original);
    }

    /// with_modifiers is no-op on Unidentified.
    #[test]
    fn with_modifiers_ignores_unidentified() {
        let event = InputEvent::Unidentified;
        let updated = event.clone().with_modifiers(Modifiers::ALL);

        assert_eq!(updated, InputEvent::Unidentified);
    }

    //=====================================================================
    // Copy/Clone Tests
    //=====================================================================

    /// KeyCode is Copy.
    #[test]
    fn keycode_is_copy() {
        let key = KeyCode::Space;
        let copied = key;
        assert_eq!(key, copied);
    }

    /// MouseButton is Copy.
    #[test]
    fn mousebutton_is_copy() {
        let btn = MouseButton::Left;
        let copied = btn;
        assert_eq!(btn, copied);
    }

    /// Modifiers is Copy.
    #[test]
    fn modifiers_is_copy() {
        let mods = Modifiers::CTRL;
        let copied = mods;
        assert_eq!(mods, copied);
    }

    /// InputEvent is Clone (but not Copy due to potential future extensions).
    #[test]
    fn input_event_is_clone() {
        let event = key_down(KeyCode::Space);
        let cloned = event.clone();
        assert_eq!(event, cloned);
    }

    //=====================================================================
    // Modifiers Tests
    //=====================================================================

    /// Verifies NONE constant has all flags false.
    #[test]
    fn modifiers_none() {
        let mods = Modifiers::NONE;
        assert!(!mods.shift && !mods.ctrl && !mods.alt);
    }

    /// Verifies SHIFT constant has only shift true.
    #[test]
    fn modifiers_shift() {
        let mods = Modifiers::SHIFT;
        assert!(mods.shift && !mods.ctrl && !mods.alt);
    }

    /// Verifies CTRL constant has only ctrl true.
    #[test]
    fn modifiers_ctrl() {
        let mods = Modifiers::CTRL;
        assert!(!mods.shift && mods.ctrl && !mods.alt);
    }

    /// Verifies ALT constant has only alt true.
    #[test]
    fn modifiers_alt() {
        let mods = Modifiers::ALT;
        assert!(!mods.shift && !mods.ctrl && mods.alt);
    }

    /// Verifies SHIFT_CTRL constant.
    #[test]
    fn modifiers_shift_ctrl() {
        let mods = Modifiers::SHIFT_CTRL;
        assert!(mods.shift && mods.ctrl && !mods.alt);
    }

    /// Verifies SHIFT_ALT constant.
    #[test]
    fn modifiers_shift_alt() {
        let mods = Modifiers::SHIFT_ALT;
        assert!(mods.shift && !mods.ctrl && mods.alt);
    }

    /// Verifies CTRL_ALT constant.
    #[test]
    fn modifiers_ctrl_alt() {
        let mods = Modifiers::CTRL_ALT;
        assert!(!mods.shift && mods.ctrl && mods.alt);
    }

    /// Verifies ALL constant has all flags true.
    #[test]
    fn modifiers_all() {
        let mods = Modifiers::ALL;
        assert!(mods.shift && mods.ctrl && mods.alt);
    }

    /// Verifies Default trait returns NONE.
    #[test]
    fn modifiers_default() {
        let mods = Modifiers::default();
        assert_eq!(mods, Modifiers::NONE);
    }

    /// Different modifier combinations are not equal.
    #[test]
    fn modifiers_inequality() {
        assert_ne!(Modifiers::NONE, Modifiers::SHIFT);
        assert_ne!(Modifiers::CTRL, Modifiers::SHIFT_CTRL);
        assert_ne!(Modifiers::ALL, Modifiers::SHIFT_ALT);
    }
}