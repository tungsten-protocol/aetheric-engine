//=========================================================================
// Scene Manager
//=========================================================================
//
// Manages scene registration, stack operations, and lifecycle.
//
// Scenes are stored in a HashMap by key and referenced via a stack
// of keys. This allows scenes to maintain state between activations.
//
//=========================================================================

//=== External Dependencies ===============================================

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use log::{debug, warn};

//=== Internal Dependencies ===============================================

use crate::core::globals::GlobalContext;
use super::Scene;

//=== Scene Transition ====================================================

/// Encapsulates scene stack operations.
///
/// Scenes are managed via a stack-based system where transitions control
/// the flow between different game states (menus, gameplay, pause, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneTransition<K: SceneKey> {
    /// Adds a new scene to the top of the stack.
    Push(K),

    /// Removes a specific scene from the stack by key.
    Remove(K),

    /// Replaces a specific scene with another scene.
    Replace(K, K),

    /// Clears all scenes from the stack.
    Clear,

    /// No transition occurs.
    Empty,
}

impl<K: SceneKey> Default for SceneTransition<K> {
    fn default() -> Self {
        Self::Empty
    }
}

//=== Scene Key Trait =====================================================

/// Marker trait for scene identifiers.
///
/// Scene keys uniquely identify scenes in the SceneManager's HashMap.
/// Typically implemented by game-specific enums.
pub trait SceneKey: Clone + Copy + Eq + Hash + Debug + Send + 'static {}

//=== Scene Manager =======================================================

/// Manages scene lifecycle and stack-based scene switching.
///
/// Scenes are registered once and referenced by key. The scene stack
/// determines which scenes are active, with the topmost scene receiving
/// input and rendering priority.
///
pub struct SceneManager<S: SceneKey> {
    scenes: HashMap<S, Box<dyn Scene<S>>>,
    stack: Vec<S>,
}

impl<S: SceneKey> SceneManager<S> {
    //--- Construction -----------------------------------------------------

    /// Creates a new scene manager with an empty stack.
    ///
    /// Scenes must be registered and pushed via transitions before any
    /// scene updates occur.
    pub fn new() -> Self {
        Self {
            scenes: HashMap::new(),
            stack: Vec::new(),
        }
    }

    //--- Registration -----------------------------------------------------

    /// Registers a scene with the manager.
    ///
    /// Scenes must be registered before being pushed to the stack.
    /// The scene is automatically boxed for storage.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aetheric_engine::prelude::*;
    /// # #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    /// # enum GameScene { Main }
    /// # impl SceneKey for GameScene {}
    /// # struct MainScene;
    /// # impl Scene<GameScene> for MainScene {
    /// #     fn update(&mut self, _ctx: &GlobalContext) {}
    /// # }
    /// # let mut manager = SceneManager::new();
    /// manager.register_scene(GameScene::Main, MainScene);
    /// ```
    pub fn register_scene<T>(&mut self, key: S, scene: T)
    where
        T: Scene<S> + 'static,
    {
        if self.scenes.insert(key, Box::new(scene)).is_some() {
            warn!("Scene {:?} was already registered and has been replaced", key);
        }
    }

    /// Registers a scene and immediately adds it to the stack as the default scene.
    ///
    /// This is a convenience method for initial scene setup during engine
    /// initialization. It combines registration and stack initialization in
    /// one call. The `on_enter` lifecycle hook will be called automatically
    /// when the engine starts running. The scene is automatically boxed for storage.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use aetheric_engine::prelude::*;
    /// # #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    /// # enum GameScene { Main }
    /// # impl SceneKey for GameScene {}
    /// # struct MainScene;
    /// # impl Scene<GameScene> for MainScene {
    /// #     fn update(&mut self, _ctx: &GlobalContext) {}
    /// # }
    /// # let mut manager = SceneManager::new();
    /// manager.register_default(GameScene::Main, MainScene);
    /// ```
    pub fn register_default<T>(&mut self, key: S, scene: T)
    where
        T: Scene<S> + 'static,
    {
        // Register the scene
        self.register_scene(key, scene);

        // Add to stack if not already present
        if self.stack.contains(&key) {
            warn!("Scene {:?} is already in the stack", key);
        } else {
            debug!("Registered scene {:?} as default and added to stack", key);
            self.stack.push(key);
        }
    }

    /// Initializes the scene manager by calling on_enter on the initial scene.
    pub fn start(&mut self, context: &GlobalContext) {
        if let Some(&initial) = self.stack.first() {
            debug!("Starting scene manager with initial scene: {:?}", initial);
            if let Some(scene) = self.scenes.get_mut(&initial) {
                scene.on_enter(context);
            } else {
                warn!("Initial scene {:?} not registered", initial);
            }
        }
    }

    //--- Update Loop ------------------------------------------------------

    /// Updates active scenes.
    ///
    /// Calls update on all transparent scenes and the topmost opaque scene.
    pub fn update(&mut self, context: &GlobalContext) {
        if self.stack.is_empty() {
            return;
        }

        // Collect active scenes (based on transparency)
        let scenes_to_update = self.collect_active_scenes();

        // Update all active scenes
        self.update_scenes(&scenes_to_update, context);
    }

    //--- Transition Processing --------------------------------------------

    /// Processes all queued scene transitions.
    ///
    /// Should be called at the tick boundary after scene updates.
    /// Transitions are processed in FIFO order, with appropriate lifecycle
    /// callbacks (on_enter/on_exit) invoked for affected scenes.
    pub fn process_transitions(&mut self, context: &mut GlobalContext) {
        // Read all scene transitions from message bus
        for transition in context.message_bus.read::<SceneTransition<S>>() {
            match transition {
                SceneTransition::Push(key) => self.push_internal(*key, context),
                SceneTransition::Remove(key) => self.remove_internal(*key, context),
                SceneTransition::Replace(old_key, new_key) => {
                    self.replace_internal(*old_key, *new_key, context)
                }
                SceneTransition::Clear => self.clear_internal(context),
                SceneTransition::Empty => {}
            }
        }

        // Clear processed transitions
        context.message_bus.clear::<SceneTransition<S>>();
    }

    //--- Internal Helpers -------------------------------------------------

    fn push_internal(&mut self, key: S, context: &GlobalContext) {
        // Check if scene is already in the stack
        if self.stack.contains(&key) {
            warn!("Scene {:?} is already in the stack, skipping push", key);
            return;
        }

        // Check if scene is registered
        if !self.scenes.contains_key(&key) {
            warn!("Attempted to push unregistered scene {:?}", key);
            return;
        }

        debug!("Pushing scene {:?} onto stack", key);
        self.stack.push(key);

        if let Some(scene) = self.scenes.get_mut(&key) {
            scene.on_enter(context);
        }
    }

    fn remove_internal(&mut self, key: S, context: &GlobalContext) {
        if let Some(pos) = self.stack.iter().position(|&k| k == key) {
            debug!("Removing scene {:?} from stack at position {}", key, pos);
            self.stack.remove(pos);

            if let Some(scene) = self.scenes.get_mut(&key) {
                scene.on_exit(context);
            }
        } else {
            debug!("Scene {:?} not found in stack, skipping removal", key);
        }
    }

    fn replace_internal(&mut self, old_key: S, new_key: S, context: &GlobalContext) {
        // Check if old scene exists in stack
        let Some(pos) = self.stack.iter().position(|&k| k == old_key) else {
            warn!("Scene {:?} not found in stack, skipping replacement", old_key);
            return;
        };

        // Check if new scene is already in the stack
        if self.stack.contains(&new_key) {
            warn!("Scene {:?} is already in the stack, skipping replacement", new_key);
            return;
        }

        // Check if new scene is registered
        if !self.scenes.contains_key(&new_key) {
            warn!("Attempted to replace with unregistered scene {:?}", new_key);
            return;
        }

        debug!("Replacing scene {:?} with {:?} at position {}", old_key, new_key, pos);

        // Call on_exit for old scene
        if let Some(scene) = self.scenes.get_mut(&old_key) {
            scene.on_exit(context);
        }

        // Replace in stack
        self.stack[pos] = new_key;

        // Call on_enter for new scene
        if let Some(scene) = self.scenes.get_mut(&new_key) {
            scene.on_enter(context);
        }
    }

    fn clear_internal(&mut self, context: &GlobalContext) {
        debug!("Clearing all scenes from stack");

        // Call on_exit for all scenes in the stack
        for &key in &self.stack {
            if let Some(scene) = self.scenes.get_mut(&key) {
                scene.on_exit(context);
            }
        }

        self.stack.clear();
    }

    fn collect_active_scenes(&self) -> Vec<S> {
        let mut active = Vec::new();

        // Iterate stack top-down, stop at first opaque scene
        for &key in self.stack.iter().rev() {
            active.insert(0, key);

            if let Some(scene) = self.scenes.get(&key) {
                if !scene.is_transparent() {
                    break;
                }
            }
        }

        active
    }

    fn update_scenes(
        &mut self,
        scenes_to_update: &[S],
        context: &GlobalContext,
    ) {
        // Update all active scenes
        for &key in scenes_to_update {
            if let Some(scene) = self.scenes.get_mut(&key) {
                scene.update(context);
            }
        }
    }
}

//=== Tests ===============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Mock types for testing
    #[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
    enum TestScene {
        A,
        B,
        C,
    }

    impl SceneKey for TestScene {}

    //--- SceneTransition Tests --------------------------------------------

    #[test]
    fn transition_default_is_empty() {
        let transition: SceneTransition<TestScene> = SceneTransition::default();
        assert_eq!(transition, SceneTransition::Empty);
    }

    #[test]
    fn transition_is_copy_and_eq() {
        let t1 = SceneTransition::Push(TestScene::A);
        let t2 = t1;
        assert_eq!(t1, t2);

        let t3 = SceneTransition::Remove(TestScene::B);
        let t4 = t3;
        assert_eq!(t3, t4);

        let t5 = SceneTransition::Replace(TestScene::A, TestScene::B);
        let t6 = t5;
        assert_eq!(t5, t6);
    }

    // TODO: Add SceneManager tests when Scene trait is available
}
