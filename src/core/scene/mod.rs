//=========================================================================
// Scene System
//=========================================================================
//
// Manages scene lifecycle and stack-based scene switching.
//
// Architecture:
//   SceneManager
//     ├─ scenes: HashMap<S, Box<dyn Scene>>
//     └─ stack: Vec<S>
//
// Flow:
//   update() → collect_active_scenes() → Scene::update()
//
//=========================================================================

//=== Internal Dependencies ===============================================

use crate::core::globals::GlobalContext;

//=== Module Declarations =================================================

mod scene_manager;

//=== Public API ==========================================================

pub use scene_manager::{SceneKey, SceneManager, SceneTransition};

//=== Scene Trait =========================================================

/// Defines scene behavior with lifecycle hooks and update logic.
///
/// Scenes are registered in SceneManager and activated via scene stack.
/// Each scene maintains its own state between activations.
///
/// # Minimal Implementation
///
/// Only `update()` is required. Lifecycle hooks have default empty implementations:
///
/// ```rust
/// # use aetheric_engine::prelude::*;
/// # #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// # enum GameScene { Main }
/// # impl SceneKey for GameScene {}
/// struct MyScene;
///
/// impl Scene<GameScene> for MyScene {
///     fn update(&mut self, context: &GlobalContext) {
///         // Only this method is required
///     }
/// }
/// ```
pub trait Scene<S: SceneKey>: Send {
    /// Called when scene enters the active stack.
    ///
    /// Default implementation does nothing. Override to initialize scene state.
    fn on_enter(&mut self, _context: &GlobalContext) {}

    /// Called when scene exits the active stack.
    ///
    /// Default implementation does nothing. Override to cleanup scene state.
    fn on_exit(&mut self, _context: &GlobalContext) {}

    /// Called every tick while scene is active on stack.
    fn update(&mut self, context: &GlobalContext);

    /// Whether scenes below this one should receive updates.
    ///
    /// Transparent scenes (e.g., pause menus) allow underlying scenes
    /// to continue updating. Opaque scenes block updates to lower stack.
    fn is_transparent(&self) -> bool {
        false
    }
}
