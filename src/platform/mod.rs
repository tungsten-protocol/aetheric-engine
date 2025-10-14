//=== Platform ============================================================
//
// Handles the platform subsystem: window creation, main event loop,
// and integration with Winit.
//
// Responsibilities:
// - Creates and manages the main window
// - Runs and integrates the event loop
// - Provides the base platform layer for the engine
//
//=========================================================================

use winit::{
    application::ApplicationHandler,
    event::{WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::Window,
};
use log::*;

//=== Platform Struct =====================================================
//
// Represents the platform layer. Stores the window handle and controls
// the application lifecycle. This is the entry point for running
// the engine on desktop platforms.
//
pub struct Platform {
    window: Option<Window>
}

impl Platform {
    //--- Initialization ---------------------------------------------------
    //
    // Initializes the platform subsystem without creating a window yet.
    // The window will be created once the application resumes.
    //

    /// Creates a new `Platform` instance with an empty initial state.
    ///
    /// # Example
    /// ```no_run
    /// let platform = Platform::new();
    /// ```
    pub fn new() -> Self {
        info!(target: "platform_subsystem", "Platform subsystem initialized (no window yet).");
        Self { window: None }
    }

    //--- Run --------------------------------------------------------------
    //
    // Starts the main event loop. Blocks execution until the window
    // is closed or the application exits.
    //
    pub fn run(&mut self) {
        info!(target: "platform_subsystem", "Starting main event loop");
        let event_loop = EventLoop::new().unwrap();
        event_loop.run_app(self).unwrap();
        info!(target: "platform_subsystem", "Event loop terminated.");
    }
}

//=== Winit Integration ===================================================
//
// Implements `ApplicationHandler` to handle Winit events.
// Manages window creation, close requests, and basic input handling.
//

impl ApplicationHandler for Platform {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop.create_window(
            winit::window::WindowAttributes::default()
                .with_title("Aetheric Engine — Day 1"),
        ).unwrap();
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                warn!(target: "platform_subsystem", "Close requested — exiting application.");
                event_loop.exit();   // Handle the close button request.
            }
            _ => {} // Ignore other events for now.
        }
    }
}