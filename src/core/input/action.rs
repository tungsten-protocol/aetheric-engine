//=========================================================================
// Action Trait & Input Context
//=========================================================================
//
// Game-defined action trait and input context system.
//
// Actions: Opaque identifiers routed by the engine, interpreted by the game.
// Contexts: Allows different bindings for different game states (gameplay vs menu).
//
//=========================================================================

//=== External Dependencies ===============================================

use std::fmt::Debug;
use std::hash::Hash;

//=== Action Trait ========================================================

/// Marker trait for game-defined action enums.
///
/// Actions represent high-level gameplay commands (Jump, Shoot, OpenMenu)
/// mapped from raw inputs. The engine routes actions without interpreting them.
///
/// # Requirements
///
/// - `Copy + Eq + Hash`: Efficient passing and deduplication
/// - `Debug`: Logging support
/// - `Send + 'static`: Thread-safe transfer
///
/// # Example
///
/// ```
/// use aetheric_engine::prelude::*;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// enum GameAction { Jump, Shoot, Reload }
///
/// impl Action for GameAction {}
/// ```
///
/// Use with `InputSystem<GameAction>` to bind keys and query actions each frame.
/// See [`InputContext`] for context-based binding (gameplay vs menu).
pub trait Action: 'static + Send + Copy + Eq + Hash + Debug {}

//=== InputContext ========================================================

/// Identifies which set of input bindings are currently active.
///
/// Enables the same key to trigger different actions based on game state.
/// For example, Space = Jump (gameplay) vs Space = Select (menu).
///
/// # Variants
///
/// - `Primary`: Default context for core gameplay
/// - `Custom(u32)`: User-defined contexts (0-4,294,967,295)
///
/// # Example
///
/// ```ignore
/// use aetheric_engine::prelude::*;
///
/// # #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// # enum GameAction { Jump, Select }
/// # impl Action for GameAction {}
/// let mut input = InputSystem::<GameAction>::default();
///
/// // Gameplay: Space = Jump
/// input.bind_key(KeyCode::Space, GameAction::Jump, InputContext::Primary);
///
/// // Menu: Space = Select
/// let menu = InputContext::custom(0);
/// input.bind_key(KeyCode::Space, GameAction::Select, menu);
///
/// // Switch context
/// input.set_context(menu); // Now Space triggers Select
/// ```
///
/// # Recommended Pattern
///
/// Define semantic constants:
/// ```
/// # use aetheric_engine::prelude::*;
/// const GAMEPLAY: InputContext = InputContext::Primary;
/// const MENU: InputContext = InputContext::custom(0);
/// const VEHICLE: InputContext = InputContext::custom(1);
/// ```
///
/// Context switching is instant. Raw queries (`is_key_down`) work regardless of context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputContext {
    /// Default context for primary gameplay.
    Primary,

    /// User-defined context (menus, dialogue, vehicles, etc.).
    Custom(u32),
}

impl InputContext {
    /// Creates a custom context.
    ///
    /// ```
    /// # use aetheric_engine::prelude::*;
    /// const MENU: InputContext = InputContext::custom(0);
    /// ```
    #[inline]
    pub const fn custom(id: u32) -> Self {
        Self::Custom(id)
    }
}

impl Default for InputContext {
    fn default() -> Self {
        Self::Primary
    }
}

//=========================================================================
// Unit Tests
//=========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum TestAction {
        Jump,
        Shoot,
    }

    impl Action for TestAction {}

    //=== Action Trait ====================================================

    #[test]
    fn action_trait_is_implementable() {
        let action = TestAction::Jump;
        let copied = action;
        assert_eq!(action, copied);
    }

    #[test]
    fn action_is_copy_preserves_original() {
        let action = TestAction::Jump;
        let copied = action;
        assert_eq!(action, TestAction::Jump);
        assert_eq!(copied, TestAction::Jump);
    }

    #[test]
    fn action_is_hashable() {
        let mut set = HashSet::new();
        set.insert(TestAction::Jump);
        set.insert(TestAction::Jump);
        set.insert(TestAction::Shoot);

        assert_eq!(set.len(), 2);
        assert!(set.contains(&TestAction::Jump));
    }

    #[test]
    fn action_equality() {
        assert_eq!(TestAction::Jump, TestAction::Jump);
        assert_ne!(TestAction::Jump, TestAction::Shoot);
    }

    #[test]
    fn action_debug_format() {
        let debug_str = format!("{:?}", TestAction::Jump);
        assert!(debug_str.contains("Jump"));
    }

    #[test]
    fn action_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<TestAction>();
    }

    #[test]
    fn action_is_static() {
        fn assert_static<T: 'static>() {}
        assert_static::<TestAction>();
    }

    //=== InputContext ====================================================

    #[test]
    fn input_context_primary_default() {
        assert_eq!(InputContext::default(), InputContext::Primary);
    }

    #[test]
    fn input_context_custom_creation() {
        let ctx1 = InputContext::custom(0);
        let ctx2 = InputContext::custom(1);

        assert_ne!(ctx1, ctx2);
        assert_eq!(ctx1, InputContext::Custom(0));
    }

    #[test]
    fn input_context_primary_vs_custom_zero() {
        // Important: Primary ≠ Custom(0)
        assert_ne!(InputContext::Primary, InputContext::custom(0));
    }

    #[test]
    fn input_context_custom_max_id() {
        let ctx = InputContext::custom(u32::MAX);
        assert_eq!(ctx, InputContext::Custom(u32::MAX));
    }

    #[test]
    fn input_context_many_unique() {
        let mut contexts = HashSet::new();
        for i in 0..100 {
            contexts.insert(InputContext::custom(i));
        }
        assert_eq!(contexts.len(), 100);
    }

    #[test]
    fn input_context_is_copy() {
        let ctx = InputContext::Primary;
        let copied = ctx;
        assert_eq!(ctx, copied);
        assert_eq!(ctx, InputContext::Primary);
    }

    #[test]
    fn input_context_clone_equals_copy() {
        let ctx = InputContext::custom(5);
        assert_eq!(ctx.clone(), ctx);
    }

    #[test]
    fn input_context_is_hashable() {
        let mut set = HashSet::new();
        set.insert(InputContext::Primary);
        set.insert(InputContext::Primary);
        set.insert(InputContext::custom(0));

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn input_context_debug_format() {
        let primary = InputContext::Primary;
        let custom = InputContext::custom(42);

        assert!(format!("{:?}", primary).contains("Primary"));
        assert!(format!("{:?}", custom).contains("42"));
    }

    #[test]
    fn input_context_custom_is_const() {
        const BUILD_MODE: InputContext = InputContext::custom(0);
        const DIALOGUE: InputContext = InputContext::custom(1);

        assert_ne!(BUILD_MODE, DIALOGUE);
    }
}