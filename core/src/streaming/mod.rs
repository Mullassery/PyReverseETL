pub mod event_schema;
pub mod event_processor;

pub use event_schema::{Event, EventType, EventSource};
pub use event_processor::{EventProcessor, EventHandler};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_module_loads() {
        // Module smoke test
    }
}
