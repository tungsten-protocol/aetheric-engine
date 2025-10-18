//=========================================================================
// Platform
//
// Entry point for the platform subsystem.
// Provides OS-level integration and event translation for the engine.
//
// Responsibilities:
// - Create and manage the main application window
// - Initialize and run the Winit event loop
// - Translate native window/input events into engine-level events
// - Forward input batches to the Engine via an MPSC channel
//
// Notes:
// The Platform layer operates independently of the Engine’s logic thread.
// It does not share state directly — all communication occurs through
// asynchronous message passing for full isolation and thread safety.
//
//=========================================================================

//=== Submodules ==========================================================
mod event_mapper;
mod input_buffer;

//=== Standard Library Imports ============================================
use std::sync::mpsc::Sender;

//=== External Crates =====================================================
use log::*;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowAttributes},
};

//=== Internal Modules ====================================================
use input_buffer::InputBuffer;
use crate::core::input::event::RawInputEvent;

//=== PlatformEvent =======================================================
//
// Messages emitted by the Platform and received by the Engine.
// Represents all high-level platform events relevant to the engine.
//
#[derive(Debug)]
pub(crate) enum PlatformEvent {
    /// Batched input events collected during the current frame.
    Inputs(Vec<RawInputEvent>),

    /// Signals that the main window was closed or the app should terminate.
    WindowClosed,
}

//=== Platform ============================================================
//
// The main abstraction for the platform layer.
// Owns the main window, maintains an input buffer, and forwards aggregated
// input batches to the Engine’s logic thread via the MPSC channel.
//
pub(crate) struct Platform {
    /// Temporary buffer used to aggregate raw input events each frame.
    buffer: InputBuffer,

    /// Handle to the main application window (created on resume).
    window: Option<Window>,

    /// Sender channel used to communicate platform events to the Engine.
    input_channel: Sender<PlatformEvent>,
}

impl Platform {
    //--- Construction -----------------------------------------------------
    //
    // Initializes the platform subsystem but does not yet create a window.
    //
    pub fn new(input_channel: Sender<PlatformEvent>) -> Self {
        debug!(
            target: "platform_subsystem",
            "Platform subsystem initialized (window not yet created)."
        );

        Self {
            window: None,
            buffer: InputBuffer::new(),
            input_channel,
        }
    }

    //--- run() ------------------------------------------------------------
    //
    // Starts the Winit event loop and delegates event handling to `self`.
    // This call blocks until the window is closed or a fatal error occurs.
    //
    pub fn run(&mut self) {
        debug!(target: "platform_subsystem", "Starting window event loop...");

        let event_loop = match EventLoop::new() {
            Ok(loop_instance) => loop_instance,
            Err(e) => {
                error!(target: "platform_subsystem", "Failed to create event loop: {e}");
                return;
            }
        };

        if let Err(e) = event_loop.run_app(self) {
            error!(
                target: "platform_subsystem",
                "Event loop terminated unexpectedly: {e}"
            );
        }

        debug!(target: "platform_subsystem", "Window event loop terminated.");
    }

    //--- process_input() --------------------------------------------------
    //
    // Flushes the buffered input events and sends them to the Engine.
    // Skips sending if no events are present or the channel is disconnected.
    //
    fn process_input(&mut self) {
        let events = self.buffer.drain();
        if !events.is_empty() {
            if let Err(e) = self.input_channel.send(PlatformEvent::Inputs(events)) {
                warn!(
                    target: "platform_subsystem",
                    "Failed to send input events to engine: {e}"
                );
            }
        }
    }
}

//=========================================================================
// Winit Integration
//
// Implements `ApplicationHandler` to bridge Winit’s native events into
// engine-level abstractions.
//=========================================================================

impl ApplicationHandler for Platform {
    //--- Lifecycle: resumed() --------------------------------------------
    //
    // Called when the application is ready to (re)create its main window.
    //
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        match event_loop.create_window(WindowAttributes::default().with_title("Aetheric Engine")) {
            Ok(window) => {
                debug!(
                    target: "platform_subsystem",
                    "Main window created successfully."
                );
                self.window = Some(window);
            }
            Err(e) => {
                error!(target: "platform_subsystem", "Failed to create window: {e}");
                event_loop.exit(); // Exit gracefully
            }
        }
    }

    //--- window_event() ---------------------------------------------------
    //
    // Handles window lifecycle and input-related events.
    //
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            //--- Window lifecycle ------------------------------------------
            WindowEvent::CloseRequested => {
                info!(
                    target: "platform_subsystem",
                    "Close requested — exiting application."
                );
                if let Err(e) = self.input_channel.send(PlatformEvent::WindowClosed) {
                    debug!(
                        target: "platform_subsystem",
                        "Failed to send window-closed event: {e}"
                    );
                }
                event_loop.exit();
            }

            //--- Continuous input events -----------------------------------
            WindowEvent::CursorMoved { .. } => {
                self.buffer.push_continuous(RawInputEvent::from(event));
            }

            //--- Discrete input events -------------------------------------
            WindowEvent::KeyboardInput { .. } | WindowEvent::MouseInput { .. } => {
                self.buffer.push_discrete(RawInputEvent::from(event));
            }

            //--- Frame tick / redraw --------------------------------------
            //
            // Used as a synchronization point to flush accumulated input.
            //
            WindowEvent::RedrawRequested => {
                self.process_input();

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            //--- Fallback ---------------------------------------------------
            //
            // Logs any unhandled events for diagnostic purposes.
            //
            _ => trace!(
                target: "platform_subsystem",
                "Unhandled window event: {:?}",
                event
            ),
        }
    }
}
