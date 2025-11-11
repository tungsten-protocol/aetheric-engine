//=========================================================================
// Message Queue Trait
//=========================================================================
//
// Type-erased trait for message queues that preserves Vec operations
// while allowing storage in HashMap without concrete type knowledge.
//
//=========================================================================

//=== External Dependencies ===============================================

use std::any::Any;

//=== Internal Dependencies ===============================================

use super::Message;

//=========================================================================

/// Type-erased trait for message queue storage and operations.
///
/// Allows clearing queues and querying length without knowing the
/// concrete message type at compile time.
pub(super) trait MessageQueue: Send {
    /// Clears all messages while preserving allocated capacity.
    fn clear_queue(&mut self);

    /// Returns the number of messages currently queued.
    fn len(&self) -> usize;

    /// Returns true if the queue is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Downcasts to `&dyn Any` for type-specific operations.
    fn as_any(&self) -> &dyn Any;

    /// Downcasts to `&mut dyn Any` for type-specific operations.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

//=========================================================================

/// Implementation of MessageQueue for Vec<M>.
impl<M: Message> MessageQueue for Vec<M> {
    fn clear_queue(&mut self) {
        self.clear(); // Vec::clear preserves capacity
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

//=========================================================================
// Tests
//=========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Clone)]
    struct TestMessage {
        value: i32,
    }

    #[test]
    fn clear_queue_preserves_capacity() {
        let mut queue: Vec<TestMessage> = Vec::with_capacity(100);
        for i in 0..50 {
            queue.push(TestMessage { value: i });
        }

        let capacity_before = queue.capacity();
        assert_eq!(queue.len(), 50);

        // Clear via trait
        let queue_trait: &mut dyn MessageQueue = &mut queue;
        queue_trait.clear_queue();

        assert_eq!(queue.len(), 0);
        assert_eq!(queue.capacity(), capacity_before);
    }

    #[test]
    fn len_and_is_empty_work() {
        let mut queue: Vec<TestMessage> = Vec::new();

        // Test empty queue
        {
            let queue_trait: &dyn MessageQueue = &queue;
            assert!(queue_trait.is_empty());
            assert_eq!(queue_trait.len(), 0);
        }

        // Add message
        queue.push(TestMessage { value: 42 });

        // Test non-empty queue
        {
            let queue_trait: &dyn MessageQueue = &queue;
            assert!(!queue_trait.is_empty());
            assert_eq!(queue_trait.len(), 1);
        }
    }

    #[test]
    fn downcast_works() {
        let mut queue: Vec<TestMessage> = Vec::new();
        queue.push(TestMessage { value: 42 });

        let queue_trait: &mut dyn MessageQueue = &mut queue;

        // Downcast and read
        let vec_ref = queue_trait.as_any().downcast_ref::<Vec<TestMessage>>();
        assert!(vec_ref.is_some());
        assert_eq!(vec_ref.unwrap()[0].value, 42);

        // Downcast and mutate
        let vec_mut = queue_trait
            .as_any_mut()
            .downcast_mut::<Vec<TestMessage>>();
        assert!(vec_mut.is_some());
        vec_mut.unwrap().push(TestMessage { value: 99 });

        assert_eq!(queue.len(), 2);
    }
}
