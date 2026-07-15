pub mod activation_pipeline;
pub mod backpressure;
pub mod latency_tracker;

pub use activation_pipeline::{ActivationPipeline, PipelineMetrics, PipelineStatus};
pub use backpressure::{BackpressureManager, BackpressureSignal};
pub use latency_tracker::{LatencyStats, LatencyTracker};

#[cfg(test)]
mod tests {
    #[test]
    fn test_pipeline_module_loads() {
        // Module smoke test
    }
}
