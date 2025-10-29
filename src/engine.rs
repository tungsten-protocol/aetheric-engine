//=========================================================================
// Aetheric Engine
//
// Main entry point and coordinator for the engine.
//
// Architecture:
// ```text
//     EngineBuilder  ──build()──>  Engine  ──run()──>  [Runtime]
//         │                          │
//         ├─ with_input_system()     └─ spawns threads
//         ├─ with_tps()                 runs platform
//         └─ with_channel_capacity()    blocks until exit
// ```
//
//=========================================================================

//=== External Crates =====================================================

use crossbeam_channel::{bounded, Sender, Receiver};
use log::{info, error};

//=== Internal Modules ====================================================

use crate::core::{input, CoreSystemsOrchestrator};
use crate::platform::{Platform, PlatformEvent};
use crate::core::input::Action;

//=== Public Re-exports ===================================================

pub use input::InputSystem;

//=== EngineBuilder =======================================================

/// Builder for configuring and constructing an [`Engine`].
///
/// Provides a fluent API for setting engine parameters before construction.
/// All configuration is optional except the input system.
///
/// # Default Values
///
/// - **TPS**: 60.0 (logic updates per second)
/// - **Channel capacity**: 128 events
///
/// # Examples
///
/// Simple usage with defaults:
/// ```no_run
/// use aetheric_engine::{Engine, InputSystem};
/// use aetheric_engine::core::input::Action;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// enum GameAction { Jump }
/// impl Action for GameAction {}
///
/// let input = InputSystem::<GameAction>::new();
/// Engine::new(input).run();
/// ```
///
/// Advanced configuration:
/// ```no_run
/// # use aetheric_engine::{EngineBuilder, InputSystem};
/// # use aetheric_engine::core::input::{Action, KeyCode};
/// # #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// # enum GameAction { Jump }
/// # impl Action for GameAction {}
/// let input = InputSystem::<GameAction>::new()
///     .with_binding(KeyCode::Space, GameAction::Jump);
///
/// EngineBuilder::new()
///     .with_input_system(input)
///     .with_tps(120.0)              // High refresh rate
///     .with_channel_capacity(256)   // Extra buffering
///     .build()
///     .run();
/// ```
pub struct EngineBuilder<A: Action> {
    input_system: Option<InputSystem<A>>,
    tps: f64,
    channel_capacity: usize,
}

impl<A: Action> EngineBuilder<A> {
    /// Creates a new builder with default settings.
    ///
    /// Input system must be provided via [`with_input_system`] before calling [`build`].
    ///
    /// [`with_input_system`]: Self::with_input_system
    /// [`build`]: Self::build
    pub fn new() -> Self {
        Self {
            input_system: None,
            tps: 60.0,
            channel_capacity: 128,
        }
    }

    /// Sets the input system (required).
    ///
    /// The input system handles raw input events and maps them to game actions.
    pub fn with_input_system(mut self, input_system: InputSystem<A>) -> Self {
        self.input_system = Some(input_system);
        self
    }

    /// Sets the target ticks per second for the logic thread.
    ///
    /// The logic thread will attempt to maintain this update rate using
    /// a fixed timestep loop. Higher values provide more responsive input
    /// and smoother simulation, but increase CPU usage.
    ///
    /// Default: 60.0
    ///
    /// # Panics
    ///
    /// Panics if `tps <= 0.0`.
    ///
    /// # Typical Values
    ///
    /// - **30 TPS**: Low-power devices, turn-based games
    /// - **60 TPS**: Standard for most games (16.67ms per tick)
    /// - **120 TPS**: High refresh rate displays, competitive games
    /// - **240+ TPS**: Server simulations, physics-heavy games
    pub fn with_tps(mut self, tps: f64) -> Self {
        assert!(tps > 0.0, "TPS must be positive, got {}", tps);
        self.tps = tps;
        self
    }

    /// Sets the channel capacity for platform → core communication.
    ///
    /// Larger values provide more buffering during frame spikes but increase
    /// memory usage. Smaller values reduce latency but may drop events if
    /// the logic thread falls behind.
    ///
    /// Default: 128
    ///
    /// # Panics
    ///
    /// Panics if `capacity == 0`.
    ///
    /// # Typical Values
    ///
    /// - **64**: Minimal buffering, low latency
    /// - **128**: Standard (default), ~2 frames at 60 FPS
    /// - **256+**: Heavy buffering for unstable frame rates
    pub fn with_channel_capacity(mut self, capacity: usize) -> Self {
        assert!(capacity > 0, "Channel capacity must be positive");
        self.channel_capacity = capacity;
        self
    }

    /// Builds the engine instance.
    ///
    /// Consumes the builder and produces a configured [`Engine`] ready to run.
    ///
    /// # Panics
    ///
    /// Panics if input system was not provided via [`with_input_system`].
    ///
    /// [`with_input_system`]: Self::with_input_system
    pub fn build(self) -> Engine<A> {
        let input_system = self.input_system
            .expect("InputSystem is required. Call .with_input_system() before .build()");

        info!("Building engine (TPS: {}, channel: {})", self.tps, self.channel_capacity);

        Engine {
            orchestrator: CoreSystemsOrchestrator::new(input_system),
            tps: self.tps,
            channel_capacity: self.channel_capacity,
        }
    }
}

impl<A: Action> Default for EngineBuilder<A> {
    fn default() -> Self {
        Self::new()
    }
}

//=== Engine ==============================================================

/// Aetheric Engine runtime.
///
/// The engine coordinates all subsystems and manages the main execution loop.
/// Create via [`EngineBuilder`] for advanced configuration, or use [`Engine::new`]
/// for quick setup with defaults.
///
/// # Architecture
///
/// ```text
/// Engine (Main Thread)
///   ├─► CoreSystemsOrchestrator (Logic Thread @ TPS)
///   │     └─► InputSystem, Physics, AI...
///   │
///   └─► Platform (Event Loop)
///         └─► Window, Input Polling
///
/// Communication: MPSC Channel (PlatformEvent)
/// ```
///
/// # Examples
///
/// Quick start:
/// ```no_run
/// use aetheric_engine::{Engine, InputSystem};
/// use aetheric_engine::core::input::Action;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// enum GameAction { Jump }
/// impl Action for GameAction {}
///
/// Engine::new(InputSystem::<GameAction>::new()).run();
/// ```
///
/// With configuration:
/// ```no_run
/// # use aetheric_engine::{EngineBuilder, InputSystem};
/// # use aetheric_engine::core::input::{Action, KeyCode};
/// # #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// # enum GameAction { Jump }
/// # impl Action for GameAction {}
/// let input = InputSystem::<GameAction>::new()
///     .with_binding(KeyCode::Space, GameAction::Jump);
///
/// EngineBuilder::new()
///     .with_input_system(input)
///     .with_tps(120.0)
///     .build()
///     .run();
/// ```
pub struct Engine<A: Action> {
    orchestrator: CoreSystemsOrchestrator<A>,
    tps: f64,
    channel_capacity: usize,
}

impl<A: Action> Engine<A> {
    //--- Convenience Constructor ------------------------------------------

    /// Creates an engine with default settings.
    ///
    /// Equivalent to:
    /// ```ignore
    /// EngineBuilder::new()
    ///     .with_input_system(input_system)
    ///     .build()
    /// ```
    ///
    /// For custom configuration (TPS, channel capacity), use [`EngineBuilder`].
    pub fn new(input_system: InputSystem<A>) -> Self {
        EngineBuilder::new()
            .with_input_system(input_system)
            .build()
    }

    //--- Execution --------------------------------------------------------

    /// Starts the engine runtime and blocks until the application exits.
    ///
    /// # Lifecycle
    ///
    /// 1. Creates MPSC channel for platform → core communication
    /// 2. Spawns logic thread running at configured TPS
    /// 3. Runs platform event loop (blocks here)
    /// 4. On window close: platform exits → channel disconnects → logic thread terminates
    ///
    /// # Panics
    ///
    /// Panics if platform initialization fails (e.g., no graphics context).
    ///
    /// # Thread Panic Handling
    ///
    /// If the logic thread panics, the error is logged and the engine attempts
    /// graceful shutdown. The platform continues running to allow the user to
    /// close the window normally.
    pub fn run(self) {
        info!("Starting engine runtime (TPS: {})", self.tps);

        //--- 1. Create communication channel -----------------------------
        let (tx, rx): (Sender<PlatformEvent>, Receiver<PlatformEvent>) =
            bounded(self.channel_capacity);

        info!("MPSC channel created (capacity: {})", self.channel_capacity);

        //--- 2. Spawn the core logic thread -------------------------------
        let core_handle = self.orchestrator.spawn_core_thread(rx, self.tps);
        info!("Core logic thread spawned");

        //--- 3. Launch the platform subsystem -----------------------------
        let mut platform = Platform::new(tx);
        info!("Platform initialized, entering event loop");

        platform.run(); // Blocks until window close

        info!("Platform event loop exited");

        //--- 4. Cleanup: Wait for logic thread to terminate --------------
        match core_handle.join() {
            Ok(()) => {
                info!("Core thread terminated cleanly");
            }
            Err(e) => {
                error!("Core thread panicked: {:?}", e);
            }
        }

        info!("Engine shutdown complete");
    }
}

//=========================================================================
// Unit Tests
//=========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::input::KeyCode;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    enum TestAction {
        Jump,
        Shoot,
    }

    impl Action for TestAction {}

    //=====================================================================
    // EngineBuilder Tests
    //=====================================================================

    #[test]
    fn builder_can_be_created() {
        let _builder = EngineBuilder::<TestAction>::new();
    }

    #[test]
    fn builder_defaults() {
        let builder = EngineBuilder::<TestAction>::new();
        assert_eq!(builder.tps, 60.0);
        assert_eq!(builder.channel_capacity, 128);
        assert!(builder.input_system.is_none());
    }

    #[test]
    fn builder_with_tps() {
        let builder = EngineBuilder::<TestAction>::new().with_tps(120.0);
        assert_eq!(builder.tps, 120.0);
    }

    #[test]
    #[should_panic(expected = "TPS must be positive")]
    fn builder_with_tps_panics_on_zero() {
        EngineBuilder::<TestAction>::new().with_tps(0.0);
    }

    #[test]
    #[should_panic(expected = "TPS must be positive")]
    fn builder_with_tps_panics_on_negative() {
        EngineBuilder::<TestAction>::new().with_tps(-60.0);
    }

    #[test]
    fn builder_with_channel_capacity() {
        let builder = EngineBuilder::<TestAction>::new().with_channel_capacity(256);
        assert_eq!(builder.channel_capacity, 256);
    }

    #[test]
    #[should_panic(expected = "Channel capacity must be positive")]
    fn builder_with_channel_capacity_panics_on_zero() {
        EngineBuilder::<TestAction>::new().with_channel_capacity(0);
    }

    #[test]
    fn builder_with_input_system() {
        let input = InputSystem::<TestAction>::new();
        let builder = EngineBuilder::new().with_input_system(input);
        assert!(builder.input_system.is_some());
    }

    #[test]
    fn builder_build_creates_engine() {
        let input = InputSystem::<TestAction>::new();
        let _engine = EngineBuilder::new()
            .with_input_system(input)
            .build();
    }

    #[test]
    #[should_panic(expected = "InputSystem is required")]
    fn builder_build_panics_without_input() {
        let _engine = EngineBuilder::<TestAction>::new().build();
    }

    #[test]
    fn builder_fluent_api_chaining() {
        let input = InputSystem::<TestAction>::new();

        let engine = EngineBuilder::new()
            .with_tps(120.0)
            .with_channel_capacity(256)
            .with_input_system(input)
            .build();

        assert_eq!(engine.tps, 120.0);
        assert_eq!(engine.channel_capacity, 256);
    }

    //=====================================================================
    // Engine Convenience Constructor Tests
    //=====================================================================

    #[test]
    fn engine_new_creates_with_defaults() {
        let input = InputSystem::<TestAction>::new();
        let engine = Engine::new(input);

        assert_eq!(engine.tps, 60.0);
        assert_eq!(engine.channel_capacity, 128);
    }

    #[test]
    fn engine_new_equivalent_to_builder() {
        let input1 = InputSystem::<TestAction>::new();
        let input2 = InputSystem::<TestAction>::new();

        let engine1 = Engine::new(input1);
        let engine2 = EngineBuilder::new()
            .with_input_system(input2)
            .build();

        assert_eq!(engine1.tps, engine2.tps);
        assert_eq!(engine1.channel_capacity, engine2.channel_capacity);
    }

    //=====================================================================
    // Integration with InputSystem
    //=====================================================================

    #[test]
    fn builder_accepts_configured_input_system() {
        let input = InputSystem::<TestAction>::new()
            .with_binding(KeyCode::Space, TestAction::Jump)
            .with_binding(KeyCode::KeyF, TestAction::Shoot);

        let _engine = EngineBuilder::new()
            .with_input_system(input)
            .build();
    }
}