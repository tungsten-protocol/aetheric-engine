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
// Architecture:
// The orchestrator runs independently from the platform layer in a
// dedicated background thread. It owns each subsystem directly and
// updates them at a fixed rate (TPS). Communication with the platform
// occurs exclusively through message passing (MPSC), ensuring isolation
// and thread safety.
//
// Thread Model:
//   Platform Thread ──┐
//                     │ MPSC Channel
//   Core Thread    ◄──┘
//     │
//     └─► InputSystem
//
//=========================================================================

//=== Standard Library Imports ============================================
use std::thread;
use std::time::{Duration, Instant};

//=== External Crates =====================================================
use log::{info, warn};
use crossbeam_channel::{Receiver, RecvTimeoutError};

//=== Internal Modules ====================================================
use crate::platform::PlatformEvent;
use crate::core::input::event::InputEvent;
use input::{InputSystem, Action};

pub mod input;

//=== TickControl =========================================================
//
// Control flow signal for the core update loop.
//
// Each system tick can signal either to continue or terminate the loop.
// This allows clean shutdown coordination between threads.
//
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TickControl {
    /// Continue running the core loop.
    Continue,

    /// Exit the core loop (shutdown requested).
    Exit,
}

//=== CoreSystemsOrchestrator =============================================
//
// Manages the lifetime and update scheduling of all engine core systems.
//
// Currently orchestrates only the InputSystem, but is designed to scale
// to additional modules (physics, AI, simulation, etc.) without requiring
// architectural changes.
//
// Design Notes:
// - Fixed timestep: All systems tick at a constant rate regardless of
//   platform frame rate or event frequency
// - Message passing: No shared state with platform thread; all data flows
//   through typed messages in a bounded channel
// - Deterministic: Given identical inputs, the simulation produces
//   identical outputs (crucial for networking/replay)
//
pub(crate) struct CoreSystemsOrchestrator<A: Action> {
    /// High-level input abstraction (raw events → game actions)
    input_system: InputSystem<A>,
}

impl<A: Action> CoreSystemsOrchestrator<A> {
    //--- Construction -----------------------------------------------------

    /// Creates a new orchestrator with the given input system.
    ///
    /// Does not start the background thread yet. Call [`spawn_core_thread`]
    /// to begin execution.
    pub(crate) fn new(input_system: InputSystem<A>) -> Self {
        Self { input_system }
    }

    //--- Public API -------------------------------------------------------

    /// Spawns the main logic thread running at fixed TPS.
    ///
    /// The thread runs until:
    /// - A `WindowClosed` event is received
    /// - The channel disconnects (sender dropped)
    ///
    /// Each tick performs the following in order:
    /// 1. Collect platform events (blocking + non-blocking drain)
    /// 2. Update all subsystems with collected data
    /// 3. Sleep to maintain fixed pacing
    ///
    /// # Arguments
    ///
    /// * `receiver` - MPSC receiver for platform events
    /// * `tps` - Target ticks per second (must be > 0)
    ///
    /// # Returns
    ///
    /// A `JoinHandle` that can be used to wait for thread completion.
    ///
    /// # Panics
    ///
    /// Panics if `tps <= 0.0`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let (tx, rx) = crossbeam_channel::unbounded();
    /// let orchestrator = CoreSystemsOrchestrator::new(input_system);
    /// let handle = orchestrator.spawn_core_thread(rx, 60.0);
    ///
    /// // ... game loop runs ...
    ///
    /// tx.send(PlatformEvent::WindowClosed).unwrap();
    /// handle.join().unwrap();
    /// ```
    pub fn spawn_core_thread(
        self,
        receiver: Receiver<PlatformEvent>,
        tps: f64,
    ) -> thread::JoinHandle<()> {
        assert!(tps > 0.0, "TPS must be positive, got {}", tps);

        let frame_duration = Duration::from_secs_f64(1.0 / tps);

        thread::spawn(move || {
            let Self { mut input_system } = self;
            let mut input_batches = Vec::with_capacity(4);

            loop {
                let frame_start = Instant::now();

                //--- Step 1: Collect platform events -------------------
                if let TickControl::Exit =
                    Self::collect_platform_events(&receiver, &mut input_batches, frame_duration)
                {
                    info!("Core thread exiting cleanly.");
                    break;
                }

                //--- Step 2: Update subsystems -------------------------
                input_system.process_frame(&input_batches);

                //--- Step 3: Maintain deterministic pacing -------------
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
        })
    }

    //--- Internal Helpers -------------------------------------------------

    /// Collects all pending platform events for this frame.
    ///
    /// Strategy:
    /// 1. Block for up to `frame_duration` waiting for first event
    ///    (ensures we don't busy-wait on idle frames)
    /// 2. Non-blocking drain of remaining events (bounded to prevent starvation)
    ///
    /// This approach balances responsiveness (low latency) with safety
    /// (bounded worst-case frame time).
    ///
    /// # Arguments
    ///
    /// * `receiver` - Channel to poll for events
    /// * `input_batches` - Output buffer for event batches (cleared first)
    /// * `frame_duration` - Maximum time to block waiting for events
    ///
    /// # Returns
    ///
    /// `TickControl::Exit` if shutdown requested, `Continue` otherwise.
    fn collect_platform_events(
        receiver: &Receiver<PlatformEvent>,
        input_batches: &mut Vec<Vec<InputEvent>>,
        frame_duration: Duration,
    ) -> TickControl {
        const MAX_EVENTS_PER_FRAME: usize = 100;

        input_batches.clear();

        //--- Primary blocking wait -------------------------------------
        // Wait for first event (or timeout). This prevents busy-waiting
        // when no input is available.
        match receiver.recv_timeout(frame_duration) {
            Ok(event) => {
                if let TickControl::Exit = Self::handle_event(event, input_batches) {
                    return TickControl::Exit;
                }
            }
            Err(RecvTimeoutError::Disconnected) => return TickControl::Exit,
            Err(RecvTimeoutError::Timeout) => return TickControl::Continue,
        }

        //--- Non-blocking drain (bounded) ------------------------------
        // Drain remaining events without blocking. Cap at MAX_EVENTS_PER_FRAME
        // to prevent frame starvation if channel is heavily backlogged.
        let mut drained = 0;
        while drained < MAX_EVENTS_PER_FRAME {
            match receiver.try_recv() {
                Ok(event) => {
                    if let TickControl::Exit = Self::handle_event(event, input_batches) {
                        return TickControl::Exit;
                    }
                    drained += 1;
                }
                Err(_) => break, // Queue empty or disconnected
            }
        }

        if drained >= MAX_EVENTS_PER_FRAME {
            warn!("Event queue backlog: drained {} events this frame", drained);
        }

        TickControl::Continue
    }

    /// Processes a single platform event.
    ///
    /// Pushes non-empty input batches into the accumulator and signals
    /// exit on shutdown events.
    fn handle_event(
        event: PlatformEvent,
        input_batches: &mut Vec<Vec<InputEvent>>,
    ) -> TickControl {
        match event {
            PlatformEvent::Inputs { discrete, continuous } => {
                if !discrete.is_empty() {
                    input_batches.push(discrete);
                }
                if !continuous.is_empty() {
                    input_batches.push(continuous);
                }
                TickControl::Continue
            }
            PlatformEvent::WindowClosed => TickControl::Exit,
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
    use crate::core::input::{InputContext, event::{KeyCode, Modifiers}};

    //--- Test Helpers -----------------------------------------------------

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum TestAction {
        Jump,
    }

    impl Action for TestAction {}

    //=====================================================================
    // Basic Flow Tests
    //=====================================================================

    /// Verifies thread exits cleanly on WindowClosed event.
    #[test]
    fn spawn_core_thread_exits_on_window_closed() {
        let (tx, rx) = unbounded();
        let input_system = InputSystem::<TestAction>::new();
        let orchestrator = CoreSystemsOrchestrator::new(input_system);

        let handle = orchestrator.spawn_core_thread(rx, 60.0);

        tx.send(PlatformEvent::WindowClosed).unwrap();

        assert!(handle.join().is_ok());
    }

    /// Verifies thread exits when channel disconnects.
    #[test]
    fn spawn_core_thread_exits_on_channel_disconnect() {
        let (tx, rx) = unbounded();
        let input_system = InputSystem::<TestAction>::new();
        let orchestrator = CoreSystemsOrchestrator::new(input_system);

        let handle = orchestrator.spawn_core_thread(rx, 60.0);

        drop(tx); // Disconnect

        assert!(handle.join().is_ok());
    }

    //=====================================================================
    // collect_platform_events Tests
    //=====================================================================

    /// Empty queue should timeout quickly and return Continue.
    #[test]
    fn collect_handles_empty_queue_debug() {
        let (_tx, rx) = unbounded::<PlatformEvent>();
        let mut batches: Vec<Vec<InputEvent>> = Vec::new();

        let result = CoreSystemsOrchestrator::<TestAction>::collect_platform_events(
            &rx,
            &mut batches,
            Duration::from_micros(100),
        );
        
        assert_eq!(result, TickControl::Continue);
    }

    /// Multiple input events should be aggregated.
    #[test]
    fn collect_aggregates_multiple_events() {
        let (tx, rx) = unbounded();
        let mut batches = Vec::new();

        tx.send(PlatformEvent::Inputs {
            discrete: vec![InputEvent::KeyDown {
                key: KeyCode::KeyA,
                modifiers: Modifiers::NONE
            }],
            continuous: vec![]
        }).unwrap();

        tx.send(PlatformEvent::Inputs {
            discrete: vec![],
            continuous: vec![InputEvent::MouseMoved { x: 10.0, y: 20.0 }]
        }).unwrap();

        let result = CoreSystemsOrchestrator::<TestAction>::collect_platform_events(
            &rx,
            &mut batches,
            Duration::from_millis(10),
        );

        assert!(matches!(result, TickControl::Continue));
        assert_eq!(batches.len(), 2);
    }

    /// WindowClosed should trigger exit.
    #[test]
    fn collect_returns_exit_on_window_closed() {
        let (tx, rx) = unbounded();
        let mut batches = Vec::new();

        tx.send(PlatformEvent::WindowClosed).unwrap();

        let result = CoreSystemsOrchestrator::<TestAction>::collect_platform_events(
            &rx,
            &mut batches,
            Duration::from_millis(10),
        );

        assert!(matches!(result, TickControl::Exit));
    }

    /// Previous frame's batches should be cleared.
    #[test]
    fn collect_clears_previous_batches() {
        let (tx, rx) = unbounded();
        let mut batches = vec![vec![InputEvent::Unidentified]];

        tx.send(PlatformEvent::Inputs {
            discrete: vec![],
            continuous: vec![]
        }).unwrap();

        CoreSystemsOrchestrator::<TestAction>::collect_platform_events(
            &rx,
            &mut batches,
            Duration::from_millis(10),
        );

        assert!(batches.is_empty());
    }

    //=====================================================================
    // Panic Tests
    //=====================================================================

    #[test]
    #[should_panic(expected = "TPS must be positive")]
    fn spawn_panics_on_zero_tps() {
        let (_, rx) = unbounded();
        let orchestrator = CoreSystemsOrchestrator::new(InputSystem::<TestAction>::new());
        orchestrator.spawn_core_thread(rx, 0.0);
    }

    #[test]
    #[should_panic(expected = "TPS must be positive")]
    fn spawn_panics_on_negative_tps() {
        let (_, rx) = unbounded();
        let orchestrator = CoreSystemsOrchestrator::new(InputSystem::<TestAction>::new());
        orchestrator.spawn_core_thread(rx, -10.0);
    }
}