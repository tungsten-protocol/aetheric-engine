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

pub mod globals;
pub mod input;
pub mod message_bus;
pub mod scene;

pub(crate) mod platform_bridge;

//=== Public API ==========================================================

pub use input::{Action, InputSystem};
pub use globals::{GlobalContext, GlobalSystems};
pub use scene::{SceneKey, SceneManager};

//=== Internal Dependencies ===============================================

use platform_bridge::{EventCollector, PlatformEvent, TickControl};

//=== CoreSystemsOrchestrator =============================================

/// Manages the lifetime and update scheduling of all engine core systems.
///
/// Runs at fixed timestep for deterministic simulation, independent of
/// platform frame rate. Communicates via message passing only.
pub(crate) struct CoreSystemsOrchestrator<S: SceneKey, A: Action> {
    context: GlobalContext,
    systems: GlobalSystems<S, A>,
}

impl<S: SceneKey, A: Action> CoreSystemsOrchestrator<S, A> {
    //--- Construction -----------------------------------------------------

    pub(crate) fn new() -> Self {
        Self {
            context: GlobalContext::new(),
            systems: GlobalSystems::new(),
        }
    }

    //--- Resource Initialization ------------------------------------------

    /// Allows external initialization of systems before spawning core thread.
    ///
    /// Provides mutable access to `GlobalSystems` for configuration
    /// (input bindings, scene registration, etc.) via a closure.
    pub(crate) fn init_systems<F>(&mut self, init_fn: F)
    where
        F: FnOnce(&mut GlobalSystems<S, A>),
    {
        init_fn(&mut self.systems);
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

        // Initialize scene manager by calling on_enter for initial scenes
        self.systems.scene_manager.start(&self.context);

        loop {
            let frame_start = Instant::now();

            // Collect events from platform thread
            if event_collector.collect_frame() == TickControl::Exit {
                info!("Core thread exiting cleanly.");
                break;
            }

            // Transfer events to context
            self.context.frame_input_events = event_collector.take_batches();

            // Update all systems (input, scenes, transitions)
            self.systems.update(&mut self.context);

            // Frame pacing
            Self::maintain_frame_rate(frame_start, frame_duration);
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
    enum TestScene {
        Main,
    }

    impl SceneKey for TestScene {}

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum TestAction {
        Jump,
    }

    impl Action for TestAction {}

    //--- Thread Lifecycle -------------------------------------------------

    #[test]
    fn spawn_core_thread_exits_on_window_closed() {
        let (tx, rx) = unbounded();
        let orchestrator = CoreSystemsOrchestrator::<TestScene, TestAction>::new();
        let handle = orchestrator.spawn_core_thread(rx, 60.0);

        tx.send(PlatformEvent::WindowClosed).unwrap();

        assert!(handle.join().is_ok());
    }

    #[test]
    fn spawn_core_thread_exits_on_channel_disconnect() {
        let (tx, rx) = unbounded();
        let orchestrator = CoreSystemsOrchestrator::<TestScene, TestAction>::new();
        let handle = orchestrator.spawn_core_thread(rx, 60.0);

        drop(tx);

        assert!(handle.join().is_ok());
    }

    //--- Panics -----------------------------------------------------------

    #[test]
    #[should_panic(expected = "TPS must be positive, got 0")]
    fn spawn_panics_on_zero_tps() {
        let (_, rx) = unbounded();
        let orchestrator = CoreSystemsOrchestrator::<TestScene, TestAction>::new();
        orchestrator.spawn_core_thread(rx, 0.0);
    }

    #[test]
    #[should_panic(expected = "TPS must be positive, got -10")]
    fn spawn_panics_on_negative_tps() {
        let (_, rx) = unbounded();
        let orchestrator = CoreSystemsOrchestrator::<TestScene, TestAction>::new();
        orchestrator.spawn_core_thread(rx, -10.0);
    }
}