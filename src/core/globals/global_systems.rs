//=========================================================================
// Global Systems
//=========================================================================
//
// Container for engine-level systems with logic.
//
// Contains systems that process input, manage scenes, and coordinate
// game logic. Systems operate on GlobalContext data.
//
//=========================================================================

//=== Internal Dependencies ===============================================

use super::GlobalContext;
use crate::core::input::{Action, InputSystem};
use crate::core::scene::{SceneKey, SceneManager};

//=== GlobalSystems =======================================================

/// Container for engine-level logic systems.
///
/// Holds systems that process data and coordinate engine behavior.
/// These systems operate on the shared GlobalContext during engine updates.
///
/// # Available Systems
///
/// - `input`: High-level input system with action mapping
/// - `scene_manager`: Stack-based scene lifecycle manager
pub struct GlobalSystems<S: SceneKey, A: Action> {
    /// The input system for action mapping and input processing.
    ///
    /// Processes raw input state from GlobalContext and generates
    /// high-level actions based on configured key bindings.
    pub input: InputSystem<A>,

    /// The scene manager for scene lifecycle and stack management.
    ///
    /// Manages scene registration, activation, updates, and transitions.
    /// Processes scene transition queue from GlobalContext.
    pub scene_manager: SceneManager<S>,
}

impl<S: SceneKey, A: Action> GlobalSystems<S, A> {
    /// Creates a new systems container with default-initialized systems.
    ///
    /// This is typically called internally by the engine. Users should access
    /// systems via [`crate::Engine::init`] instead.
    pub(crate) fn new() -> Self {
        Self {
            input: InputSystem::new(),
            scene_manager: SceneManager::new(),
        }
    }

    //--- Update Loop ------------------------------------------------------

    /// Updates all engine systems for the current frame.
    ///
    /// Processes input events, publishes actions to message bus, updates
    /// active scenes, and handles scene transitions. Called by
    /// CoreSystemsOrchestrator each tick.
    ///
    /// # Processing Pipeline
    ///
    /// 1. **Input Processing**: Converts platform events to input state and actions
    /// 2. **Action Publishing**: Clears stale actions, publishes fresh actions to message bus
    /// 3. **Scene Update**: Updates all active scenes with current context
    /// 4. **Transition Processing**: Applies queued scene transitions
    ///
    /// # Arguments
    ///
    /// * `context` - Shared context containing input state, message bus, events, and transition queue
    pub(crate) fn update(&mut self, context: &mut GlobalContext) {
        // 1. Process input events into state and actions
        self.input.process_frame(
            &mut context.input_state,
            &context.frame_input_events
        );
        context.frame_input_events.clear();

        // 2. Clear previous frame's actions and publish fresh ones
        context.message_bus.clear::<A>();
        for action in self.input.actions() {
            context.message_bus.push(*action);
        }

        // 3. Update active scenes (can read actions from message bus)
        self.scene_manager.update(context);

        // 4. Process scene transitions
        self.scene_manager.process_transitions(context);
    }
}
