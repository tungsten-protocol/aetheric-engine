//=========================================================================
// Core Systems Orchestrator
//=========================================================================
//
// Central coordinator for engine subsystems running on the logic thread.
//
// Runs independently at fixed TPS, receiving platform events via MPSC
// and updating all core systems (input, physics, AI, etc.).
//
// Thread Model:
//   Platform Thread ──(MPSC)──► Core Thread ──► Systems
//
//=========================================================================

//=== External Dependencies ===============================================

use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::Receiver;
use log::{info, warn};

//=== Module Declarations =================================================

pub mod input;
pub mod global_resources;

pub(crate) mod platform_bridge;

//=== Public API ==========================================================

pub use input::{Action, InputSystem};
pub use global_resources::GlobalResources;

//=== Internal Dependencies ===============================================

use platform_bridge::{EventCollector, PlatformEvent, TickControl};

//=== CoreSystemsOrchestrator =============================================

/// Manages the lifetime and update scheduling of all engine core systems.
///
/// Runs at fixed timestep for deterministic simulation, independent of
/// platform frame rate. Communicates via message passing only.
pub(crate) struct CoreSystemsOrchestrator<A: Action> {
    ctx: GlobalResources<A>,
}

impl<A: Action> CoreSystemsOrchestrator<A> {
    //--- Construction -----------------------------------------------------

    pub(crate) fn new(input_system: InputSystem<A>) -> Self {
        Self { ctx: GlobalResources::new(input_system) }
    }

    //--- Resource Initialization ------------------------------------------

    /// Allows external initialization of resources before spawning core thread.
    ///
    /// Provides mutable access to `GlobalResources` for configuration
    /// (input bindings, system setup, etc.) via a closure.
    pub(crate) fn init_resources<F>(&mut self, init_fn: F)
    where
        F: FnOnce(&mut GlobalResources<A>),
    {
        init_fn(&mut self.ctx);
    }

    //--- Thread Lifecycle -------------------------------------------------

    /// Spawns the main logic thread running at fixed TPS.
    ///
    /// Thread exits on `WindowClosed` event or channel disconnect.
    ///
    /// # Panics
    /// Panics if `tps <= 0.0`.
    pub(crate) fn spawn_core_thread(
        mut self,
        receiver: Receiver<PlatformEvent>,
        tps: f64
    ) -> thread::JoinHandle<()> {
        assert!(tps > 0.0, "TPS must be positive, got {}", tps);

        let frame_duration = Duration::from_secs_f64(1.0 / tps);

        thread::spawn(move || {
            self.run_loop(receiver, frame_duration);
        })
    }

    fn run_loop(&mut self, receiver: Receiver<PlatformEvent>, frame_duration: Duration) {
        let mut event_collector = EventCollector::new(receiver);

        loop {
            let frame_start = Instant::now();

            // --- input collection phase ---
            if event_collector.collect_frame() == TickControl::Exit {
                info!("Core thread exiting cleanly.");
                break;
            }

            // --- input update phase ---
            self.ctx.input.process_frame(event_collector.batches());

            // --- systems update phase ---
            self.update_systems();

            // --- frame pacing ---
            Self::maintain_frame_rate(frame_start, frame_duration);
        }
    }

    //--- System Updates ---------------------------------------------------
    fn update_systems(&mut self) {
        Self::prova_system(&self.ctx);

        // Future: self.physics_system.update(&mut ctx);
    }

    fn prova_system(ctx: &GlobalResources<A>){
        for action in ctx.input.actions(){
            println!("{:?}", action);
        }
    }

    //--- Frame Pacing -----------------------------------------------------

    fn maintain_frame_rate(frame_start: Instant, frame_duration: Duration) {
        let elapsed = frame_start.elapsed();

        if elapsed >= frame_duration {
            warn!(
                "Core thread slow: {:.2}ms (target: {:.2}ms)",
                elapsed.as_secs_f64() * 1000.0,
                frame_duration.as_secs_f64() * 1000.0
            );
        } else {
            thread::sleep(frame_duration - elapsed);
        }
    }
}

//=========================================================================
// Unit Tests
//=========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::unbounded;
    use crate::core::input::event::{KeyCode, Modifiers, InputEvent};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum TestAction {
        Jump,
    }

    impl Action for TestAction {}

    //--- Thread Lifecycle -------------------------------------------------

    #[test]
    fn spawn_core_thread_exits_on_window_closed() {
        let (tx, rx) = unbounded();
        let orchestrator = CoreSystemsOrchestrator::new(InputSystem::<TestAction>::new());
        let handle = orchestrator.spawn_core_thread(rx, 60.0);

        tx.send(PlatformEvent::WindowClosed).unwrap();

        assert!(handle.join().is_ok());
    }

    #[test]
    fn spawn_core_thread_exits_on_channel_disconnect() {
        let (tx, rx) = unbounded();
        let orchestrator = CoreSystemsOrchestrator::new(InputSystem::<TestAction>::new());
        let handle = orchestrator.spawn_core_thread(rx, 60.0);

        drop(tx);

        assert!(handle.join().is_ok());
    }

    //--- Panics -----------------------------------------------------------

    #[test]
    #[should_panic(expected = "TPS must be positive, got 0")]
    fn spawn_panics_on_zero_tps() {
        let (_, rx) = unbounded();
        let orchestrator = CoreSystemsOrchestrator::new(InputSystem::<TestAction>::new());
        orchestrator.spawn_core_thread(rx, 0.0);
    }

    #[test]
    #[should_panic(expected = "TPS must be positive, got -10")]
    fn spawn_panics_on_negative_tps() {
        let (_, rx) = unbounded();
        let orchestrator = CoreSystemsOrchestrator::new(InputSystem::<TestAction>::new());
        orchestrator.spawn_core_thread(rx, -10.0);
    }
}