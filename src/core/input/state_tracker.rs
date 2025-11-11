//=========================================================================
// State Tracker
//=========================================================================
//
// Low-level input state tracking with per-frame delta tracking.
//
// Architecture:
//   InputEvent → process_events() → HashSet (keys/buttons held) → query
//
// Frame lifecycle: clear() → process_events() → finalize_frame() → query
//
//=========================================================================

//=== External Dependencies ===============================================

use std::collections::HashSet;

//=== Internal Dependencies ===============================================

use super::event::{Modifiers, InputEvent, KeyCode, MouseButton};

//=== StateTracker ========================================================

/// Tracks persistent state (keys held) and per-frame deltas (keys pressed/released).
/// Frame lifecycle: clear() → process_events() → finalize_frame() → query.
pub struct StateTracker {
    //--- Persistent State (survives frame boundary) ----------------------
    keys_down: HashSet<KeyCode>,
    mouse_buttons_down: HashSet<MouseButton>,
    mouse_position: (f32, f32),
    modifiers: Modifiers,

    //--- Frame Deltas (reset each frame via clear()) --------------------
    keys_pressed_this_frame: HashSet<KeyCode>,
    keys_released_this_frame: HashSet<KeyCode>,
    mouse_buttons_pressed_this_frame: HashSet<MouseButton>,
    mouse_buttons_released_this_frame: HashSet<MouseButton>,

    //--- Continuous Input (accumulated/calculated) -----------------------
    mouse_delta: (f32, f32),
    last_mouse_position: (f32, f32),
}

impl StateTracker {
    /// Creates a new state tracker with empty state.
    pub fn new() -> Self {
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

    /// Clears frame-specific deltas (pressed/released flags).
    pub(super) fn clear(&mut self) {
        self.keys_pressed_this_frame.clear();
        self.keys_released_this_frame.clear();
        self.mouse_buttons_pressed_this_frame.clear();
        self.mouse_buttons_released_this_frame.clear();
        self.last_mouse_position = self.mouse_position;
    }

    /// Processes input events, updating internal state.
    pub(super) fn process_events(&mut self, events: &[InputEvent]) {
        for event in events {
            self.process_event(event);
        }
    }

    /// Finalizes frame calculations (calculates mouse delta).
    pub(super) fn finalize_frame(&mut self) {
        self.mouse_delta = (
            self.mouse_position.0 - self.last_mouse_position.0,
            self.mouse_position.1 - self.last_mouse_position.1,
        );
    }

    //--- Internal Helpers -------------------------------------------------
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

    /// Returns `true` if key transitioned UP → DOWN (one frame only).
    ///
    /// Use for discrete actions like jumping or toggling menus.
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_pressed_this_frame.contains(&key)
    }

    /// Returns `true` while key is held.
    ///
    /// Use for continuous actions like movement or charging.
    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    /// Returns `true` if key transitioned DOWN → UP.
    ///
    /// Use for release-dependent actions like ending a charge attack.
    pub fn is_key_released(&self, key: KeyCode) -> bool {
        self.keys_released_this_frame.contains(&key)
    }

    //=====================================================================
    // Query API - Mouse Buttons
    //=====================================================================

    /// Like [`is_key_pressed`](Self::is_key_pressed) but for mouse buttons.
    pub fn is_button_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons_pressed_this_frame.contains(&button)
    }

    /// Like [`is_key_down`](Self::is_key_down) but for mouse buttons.
    pub fn is_button_down(&self, button: MouseButton) -> bool {
        self.mouse_buttons_down.contains(&button)
    }

    /// Like [`is_key_released`](Self::is_key_released) but for mouse buttons.
    pub fn is_button_released(&self, button: MouseButton) -> bool {
        self.mouse_buttons_released_this_frame.contains(&button)
    }

    //=====================================================================
    // Query API - Mouse Position & Movement
    //=====================================================================

    /// Returns mouse position in screen coordinates (pixels, top-left origin).
    pub fn mouse_position(&self) -> (f32, f32) {
        self.mouse_position
    }

    /// Returns mouse movement delta (0,0 if no movement).
    ///
    /// Useful for camera control, drag operations, etc.
    pub fn mouse_delta(&self) -> (f32, f32) {
        self.mouse_delta
    }


    //=====================================================================
    // Query API - Modifiers
    //=====================================================================

    /// Returns the current modifier key state.
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// Returns `true` if Shift is currently held.
    pub fn shift_held(&self) -> bool {
        self.modifiers.shift
    }

    /// Returns `true` if Ctrl is currently held.
    pub fn ctrl_held(&self) -> bool {
        self.modifiers.ctrl
    }

    /// Returns `true` if Alt is currently held.
    pub fn alt_held(&self) -> bool {
        self.modifiers.alt
    }

    //=====================================================================
    // Query API - Iteration
    //=====================================================================

    /// Returns an iterator over all keys currently held.
    pub fn keys_down(&self) -> impl Iterator<Item = &KeyCode> {
        self.keys_down.iter()
    }
    
    /// Returns an iterator over all keys pressed.
    pub fn keys_pressed(&self) -> impl Iterator<Item = &KeyCode> {
        self.keys_pressed_this_frame.iter()
    }

    /// Returns an iterator over all keys released.
    pub fn keys_released(&self) -> impl Iterator<Item = &KeyCode> {
        self.keys_released_this_frame.iter()
    }

    /// Returns an iterator over all mouse buttons currently held.
    pub fn buttons_down(&self) -> impl Iterator<Item = &MouseButton> {
        self.mouse_buttons_down.iter()
    }

    /// Returns an iterator over all mouse buttons pressed.
    pub fn buttons_pressed(&self) -> impl Iterator<Item = &MouseButton> {
        self.mouse_buttons_pressed_this_frame.iter()
    }

    /// Returns an iterator over all mouse buttons released.
    pub fn buttons_released(&self) -> impl Iterator<Item = &MouseButton> {
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