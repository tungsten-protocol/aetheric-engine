//=========================================================================
// Aetheric Engine — Library Root
//
// Defines the public API surface of the Aetheric Engine crate.
//
// Responsibilities:
// - Expose the main Engine interface
// - Keep platform and internal systems private
// - Maintain clear separation between high-level façade
//   and low-level subsystems (core logic, OS integration)
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

//=== Public Modules ======================================================
//
// The `core` module hosts all fundamental engine systems (input,
// ECS, etc.). It is public for advanced engine-level extensions,
// but typical applications will interact primarily through `Engine`.
//
pub mod core;

//=== Internal Modules ====================================================
//
// - `platform`: OS integration layer (windowing, Winit, I/O)
// - `engine`:   main entry point and initialization logic
//
// Both are internal and not exposed as part of the public API.
//
mod platform;
mod engine;

//=== Public Exports ======================================================
//
// Re-exports the `Engine` façade as the canonical entry point.
// This allows application code to simply:
//     use aetheric::Engine;
// without referencing internal module paths.
//
pub use engine::Engine;
