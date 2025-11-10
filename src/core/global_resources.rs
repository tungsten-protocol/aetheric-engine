//=========================================================================
// Global Resources
//=========================================================================
//
// Engine-level resources container.
//
// Contains all systems and resources shared across the game engine (input,
// ECS, physics, AI, etc.). Currently holds only the input system.
//
//=========================================================================

//=== Internal Dependencies ===============================================

use crate::core::input::{Action, InputSystem};
use crate::core::scene::{SceneKey, TransitionQueue};

/// Container for all engine-level systems and resources.
///
/// Provides access to the input system and shared resources accessible
/// from scene updates. Exposed during engine initialization via
/// [`crate::Engine::init`] for configuration before the engine starts running.
///
/// # Available Systems
///
/// - `input`: The [`InputSystem`] for binding and querying input
/// - `scene_transitions`: Queue for scene transition requests
///
/// Future planned systems: ECS, physics, AI, audio, rendering.
pub struct GlobalResources<S: SceneKey, A: Action> {
    /// The input system for managing key bindings and querying input state.
    ///
    /// Use this to bind keys/buttons to actions and query input each frame.
    pub input: InputSystem<A>,

    /// Transition queue for scene changes.
    ///
    /// Scenes queue transitions here during updates. The scene manager
    /// processes this queue at tick boundaries.
    pub scene_transitions: TransitionQueue<S>,
}

impl<S: SceneKey, A: Action> GlobalResources<S, A> {
    /// Creates a new resources container with the given input system.
    ///
    /// This is typically called internally by the engine. Users should access
    /// resources via [`crate::Engine::init`] instead.
    pub fn new(input_system: InputSystem<A>) -> Self {
        Self {
            input: input_system,
            scene_transitions: TransitionQueue::new(),
        }
    }
}
