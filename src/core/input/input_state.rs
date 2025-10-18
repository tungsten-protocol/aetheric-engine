//=========================================================================
// Input State
//
// Maintains the current input snapshot for the engine.
// Consumes normalized `RawInputEvent`s from the platform layer and updates
// persistent states (keyboard, mouse, etc.).
//
// Responsibilities:
// - Track discrete inputs (pressed / released keys & buttons)
// - Track continuous inputs (mouse position, analog values, etc.)
// - Detect per-frame state changes for efficient updates
//
// Notes:
// This module is internal to the InputSystem and not exposed directly.
// It is responsible for mutating state only — queries are performed
// through higher-level system APIs.
//
//=========================================================================

//=== Standard Library Imports ============================================
use std::collections::HashSet;
use std::fmt;

//=== External Crates =====================================================
use log::warn;

//=== Internal Modules ====================================================
use crate::core::input::event::{KeyCode, MouseButton, RawInputEvent};

//=== DiscreteInput =======================================================
//
// Represents binary input elements such as keyboard keys or mouse buttons.
// Presence in the `discrete` set indicates an active (pressed) state.
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum DiscreteInput {
    Key(KeyCode),
    Button(MouseButton),
}

//=== InputState ==========================================================
//
// Core engine-level input state container.
// Tracks both discrete (on/off) and continuous (positional) input values.
//
pub struct InputState {
    pub(super) discrete: HashSet<DiscreteInput>,
    pub(super) mouse: (f32, f32),
    pub(super) has_changed: bool,
}

impl InputState {
    //--- Constructor ------------------------------------------------------
    //
    // Creates a new, empty input state.
    //
    pub fn new() -> Self {
        const DISCRETE_BASE: usize = 128;
        Self {
            discrete: HashSet::with_capacity(DISCRETE_BASE),
            mouse: (0.0, 0.0),
            has_changed: false,
        }
    }

    //--- press_discrete() -------------------------------------------------
    //
    // Marks a discrete input as pressed.
    // Returns `true` if it was newly added (state changed).
    //
    fn press_discrete(&mut self, input: DiscreteInput) -> bool {
        self.discrete.insert(input)
    }

    //--- release_discrete() ----------------------------------------------
    //
    // Marks a discrete input as released.
    // Returns `true` if it was previously pressed (state changed).
    //
    fn release_discrete(&mut self, input: DiscreteInput) -> bool {
        self.discrete.remove(&input)
    }

    //--- digest_input_buffer() -------------------------------------------
    //
    // Processes a batch of `RawInputEvent`s and updates the current state.
    //
    // Discrete inputs are deduplicated via the `HashSet`. Continuous inputs
    // (like mouse movement) always trigger `has_changed` if their value differs.
    //
    pub(super) fn digest_input_buffer(&mut self, event_buffer: &[RawInputEvent]) {
        if event_buffer.is_empty() {
            self.has_changed = false;
            return;
        }

        let mut changed = false;

        for event in event_buffer {
            match *event {
                //--- Discrete Inputs -------------------------------------
                RawInputEvent::KeyDown(key) => {
                    changed |= self.press_discrete(DiscreteInput::Key(key));
                }
                RawInputEvent::KeyUp(key) => {
                    changed |= self.release_discrete(DiscreteInput::Key(key));
                }
                RawInputEvent::MouseButtonDown(btn) => {
                    changed |= self.press_discrete(DiscreteInput::Button(btn));
                }
                RawInputEvent::MouseButtonUp(btn) => {
                    changed |= self.release_discrete(DiscreteInput::Button(btn));
                }

                //--- Continuous Inputs -----------------------------------
                RawInputEvent::MouseMoved { x, y } => {
                    let old_pos = self.mouse;
                    if old_pos != (x, y) {
                        self.mouse = (x, y);
                        changed = true;
                    }
                }

                _ => warn!("Unhandled input event: {:?}", event),
            }
        }

        self.has_changed = changed;
    }

    //--- reset_changed() --------------------------------------------------
    //
    // Resets the "changed" flag at the end of each frame.
    //
    pub(super) fn reset_changed(&mut self) {
        self.has_changed = false;
    }
}

//=== Debug Trait =========================================================
//
// Provides human-readable logging for debugging purposes.
// Example:
//
// ```text
// InputState {
//     mouse: (420.0, 255.0),
//     has_changed: true,
//     pressed: ["Key(W)", "Button(Left)"]
// }
// ```
//
impl fmt::Debug for InputState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pressed: Vec<_> = self
            .discrete
            .iter()
            .map(|k| format!("{:?}", k))
            .collect();

        f.debug_struct("InputState")
            .field("mouse", &self.mouse)
            .field("has_changed", &self.has_changed)
            .field("pressed", &pressed)
            .finish()
    }
}
