//=========================================================================
// Global Context
//=========================================================================
//
// Shared data container for scenes.
//
// Contains state data that scenes read/write:
// - input_state: Low-level input state (keys, mouse, modifiers)
// - message_bus: Universal message queue (actions, events, transitions)
//
//=========================================================================

//=== Internal Dependencies ===============================================

use crate::core::input::{InputEvent, StateTracker};
use crate::core::message_bus::MessageBus;

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
/// - `message_bus`: Multi-consumer message queue (actions, events, scene transitions)
/// - `frame_events`: Current frame's input events (internal, processed by systems)
pub struct GlobalContext {
    /// Raw input state tracker for low-level input queries.
    ///
    /// Provides direct access to keyboard, mouse, and modifier state.
    /// For high-level action mapping, read actions from message_bus.
    pub input_state: StateTracker,

    /// Message bus for inter-system communication.
    ///
    /// Systems publish messages (e.g., actions, scene transitions) and
    /// scenes/systems read them. Supports multi-consumer pattern: multiple
    /// readers can process the same message in a single frame.
    ///
    /// Actions are published here each frame after input processing.
    /// Scene transitions are published by scenes and processed by SceneManager.
    pub message_bus: MessageBus,

    /// Input events for the current frame.
    ///
    /// Populated by the platform thread and consumed by InputSystem during
    /// the update phase. Cleared after processing. Not directly accessible
    /// to scenes (use `input_state` instead).
    pub(crate) frame_input_events: Vec<Vec<InputEvent>>,
}

impl GlobalContext {
    /// Creates a new context with empty state.
    pub(crate) fn new() -> Self {
        Self {
            input_state: StateTracker::new(),
            message_bus: MessageBus::new(),
            frame_input_events: Vec::new(),
        }
    }
}
