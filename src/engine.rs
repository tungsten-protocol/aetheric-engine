//=========================================================================
// Engine
//
// Entry point for the Aetheric game engine.
//
// Responsibilities:
// - Initialize global engine systems (input, etc.)
// - Instantiate and delegate control to the platform layer
// - Provide a clean, minimal API surface for the game/application
//
// Notes:
// The engine owns its core systems (e.g. InputManager) and shares them
// with the platform layer through reference-counted smart pointers (`Rc`).
// This allows `Platform` to access and mutate shared state (e.g. input)
// during the Winit-driven event loop.
//
//=========================================================================
use std::rc::Rc;
use std::cell::RefCell;

use crate::platform::Platform;
use crate::core::input::InputManager;

//=== Engine Struct =======================================================
//
// The main engine facade exposed to the application.
//
// Holds references to global systems that persist for the entire lifetime
// of the program. Currently includes only the input subsystem.
//
pub struct Engine {
    /// Shared handle to the input subsystem.
    ///
    /// Wrapped in `Rc<RefCell<...>>` to allow both the Engine
    /// and the Platform (running the Winit loop) to access and
    /// mutate input state safely.
    input_manager: Rc<RefCell<InputManager>>,
}

impl Engine {
    //--- Construction -----------------------------------------------------
    //
    // Initializes the engine and its subsystems, but does not start
    // the main loop yet.
    //
    pub fn new() -> Self {
        Self {
            input_manager: Rc::new(RefCell::new(InputManager::new())),
        }
    }

    //--- Run --------------------------------------------------------------
    //
    // Starts the engine, delegates control to the platform layer,
    // and blocks until the user closes the window or requests exit.
    //
    // Example:
    // ```no_run
    // fn main() {
    //     aetheric::Engine::run();
    // }
    // ```
    pub fn run(&mut self) {
        // Clone the shared InputManager handle so that Platform
        // can mutate it during the event loop.
        let mut platform = Platform::new(self.input_manager.clone());

        // Transfer control to the platform (Winit event loop).
        // This call blocks until the app exits.
        platform.run();
    }
}
