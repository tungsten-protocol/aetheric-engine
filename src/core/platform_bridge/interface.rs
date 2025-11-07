//=========================================================================
// Platform Bridge Interface
//=========================================================================
//
// Platform-to-core interface types (events and errors).
//
// Defines the contract for communication between platform and core threads.
//
//=========================================================================

//=== Internal Dependencies ===============================================

use crate::core::input::event::InputEvent;

//=== PlatformEvent =======================================================

/// Events sent from platform to core via MPSC.
#[derive(Debug, Clone)]
pub(crate) enum PlatformEvent {
    /// Batched input events for a frame.
    Inputs {
        discrete: Vec<InputEvent>,
        continuous: Vec<InputEvent>,
    },

    /// Window close requested.
    WindowClosed,
}

//=== PlatformError =======================================================

/// Platform initialization and runtime errors.
#[derive(Debug)]
pub(crate) enum PlatformError {
    /// Event loop creation failed (OS-level issue).
    EventLoopCreation(String),

    /// Event loop execution error.
    EventLoopExecution(String),
}

impl std::fmt::Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EventLoopCreation(e) => write!(f, "Event loop creation failed: {}", e),
            Self::EventLoopExecution(e) => write!(f, "Event loop error: {}", e),
        }
    }
}

impl std::error::Error for PlatformError {}