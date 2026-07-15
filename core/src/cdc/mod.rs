pub mod change_detector;
pub mod changelog;
pub mod checkpoint;

pub use change_detector::{Change, ChangeDetector, ChangeType};
pub use changelog::{ChangeLog, ChangeLogEntry};
pub use checkpoint::{Checkpoint, CheckpointManager};

#[cfg(test)]
mod tests {
    #[test]
    fn test_cdc_module_loads() {
        // Module smoke test
    }
}
