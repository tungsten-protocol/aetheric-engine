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

/// Container for all engine-level systems and resources.
///
/// Provides access to the input system and other game systems. Exposed during
/// engine initialization via [`crate::Engine::init`] for configuration before
/// the engine starts running.
///
/// # Usage
///
/// Access resources during initialization to configure systems:
/// ```no_run
/// use aetheric_engine::EngineBuilder;
/// use aetheric_engine::core::input::{Action, KeyCode, InputContext};
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// enum GameAction { Jump, Shoot }
/// impl Action for GameAction {}
///
/// EngineBuilder::<GameAction>::new()
///     .build()
///     .init(|resources| {
///         // Configure input bindings
///         resources.input.bind_key(
///             KeyCode::Space,
///             GameAction::Jump,
///             InputContext::Primary
///         );
///         resources.input.bind_key(
///             KeyCode::KeyF,
///             GameAction::Shoot,
///             InputContext::Primary
///         );
///     })
///     .run();
/// ```
///
/// # Available Systems
///
/// - `input`: The [`InputSystem`] for binding and querying input
///
/// Future planned systems: ECS, physics, AI, audio, rendering.
pub struct GlobalResources<A: Action> {
    /// The input system for managing key bindings and querying input state.
    ///
    /// Use this to bind keys/buttons to actions and query input each frame.
    pub input: InputSystem<A>,
}

impl<A: Action> GlobalResources<A> {
    /// Creates a new resources container with the given input system.
    ///
    /// This is typically called internally by the engine. Users should access
    /// resources via [`crate::Engine::init`] instead.
    pub fn new(input_system: InputSystem<A>) -> Self {
        Self { input: input_system }
    }
}
