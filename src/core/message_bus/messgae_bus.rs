//=========================================================================
// Message Bus
//=========================================================================
//
// Type-safe multi-consumer message queue for inter-system communication.
//
// Architecture:
//   Systems → push<M>() → HashMap<TypeId, Vec<M>>
//                              ↓
//   Multiple consumers ← read<M>() (shared)
//                              ↓
//   Coordinator ────────→ clear<M>() at tick boundary
//
// Pattern: push → read (N consumers) → clear → repeat
//
//=========================================================================

//=== External Dependencies ===============================================

use std::any::TypeId;
use std::collections::HashMap;

//=== Internal Dependencies ===============================================

use super::message_queue::MessageQueue;

//=== Public API ==========================================================

/// Marker trait for types that can be sent through the MessageBus.
///
/// Automatically implemented for all types that are Send + 'static.
pub trait Message: Send + 'static {}

// Blanket implementation
impl<T: Send + 'static> Message for T {}

//=========================================================================

/// Type-safe message queue for batched inter-system communication.
///
/// Maintains separate queues per message type, allowing systems to push
/// messages during updates and process them at tick boundaries.
pub struct MessageBus {
    queues: HashMap<TypeId, Box<dyn MessageQueue>>,
}

impl MessageBus {
    /// Creates a new empty message bus.
    pub fn new() -> Self {
        MessageBus {
            queues: HashMap::new(),
        }
    }

    //--- Message Operations -----------------------------------------------

    /// Pushes a message into the queue for its type.
    pub fn push<M: Message>(&mut self, msg: M) {
        let type_id = TypeId::of::<M>();

        let boxed_queue: &mut Box<dyn MessageQueue> = self.queues
            .entry(type_id)
            .or_insert_with(|| Box::new(Vec::<M>::new()));

        let queue: &mut Vec<M> = boxed_queue
            .as_any_mut()
            .downcast_mut::<Vec<M>>()
            .expect("Type mismatch in MessageBus queue");

        queue.push(msg);
    }

    /// Returns a slice of all messages of type M currently queued.
    ///
    /// Supports multi-consumer pattern: multiple systems can read the same
    /// messages in a single frame. Call `clear<M>()` after all consumers
    /// have processed the messages.
    pub fn read<M: Message>(&self) -> &[M] {
        self.queues
            .get(&TypeId::of::<M>())
            .and_then(|q| q.as_any().downcast_ref::<Vec<M>>())
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    //--- Query API --------------------------------------------------------

    /// Returns true if there are any messages of type M queued.
    pub fn has_messages<M: Message>(&self) -> bool {
        self.queues
            .get(&TypeId::of::<M>())
            .and_then(|q| q.as_any().downcast_ref::<Vec<M>>())
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    }

    /// Returns the number of messages of type M currently queued.
    pub fn count<M: Message>(&self) -> usize {
        self.queues
            .get(&TypeId::of::<M>())
            .and_then(|q| q.as_any().downcast_ref::<Vec<M>>())
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Clears all messages of type M, preserving allocated capacity.
    ///
    /// Does not deallocate the underlying Vec, allowing efficient reuse
    /// across frames for recurring message types.
    pub fn clear<M: Message>(&mut self) {
        if let Some(queue) = self.queues.get_mut(&TypeId::of::<M>()) {
            if let Some(vec) = queue.as_any_mut().downcast_mut::<Vec<M>>() {
                vec.clear();
            }
        }
    }

    /// Clears all queues for all message types, preserving capacity.
    ///
    /// Iterates through all queues and calls clear() on each, preserving
    /// both HashMap entries and Vec capacity for efficient reuse.
    pub fn clear_all(&mut self) {
        for queue in self.queues.values_mut() {
            queue.clear_queue();
        }
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

    #[derive(Debug, PartialEq, Clone)]
    struct OtherMessage {
        text: String,
    }

    #[test]
    fn new_bus_is_empty() {
        let bus = MessageBus::new();
        assert!(!bus.has_messages::<TestMessage>());
        assert_eq!(bus.count::<TestMessage>(), 0);
        assert_eq!(bus.read::<TestMessage>().len(), 0);
    }

    #[test]
    fn push_and_read_single_message() {
        let mut bus = MessageBus::new();
        bus.push(TestMessage { value: 42 });

        assert!(bus.has_messages::<TestMessage>());
        assert_eq!(bus.count::<TestMessage>(), 1);

        let messages = bus.read::<TestMessage>();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].value, 42);

        // Messages still present until clear
        assert!(bus.has_messages::<TestMessage>());
    }

    #[test]
    fn push_multiple_messages_same_type() {
        let mut bus = MessageBus::new();
        bus.push(TestMessage { value: 1 });
        bus.push(TestMessage { value: 2 });
        bus.push(TestMessage { value: 3 });

        assert_eq!(bus.count::<TestMessage>(), 3);

        let messages = bus.read::<TestMessage>();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].value, 1);
        assert_eq!(messages[1].value, 2);
        assert_eq!(messages[2].value, 3);
    }

    #[test]
    fn separate_queues_per_type() {
        let mut bus = MessageBus::new();
        bus.push(TestMessage { value: 42 });
        bus.push(OtherMessage {
            text: "hello".to_string(),
        });
        bus.push(TestMessage { value: 99 });

        assert_eq!(bus.count::<TestMessage>(), 2);
        assert_eq!(bus.count::<OtherMessage>(), 1);

        let test_msgs = bus.read::<TestMessage>();
        assert_eq!(test_msgs.len(), 2);
        assert_eq!(test_msgs[0].value, 42);
        assert_eq!(test_msgs[1].value, 99);

        // Both queues still have messages
        assert_eq!(bus.count::<TestMessage>(), 2);
        assert_eq!(bus.count::<OtherMessage>(), 1);

        let other_msgs = bus.read::<OtherMessage>();
        assert_eq!(other_msgs.len(), 1);
        assert_eq!(other_msgs[0].text, "hello");
    }

    #[test]
    fn read_empty_queue_returns_empty_slice() {
        let bus = MessageBus::new();
        let messages = bus.read::<TestMessage>();
        assert!(messages.is_empty());
    }

    #[test]
    fn clear_removes_messages() {
        let mut bus = MessageBus::new();
        bus.push(TestMessage { value: 42 });
        bus.push(TestMessage { value: 99 });

        assert_eq!(bus.count::<TestMessage>(), 2);
        bus.clear::<TestMessage>();
        assert_eq!(bus.count::<TestMessage>(), 0);
        assert!(!bus.has_messages::<TestMessage>());
    }

    #[test]
    fn clear_all_removes_all_types() {
        let mut bus = MessageBus::new();
        bus.push(TestMessage { value: 42 });
        bus.push(OtherMessage {
            text: "test".to_string(),
        });

        bus.clear_all();
        assert_eq!(bus.count::<TestMessage>(), 0);
        assert_eq!(bus.count::<OtherMessage>(), 0);
    }

    #[test]
    fn clear_all_preserves_capacity_and_entries() {
        let mut bus = MessageBus::new();

        // Allocate multiple queues with significant capacity
        for i in 0..50 {
            bus.push(TestMessage { value: i });
            bus.push(OtherMessage {
                text: format!("msg_{}", i),
            });
        }

        assert_eq!(bus.count::<TestMessage>(), 50);
        assert_eq!(bus.count::<OtherMessage>(), 50);

        // Clear all should preserve HashMap entries and Vec capacity
        bus.clear_all();

        assert_eq!(bus.count::<TestMessage>(), 0);
        assert_eq!(bus.count::<OtherMessage>(), 0);

        // Push again - should reuse existing allocations efficiently
        bus.push(TestMessage { value: 1 });
        bus.push(OtherMessage {
            text: "new".to_string(),
        });

        assert_eq!(bus.count::<TestMessage>(), 1);
        assert_eq!(bus.count::<OtherMessage>(), 1);
    }

    #[test]
    fn multiple_read_calls_return_same_data() {
        let mut bus = MessageBus::new();
        bus.push(TestMessage { value: 42 });
        bus.push(TestMessage { value: 99 });

        // Multi-consumer pattern: both "systems" see the same messages
        let first_read = bus.read::<TestMessage>();
        assert_eq!(first_read.len(), 2);
        assert_eq!(first_read[0].value, 42);

        let second_read = bus.read::<TestMessage>();
        assert_eq!(second_read.len(), 2);
        assert_eq!(second_read[0].value, 42);

        // Data unchanged until clear
        assert_eq!(bus.count::<TestMessage>(), 2);
    }

    #[test]
    fn clear_preserves_capacity() {
        let mut bus = MessageBus::new();

        // Push messages to allocate capacity
        for i in 0..100 {
            bus.push(TestMessage { value: i });
        }

        assert_eq!(bus.count::<TestMessage>(), 100);

        // Clear should keep capacity
        bus.clear::<TestMessage>();
        assert_eq!(bus.count::<TestMessage>(), 0);

        // Push again - should reuse allocation
        bus.push(TestMessage { value: 1 });
        assert_eq!(bus.count::<TestMessage>(), 1);
    }

    #[test]
    fn read_clear_read_pattern() {
        let mut bus = MessageBus::new();
        bus.push(TestMessage { value: 42 });

        // Frame 1: read and clear
        let msgs = bus.read::<TestMessage>();
        assert_eq!(msgs.len(), 1);
        bus.clear::<TestMessage>();

        // Frame 2: read should be empty
        let msgs = bus.read::<TestMessage>();
        assert!(msgs.is_empty());

        // Frame 3: new messages
        bus.push(TestMessage { value: 99 });
        let msgs = bus.read::<TestMessage>();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].value, 99);
    }
}
