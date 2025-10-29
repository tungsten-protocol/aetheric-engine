//=========================================================================
// State Tracker
//
// Low-level input state tracking across frames.
//
// Maintains both persistent state (keys/buttons currently held) and
// frame deltas (transitions that occurred this frame) for efficient
// querying by the input system.
//
// Architecture:
// ```text
// State Tracker Lifecycle (per frame):
//
//   1. clear()
//      ├─► Clears pressed/released deltas
//      └─► Saves last_mouse_position
//
//   2. process_events(&[batch1, batch2, ...])
//      ├─► Updates keys_down (persistent)
//      ├─► Updates keys_pressed_this_frame (delta)
//      └─► Updates modifiers
//
//   3. finalize_frame()
//      └─► Calculates mouse_delta
//
// Data Organization:
// ┌──────────────────────────────────────┐
// │       StateTracker                   │
// ├──────────────────────────────────────┤
// │ Persistent State:                    │
// │  • keys_down            (HashSet)    │
// │  • mouse_buttons_down   (HashSet)    │
// │  • mouse_position       (f32, f32)   │
// │  • modifiers            (struct)     │
// ├──────────────────────────────────────┤
// │ Frame Deltas (reset each frame):     │
// │  • keys_pressed_this_frame           │
// │  • keys_released_this_frame          │
// │  • mouse_buttons_pressed_this_frame  │
// │  • mouse_buttons_released_this_frame │
// ├──────────────────────────────────────┤
// │ Continuous Input:                    │
// │  • mouse_delta (calculated)          │
// │  • last_mouse_position (tracking)    │
// └──────────────────────────────────────┘
//
//=========================================================================

//=== Internal Imports ====================================================

use super::event::{Modifiers, InputEvent, KeyCode, MouseButton};
use std::collections::HashSet;

//=== StateTracker ========================================================

/// Low-level input state tracker.
///
/// Processes raw input events and maintains queryable state for both
/// persistent conditions (keys held) and frame transitions (keys pressed).
///
/// # Usage
///
/// Call in this order each frame:
/// 1. [`clear()`](Self::clear) - Reset deltas
/// 2. [`process_events()`](Self::process_events) - Update from events
/// 3. [`finalize_frame()`](Self::finalize_frame) - Calculate derived values
/// 4. Query state via `is_key_*()` methods
///
/// # Internal Use Only
///
/// This type is `pub(crate)` - it's used by [`InputSystem`] but not exposed
/// to game code. Games should use [`InputSystem`]'s query methods instead.
pub(crate) struct StateTracker {
    //--- Persistent State (survives frame boundary) ----------------------

    /// Keys currently held down.
    ///
    /// Updated on KeyDown (insert) and KeyUp (remove). Persists across
    /// frames until explicitly released.
    keys_down: HashSet<KeyCode>,

    /// Mouse buttons currently held down.
    mouse_buttons_down: HashSet<MouseButton>,

    /// Current mouse position in screen coordinates (pixels, top-left origin).
    mouse_position: (f32, f32),

    /// Current modifier key state (Shift, Ctrl, Alt).
    ///
    /// Updated on every key/button event. Reflects the most recent modifier
    /// state reported by the platform.
    modifiers: Modifiers,

    //--- Frame Deltas (reset each frame via clear()) --------------------

    /// Keys that transitioned UP → DOWN this frame.
    ///
    /// Only contains keys that were NOT down last frame. Used for discrete
    /// actions like jumping or toggling menus.
    keys_pressed_this_frame: HashSet<KeyCode>,

    /// Keys that transitioned DOWN → UP this frame.
    keys_released_this_frame: HashSet<KeyCode>,

    /// Mouse buttons that transitioned UP → DOWN this frame.
    mouse_buttons_pressed_this_frame: HashSet<MouseButton>,

    /// Mouse buttons that transitioned DOWN → UP this frame.
    mouse_buttons_released_this_frame: HashSet<MouseButton>,

    //--- Continuous Input (accumulated/calculated) -----------------------

    /// Mouse movement delta for this frame.
    ///
    /// Calculated in [`finalize_frame()`](Self::finalize_frame) as:
    /// `mouse_position - last_mouse_position`.
    mouse_delta: (f32, f32),

    /// Mouse position at the start of this frame.
    ///
    /// Used to calculate `mouse_delta`. Updated in [`clear()`](Self::clear).
    last_mouse_position: (f32, f32),
}

impl StateTracker {
    //--- Construction -----------------------------------------------------

    /// Creates a new state tracker with empty state.
    ///
    /// All keys/buttons are unpressed, mouse is at origin, no modifiers held.
    pub(crate) fn new() -> Self {
        Self {
            keys_down: HashSet::new(),
            mouse_buttons_down: HashSet::new(),
            mouse_position: (0.0, 0.0),
            modifiers: Modifiers::NONE,
            keys_pressed_this_frame: HashSet::new(),
            keys_released_this_frame: HashSet::new(),
            mouse_buttons_pressed_this_frame: HashSet::new(),
            mouse_buttons_released_this_frame: HashSet::new(),
            mouse_delta: (0.0, 0.0),
            last_mouse_position: (0.0, 0.0),
        }
    }

    //--- Frame Processing -------------------------------------------------

    /// Clears frame-specific deltas in preparation for new events.
    ///
    /// **Must be called at the start of each frame** before processing events.
    ///
    /// Clears:
    /// - `keys_pressed_this_frame`
    /// - `keys_released_this_frame`
    /// - `mouse_buttons_pressed_this_frame`
    /// - `mouse_buttons_released_this_frame`
    ///
    /// Preserves:
    /// - `keys_down` (persistent state)
    /// - `mouse_buttons_down`
    /// - `modifiers`
    ///
    /// Also updates `last_mouse_position` for delta calculation.
    pub(crate) fn clear(&mut self) {
        self.keys_pressed_this_frame.clear();
        self.keys_released_this_frame.clear();
        self.mouse_buttons_pressed_this_frame.clear();
        self.mouse_buttons_released_this_frame.clear();
        self.last_mouse_position = self.mouse_position;
    }

    /// Processes a batch of input events.
    ///
    /// Updates both persistent state (keys_down) and frame deltas
    /// (keys_pressed_this_frame). Call once per event batch.
    ///
    /// Multiple batches can be processed per frame (e.g., discrete events
    /// followed by continuous events).
    pub(crate) fn process_events(&mut self, events: &[InputEvent]) {
        for event in events {
            self.process_event(event);
        }
    }

    /// Finalizes frame calculations after all events are processed.
    ///
    /// **Must be called after all event batches** and before querying state.
    ///
    /// Currently calculates:
    /// - `mouse_delta` = `mouse_position - last_mouse_position`
    pub(crate) fn finalize_frame(&mut self) {
        self.mouse_delta = (
            self.mouse_position.0 - self.last_mouse_position.0,
            self.mouse_position.1 - self.last_mouse_position.1,
        );
    }

    //--- Internal Helpers -------------------------------------------------

    /// Processes a single input event.
    ///
    /// Updates state based on event type. Only tracks transitions for
    /// pressed/released - duplicate events are ignored (e.g., KeyDown
    /// while already down).
    fn process_event(&mut self, event: &InputEvent) {
        match event {
            InputEvent::KeyDown { key, modifiers } => {
                self.modifiers = *modifiers;
                // Only mark as pressed if it wasn't already down
                if self.keys_down.insert(*key) {
                    self.keys_pressed_this_frame.insert(*key);
                }
            }

            InputEvent::KeyUp { key, modifiers } => {
                self.modifiers = *modifiers;
                // Only mark as released if it was actually down
                if self.keys_down.remove(key) {
                    self.keys_released_this_frame.insert(*key);
                }
            }

            InputEvent::MouseButtonDown { button, modifiers } => {
                self.modifiers = *modifiers;
                if self.mouse_buttons_down.insert(*button) {
                    self.mouse_buttons_pressed_this_frame.insert(*button);
                }
            }

            InputEvent::MouseButtonUp { button, modifiers } => {
                self.modifiers = *modifiers;
                if self.mouse_buttons_down.remove(button) {
                    self.mouse_buttons_released_this_frame.insert(*button);
                }
            }

            InputEvent::MouseMoved { x, y } => {
                self.mouse_position = (*x, *y);
            }

            InputEvent::Unidentified => {
                // Ignore unrecognized events
            }
        }
    }

    //=====================================================================
    // Query API - Keyboard
    //=====================================================================

    /// Returns `true` if the key transitioned from UP to DOWN this frame.
    ///
    /// Only true on the frame the key was first pressed. Use [`is_key_down`]
    /// for continuous input (held keys).
    ///
    /// Use for discrete actions: jump, interact, toggle menu.
    pub(crate) fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed_this_frame.contains(&key)
    }

    /// Returns `true` while the key is held down.
    ///
    /// True on every frame from press until release (inclusive of first frame).
    ///
    /// Use for continuous actions: movement, charging attacks.
    pub(crate) fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    /// Returns `true` if the key transitioned from DOWN to UP this frame.
    ///
    /// Use for release-dependent actions: end charge attack.
    pub(crate) fn is_key_released(&self, key: KeyCode) -> bool {
        self.keys_released_this_frame.contains(&key)
    }

    //=====================================================================
    // Query API - Mouse Buttons
    //=====================================================================

    /// Returns `true` if the button transitioned from UP to DOWN this frame.
    pub(crate) fn is_button_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_pressed_this_frame.contains(&button)
    }

    /// Returns `true` while the button is held down.
    pub(crate) fn is_button_down(&self, button: MouseButton) -> bool {
        self.mouse_buttons_down.contains(&button)
    }

    /// Returns `true` if the button transitioned from DOWN to UP this frame.
    pub(crate) fn is_button_released(&self, button: MouseButton) -> bool {
        self.mouse_buttons_released_this_frame.contains(&button)
    }

    //=====================================================================
    // Query API - Mouse Position & Movement
    //=====================================================================

    /// Returns the current mouse position in screen coordinates.
    ///
    /// Coordinates are in pixels with origin at top-left (0,0).
    pub(crate) fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    /// Returns the mouse movement delta for this frame.
    ///
    /// `(0.0, 0.0)` if mouse didn't move. Positive x = right, positive y = down.
    ///
    /// Useful for camera control, drag operations, cursor acceleration.
    pub(crate) fn mouse_delta(&self) -> (f32, f32) {
        self.mouse_delta
    }

    //=====================================================================
    // Query API - Modifiers
    //=====================================================================

    /// Returns the current modifier key state.
    pub(crate) fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// Returns `true` if Shift is currently held.
    pub(crate) fn shift_held(&self) -> bool {
        self.modifiers.shift
    }

    /// Returns `true` if Ctrl is currently held.
    pub(crate) fn ctrl_held(&self) -> bool {
        self.modifiers.ctrl
    }

    /// Returns `true` if Alt is currently held.
    pub(crate) fn alt_held(&self) -> bool {
        self.modifiers.alt
    }

    //=====================================================================
    // Query API - Iteration
    //=====================================================================

    /// Returns an iterator over all keys pressed this frame.
    ///
    /// Useful for action mapping systems that need to check all inputs.
    pub(crate) fn keys_pressed(&self) -> impl Iterator<Item = &KeyCode> {
        self.keys_pressed_this_frame.iter()
    }

    /// Returns an iterator over all keys released this frame.
    pub(crate) fn keys_released(&self) -> impl Iterator<Item = &KeyCode> {
        self.keys_released_this_frame.iter()
    }

    /// Returns an iterator over all mouse buttons pressed this frame.
    pub(crate) fn buttons_pressed(&self) -> impl Iterator<Item = &MouseButton> {
        self.mouse_buttons_pressed_this_frame.iter()
    }

    /// Returns an iterator over all mouse buttons released this frame.
    pub(crate) fn buttons_released(&self) -> impl Iterator<Item = &MouseButton> {
        self.mouse_buttons_released_this_frame.iter()
    }
}

//--- Trait Implementations -----------------------------------------------

impl Default for StateTracker {
    fn default() -> Self {
        Self::new()
    }
}

//=========================================================================
// Unit Tests
//=========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    //--- Test Helpers -----------------------------------------------------

    fn key_down(key: KeyCode) -> InputEvent {
        InputEvent::KeyDown { key, modifiers: Modifiers::NONE }
    }

    fn key_up(key: KeyCode) -> InputEvent {
        InputEvent::KeyUp { key, modifiers: Modifiers::NONE }
    }

    fn mouse_down(btn: MouseButton) -> InputEvent {
        InputEvent::MouseButtonDown { button: btn, modifiers: Modifiers::NONE }
    }

    fn mouse_up(btn: MouseButton) -> InputEvent {
        InputEvent::MouseButtonUp { button: btn, modifiers: Modifiers::NONE }
    }

    fn mouse_move(x: f32, y: f32) -> InputEvent {
        InputEvent::MouseMoved { x, y }
    }

    //=====================================================================
    // Keyboard Tests
    //=====================================================================

    /// Tests that key_pressed only returns true on transition frame.
    #[test]
    fn key_pressed_only_on_transition_frame() {
        let mut system = StateTracker::new();

        // Frame 1: Key down
        system.clear();
        system.process_events(&[key_down(KeyCode::KeyA)]);
        system.finalize_frame();
        assert!(system.is_key_pressed(KeyCode::KeyA));
        assert!(system.is_key_down(KeyCode::KeyA));

        // Frame 2: Still held
        system.clear();
        system.process_events(&[]);
        system.finalize_frame();
        assert!(!system.is_key_pressed(KeyCode::KeyA));
        assert!(system.is_key_down(KeyCode::KeyA));

        // Frame 3: Released
        system.clear();
        system.process_events(&[key_up(KeyCode::KeyA)]);
        system.finalize_frame();
        assert!(!system.is_key_pressed(KeyCode::KeyA));
        assert!(!system.is_key_down(KeyCode::KeyA));
        assert!(system.is_key_released(KeyCode::KeyA));
    }

    /// Tests that key_down persists across frames.
    #[test]
    fn key_down_persists_across_frames() {
        let mut system = StateTracker::new();

        system.process_events(&[key_down(KeyCode::KeyW)]);
        assert!(system.is_key_down(KeyCode::KeyW));

        // Hold for multiple frames
        for _ in 0..10 {
            system.clear();
            system.process_events(&[]);
            system.finalize_frame();
            assert!(system.is_key_down(KeyCode::KeyW), "Key should remain down");
        }
    }

    /// Tests that multiple keys are tracked independently.
    #[test]
    fn multiple_keys_tracked_independently() {
        let mut system = StateTracker::new();

        system.process_events(&[
            key_down(KeyCode::KeyW),
            key_down(KeyCode::KeyA),
            key_down(KeyCode::KeyS),
        ]);

        assert!(system.is_key_down(KeyCode::KeyW));
        assert!(system.is_key_down(KeyCode::KeyA));
        assert!(system.is_key_down(KeyCode::KeyS));
        assert!(!system.is_key_down(KeyCode::KeyD));

        // Release one
        system.clear();
        system.process_events(&[key_up(KeyCode::KeyA)]);

        assert!(system.is_key_down(KeyCode::KeyW));
        assert!(!system.is_key_down(KeyCode::KeyA));
        assert!(system.is_key_down(KeyCode::KeyS));
    }

    /// Tests fast tap (press + release same frame).
    #[test]
    fn fast_tap_both_transitions_captured() {
        let mut system = StateTracker::new();

        // Same frame: press AND release
        system.process_events(&[
            key_down(KeyCode::KeyA),
            key_up(KeyCode::KeyA),
        ]);

        assert!(system.is_key_pressed(KeyCode::KeyA), "Should register press");
        assert!(system.is_key_released(KeyCode::KeyA), "Should register release");
        assert!(!system.is_key_down(KeyCode::KeyA), "Should end up not down");
    }

    /// Tests duplicate KeyDown is ignored.
    #[test]
    fn duplicate_key_down_ignored() {
        let mut system = StateTracker::new();

        // Press key
        system.process_events(&[key_down(KeyCode::KeyA)]);
        assert!(system.is_key_pressed(KeyCode::KeyA));

        system.clear();

        // Press same key again (shouldn't happen, but handle gracefully)
        system.process_events(&[key_down(KeyCode::KeyA)]);
        assert!(!system.is_key_pressed(KeyCode::KeyA), "Duplicate press should not trigger");
        assert!(system.is_key_down(KeyCode::KeyA), "Should still be down");
    }

    /// Tests spurious KeyUp is ignored.
    #[test]
    fn key_up_without_down_ignored() {
        let mut system = StateTracker::new();

        // Release key that was never pressed
        system.process_events(&[key_up(KeyCode::KeyZ)]);

        assert!(!system.is_key_released(KeyCode::KeyZ), "Should not register spurious release");
    }

    //=====================================================================
    // Mouse Button Tests
    //=====================================================================

    /// Tests mouse button pressed and down states.
    #[test]
    fn mouse_button_pressed_and_down() {
        let mut system = StateTracker::new();

        system.process_events(&[mouse_down(MouseButton::Left)]);

        assert!(system.is_button_pressed(MouseButton::Left));
        assert!(system.is_button_down(MouseButton::Left));

        // Next frame: still down
        system.clear();
        system.process_events(&[]);

        assert!(!system.is_button_pressed(MouseButton::Left));
        assert!(system.is_button_down(MouseButton::Left));
    }

    /// Tests mouse button released.
    #[test]
    fn mouse_button_released() {
        let mut system = StateTracker::new();

        system.process_events(&[mouse_down(MouseButton::Right)]);

        system.clear();
        system.process_events(&[mouse_up(MouseButton::Right)]);

        assert!(system.is_button_released(MouseButton::Right));
        assert!(!system.is_button_down(MouseButton::Right));
    }

    //=====================================================================
    // Mouse Movement Tests
    //=====================================================================

    /// Tests mouse position is updated.
    #[test]
    fn mouse_position_updated() {
        let mut system = StateTracker::new();

        system.process_events(&[mouse_move(100.0, 200.0)]);

        assert_eq!(system.mouse_position(), (100.0, 200.0));
    }

    /// Tests mouse delta is calculated correctly.
    #[test]
    fn mouse_delta_calculated() {
        let mut system = StateTracker::new();

        // Frame 1: move to (100, 100)
        system.clear();
        system.process_events(&[mouse_move(100.0, 100.0)]);
        system.finalize_frame();
        assert_eq!(system.mouse_delta(), (100.0, 100.0));

        // Frame 2: move to (150, 120)
        system.clear();
        system.process_events(&[mouse_move(150.0, 120.0)]);
        system.finalize_frame();
        assert_eq!(system.mouse_delta(), (50.0, 20.0));

        // Frame 3: no movement
        system.clear();
        system.process_events(&[]);
        system.finalize_frame();
        assert_eq!(system.mouse_delta(), (0.0, 0.0));
    }

    //=====================================================================
    // Modifier Tests
    //=====================================================================

    /// Tests that modifiers are updated from events.
    #[test]
    fn modifiers_updated_on_key_events() {
        let mut system = StateTracker::new();

        system.process_events(&[InputEvent::KeyDown {
            key: KeyCode::KeyA,
            modifiers: Modifiers::CTRL,
        }]);

        assert!(system.ctrl_held());
        assert!(!system.shift_held());
        assert_eq!(system.modifiers(), Modifiers::CTRL);
    }

    //=====================================================================
    // Iterator Tests
    //=====================================================================

    /// Tests keys_pressed iterator.
    #[test]
    fn keys_pressed_iterator() {
        let mut system = StateTracker::new();

        system.process_events(&[
            key_down(KeyCode::KeyA),
            key_down(KeyCode::KeyB),
        ]);

        let pressed: Vec<_> = system.keys_pressed().copied().collect();
        assert_eq!(pressed.len(), 2);
        assert!(pressed.contains(&KeyCode::KeyA));
        assert!(pressed.contains(&KeyCode::KeyB));
    }

    /// Tests buttons_pressed iterator.
    #[test]
    fn buttons_pressed_iterator() {
        let mut system = StateTracker::new();

        system.process_events(&[
            mouse_down(MouseButton::Left),
            mouse_down(MouseButton::Right),
        ]);

        let pressed: Vec<_> = system.buttons_pressed().copied().collect();
        assert_eq!(pressed.len(), 2);
    }

    //=====================================================================
    // clear() Tests
    //=====================================================================

    /// Tests that clear resets frame deltas but preserves persistent state.
    #[test]
    fn clear_resets_frame_deltas() {
        let mut system = StateTracker::new();

        system.process_events(&[key_down(KeyCode::KeyA)]);
        assert!(system.is_key_pressed(KeyCode::KeyA));

        system.clear();

        assert!(!system.is_key_pressed(KeyCode::KeyA)); // Delta cleared
        assert!(system.is_key_down(KeyCode::KeyA));     // Persistent state remains
    }

    /// Tests that clear updates last_mouse_position.
    #[test]
    fn clear_updates_last_mouse_position() {
        let mut system = StateTracker::new();

        system.process_events(&[mouse_move(100.0, 200.0)]);
        system.finalize_frame();

        let old_pos = system.mouse_position();
        system.clear();

        // last_mouse_position should now equal old position
        // (We can't directly access it, but finalize should give 0 delta)
        system.finalize_frame();
        assert_eq!(system.mouse_delta(), (0.0, 0.0));
    }

    //=====================================================================
    // finalize_frame() Tests
    //=====================================================================

    /// Tests finalize_frame calculates mouse delta correctly.
    #[test]
    fn finalize_frame_calculates_delta() {
        let mut system = StateTracker::new();

        // Manually set positions to test calculation
        system.mouse_position = (100.0, 100.0);
        system.last_mouse_position = (80.0, 90.0);

        system.finalize_frame();

        assert_eq!(system.mouse_delta(), (20.0, 10.0));
    }

    //=====================================================================
    // Edge Cases
    //=====================================================================

    /// Tests that unidentified events are safely ignored.
    #[test]
    fn unidentified_events_ignored() {
        let mut system = StateTracker::new();

        system.process_events(&[InputEvent::Unidentified]);

        // Should not panic or change state
        assert_eq!(system.mouse_position(), (0.0, 0.0));
    }

    /// Tests empty event batch is handled correctly.
    #[test]
    fn empty_event_batch_handled() {
        let mut system = StateTracker::new();

        system.clear();
        system.process_events(&[]);
        system.finalize_frame();

        // Should not panic
        assert_eq!(system.mouse_delta(), (0.0, 0.0));
    }
}