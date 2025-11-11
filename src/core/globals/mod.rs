//=========================================================================
// Global Engine State
//=========================================================================
//
// Separates systems (logic components) from context (shared data).
//
// Architecture:
//   GlobalSystems: InputSystem + SceneManager (owned by orchestrator)
//   GlobalContext: StateTracker + TransitionQueue (passed to scenes)
//
//=========================================================================

//=== Module Declarations =================================================

mod global_context;
mod global_systems;

//=== Public API ==========================================================

pub use global_context::GlobalContext;
pub use global_systems::GlobalSystems;
