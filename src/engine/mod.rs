use crate::platform::Platform;


pub mod event;
pub struct Engine {
    platform: Platform,
}

impl Engine {
    pub fn new() -> Self {
        let platform = Platform::new();
        Self { platform }
    }

    pub fn run(&mut self) {
        self.platform.run();
    }
}