//=========================================================================
// Aetheric Engine
//=========================================================================
//
// High-performance game engine with ECS and cross-platform support.
//
// Main entry: Engine::new() or EngineBuilder::new()
//
//=========================================================================

//=== Module Declarations =================================================

pub mod core;
pub mod prelude;

mod platform;
mod engine;

//=== Public API ==========================================================

pub use core::{GlobalContext, GlobalSystems, InputSystem};
pub use engine::{Engine, EngineBuilder};
