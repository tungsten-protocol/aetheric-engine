//=========================================================================
// Global Context
//=========================================================================
//
// Shared data container for scenes.
//
// Contains state data that scenes read/write:
// - input_state: Low-level input state (keys, mouse, modifiers)
// - scene_transitions: Command queue for scene changes
//
//=========================================================================

//=== Internal Dependencies ===============================================

use crate::core::input::{InputEvent, StateTracker};
use crate::core::scene::{SceneKey, TransitionQueue};

//=== GlobalContext =======================================================

/// Shared context data accessible to scenes during updates.
///
/// Scenes receive `&GlobalContext` (or `&mut` where needed) during their
/// lifecycle methods. This separates scene-accessible data from internal
/// engine systems.
///
/// # Available Data
///
/// - `input_state`: Raw input state (keys pressed/down/released, mouse)
/// - `scene_transitions`: Queue for requesting scene changes
/// - `frame_events`: Current frame's input events (internal, processed by systems)
pub struct GlobalContext<S: SceneKey> {
    /// Raw input state tracker for low-level input queries.
    ///
    /// Provides direct access to keyboard, mouse, and modifier state.
    /// For high-level action mapping, use InputSystem in GlobalSystems.
    pub input_state: StateTracker,

    /// Transition queue for scene changes.
    ///
    /// Scenes queue transitions here during updates. The scene manager
    /// processes this queue at tick boundaries.
    pub scene_transitions: TransitionQueue<S>,

    /// Input events for the current frame.
    ///
    /// Populated by the platform thread and consumed by InputSystem during
    /// the update phase. Cleared after processing. Not directly accessible
    /// to scenes (use `input_state` instead).
    pub(crate) frame_events: Vec<Vec<InputEvent>>,
}

impl<S: SceneKey> GlobalContext<S> {
    /// Creates a new context with empty state.
    pub(crate) fn new() -> Self {
        Self {
            input_state: StateTracker::new(),
            scene_transitions: TransitionQueue::new(),
            frame_events: Vec::new(),
        }
    }
}
