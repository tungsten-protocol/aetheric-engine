//=========================================================================
// Core Systems Orchestrator
//
// Central coordinator for all engine subsystems running on the logic
// (non-platform) thread.
//
// Responsibilities:
// - Manage and update all core systems (e.g. Input, Physics, AI)
// - Receive and process platform events via MPSC channel
// - Maintain deterministic pacing using a fixed tick rate (TPS)
// - Provide the execution backbone for simulation and game logic
//
// Notes:
// The orchestrator runs independently from the platform layer.
// It owns each subsystem directly and updates them at a fixed rate
// in a background thread. Communication with the platform occurs only
// through message passing (MPSC), ensuring isolation and thread safety.
//
//=========================================================================

//=== Standard Library Imports ============================================
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

//=== External Crates =====================================================
use log::info;

//=== Internal Modules ====================================================
use crate::platform::PlatformEvent;
use input::InputSystem;
use crate::core::input::event::RawInputEvent;

pub mod input;

//=== TickControl =========================================================
//
// Defines control flow for the core update loop.
// Each system tick can signal either to continue or terminate the loop.
//
pub(crate) enum TickControl {
    Continue,
    Exit,
}

//=== CoreSystemsOrchestrator =============================================
//
// Manages the lifetime and update scheduling of all engine core systems.
// Currently orchestrates only the InputSystem, but is designed to scale
// to additional modules (physics, AI, simulation, etc.).
//
pub(crate) struct CoreSystemsOrchestrator {
    input_batches: Vec<Vec<RawInputEvent>>,
    input_system: InputSystem,
}

impl CoreSystemsOrchestrator {
    //--- Construction -----------------------------------------------------
    //
    // Initializes all core systems but does not yet start the logic thread.
    //
    pub fn new() -> Self {
        Self {
            input_system: InputSystem::new(),
            input_batches: Vec::with_capacity(8),
        }
    }

    //--- spawn_core_thread() ---------------------------------------------
    //
    // Spawns the main logic thread responsible for ticking all core systems
    // at a fixed update frequency (TPS - ticks per second).
    //
    // Each tick:
    //  1. Collects and processes input events
    //  2. Updates registered subsystems
    //  3. Sleeps to maintain fixed pacing
    //  4. Exits cleanly when a shutdown signal is received
    //
    pub fn spawn_core_thread(
        self,
        receiver: Receiver<PlatformEvent>,
        tps: f64,
    ) -> thread::JoinHandle<()> {
        let frame_duration = Duration::from_secs_f64(1.0 / tps);

        thread::spawn(move || {
            let mut input_system = self.input_system;
            let mut input_batches = self.input_batches;

            loop {
                let frame_start = Instant::now();

                //--- Step 1: Gather platform events ------------------------
                if let TickControl::Exit =
                    Self::collect_platform_events(&receiver, &mut input_batches, frame_duration)
                {
                    info!("Core thread exiting.");
                    break;
                }

                //--- Step 2: Update subsystems -----------------------------
                input_system.update(&mut input_batches);

                //--- Step 3: Maintain deterministic pacing ----------------
                let elapsed = frame_start.elapsed();
                if elapsed < frame_duration {
                    thread::sleep(frame_duration - elapsed);
                }
            }
        })
    }

    //--- collect_platform_events() ---------------------------------------
    //
    // Aggregates all input events received from the platform during this frame.
    // Returns a TickControl indicating whether to continue or exit.
    //
    fn collect_platform_events(
        receiver: &Receiver<PlatformEvent>,
        input_batches: &mut Vec<Vec<RawInputEvent>>,
        frame_duration: Duration,
    ) -> TickControl {
        input_batches.clear();

        // Wait for at least one event this frame
        match receiver.recv_timeout(frame_duration) {
            Ok(PlatformEvent::Inputs(batch)) => input_batches.push(batch),
            Ok(PlatformEvent::WindowClosed) => return TickControl::Exit,
            Err(RecvTimeoutError::Disconnected) => return TickControl::Exit,
            Err(RecvTimeoutError::Timeout) => {}
        }

        // Drain additional events queued during this frame
        while let Ok(event) = receiver.try_recv() {
            match event {
                PlatformEvent::Inputs(batch) => input_batches.push(batch),
                PlatformEvent::WindowClosed => return TickControl::Exit,
            }
        }

        TickControl::Continue
    }
}
