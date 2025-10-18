//=========================================================================
// Engine
//
// Entry point for the Aetheric Engine.
//
// Responsibilities:
// - Initialize and own all core subsystems (input, logic, etc.)
// - Spawn the core runtime thread (logic + input processing)
// - Instantiate and delegate control to the platform layer
// - Maintain deterministic pacing at a fixed tick rate (TPS)
//
// Notes:
// The Engine acts as the root coordinator. It delegates:
//   • Input and logic updates → CoreSystemsOrchestrator (logic thread)
//   • Platform integration → Platform subsystem (main thread)
//
// Communication between these two layers is asynchronous via MPSC,
// ensuring full isolation, thread safety, and zero shared-state locking.
//
//=========================================================================

//=== Standard Library Imports ============================================
use std::sync::mpsc::{channel, Sender, Receiver};

//=== Internal Modules ====================================================
use crate::core::{input, CoreSystemsOrchestrator};
use crate::platform::{Platform, PlatformEvent};

//=== Public Re-exports ===================================================
pub use input::InputSystem;

//=== Engine Struct =======================================================
//
// Main façade of the Aetheric Engine.
//
// Owns the core systems orchestrator, configures timing, and drives
// both the core (logic) and platform (window/input) threads.
//
pub struct Engine {
    orchestrator: CoreSystemsOrchestrator,
    tps: f64,
}

impl Engine {
    //--- Construction -----------------------------------------------------
    //
    // Creates a new Engine instance with default settings.
    // Core systems are initialized, but no threads are started yet.
    //
    pub fn new() -> Self {
        Self {
            orchestrator: CoreSystemsOrchestrator::new(),
            tps: 60.0,
        }
    }

    //--- run() ------------------------------------------------------------
    //
    // Starts the engine runtime and blocks until the application exits.
    //
    // Sequence:
    //  1. Creates an MPSC channel for platform → engine communication.
    //  2. Spawns the core thread that runs logic and input updates at `tps`.
    //  3. Creates and runs the Platform (e.g. Winit event loop).
    //
    // Once the Platform exits (e.g. window closed), the core thread
    // will terminate automatically.
    //
    // Example:
    // ```no_run
    // fn main() {
    //     aetheric::Engine::new().run();
    // }
    // ```
    pub fn run(self) {
        //--- 1. Create communication channel -----------------------------
        let (tx, rx): (Sender<PlatformEvent>, Receiver<PlatformEvent>) = channel();

        //--- 2. Spawn the core logic thread -------------------------------
        self.orchestrator.spawn_core_thread(rx, self.tps);

        //--- 3. Launch the platform subsystem -----------------------------
        let mut platform = Platform::new(tx);
        platform.run(); // Blocks until window close
    }
}
