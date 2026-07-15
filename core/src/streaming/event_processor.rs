use super::Event;
use crate::adapters::AdapterError;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Event handler trait for processing events
#[async_trait::async_trait]
pub trait EventHandler: Send + Sync {
    /// Handle an event
    async fn handle(&self, event: &Event) -> Result<(), AdapterError>;

    /// Get handler name
    fn name(&self) -> &str {
        "unnamed_handler"
    }
}

/// Event processor for queuing and batch processing
pub struct EventProcessor {
    handlers: Arc<Mutex<Vec<Arc<dyn EventHandler>>>>,
    queue: Arc<Mutex<Vec<Event>>>,
    buffer_size: usize,
    processed_count: Arc<Mutex<u64>>,
}

impl EventProcessor {
    /// Create a new event processor
    pub fn new(buffer_size: usize) -> Self {
        Self {
            handlers: Arc::new(Mutex::new(Vec::new())),
            queue: Arc::new(Mutex::new(Vec::with_capacity(buffer_size))),
            buffer_size,
            processed_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Add an event handler
    pub async fn add_handler(&self, handler: Arc<dyn EventHandler>) {
        self.handlers.lock().await.push(handler);
    }

    /// Process a single event
    pub async fn process_event(&self, event: Event) -> Result<(), AdapterError> {
        let handlers = self.handlers.lock().await;

        for handler in handlers.iter() {
            handler.handle(&event).await?;
        }

        let mut queue = self.queue.lock().await;
        queue.push(event);

        if queue.len() >= self.buffer_size {
            self.flush().await?;
        }

        Ok(())
    }

    /// Flush buffered events
    pub async fn flush(&self) -> Result<(), AdapterError> {
        let mut queue = self.queue.lock().await;
        let count = queue.len() as u64;
        queue.clear();

        let mut processed = self.processed_count.lock().await;
        *processed += count;

        Ok(())
    }

    /// Get processed event count
    pub async fn processed_count(&self) -> u64 {
        *self.processed_count.lock().await
    }

    /// Get current queue size
    pub async fn queue_size(&self) -> usize {
        self.queue.lock().await.len()
    }

    /// Get handler count
    pub async fn handler_count(&self) -> usize {
        self.handlers.lock().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct TestHandler {
        name: String,
        processed: Arc<Mutex<usize>>,
    }

    #[async_trait::async_trait]
    impl EventHandler for TestHandler {
        async fn handle(&self, _event: &Event) -> Result<(), AdapterError> {
            let mut count = self.processed.lock().await;
            *count += 1;
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    #[tokio::test]
    async fn test_event_processor_creation() {
        let processor = EventProcessor::new(10);
        assert_eq!(processor.queue_size().await, 0);
        assert_eq!(processor.handler_count().await, 0);
    }

    #[tokio::test]
    async fn test_add_handler() {
        let processor = EventProcessor::new(10);
        let handler = Arc::new(TestHandler {
            name: "test".to_string(),
            processed: Arc::new(Mutex::new(0)),
        });

        processor.add_handler(handler).await;
        assert_eq!(processor.handler_count().await, 1);
    }

    #[tokio::test]
    async fn test_process_single_event() {
        let processor = EventProcessor::new(10);
        let handler = Arc::new(TestHandler {
            name: "test".to_string(),
            processed: Arc::new(Mutex::new(0)),
        });

        processor.add_handler(handler.clone()).await;

        let event = Event::new(
            super::super::EventType::EntityCreated,
            super::super::EventSource::Kafka {
                topic: "test".to_string(),
                partition: 0,
            },
            json!({"id": "1"}),
        );

        processor.process_event(event).await.unwrap();

        assert_eq!(*handler.processed.lock().await, 1);
        assert_eq!(processor.queue_size().await, 1);
    }

    #[tokio::test]
    async fn test_batch_flush() {
        let processor = EventProcessor::new(3);

        for i in 1..=5 {
            let event = Event::new(
                super::super::EventType::EntityCreated,
                super::super::EventSource::Kafka {
                    topic: "test".to_string(),
                    partition: 0,
                },
                json!({"id": i}),
            );
            processor.process_event(event).await.unwrap();
        }

        processor.flush().await.unwrap();
        assert_eq!(processor.queue_size().await, 0);
    }

    #[tokio::test]
    async fn test_processed_count() {
        let processor = EventProcessor::new(2);

        for i in 1..=3 {
            let event = Event::new(
                super::super::EventType::EntityCreated,
                super::super::EventSource::Kafka {
                    topic: "test".to_string(),
                    partition: 0,
                },
                json!({"id": i}),
            );
            processor.process_event(event).await.unwrap();
        }

        assert_eq!(processor.processed_count().await, 2); // Flushed after 2 events
    }
}
