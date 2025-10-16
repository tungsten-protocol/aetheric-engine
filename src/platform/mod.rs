//=========================================================================
// Platform
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

mod event_mapper;
mod input_buffer;

use winit::{
    application::ApplicationHandler,
    event::{WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::Window,
};
use log::*;
use std::rc::Rc;
use std::cell::RefCell;
use input_buffer::InputBuffer;
use crate::core::input::event::RawInputEvent;
use crate::core::input::input_manager::InputManager;

//=== Platform Struct =====================================================

pub(crate) struct Platform {
    buffer: InputBuffer,
    input_manager: Rc<RefCell<InputManager>>,
    window: Option<Window>,
}

impl Platform {
    pub fn new(input_manager: Rc<RefCell<InputManager>>) -> Self {
        info!(target: "platform_subsystem", "Platform subsystem initialized (no window yet).");
        Self {
            window: None,
            buffer: InputBuffer::new(),
            input_manager,
        }
    }

    pub fn run(&mut self) {
        info!(target: "platform_subsystem", "Starting main event loop");
        let event_loop = EventLoop::new().unwrap();
        event_loop.run_app(self).unwrap();
        info!(target: "platform_subsystem", "Event loop terminated.");
    }

    fn process_input(&mut self) {
        let events = self.buffer.drain();
        let mut im = self.input_manager.borrow_mut();
        im.digest_input_buffer(events);

        if im.has_changed() {
            info!("Input updated: {:?}", *im);
        }
    }
}

//=== Winit Integration ===================================================

impl ApplicationHandler for Platform {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(
                winit::window::WindowAttributes::default()
                    .with_title("Aetheric Engine — Day 1"),
            )
            .unwrap();
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
                event_loop.exit();
            }

            WindowEvent::CursorMoved { .. } =>
                self.buffer.push_continuous(RawInputEvent::from(event)),

            WindowEvent::KeyboardInput { .. } | WindowEvent::MouseInput { .. } =>
                self.buffer.push_discrete(RawInputEvent::from(event)),

            WindowEvent::RedrawRequested => {
                self.process_input();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }

            _ => warn!("Unhandled window event: {:?}", event),
        }
    }
}
