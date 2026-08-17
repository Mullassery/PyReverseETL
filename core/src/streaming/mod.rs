pub mod event_processor;
pub mod event_schema;

pub use event_processor::{EventHandler, EventProcessor};
pub use event_schema::{Event, EventSource, EventType};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_module_loads() {
        // Module smoke test
    }
}
