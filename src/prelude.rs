//=========================================================================
// Prelude
//=========================================================================
//
// Convenience module that re-exports commonly used types and traits.
//
// Usage:
//   use aetheric_engine::prelude::*;
//
//=========================================================================

//=== Public API ==========================================================

// Engine core
pub use crate::engine::{Engine, EngineBuilder};

// Global systems and context
pub use crate::core::globals::{GlobalContext, GlobalSystems};

// Input system
pub use crate::core::input::{Action, InputContext, KeyCode, Modifiers, MouseButton};

// Scene system
pub use crate::core::scene::{Scene, SceneKey, SceneTransition};

// Message bus
pub use crate::core::message_bus::MessageBus;
