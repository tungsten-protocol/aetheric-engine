//=========================================================================
// Action Mapper
//=========================================================================
//
// Maps raw input events to game actions based on configured bindings and context.
//
// Architecture:
//   (key/button, modifiers, context) → HashMap → Action
//
// Only bindings in the active context resolve to actions.
//
//=========================================================================

//=== External Dependencies ===============================================

use std::collections::HashMap;

//=== Internal Dependencies ===============================================

use super::{
    action::{Action, InputContext},
    event::{InputEvent, KeyCode, MouseButton, Modifiers}
};

//=== ActionMapper ========================================================

/// Maps input events to actions via (key/button, modifiers, context) lookups.
/// Only bindings in the active context resolve to actions.
pub(crate) struct ActionMapper<A: Action> {
    /// Key bindings: (key, modifiers, context) → action
    key_bindings: HashMap<(KeyCode, Modifiers, InputContext), A>,

    /// Mouse button bindings: (button, modifiers, context) → action
    mouse_bindings: HashMap<(MouseButton, Modifiers, InputContext), A>,

    /// Currently active input context
    current_context: InputContext,
}

impl<A: Action> ActionMapper<A> {
    /// Creates a new mapper with Primary context active and no bindings.
    pub(crate) fn new() -> Self {
        Self {
            key_bindings: HashMap::new(),
            mouse_bindings: HashMap::new(),
            current_context: InputContext::Primary,
        }
    }

    //--- Binding API ------------------------------------------------------
    /// Binds a key to an action (no modifiers).
    pub(crate) fn bind_key(
        &mut self,
        key: KeyCode,
        action: A,
        context: InputContext,
    ) {
        self.bind_key_with_mods(key, Modifiers::NONE, action, context);
    }

    /// Binds a key with modifiers to an action (exact match required).
    pub(crate) fn bind_key_with_mods(
        &mut self,
        key: KeyCode,
        modifiers: Modifiers,
        action: A,
        context: InputContext,
    ) {
        self.key_bindings.insert((key, modifiers, context), action);
    }

    /// Binds a mouse button to an action (no modifiers).
    pub(crate) fn bind_mouse(
        &mut self,
        button: MouseButton,
        action: A,
        context: InputContext,
    ) {
        self.bind_mouse_with_mods(button, Modifiers::NONE, action, context);
    }

    /// Binds a mouse button with modifiers to an action.
    pub(crate) fn bind_mouse_with_mods(
        &mut self,
        button: MouseButton,
        modifiers: Modifiers,
        action: A,
        context: InputContext,
    ) {
        self.mouse_bindings.insert((button, modifiers, context), action);
    }

    /// Removes a specific key binding (exact modifier match).
    pub(crate) fn unbind_key_with_mods(
        &mut self,
        key: KeyCode,
        modifiers: Modifiers,
        context: InputContext,
    ) {
        self.key_bindings.remove(&(key, modifiers, context));
    }

    /// Removes key binding without modifiers (does NOT remove modified variants).
    pub(crate) fn unbind_key(&mut self, key: KeyCode, context: InputContext) {
        self.unbind_key_with_mods(key, Modifiers::NONE, context);
    }

    /// Removes ALL bindings for a key in context (all modifier combinations).
    pub(crate) fn unbind_key_all_variants(
        &mut self,
        key: KeyCode,
        context: InputContext,
    ) {
        self.key_bindings.retain(|&(k, _, ctx), _| !(k == key && ctx == context));
    }

    /// Removes ALL bindings for a mouse button in context (all modifier combinations).
    pub(crate) fn unbind_mouse_all_variants(
        &mut self,
        button: MouseButton,
        context: InputContext,
    ) {
        self.mouse_bindings.retain(|&(btn, _, ctx), _| !(btn == button && ctx == context));
    }

    /// Removes a specific mouse button binding (exact modifier match).
    pub(crate) fn unbind_mouse_with_mods(
        &mut self,
        button: MouseButton,
        modifiers: Modifiers,
        context: InputContext,
    ) {
        self.mouse_bindings.remove(&(button, modifiers, context));
    }

    /// Removes mouse button binding without modifiers (does NOT remove modified variants).
    pub(crate) fn unbind_mouse(&mut self, button: MouseButton, context: InputContext) {
        self.unbind_mouse_with_mods(button, Modifiers::NONE, context);
    }

    /// Clears all bindings for a context (keys and mouse buttons).
    pub(crate) fn clear_context(&mut self, context: InputContext) {
        self.key_bindings.retain(|&(_, _, ctx), _| ctx != context);
        self.mouse_bindings.retain(|&(_, _, ctx), _| ctx != context);
    }

    //--- Event Mapping ----------------------------------------------------
    /// Maps an input event to an action in the active context.
    pub(crate) fn map_event(&self, event: &InputEvent) -> Option<A> {
        match event {
            InputEvent::KeyDown { key, modifiers } => {
                self.map_key(*key, *modifiers)
            }
            InputEvent::MouseButtonDown { button, modifiers } => {
                self.map_button(*button, *modifiers)
            }
            _ => None,
        }
    }

    //--- Internal Mapping Helpers -----------------------------------------
    /// Maps a key press to an action.
    pub(super) fn map_key(&self, key: KeyCode, modifiers: Modifiers) -> Option<A> {
        let binding_key = (key, modifiers, self.current_context);
        self.key_bindings.get(&binding_key).copied()
    }

    /// Maps a mouse button press to an action.
    pub(super) fn map_button(&self, btn: MouseButton, modifiers: Modifiers) -> Option<A> {
        let binding_key = (btn, modifiers, self.current_context);
        self.mouse_bindings.get(&binding_key).copied()
    }

    /// Sets the active input context.
    pub(crate) fn set_context(&mut self, context: InputContext) {
        self.current_context = context;
    }

    /// Returns the current active context.
    pub(crate) fn current_context(&self) -> InputContext {
        self.current_context
    }
}

//=========================================================================
// Unit Tests
//=========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    //--- Test Action Type -------------------------------------------------

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum TestAction {
        Jump,
        Shoot,
        Save,
    }

    impl Action for TestAction {}

    //--- Test Helper Functions --------------------------------------------

    fn key_down(key: KeyCode) -> InputEvent {
        InputEvent::KeyDown { key, modifiers: Modifiers::NONE }
    }

    fn key_down_with_mods(key: KeyCode, modifiers: Modifiers) -> InputEvent {
        InputEvent::KeyDown { key, modifiers }
    }

    fn mouse_down(btn: MouseButton) -> InputEvent {
        InputEvent::MouseButtonDown { button: btn, modifiers: Modifiers::NONE }
    }

    fn mouse_down_with_mods(btn: MouseButton, modifiers: Modifiers) -> InputEvent {
        InputEvent::MouseButtonDown { button: btn, modifiers }
    }

    fn key_up(key: KeyCode) -> InputEvent {
        InputEvent::KeyUp { key, modifiers: Modifiers::NONE }
    }

    //=====================================================================
    // Basic Binding Tests
    //=====================================================================

    /// Verifies that a simple key binding without modifiers works correctly.
    #[test]
    fn bind_and_map_simple_key() {
        let mut mapper = ActionMapper::<TestAction>::new();

        mapper.bind_key(KeyCode::Space, TestAction::Jump, InputContext::Primary);

        let event = key_down(KeyCode::Space);
        let action = mapper.map_event(&event);

        assert_eq!(action, Some(TestAction::Jump));
    }

    /// Ensures that querying an unbound key returns None.
    #[test]
    fn map_event_returns_none_if_no_binding() {
        let mapper = ActionMapper::<TestAction>::new();

        let event = key_down(KeyCode::Space);
        let action = mapper.map_event(&event);

        assert_eq!(action, None);
    }

    //=====================================================================
    // Modifier Tests
    //=====================================================================

    /// Tests that same key can have different bindings with/without modifiers.
    #[test]
    fn bind_with_modifiers() {
        let mut mapper = ActionMapper::<TestAction>::new();

        // Bind S to Shoot (no mods)
        mapper.bind_key(KeyCode::KeyS, TestAction::Shoot, InputContext::Primary);

        // Bind Ctrl+S to Save
        mapper.bind_key_with_mods(
            KeyCode::KeyS,
            Modifiers::CTRL,
            TestAction::Save,
            InputContext::Primary
        );

        // Test 1: S alone → Shoot
        let event = key_down(KeyCode::KeyS);
        assert_eq!(mapper.map_event(&event), Some(TestAction::Shoot));

        // Test 2: Ctrl+S → Save
        let event_ctrl_s = key_down_with_mods(KeyCode::KeyS, Modifiers::CTRL);
        assert_eq!(mapper.map_event(&event_ctrl_s), Some(TestAction::Save));
    }

    /// Verifies that modifiers must match exactly (no partial matching).
    #[test]
    fn modifiers_must_match_exactly() {
        let mut mapper = ActionMapper::<TestAction>::new();

        // Bind Ctrl+S to Save
        mapper.bind_key_with_mods(
            KeyCode::KeyS,
            Modifiers::CTRL,
            TestAction::Save,
            InputContext::Primary
        );

        // Press S without Ctrl → no match
        let event = key_down(KeyCode::KeyS);
        assert_eq!(mapper.map_event(&event), None);

        // Press Ctrl+S → matches
        let event_ctrl = key_down_with_mods(KeyCode::KeyS, Modifiers::CTRL);
        assert_eq!(mapper.map_event(&event_ctrl), Some(TestAction::Save));
    }

    /// Tests that multiple modifier combinations on same key are independent.
    #[test]
    fn all_modifier_combinations_independent() {
        let mut mapper = ActionMapper::<TestAction>::new();

        mapper.bind_key(KeyCode::KeyA, TestAction::Jump, InputContext::Primary);
        mapper.bind_key_with_mods(KeyCode::KeyA, Modifiers::SHIFT, TestAction::Shoot, InputContext::Primary);
        mapper.bind_key_with_mods(KeyCode::KeyA, Modifiers::CTRL, TestAction::Save, InputContext::Primary);

        mapper.set_context(InputContext::Primary);

        assert_eq!(mapper.map_key(KeyCode::KeyA, Modifiers::NONE), Some(TestAction::Jump));
        assert_eq!(mapper.map_key(KeyCode::KeyA, Modifiers::SHIFT), Some(TestAction::Shoot));
        assert_eq!(mapper.map_key(KeyCode::KeyA, Modifiers::CTRL), Some(TestAction::Save));
    }

    //=====================================================================
    // Context Tests
    //=====================================================================

    /// Tests that same key can trigger different actions in different contexts.
    #[test]
    fn context_priority() {
        let mut mapper = ActionMapper::<TestAction>::new();

        let gameplay = InputContext::Primary;
        let menu = InputContext::custom(0);

        // Bind Space differently in two contexts
        mapper.bind_key(KeyCode::Space, TestAction::Jump, gameplay);
        mapper.bind_key(KeyCode::Space, TestAction::Shoot, menu);

        let event = key_down(KeyCode::Space);

        // Gameplay context → Jump
        mapper.set_context(gameplay);
        assert_eq!(mapper.map_event(&event), Some(TestAction::Jump));

        // Menu context → Shoot
        mapper.set_context(menu);
        assert_eq!(mapper.map_event(&event), Some(TestAction::Shoot));
    }

    /// Verifies that clear_context removes all bindings for that context only.
    #[test]
    fn clear_context() {
        let mut mapper = ActionMapper::<TestAction>::new();

        let gameplay = InputContext::Primary;
        let menu = InputContext::custom(0);

        // Bind keys in both contexts
        mapper.bind_key(KeyCode::Space, TestAction::Jump, gameplay);
        mapper.bind_key(KeyCode::KeyS, TestAction::Shoot, gameplay);
        mapper.bind_key(KeyCode::KeyE, TestAction::Save, menu);

        // Clear gameplay context
        mapper.clear_context(gameplay);

        // Gameplay bindings gone
        mapper.set_context(gameplay);
        assert_eq!(mapper.map_event(&key_down(KeyCode::Space)), None);
        assert_eq!(mapper.map_event(&key_down(KeyCode::KeyS)), None);

        // Menu binding still exists
        mapper.set_context(menu);
        assert_eq!(mapper.map_event(&key_down(KeyCode::KeyE)), Some(TestAction::Save));
    }

    /// Ensures clearing an empty context doesn't panic.
    #[test]
    fn clear_empty_context_is_noop() {
        let mut mapper = ActionMapper::<TestAction>::new();
        mapper.clear_context(InputContext::custom(99));
        // Should not panic
    }

    //=====================================================================
    // Unbind Tests
    //=====================================================================

    /// Tests basic unbind functionality (no-modifier variant only).
    #[test]
    fn unbind_key() {
        let mut mapper = ActionMapper::<TestAction>::new();

        mapper.bind_key(KeyCode::Space, TestAction::Jump, InputContext::Primary);

        // Verify binding works
        assert_eq!(mapper.map_event(&key_down(KeyCode::Space)), Some(TestAction::Jump));

        // Unbind
        mapper.unbind_key(KeyCode::Space, InputContext::Primary);

        // Should produce no action
        assert_eq!(mapper.map_event(&key_down(KeyCode::Space)), None);
    }

    /// Verifies that unbind_key only removes no-modifier variant.
    #[test]
    fn unbind_key_removes_only_no_modifier_variant() {
        let mut mapper = ActionMapper::<TestAction>::new();

        mapper.bind_key(KeyCode::Space, TestAction::Jump, InputContext::Primary);
        mapper.bind_key_with_mods(
            KeyCode::Space,
            Modifiers::CTRL,
            TestAction::Save,
            InputContext::Primary
        );

        // Unbind only no-modifier variant
        mapper.unbind_key(KeyCode::Space, InputContext::Primary);

        // No-modifier variant gone
        assert_eq!(mapper.map_event(&key_down(KeyCode::Space)), None);

        // Ctrl variant still exists
        let event_ctrl = key_down_with_mods(KeyCode::Space, Modifiers::CTRL);
        assert_eq!(mapper.map_event(&event_ctrl), Some(TestAction::Save));
    }

    /// Tests unbind_key_all_variants removes all modifier combinations.
    #[test]
    fn unbind_key_all_variants_removes_everything() {
        let mut mapper = ActionMapper::<TestAction>::new();
        let ctx = InputContext::Primary;

        // Bind same key with multiple modifier combinations
        mapper.bind_key(KeyCode::Space, TestAction::Jump, ctx);
        mapper.bind_key_with_mods(KeyCode::Space, Modifiers::SHIFT, TestAction::Shoot, ctx);
        mapper.bind_key_with_mods(KeyCode::Space, Modifiers::CTRL, TestAction::Save, ctx);
        mapper.bind_key_with_mods(KeyCode::Space, Modifiers::SHIFT_CTRL, TestAction::Save, ctx);

        // Remove all variants
        mapper.unbind_key_all_variants(KeyCode::Space, ctx);

        // All gone
        assert_eq!(mapper.map_key(KeyCode::Space, Modifiers::NONE), None);
        assert_eq!(mapper.map_key(KeyCode::Space, Modifiers::SHIFT), None);
        assert_eq!(mapper.map_key(KeyCode::Space, Modifiers::CTRL), None);
        assert_eq!(mapper.map_key(KeyCode::Space, Modifiers::SHIFT_CTRL), None);
    }

    /// Verifies unbind_key_all_variants is context-specific.
    #[test]
    fn unbind_key_all_variants_context_specific() {
        let mut mapper = ActionMapper::<TestAction>::new();
        let ctx1 = InputContext::Primary;
        let ctx2 = InputContext::custom(0);

        // Bind Space in both contexts
        mapper.bind_key(KeyCode::Space, TestAction::Jump, ctx1);
        mapper.bind_key_with_mods(KeyCode::Space, Modifiers::CTRL, TestAction::Save, ctx1);
        mapper.bind_key(KeyCode::Space, TestAction::Shoot, ctx2);

        // Remove only from ctx1
        mapper.unbind_key_all_variants(KeyCode::Space, ctx1);

        // ctx1 bindings gone
        mapper.set_context(ctx1);
        assert_eq!(mapper.map_key(KeyCode::Space, Modifiers::NONE), None);
        assert_eq!(mapper.map_key(KeyCode::Space, Modifiers::CTRL), None);

        // ctx2 binding still exists
        mapper.set_context(ctx2);
        assert_eq!(mapper.map_key(KeyCode::Space, Modifiers::NONE), Some(TestAction::Shoot));
    }

    /// Tests that unbind_key_all_variants doesn't affect other keys.
    #[test]
    fn unbind_key_all_variants_leaves_other_keys() {
        let mut mapper = ActionMapper::<TestAction>::new();
        let ctx = InputContext::Primary;

        mapper.bind_key(KeyCode::Space, TestAction::Jump, ctx);
        mapper.bind_key(KeyCode::KeyW, TestAction::Shoot, ctx);
        mapper.bind_key_with_mods(KeyCode::KeyW, Modifiers::CTRL, TestAction::Save, ctx);

        // Remove all Space variants
        mapper.unbind_key_all_variants(KeyCode::Space, ctx);

        // Space gone
        assert_eq!(mapper.map_key(KeyCode::Space, Modifiers::NONE), None);

        // KeyW bindings untouched
        assert_eq!(mapper.map_key(KeyCode::KeyW, Modifiers::NONE), Some(TestAction::Shoot));
        assert_eq!(mapper.map_key(KeyCode::KeyW, Modifiers::CTRL), Some(TestAction::Save));
    }

    /// Ensures unbinding non-existent key doesn't panic.
    #[test]
    fn unbind_nonexistent_is_noop() {
        let mut mapper = ActionMapper::<TestAction>::new();
        mapper.unbind_key(KeyCode::Space, InputContext::Primary);
        mapper.unbind_key_all_variants(KeyCode::KeyW, InputContext::Primary);
        // Should not panic
    }

    /// Compares behavior of unbind_key vs unbind_key_all_variants.
    #[test]
    fn unbind_key_vs_unbind_key_all_variants() {
        let mut mapper = ActionMapper::<TestAction>::new();
        let ctx = InputContext::Primary;

        // Test unbind_key (removes ONLY no-modifier)
        mapper.bind_key(KeyCode::KeyA, TestAction::Jump, ctx);
        mapper.bind_key_with_mods(KeyCode::KeyA, Modifiers::CTRL, TestAction::Save, ctx);

        mapper.unbind_key(KeyCode::KeyA, ctx);

        assert_eq!(mapper.map_key(KeyCode::KeyA, Modifiers::NONE), None);
        assert_eq!(mapper.map_key(KeyCode::KeyA, Modifiers::CTRL), Some(TestAction::Save)); // Still exists

        // Test unbind_key_all_variants (removes ALL)
        mapper.bind_key(KeyCode::KeyB, TestAction::Jump, ctx);
        mapper.bind_key_with_mods(KeyCode::KeyB, Modifiers::CTRL, TestAction::Save, ctx);

        mapper.unbind_key_all_variants(KeyCode::KeyB, ctx);

        assert_eq!(mapper.map_key(KeyCode::KeyB, Modifiers::NONE), None);
        assert_eq!(mapper.map_key(KeyCode::KeyB, Modifiers::CTRL), None); // Gone too
    }

    //=====================================================================
    // Mouse Tests
    //=====================================================================

    /// Tests basic mouse button binding.
    #[test]
    fn mouse_button_binding() {
        let mut mapper = ActionMapper::<TestAction>::new();

        mapper.bind_mouse(MouseButton::Left, TestAction::Shoot, InputContext::Primary);

        let event = mouse_down(MouseButton::Left);
        assert_eq!(mapper.map_event(&event), Some(TestAction::Shoot));
    }

    /// Tests mouse button with modifiers.
    #[test]
    fn mouse_button_with_modifiers() {
        let mut mapper = ActionMapper::<TestAction>::new();

        // Click = Shoot
        mapper.bind_mouse(MouseButton::Left, TestAction::Shoot, InputContext::Primary);

        // Ctrl+Click = Save
        mapper.bind_mouse_with_mods(
            MouseButton::Left,
            Modifiers::CTRL,
            TestAction::Save,
            InputContext::Primary
        );

        // Test normal click
        let event = mouse_down(MouseButton::Left);
        assert_eq!(mapper.map_event(&event), Some(TestAction::Shoot));

        // Test Ctrl+click
        let event_ctrl_click = mouse_down_with_mods(MouseButton::Left, Modifiers::CTRL);
        assert_eq!(mapper.map_event(&event_ctrl_click), Some(TestAction::Save));
    }

    /// Tests unbind_mouse_all_variants works correctly.
    #[test]
    fn unbind_mouse_all_variants_works() {
        let mut mapper = ActionMapper::<TestAction>::new();
        let ctx = InputContext::Primary;

        mapper.bind_mouse(MouseButton::Left, TestAction::Shoot, ctx);
        mapper.bind_mouse_with_mods(MouseButton::Left, Modifiers::CTRL, TestAction::Save, ctx);

        // Verify exist
        assert_eq!(mapper.map_button(MouseButton::Left, Modifiers::NONE), Some(TestAction::Shoot));
        assert_eq!(mapper.map_button(MouseButton::Left, Modifiers::CTRL), Some(TestAction::Save));

        // Remove all
        mapper.unbind_mouse_all_variants(MouseButton::Left, ctx);

        // All gone
        assert_eq!(mapper.map_button(MouseButton::Left, Modifiers::NONE), None);
        assert_eq!(mapper.map_button(MouseButton::Left, Modifiers::CTRL), None);
    }

    //=====================================================================
    // Edge Cases
    //=====================================================================

    /// Verifies that rebinding same key replaces previous action.
    #[test]
    fn rebinding_replaces_previous() {
        let mut mapper = ActionMapper::<TestAction>::new();

        mapper.bind_key(KeyCode::Space, TestAction::Jump, InputContext::Primary);
        mapper.bind_key(KeyCode::Space, TestAction::Shoot, InputContext::Primary);

        let event = key_down(KeyCode::Space);
        assert_eq!(mapper.map_event(&event), Some(TestAction::Shoot)); // Last wins
    }

    /// Ensures KeyUp events don't produce actions.
    #[test]
    fn ignore_key_up_events() {
        let mut mapper = ActionMapper::<TestAction>::new();

        mapper.bind_key(KeyCode::Space, TestAction::Jump, InputContext::Primary);

        let event = key_up(KeyCode::Space);
        assert_eq!(mapper.map_event(&event), None);
    }

    /// Ensures MouseMoved events don't produce actions.
    #[test]
    fn ignore_mouse_move_events() {
        let mapper = ActionMapper::<TestAction>::new();

        let event = InputEvent::MouseMoved { x: 100.0, y: 200.0 };
        assert_eq!(mapper.map_event(&event), None);
    }
}