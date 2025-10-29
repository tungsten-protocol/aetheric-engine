//=========================================================================
// Platform Subsystem
//
// Bridges Winit (OS-level events) with the engine's core thread via MPSC.
//
// Architecture:
// ```text
//  Main Thread:                     Logic Thread:
//  ┌──────────────────────────┐    ┌──────────────────┐
//  │  Winit Event Loop        │    │  Core Systems    │
//  │   ↓                      │    │                  │
//  │  InputProcessor          │    │  InputSystem     │
//  │   ├─ Converts Winit      │    │  ↓               │
//  │   └─ Tracks modifiers    │    │  ActionMapper    │
//  │   ↓                      │    │  ↓               │
//  │  InputBuffer             │    │  Game Logic      │
//  │   ├─ discrete: Vec<>     │    │                  │
//  │   └─ continuous: Vec<>   │    └──────────────────┘
//  │   ↓                      │             ↑
//  │  RedrawRequested         │             │
//  │   ↓ (flush)              │             │
//  │  MPSC Channel ───────────┼─────────────┘
//  └──────────────────────────┘    PlatformEvent
//
//  Frame Boundary: RedrawRequested
//    → All buffered input sent atomically
//    → Core processes at fixed TPS (independent of refresh rate)
//    → Empty buffers NOT sent (optimization)
// ```
//
// Key Design Decisions:
// - **RedrawRequested = frame boundary**: Batches all input atomically,
//   ensuring deterministic order even with high event rates
// - **Sticky modifiers**: Modifier state persists across events until
//   explicitly changed (matches platform behavior)
// - **Graceful channel disconnect**: If core thread dies, platform logs
//   warning but continues running to allow window closure
// - **Main thread requirement**: Winit mandates main thread on macOS/iOS,
//   so this runs on the thread that called `Engine::run()`
//
// Responsibilities:
// - Create and manage OS window
// - Poll Winit events at refresh rate
// - Convert Winit types → engine InputEvents
// - Buffer input until frame boundary
// - Send batched events to core thread
//
//=========================================================================

//=== Submodules ==========================================================

mod input_buffer;
mod input_processor;

//=== External Crates =====================================================

use crossbeam_channel::Sender;
use log::*;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes},
    dpi::LogicalSize,
};

//=== Internal Imports ====================================================

use crate::core::input::event::InputEvent;
use input_buffer::InputBuffer;
use input_processor::InputProcessor;

//=== PlatformEvent =======================================================

/// Events sent from the platform layer to the core thread.
///
/// These are the only messages that cross the thread boundary. All variants
/// are cloneable for flexibility (e.g., logging, debugging), though typically
/// consumed immediately by the core thread.
#[derive(Debug, Clone)]
pub(crate) enum PlatformEvent {
    /// Batched input events for a single frame.
    ///
    /// Sent on every `RedrawRequested` event (typically 60-144Hz depending
    /// on monitor refresh rate). Contains:
    /// - `discrete`: Keyboard/mouse button events (order significant)
    /// - `continuous`: Mouse movement (may be coalesced by OS)
    ///
    /// **Note**: Empty batches are NOT sent (optimization).
    Inputs {
        discrete: Vec<InputEvent>,
        continuous: Vec<InputEvent>,
    },

    /// Window close requested by user or OS.
    ///
    /// Sent when:
    /// - User clicks window X button
    /// - OS requests shutdown (logout, restart, etc.)
    /// - Alt+F4 / Cmd+Q pressed
    ///
    /// Core thread should terminate cleanly upon receiving this.
    WindowClosed,
}

//=== PlatformError =======================================================

/// Platform initialization and runtime errors.
///
/// These are typically fatal - if the event loop can't be created,
/// the engine cannot run.
#[derive(Debug)]
pub(crate) enum PlatformError {
    /// Failed to create event loop (rare, indicates OS-level issue).
    EventLoopCreation(winit::error::EventLoopError),

    /// Event loop execution error (rare, indicates corruption).
    EventLoopExecution(winit::error::EventLoopError),
}

//--- Trait Implementations -----------------------------------------------

impl std::fmt::Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EventLoopCreation(e) => write!(f, "Event loop creation failed: {}", e),
            Self::EventLoopExecution(e) => write!(f, "Event loop error: {}", e),
        }
    }
}

impl std::error::Error for PlatformError {}

//=== Platform ============================================================

/// Window manager and input event aggregator.
///
/// Runs on the main thread (Winit requirement on macOS/iOS) and sends
/// batched events to the core thread via MPSC channel.
///
/// # Lifecycle
///
/// 1. **Construction**: `Platform::new(sender)` - initializes subsystems
/// 2. **Execution**: `platform.run()` - starts event loop (never returns)
/// 3. **Event processing**: Winit calls `ApplicationHandler` methods
/// 4. **Shutdown**: User closes window → sends `WindowClosed` → exits
///
/// # Thread Safety
///
/// This type is NOT Send/Sync - it must remain on the main thread.
/// Communication with other threads occurs exclusively via the MPSC sender.
///
/// # Fields
///
/// - `window`: Created lazily in `resumed()` (mobile compatibility)
/// - `buffer`: Accumulates events until `RedrawRequested`
/// - `event_sender`: MPSC channel to core thread
/// - `input_processor`: Converts Winit events → engine events
pub(crate) struct Platform {
    /// OS window handle (None until `resumed()` called).
    window: Option<Window>,

    /// Buffers discrete/continuous input until frame boundary.
    buffer: InputBuffer,

    /// Channel to send events to core thread.
    event_sender: Sender<PlatformEvent>,

    /// Converts Winit events to engine InputEvents.
    input_processor: InputProcessor,
}

impl Platform {
    //--- Construction -----------------------------------------------------

    /// Creates a new platform instance with the given event sender.
    ///
    /// Does not create window yet - that happens lazily in `resumed()`.
    pub fn new(event_sender: Sender<PlatformEvent>) -> Self {
        info!(target: "platform", "Platform subsystem initialized");
        Self {
            window: None,
            buffer: InputBuffer::new(),
            event_sender,
            input_processor: InputProcessor::new(),
        }
    }

    //--- Execution --------------------------------------------------------

    /// Starts the event loop (never returns normally).
    ///
    /// This method blocks forever, running the Winit event loop. It only
    /// returns if event loop creation fails. Once running, the loop continues
    /// until `exit()` is called (which terminates the process).
    ///
    /// **Important**: Code after calling `run()` is unreachable. The event
    /// loop either runs forever or the process exits via `std::process::exit()`.
    ///
    /// # Errors
    ///
    /// Returns [`PlatformError`] only if event loop creation fails before
    /// starting. Once started, errors are handled internally (logged and
    /// graceful shutdown attempted).
    ///
    /// # Panics
    ///
    /// Panics if called off the main thread (macOS/iOS Winit requirement).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use crossbeam_channel::unbounded;
    /// # struct Platform;
    /// # impl Platform {
    /// #   fn new(_: crossbeam_channel::Sender<()>) -> Self { Self }
    /// #   fn run(self) -> Result<(), ()> { Ok(()) }
    /// # }
    /// let (tx, rx) = unbounded();
    /// let platform = Platform::new(tx);
    ///
    /// platform.run()?;
    ///
    /// // ⚠️ This line is NEVER reached!
    /// println!("Event loop finished");
    /// # Ok::<(), ()>(())
    /// ```
    pub fn run(mut self) -> Result<(), PlatformError> {
        debug!(target: "platform", "Starting Winit event loop");

        let event_loop = EventLoop::new()
            .map_err(PlatformError::EventLoopCreation)?;

        // run_app() never returns (type: !) - terminates process on exit
        event_loop.run_app(&mut self)
            .map_err(PlatformError::EventLoopExecution)
    }

    //--- Internal Helpers -------------------------------------------------

    /// Flushes buffered input events to the core thread.
    ///
    /// Drains both discrete and continuous event buffers, sending them as a
    /// single [`PlatformEvent::Inputs`] message. Called on every
    /// `RedrawRequested` event.
    ///
    /// # Error Handling
    ///
    /// If the channel is disconnected (core thread panicked or exited early),
    /// logs a warning and **silently drops the events**. This is intentional:
    /// - Prevents platform thread from panicking
    /// - Allows user to close window normally
    /// - Core thread shutdown is logged separately
    ///
    /// In normal shutdown, `WindowClosed` is sent first, so core exits cleanly
    /// before channel disconnect occurs.
    ///
    /// # Performance
    ///
    /// Empty buffers are not sent (optimization). Typical frame has 0-10 events.
    fn flush_input_buffer(&mut self) {
        if let Some((discrete, continuous)) = self.buffer.drain() {
            let discrete_count = discrete.len();
            let continuous_count = continuous.len();
            let total_events = discrete_count + continuous_count;

            trace!(
            target: "platform::input",
            "Flushing {} discrete + {} continuous events",
            discrete_count,
            continuous_count
        );

            if self.event_sender.send(PlatformEvent::Inputs { discrete, continuous }).is_err() {
                warn!(
                target: "platform::input",
                "Channel disconnected, dropping {} events ({} discrete, {} continuous)",
                total_events,
                discrete_count,
                continuous_count
            );
            }
        }
    }

    //--- Test Accessors ---------------------------------------------------

    #[cfg(test)]
    pub(crate) fn window(&self) -> Option<&Window> {
        self.window.as_ref()
    }
}

//=== Winit Integration ===================================================

impl ApplicationHandler for Platform {
    /// Called when app becomes active (startup or mobile resume).
    ///
    /// Creates the window if it doesn't exist yet. On mobile, this may be
    /// called multiple times (suspend/resume cycle).
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            debug!(target: "platform", "Window already exists (mobile resume?)");
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title("Aetheric Engine")
            .with_inner_size(LogicalSize::new(800, 600));

        match event_loop.create_window(attrs) {
            Ok(window) => {
                info!(
                    target: "platform",
                    "Window created: {}x{} @ {}x DPI",
                    window.inner_size().width,
                    window.inner_size().height,
                    window.scale_factor()
                );
                window.request_redraw();
                self.window = Some(window);
            }
            Err(e) => {
                error!(target: "platform", "Window creation failed: {}", e);
                // Notify core of fatal error
                let _ = self.event_sender.send(PlatformEvent::WindowClosed);
                event_loop.exit();
            }
        }
    }

    /// Handles per-window events.
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match &event {
            WindowEvent::CloseRequested => {
                info!(target: "platform", "Window close requested");
                let _ = self.event_sender.send(PlatformEvent::WindowClosed);
                event_loop.exit();
            }

            WindowEvent::ModifiersChanged(state) => {
                trace!(target: "platform::input", "Modifiers changed: {:?}", state);
                self.input_processor.update_modifiers(state.state());
            }

            WindowEvent::CursorMoved { position, .. } => {
                let event = self.input_processor.process_mouse_move(
                    position.x as f32,
                    position.y as f32
                );
                self.buffer.push_continuous(event);
            }

            WindowEvent::KeyboardInput { event: key_event, .. } => {
                if let Some(event) = self.input_processor.process_key_event(key_event) {
                    self.buffer.push_discrete(event);
                } else {
                    trace!(target: "platform::input", "Unmapped key ignored");
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let event = self.input_processor.process_mouse_button(*button, *state);
                self.buffer.push_discrete(event);
            }

            WindowEvent::RedrawRequested => {
                // Frame boundary: flush all buffered input
                self.flush_input_buffer();

                // Request next frame
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            _ => {
                // Ignore: Resized, Focused, etc. (not needed for input)
            }
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
    use crate::core::input::{KeyCode, Modifiers};

    //=====================================================================
    // PlatformEvent Tests
    //=====================================================================

    #[test]
    fn platform_event_inputs_is_cloneable() {
        let event = PlatformEvent::Inputs {
            discrete: vec![],
            continuous: vec![],
        };
        let _cloned = event.clone();
    }

    #[test]
    fn platform_event_window_closed_is_cloneable() {
        let event = PlatformEvent::WindowClosed;
        let _cloned = event.clone();
    }

    #[test]
    fn platform_event_is_debug() {
        let event = PlatformEvent::WindowClosed;
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("WindowClosed"));
    }

    //=====================================================================
    // Platform Tests
    //=====================================================================

    #[test]
    fn platform_creation() {
        let (tx, _rx) = unbounded();
        let platform = Platform::new(tx);
        assert!(platform.window().is_none(), "Window should be created lazily");
    }

    #[test]
    fn flush_empty_buffer_is_noop() {
        let (tx, rx) = unbounded();
        let mut platform = Platform::new(tx);

        platform.flush_input_buffer();

        assert!(rx.try_recv().is_err(), "No events should be sent for empty buffer");
    }

    #[test]
    fn flush_sends_buffered_events() {
        let (tx, rx) = unbounded();
        let mut platform = Platform::new(tx);

        // Simulate buffering some events
        let event = InputEvent::KeyDown {
            key: KeyCode::Space,
            modifiers: Modifiers::NONE,
        };
        platform.buffer.push_discrete(event);

        platform.flush_input_buffer();

        // Should have sent Inputs event
        match rx.try_recv() {
            Ok(PlatformEvent::Inputs { discrete, continuous }) => {
                assert_eq!(discrete.len(), 1, "Should have 1 discrete event");
                assert!(continuous.is_empty(), "Should have no continuous events");
            }
            other => panic!("Expected Inputs event, got {:?}", other),
        }
    }

    #[test]
    fn flush_handles_disconnected_channel() {
        let (tx, rx) = unbounded();
        let mut platform = Platform::new(tx);

        // Simulate buffering
        let event = InputEvent::KeyDown {
            key: KeyCode::Space,
            modifiers: Modifiers::NONE,
        };
        platform.buffer.push_discrete(event);

        // Drop receiver to disconnect
        drop(rx);

        // Should not panic, just log warning
        platform.flush_input_buffer();
    }

    #[test]
    fn multiple_flushes_clear_buffer() {
        let (tx, rx) = unbounded();
        let mut platform = Platform::new(tx);

        platform.buffer.push_discrete(InputEvent::KeyDown {
            key: KeyCode::KeyA,
            modifiers: Modifiers::NONE,
        });

        platform.flush_input_buffer();
        platform.flush_input_buffer(); // Second flush should be no-op

        // Should have received only 1 event
        assert!(rx.try_recv().is_ok(), "First flush should send");
        assert!(rx.try_recv().is_err(), "Second flush should not send");
    }

    //=====================================================================
    // PlatformError Tests
    //=====================================================================

    #[test]
    fn platform_error_is_error_trait() {
        fn assert_error<T: std::error::Error>() {}
        assert_error::<PlatformError>();
    }

    #[test]
    fn platform_error_display_format() {
        // Note: Hard to construct real EventLoopError without running event loop
        // This test validates the trait bounds exist
        fn assert_display<T: std::fmt::Display>() {}
        assert_display::<PlatformError>();
    }
}