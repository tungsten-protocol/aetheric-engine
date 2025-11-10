//=========================================================================
// Transition Queue
//=========================================================================
//
// Queue for scene transitions.
//
// Scenes queue transitions here during updates. The scene manager
// processes this queue at tick boundaries.
//
// Note: This will evolve into a general message bus in the future.
//
//=========================================================================

//=== Internal Dependencies ===============================================

use super::{SceneKey, SceneTransition};

//=== Transition Queue ====================================================

/// Queue for scene transitions.
///
/// Scenes queue transitions here during updates. The scene manager
/// processes this queue at tick boundaries.
///
/// Note: This will evolve into a general message bus in the future.
pub struct TransitionQueue<S: SceneKey> {
    queue: Vec<SceneTransition<S>>,
}

impl<S: SceneKey> TransitionQueue<S> {
    /// Creates a new empty transition queue.
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }

    /// Queues a scene transition to be processed at the next tick boundary.
    pub fn push(&mut self, transition: SceneTransition<S>) {
        self.queue.push(transition);
    }

    /// Returns an iterator over the queued transitions.
    pub fn iter(&self) -> impl Iterator<Item = &SceneTransition<S>> {
        self.queue.iter()
    }

    /// Returns true if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Returns the number of queued transitions.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Clears all queued transitions.
    pub fn clear(&mut self) {
        self.queue.clear()
    }

    /// Returns an iterator that drains all transitions from the queue.
    pub fn drain(&mut self) -> impl Iterator<Item = SceneTransition<S>> + '_ {
        self.queue.drain(..)
    }

    /// Takes all transitions from the queue, leaving it empty.
    ///
    /// Efficient operation using mem::swap internally. Used by scene manager
    /// to process all queued transitions.
    pub fn take(&mut self) -> Vec<SceneTransition<S>> {
        std::mem::take(&mut self.queue)
    }
}

impl<S: SceneKey> Default for TransitionQueue<S> {
    fn default() -> Self {
        Self::new()
    }
}
