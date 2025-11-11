//=========================================================================
// Event Collector
//=========================================================================
//
// Platform event collector with bounded polling and shutdown detection.
//
// Architecture:
//   Receiver<PlatformEvent> → collect_frame() → input_batches → TickControl
//
// Bounded polling prevents starvation. Idle sleep reduces CPU usage.
//
//=========================================================================

//=== External Dependencies ===============================================

use std::thread;
use std::time::Duration;

use crossbeam_channel::{Receiver, TryRecvError};
use log::warn;

//=== Internal Dependencies ===============================================

use super::PlatformEvent;
use crate::core::input::event::InputEvent;

//=== TickControl =========================================================

/// Update loop control signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TickControl {
    Continue,
    Exit,
}

//=== EventCollector ======================================================

/// Collects platform events with bounded polling and batch extraction.
pub(crate) struct EventCollector {
    receiver: Receiver<PlatformEvent>,
    input_batches: Vec<Vec<InputEvent>>,
}

impl EventCollector {
    pub(crate) fn new(receiver: Receiver<PlatformEvent>) -> Self {
        Self {
            receiver,
            input_batches: Vec::with_capacity(4),
        }
    }

    /// Collects pending platform events (bounded to prevent starvation).
    pub(crate) fn collect_frame(&mut self) -> TickControl {
        const MAX_EVENTS_PER_FRAME: usize = 100;
        const IDLE_SLEEP_MS: u64 = 10;

        self.input_batches.clear();
        let mut had_event = false;
        let mut drained = 0;

        while drained < MAX_EVENTS_PER_FRAME {
            match self.receiver.try_recv() {
                Ok(event) => {
                    had_event = true;
                    if self.handle_event(event) == TickControl::Exit {
                        return TickControl::Exit;
                    }
                    drained += 1;
                }
                Err(TryRecvError::Disconnected) => return TickControl::Exit,
                Err(TryRecvError::Empty) => break,
            }
        }

        if drained >= MAX_EVENTS_PER_FRAME {
            warn!("Event queue backlog: drained {} events this frame", drained);
        }

        if !had_event {
            thread::sleep(Duration::from_millis(IDLE_SLEEP_MS));
        }

        TickControl::Continue
    }

    /// Returns collected input batches for this frame.
    pub(crate) fn batches(&self) -> &[Vec<InputEvent>] {
        &self.input_batches
    }

    /// Takes ownership of collected input batches, leaving empty vec.
    ///
    /// Efficient transfer without allocation. The internal buffer is
    /// replaced with an empty Vec (will be cleared next frame anyway).
    pub(crate) fn take_batches(&mut self) -> Vec<Vec<InputEvent>> {
        std::mem::take(&mut self.input_batches)
    }

    fn handle_event(&mut self, event: PlatformEvent) -> TickControl {
        match event {
            PlatformEvent::Inputs { discrete, continuous } => {
                if !discrete.is_empty() {
                    self.input_batches.push(discrete);
                }
                if !continuous.is_empty() {
                    self.input_batches.push(continuous);
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
    use crate::core::input::{KeyCode, Modifiers};

    #[test]
    fn collect_handles_empty_queue() {
        let (_tx, rx) = unbounded::<PlatformEvent>();
        let mut collector = EventCollector::new(rx);

        let result = collector.collect_frame();

        assert_eq!(result, TickControl::Continue);
        assert!(collector.batches().is_empty());
    }

    #[test]
    fn collect_aggregates_multiple_events() {
        let (tx, rx) = unbounded();
        let mut collector = EventCollector::new(rx);

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

        let result = collector.collect_frame();

        assert_eq!(result, TickControl::Continue);
        assert_eq!(collector.batches().len(), 2);
    }

    #[test]
    fn collect_returns_exit_on_window_closed() {
        let (tx, rx) = unbounded();
        let mut collector = EventCollector::new(rx);

        tx.send(PlatformEvent::WindowClosed).unwrap();

        let result = collector.collect_frame();

        assert_eq!(result, TickControl::Exit);
    }

    #[test]
    fn collect_clears_previous_batches() {
        let (tx, rx) = unbounded();
        let mut collector = EventCollector::new(rx);

        tx.send(PlatformEvent::Inputs {
            discrete: vec![InputEvent::KeyDown {
                key: KeyCode::Space,
                modifiers: Modifiers::NONE
            }],
            continuous: vec![]
        }).unwrap();

        collector.collect_frame();
        assert_eq!(collector.batches().len(), 1);

        tx.send(PlatformEvent::Inputs {
            discrete: vec![],
            continuous: vec![]
        }).unwrap();

        collector.collect_frame();
        assert!(collector.batches().is_empty());
    }

    #[test]
    fn collect_returns_exit_on_disconnect() {
        let (tx, rx) = unbounded::<PlatformEvent>();
        let mut collector = EventCollector::new(rx);

        drop(tx);

        let result = collector.collect_frame();

        assert_eq!(result, TickControl::Exit);
    }
}