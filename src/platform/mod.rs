//=========================================================================
// Platform Subsystem
//=========================================================================
//
// Winit adapter: translates OS events to engine InputEvents via MPSC.
//
// Architecture:
//   Winit Events → InputProcessor → InputBuffer → PlatformEvent (MPSC) → Core
//
// Frame Boundary: RedrawRequested triggers flush of all buffered input.
//
// Thread Model: Must run on main thread (macOS/iOS requirement).
//
//=========================================================================

//=== External Dependencies ===============================================

use crossbeam_channel::Sender;
use log::*;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowAttributes},
};

//=== Internal Dependencies ===============================================

use input_buffer::InputBuffer;
use input_processor::InputProcessor;

use crate::core::platform_bridge::{PlatformError, PlatformEvent};

//=== Module Declarations =================================================

mod input_buffer;
mod input_processor;

//=== Platform ============================================================

/// Winit wrapper: manages window and sends input to core thread.
pub(crate) struct Platform {
    window: Option<Window>,
    buffer: InputBuffer,
    event_sender: Sender<PlatformEvent>,
    input_processor: InputProcessor,
}

impl Platform {
    //--- Construction -----------------------------------------------------

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

    /// Starts Winit event loop (never returns normally).
    ///
    /// # Errors
    /// Returns `PlatformError` if event loop creation fails.
    ///
    /// # Panics
    /// Panics if called off main thread (macOS/iOS).
    pub fn run(mut self) -> Result<(), PlatformError> {
        debug!(target: "platform", "Starting Winit event loop");

        let event_loop = EventLoop::new()
            .map_err(|e| PlatformError::EventLoopCreation(e.to_string()))?;

        event_loop.set_control_flow(ControlFlow::Poll);

        event_loop.run_app(&mut self)
            .map_err(|e| PlatformError::EventLoopExecution(e.to_string()))
    }

    //--- Internal ---------------------------------------------------------

    fn flush_input_buffer(&mut self) {
        if let Some((discrete, continuous)) = self.buffer.drain() {
            trace!(
                target: "platform::input",
                "Flushing {} discrete + {} continuous events",
                discrete.len(),
                continuous.len()
            );

            if self.event_sender.send(PlatformEvent::Inputs { discrete, continuous }).is_err() {
                warn!(target: "platform::input", "Channel disconnected, dropping events");
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn window(&self) -> Option<&Window> {
        self.window.as_ref()
    }
}

//=== Winit Integration ===================================================

impl ApplicationHandler for Platform {
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

                let _ = self.event_sender.send(PlatformEvent::WindowClosed);
                event_loop.exit();
            }
        }
    }

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
                self.flush_input_buffer();

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            _ => {}
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
    use crate::core::input::event::InputEvent;

    #[test]
    fn platform_creation() {
        let (tx, _rx) = unbounded();
        let platform = Platform::new(tx);
        assert!(platform.window().is_none());
    }

    #[test]
    fn flush_empty_buffer_is_noop() {
        let (tx, rx) = unbounded();
        let mut platform = Platform::new(tx);

        platform.flush_input_buffer();

        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn flush_sends_buffered_events() {
        let (tx, rx) = unbounded();
        let mut platform = Platform::new(tx);

        platform.buffer.push_discrete(InputEvent::KeyDown {
            key: KeyCode::Space,
            modifiers: Modifiers::NONE,
        });

        platform.flush_input_buffer();

        match rx.try_recv() {
            Ok(PlatformEvent::Inputs { discrete, continuous }) => {
                assert_eq!(discrete.len(), 1);
                assert!(continuous.is_empty());
            }
            other => panic!("Expected Inputs event, got {:?}", other),
        }
    }

    #[test]
    fn flush_handles_disconnected_channel() {
        let (tx, rx) = unbounded();
        let mut platform = Platform::new(tx);

        platform.buffer.push_discrete(InputEvent::KeyDown {
            key: KeyCode::Space,
            modifiers: Modifiers::NONE,
        });

        drop(rx);

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
        platform.flush_input_buffer();

        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_err());
    }
}