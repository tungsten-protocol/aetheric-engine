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

use crate::core::input::Action;
use crate::core::global_resources::GlobalResources;

//=== Module Declarations =================================================

mod manager;
mod transition_queue;

//=== Public API ==========================================================

pub use manager::{SceneKey, SceneManager, SceneTransition};
pub use transition_queue::TransitionQueue;

//=== Scene Trait =========================================================

/// Defines scene behavior with lifecycle hooks and update logic.
///
/// Scenes are registered in SceneManager and activated via scene stack.
/// Each scene maintains its own state between activations.
pub trait Scene<S: SceneKey, A: Action>: Send {
    /// Called when scene enters the active stack.
    fn on_enter(&mut self, globals: &GlobalResources<S, A>);

    /// Called when scene exits the active stack.
    fn on_exit(&mut self, globals: &GlobalResources<S, A>);

    /// Called every tick while scene is active on stack.
    fn update(&mut self, globals: &GlobalResources<S, A>);

    /// Whether scenes below this one should receive updates.
    ///
    /// Transparent scenes (e.g., pause menus) allow underlying scenes
    /// to continue updating. Opaque scenes block updates to lower stack.
    fn is_transparent(&self) -> bool {
        false
    }
}
