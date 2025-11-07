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

mod platform;
mod engine;

//=== Public API ==========================================================

pub use core::{GlobalResources, InputSystem};
pub use engine::{Engine, EngineBuilder};
