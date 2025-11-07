//=========================================================================
// World Resources
//=========================================================================
//
// World-level resources container.
//
// Contains all systems and resources shared across the game world (input,
// ECS, physics, AI, etc.). Currently holds only the input system.
//
//=========================================================================

//=== Internal Dependencies ===============================================

use crate::core::input::{Action, InputSystem};

/// Container for world-level resources (input system, etc.).
pub struct WorldResources<A: Action> {
    pub input: InputSystem<A>,
}

impl<A: Action> WorldResources<A> {
    pub fn new(input_system: InputSystem<A>) -> Self {
        Self { input: input_system }
    }
}
