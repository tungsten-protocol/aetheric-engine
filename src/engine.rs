//=========================================================================
// Aetheric Engine
//
// Main entry point and coordinator for the engine.
//
// Architecture:
// ```text
//     EngineBuilder  ──build()──>  Engine  ──run()──>  [Runtime]
//         │                          │
//         ├─ with_tps()              └─ spawns threads
//         └─ with_channel_capacity()    runs platform
//                                       blocks until exit
// ```
//
//=========================================================================

//=== External Dependencies ===============================================

use crossbeam_channel::{bounded, Receiver, Sender};
use log::{error, info};

//=== Internal Dependencies ===============================================

use crate::core::platform_bridge::PlatformEvent;
use crate::core::{Action, CoreSystemsOrchestrator, GlobalSystems, SceneKey};
use crate::platform::Platform;

//=== EngineBuilder =======================================================

/// Builder for configuring and constructing an [`Engine`].
///
/// Provides a fluent API for setting engine parameters before construction.
/// Engine systems are automatically initialized.
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
/// use aetheric_engine::EngineBuilder;
/// use aetheric_engine::core::input::Action;
/// use aetheric_engine::core::scene::SceneKey;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// enum GameScene { Main }
/// impl SceneKey for GameScene {}
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// enum GameAction { Jump }
/// impl Action for GameAction {}
///
/// EngineBuilder::<GameScene, GameAction>::new().build().run();
/// ```
///
/// Advanced configuration:
/// ```no_run
/// # use aetheric_engine::EngineBuilder;
/// # use aetheric_engine::core::input::Action;
/// # use aetheric_engine::core::scene::SceneKey;
/// # #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// # enum GameScene { Main }
/// # impl SceneKey for GameScene {}
/// # #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// # enum GameAction { Jump }
/// # impl Action for GameAction {}
///
/// EngineBuilder::<GameScene, GameAction>::new()
///     .with_tps(120.0)              // High refresh rate
///     .with_channel_capacity(256)   // Extra buffering
///     .build()
///     .run();
/// ```
///
/// With initialization:
/// ```no_run
/// # use aetheric_engine::EngineBuilder;
/// # use aetheric_engine::core::input::Action;
/// # use aetheric_engine::core::scene::SceneKey;
/// # #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// # enum GameScene { Main }
/// # impl SceneKey for GameScene {}
/// # #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// # enum GameAction { Jump }
/// # impl Action for GameAction {}
///
/// EngineBuilder::<GameScene, GameAction>::new()
///     .with_tps(120.0)
///     .build()
///     .init(|systems| {
///         // Initialize game systems
///         systems.input.bind_key(/* ... */);
///         systems.scene_manager.register_scene(/* ... */);
///     })
///     .run();
/// ```
pub struct EngineBuilder<S: SceneKey, A: Action> {
    tps: f64,
    channel_capacity: usize,
    _phantom: std::marker::PhantomData<(S, A)>,
}

impl<S: SceneKey, A: Action> EngineBuilder<S, A> {
    /// Creates a new builder with default settings.
    pub fn new() -> Self {
        Self {
            tps: 60.0,
            channel_capacity: 128,
            _phantom: std::marker::PhantomData,
        }
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
    pub fn with_channel_capacity(mut self, capacity: usize) -> Self {
        assert!(capacity > 0, "Channel capacity must be positive");
        self.channel_capacity = capacity;
        self
    }

    /// Builds the engine instance.
    ///
    /// Consumes the builder and produces a configured [`Engine`] ready for
    /// initialization or execution. Call [`Engine::init`] to initialize
    /// systems before running, or call [`Engine::run`] directly.
    /// All engine systems are automatically created.
    pub fn build(self) -> Engine<S, A> {
        info!("Building engine (TPS: {}, channel: {})", self.tps, self.channel_capacity);

        Engine {
            orchestrator: CoreSystemsOrchestrator::new(),
            tps: self.tps,
            channel_capacity: self.channel_capacity,
        }
    }
}

impl<S: SceneKey, A: Action> Default for EngineBuilder<S, A> {
    fn default() -> Self {
        Self::new()
    }
}

//=== Engine ==============================================================

/// Aetheric Engine runtime.
///
/// The engine coordinates all subsystems and manages the main execution loop.
/// Create via [`EngineBuilder`] with `EngineBuilder::new().build()`.
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
/// use aetheric_engine::EngineBuilder;
/// use aetheric_engine::core::input::Action;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// enum GameAction { Jump }
/// impl Action for GameAction {}
///
/// EngineBuilder::<GameAction>::new().build().run();
/// ```
///
/// With configuration:
/// ```no_run
/// # use aetheric_engine::EngineBuilder;
/// # use aetheric_engine::core::input::Action;
/// # #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// # enum GameAction { Jump }
/// # impl Action for GameAction {}
///
/// EngineBuilder::<GameAction>::new()
///     .with_tps(120.0)
///     .build()
///     .run();
/// ```
pub struct Engine<S: SceneKey, A: Action> {
    orchestrator: CoreSystemsOrchestrator<S, A>,
    tps: f64,
    channel_capacity: usize,
}

impl<S: SceneKey, A: Action> Engine<S, A> {
    //--- Initialization ---------------------------------------------------

    /// Initializes engine systems before execution.
    ///
    /// Provides mutable access to [`GlobalSystems`] for configuring
    /// game systems (input bindings, scene registration, etc.) before
    /// the engine starts running.
    ///
    /// This method can only be called once before [`Engine::run`].
    /// After calling `run`, the engine consumes itself and cannot be
    /// reinitialized.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use aetheric_engine::EngineBuilder;
    /// # use aetheric_engine::core::input::{Action, KeyCode};
    /// # use aetheric_engine::core::scene::SceneKey;
    /// # #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    /// # enum GameScene { Main }
    /// # impl SceneKey for GameScene {}
    /// # #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    /// # enum GameAction { Jump, Shoot }
    /// # impl Action for GameAction {}
    ///
    /// EngineBuilder::<GameScene, GameAction>::new()
    ///     .build()
    ///     .init(|systems| {
    ///         systems.input.bind_key(KeyCode::Space, GameAction::Jump);
    ///         systems.input.bind_key(KeyCode::KeyF, GameAction::Shoot);
    ///         // systems.scene_manager.register_scene(...);
    ///     })
    ///     .run();
    /// ```
    pub fn init<F>(mut self, init_fn: F) -> Self
    where
        F: FnOnce(&mut GlobalSystems<S, A>),
    {
        info!("Initializing engine systems");

        self.orchestrator.init_systems(init_fn);

        info!("Engine initialization complete");
        self
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
        let platform = Platform::new(tx);
        info!("Platform initialized, entering event loop");

        if let Err(e) = platform.run() {
            error!("Platform error: {:?}", e);
        }

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
    enum TestScene {
        Main,
    }

    impl SceneKey for TestScene {}

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
        let _builder = EngineBuilder::<TestScene, TestAction>::new();
    }

    #[test]
    fn builder_defaults() {
        let builder = EngineBuilder::<TestScene, TestAction>::new();
        assert_eq!(builder.tps, 60.0);
        assert_eq!(builder.channel_capacity, 128);
    }

    #[test]
    fn builder_with_tps() {
        let builder = EngineBuilder::<TestScene, TestAction>::new().with_tps(120.0);
        assert_eq!(builder.tps, 120.0);
    }

    #[test]
    #[should_panic(expected = "TPS must be positive")]
    fn builder_with_tps_panics_on_zero() {
        EngineBuilder::<TestScene, TestAction>::new().with_tps(0.0);
    }

    #[test]
    #[should_panic(expected = "TPS must be positive")]
    fn builder_with_tps_panics_on_negative() {
        EngineBuilder::<TestScene, TestAction>::new().with_tps(-60.0);
    }

    #[test]
    fn builder_with_channel_capacity() {
        let builder = EngineBuilder::<TestScene, TestAction>::new().with_channel_capacity(256);
        assert_eq!(builder.channel_capacity, 256);
    }

    #[test]
    #[should_panic(expected = "Channel capacity must be positive")]
    fn builder_with_channel_capacity_panics_on_zero() {
        EngineBuilder::<TestScene, TestAction>::new().with_channel_capacity(0);
    }

    #[test]
    fn builder_build_creates_engine() {
        let _engine = EngineBuilder::<TestScene, TestAction>::new().build();
    }

    #[test]
    fn builder_fluent_api_chaining() {
        let engine = EngineBuilder::<TestScene, TestAction>::new()
            .with_tps(120.0)
            .with_channel_capacity(256)
            .build();

        assert_eq!(engine.tps, 120.0);
        assert_eq!(engine.channel_capacity, 256);
    }
}