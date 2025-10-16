//=========================================================================
// Aetheric Engine — Library Root
//
// This crate defines the public API surface of the Aetheric Engine.
//
// Responsibilities:
// - Expose the core engine interface (`Engine`)
// - Keep internal modules (like `platform`) hidden from end users
// - Provide clean separation between the high-level engine facade
//   and lower-level subsystems (input, rendering, OS integration)
//
// Typical usage:
// ```no_run
// use aetheric::Engine;
//
// fn main() {
//     Engine::new().run();
// }
// ```
//
//=========================================================================

//--- Public Modules ------------------------------------------------------
//
// `core` contains all internal engine systems and logic (input, ECS, etc.)
// It is exposed publicly for engine-level extensibility, but normal
// application code will mostly use the top-level `Engine` facade.
//
pub mod core;

//--- Internal Modules ----------------------------------------------------
//
// `platform` contains OS-specific logic (window, Winit integration,
// event loop, etc.) and is kept private, as it is not part of the
// public API surface.
//
// `engine` defines the main engine entry point and initialization logic.
//
mod platform;
mod engine;

//--- Public Exports ------------------------------------------------------
//
// Re-exports the `Engine` struct as the main entry point for applications.
// This allows users to simply `use aetheric::Engine;` without having to
// know the internal module structure.
//
pub use engine::Engine;
