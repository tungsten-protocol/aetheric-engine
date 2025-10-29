//=========================================================================
// Action System - Core Traits and Types
//
// Foundation for game-defined input actions and context management.
//
// This module defines the minimal interface that games must implement
// to integrate with the engine's input system. The design is intentionally
// generic and trait-based to allow each game to define its own action
// vocabulary without modifying engine code.
//
// Architecture:
// ```text
//  Game Side:                       Engine Side:
//  ┌─────────────────┐             ┌──────────────────────┐
//  │ enum GameAction │             │   ActionMapper<A>    │
//  │ {               │   impl      │                      │
//  │   Jump,         │  ──────►    │ HashMap<             │
//  │   Shoot,        │  Action     │  (Key, Mods, Ctx),   │
//  │   Interact,     │             │  A                   │
//  │ }               │             │ >                    │
//  └─────────────────┘             └──────────────────────┘
//                                           │
//                                           ↓
//                                    ┌──────────────┐
//                                    │ InputContext │
//                                    │  (Primary,   │
//                                    │   Custom)    │
//                                    └──────────────┘
//
//  Data Flow:
//  1. Platform → KeyDown{Space, NONE}
//  2. StateTracker → records key press
//  3. InputSystem → calls ActionMapper::map_key()
//  4. ActionMapper → checks (Space, NONE, CurrentContext) → Jump
//  5. Game code → if has_action(Jump) { player.jump() }
//
//  Context Switching:
//  - Primary → Gameplay bindings active
//  - Menu → Menu bindings active (Space → Select instead of Jump)
//  - Dialogue → Dialogue bindings active (Space → Advance)
// ```
//
// Responsibilities:
// - Define Action trait (minimal, type-safe interface)
// - Provide InputContext for binding isolation
// - Enable game-specific action vocabularies without engine changes
//
// Design Philosophy:
// - **Minimal trait**: Only requires Copy + Eq + Hash + Debug + Send
// - **Type safety**: Generic `<A: Action>` throughout engine
// - **Zero overhead**: All trait methods inline, no vtables
// - **Game ownership**: Games define actions, engine just coordinates
//
//=========================================================================

//=== Standard Library Imports ============================================

use std::fmt::Debug;
use std::hash::Hash;

//=== Action Trait ========================================================

/// Core trait for game-defined actions.
///
/// This trait is intentionally minimal - it only requires properties needed
/// for the input system to work efficiently:
/// - `Copy`: Actions must be cheaply copyable (typically enum variants)
/// - `Eq + Hash`: Used as keys in binding HashMaps and deduplication
/// - `Debug`: For logging and debugging input state
/// - `Send + 'static`: Thread-safe and owned (no lifetimes)
///
/// # Design Rationale
///
/// The trait has no methods because actions are pure data. The engine treats
/// actions as opaque identifiers - it's the game's responsibility to interpret
/// them and trigger behavior.
///
/// # Performance
///
/// Actions are typically zero-sized enum variants, so `Copy` is free.
/// HashMap lookups are O(1) due to `Hash + Eq`.
///
/// # Examples
///
/// Basic action enum:
/// ```
/// use aetheric_engine::core::input::Action;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// pub enum GameAction {
///     Jump,
///     Shoot,
///     Interact,
/// }
///
/// impl Action for GameAction {}
/// ```
///
/// With associated data (less common):
/// ```
/// # use aetheric_engine::core::input::Action;
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// pub enum GameAction {
///     MoveTo { x: i32, y: i32 },  // Still Copy if fields are Copy
///     UseItem { id: u32 },
/// }
///
/// impl Action for GameAction {}
/// ```
pub trait Action: 'static + Send + Copy + Eq + Hash + Debug {}

//=== InputContext ========================================================

/// Identifies which set of input bindings are currently active.
///
/// Contexts allow different control schemes for different game states
/// (gameplay, menu, dialogue, etc.). Only bindings in the active context
/// will generate actions.
///
/// # Usage Pattern
///
/// ```text
/// Gameplay Context Active:
///   Space → Jump
///   F → Shoot
///
/// Menu Context Active:
///   Space → Select
///   F → (no binding)
///
/// Same key, different meaning per context!
/// ```
///
/// # Defining Contexts
///
/// The engine provides [`Primary`](Self::Primary) as the default context,
/// but games should define semantic constants using [`custom`](Self::custom)
/// for clarity:
///
/// ```
/// use aetheric_engine::core::input::InputContext;
///
/// // Define semantic context constants
/// pub const GAMEPLAY: InputContext = InputContext::Primary;
/// pub const BUILD_MODE: InputContext = InputContext::custom(0);
/// pub const DIALOGUE: InputContext = InputContext::custom(1);
/// pub const MENU: InputContext = InputContext::custom(2);
/// pub const PAUSE: InputContext = InputContext::custom(3);
/// ```
///
/// # Context Switching
///
/// Switch contexts at runtime via [`InputSystem::set_context`]:
///
/// ```ignore
/// // Enter build mode
/// input.set_context(BUILD_MODE);
///
/// // Now only BUILD_MODE bindings are active
/// // Gameplay bindings (Space → Jump) won't trigger
/// ```
///
/// # Performance
///
/// Context switching is O(1) - it just updates a field. No data structures
/// are rebuilt. Context checks during action mapping are also O(1) as they're
/// part of the HashMap key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputContext {
    /// Default primary context, typically used for main gameplay.
    ///
    /// This is the active context on engine initialization.
    Primary,

    /// Custom application-defined context.
    ///
    /// Use distinct numeric IDs to create separate contexts.
    /// IDs have no semantic meaning - they're just identifiers.
    Custom(u32),
}

//--- Construction ---------------------------------------------------------

impl InputContext {
    /// Creates a custom context with the given ID.
    ///
    /// It's recommended to define named constants for clarity rather than
    /// using numeric literals throughout your codebase.
    ///
    /// # Examples
    ///
    /// ```
    /// use aetheric_engine::core::input::InputContext;
    ///
    /// // Good: Named constants
    /// pub const BUILD_MODE: InputContext = InputContext::custom(0);
    /// pub const DIALOGUE: InputContext = InputContext::custom(1);
    ///
    /// // Less clear: Magic numbers
    /// let ctx = InputContext::custom(42); // What does 42 mean?
    /// ```
    #[inline]
    pub const fn custom(id: u32) -> Self {
        Self::Custom(id)
    }
}

//--- Trait Implementations -----------------------------------------------

impl Default for InputContext {
    /// Returns [`InputContext::Primary`].
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

    //--- Test Action Type -------------------------------------------------

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum TestAction {
        Jump,
        Shoot,
    }

    impl Action for TestAction {}

    //=====================================================================
    // Action Trait Tests
    //=====================================================================

    /// Verifies Action trait can be implemented on simple enums.
    #[test]
    fn action_trait_is_implementable() {
        let action = TestAction::Jump;
        let copied = action;
        assert_eq!(action, copied);
    }

    /// Verifies Copy preserves original (no move).
    #[test]
    fn action_is_copy_preserves_original() {
        let action = TestAction::Jump;
        let copied = action;
        // Original still valid after copy
        assert_eq!(action, TestAction::Jump);
        assert_eq!(copied, TestAction::Jump);
    }

    /// Verifies actions can be stored in HashSets.
    #[test]
    fn action_is_hashable() {
        let mut set = HashSet::new();
        set.insert(TestAction::Jump);
        set.insert(TestAction::Jump); // Duplicate
        set.insert(TestAction::Shoot);

        assert_eq!(set.len(), 2);
        assert!(set.contains(&TestAction::Jump));
        assert!(set.contains(&TestAction::Shoot));
    }

    /// Verifies equality semantics.
    #[test]
    fn action_equality() {
        assert_eq!(TestAction::Jump, TestAction::Jump);
        assert_ne!(TestAction::Jump, TestAction::Shoot);
    }

    /// Verifies Debug trait produces readable output.
    #[test]
    fn action_debug_format() {
        let action = TestAction::Jump;
        let debug_str = format!("{:?}", action);
        assert!(debug_str.contains("Jump"));
    }

    /// Verifies Send bound for thread safety.
    #[test]
    fn action_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<TestAction>();
    }

    /// Verifies 'static bound (no lifetimes).
    #[test]
    fn action_is_static() {
        fn assert_static<T: 'static>() {}
        assert_static::<TestAction>();
    }

    //=====================================================================
    // InputContext Tests
    //=====================================================================

    /// Verifies Primary is the default context.
    #[test]
    fn input_context_primary_default() {
        assert_eq!(InputContext::default(), InputContext::Primary);
    }

    /// Verifies custom contexts can be created with distinct IDs.
    #[test]
    fn input_context_custom_creation() {
        let ctx1 = InputContext::custom(0);
        let ctx2 = InputContext::custom(1);

        assert_ne!(ctx1, ctx2);
        assert_eq!(ctx1, InputContext::Custom(0));
        assert_eq!(ctx2, InputContext::Custom(1));
    }

    /// Verifies Primary and Custom(0) are distinct.
    #[test]
    fn input_context_primary_vs_custom_zero() {
        let primary = InputContext::Primary;
        let custom_zero = InputContext::custom(0);

        assert_ne!(primary, custom_zero);
    }

    /// Verifies full u32 range can be used for context IDs.
    #[test]
    fn input_context_custom_max_id() {
        let ctx = InputContext::custom(u32::MAX);
        assert_eq!(ctx, InputContext::Custom(u32::MAX));
    }

    /// Verifies many unique contexts can coexist.
    #[test]
    fn input_context_many_unique() {
        let mut contexts = HashSet::new();
        for i in 0..100 {
            contexts.insert(InputContext::custom(i));
        }
        assert_eq!(contexts.len(), 100);
    }

    /// Verifies Copy trait preserves original.
    #[test]
    fn input_context_is_copy() {
        let ctx = InputContext::Primary;
        let copied = ctx;
        assert_eq!(ctx, copied);
        assert_eq!(ctx, InputContext::Primary); // Original preserved
    }

    /// Verifies Clone and Copy produce identical results.
    #[test]
    fn input_context_clone_equals_copy() {
        let ctx = InputContext::custom(5);
        assert_eq!(ctx.clone(), ctx);
    }

    /// Verifies contexts can be stored in HashSets.
    #[test]
    fn input_context_is_hashable() {
        let mut set = HashSet::new();
        set.insert(InputContext::Primary);
        set.insert(InputContext::Primary); // Duplicate
        set.insert(InputContext::custom(0));

        assert_eq!(set.len(), 2);
    }

    /// Verifies Debug output is readable.
    #[test]
    fn input_context_debug_format() {
        let primary = InputContext::Primary;
        let custom = InputContext::custom(42);

        assert!(format!("{:?}", primary).contains("Primary"));
        assert!(format!("{:?}", custom).contains("Custom"));
        assert!(format!("{:?}", custom).contains("42"));
    }

    //=====================================================================
    // Const Construction Tests
    //=====================================================================

    /// Verifies custom() is const and can be used in const contexts.
    #[test]
    fn input_context_custom_is_const() {
        const BUILD_MODE: InputContext = InputContext::custom(0);
        const DIALOGUE: InputContext = InputContext::custom(1);

        assert_ne!(BUILD_MODE, DIALOGUE);
    }
}