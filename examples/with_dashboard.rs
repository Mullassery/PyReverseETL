/// Example: Running ActivationPipeline with Stats Dashboard
///
/// This example demonstrates:
/// 1. Creating an ActivationPipeline
/// 2. Launching a separate stats dashboard in a terminal window
/// 3. Real-time metric monitoring while processing events
/// 4. Platform-aware terminal launching (macOS/Linux)

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::INFO)
            .finish(),
    ).ok();

    tracing::info!("PyReverseETL Dashboard Example Starting...");

    // Create workflow and activation
    let workflow = Arc::new(pyreverseetl_core::Workflow::new(
        "orders_sync".to_string(),
        "demo_owner".to_string(),
        pyreverseetl_core::workflow::SourceType::Table {
            table_name: "orders".to_string(),
        },
    ));

    let activation = Arc::new(pyreverseetl_core::Activation::new(
        "orders_activation".to_string(),
        workflow.id.clone(),
        "demo_owner".to_string(),
    ));

    // Create the activation pipeline
    let pipeline = Arc::new(
        pyreverseetl_core::ActivationPipeline::new(workflow, activation)
            .await?
    );

    // Start the pipeline
    pipeline.start().await?;
    tracing::info!("Pipeline started");

    // Configure and launch the dashboard
    let dashboard_config = pyreverseetl_core::DashboardConfig {
        server_url: "http://localhost:9999".to_string(),
        refresh_interval_ms: 1000,
        history_size: 300,
    };

    tracing::info!("Launching stats dashboard...");
    match pyreverseetl_core::launch_dashboard(dashboard_config) {
        Ok(mut child) => {
            tracing::info!("Dashboard launched successfully (PID: {:?})", child.id());

            // Give the dashboard a moment to start
            sleep(Duration::from_millis(500)).await;
        }
        Err(e) => {
            tracing::warn!("Failed to launch dashboard: {}. Continuing without dashboard.", e);
        }
    }

    // Create a metrics server for the dashboard to connect to
    let metrics_server = Arc::new(pyreverseetl_core::MetricsServer::new());
    tracing::info!("Metrics server created");

    // Simulate processing events
    tracing::info!("Starting event processing simulation...");
    let start_time = std::time::Instant::now();

    for i in 0..100 {
        // Simulate an event
        let event = pyreverseetl_core::Event::new(
            format!("order-{}", i),
            pyreverseetl_core::EventType::Created,
            Default::default(),
        );

        // Process the event
        match pipeline.process_event(event).await {
            Ok(_) => {
                if i % 10 == 0 {
                    tracing::info!("Processed {} events", i);
                }
            }
            Err(e) => {
                tracing::error!("Error processing event: {}", e);
            }
        }

        // Record metrics periodically
        if i % 5 == 0 {
            let metrics = pipeline.metrics().await;
            let status = pipeline.status().await;

            metrics_server.record_metrics(&metrics, status.error_count).await;

            tracing::debug!(
                "Metrics: {} evt/sec, {} ms latency",
                metrics.throughput_eps,
                metrics.average_latency_ms
            );
        }

        // Small delay to simulate real processing
        sleep(Duration::from_millis(10)).await;
    }

    // Get final metrics
    let final_status = pipeline.status().await;
    let final_metrics = &final_status.metrics;

    tracing::info!("Event processing completed!");
    tracing::info!("Performance Summary:");
    tracing::info!("  Events processed: {}", final_metrics.events_processed);
    tracing::info!("  Events failed: {}", final_metrics.events_failed);
    tracing::info!("  Throughput: {:.1} evt/sec", final_metrics.throughput_eps);
    tracing::info!("  Avg latency: {:.1} ms", final_metrics.average_latency_ms);
    tracing::info!("  P99 latency: {} ms", final_metrics.p99_latency_ms);
    tracing::info!("  Quality checks passed: {}", final_metrics.quality_checks_passed);
    tracing::info!("  Quality checks failed: {}", final_metrics.quality_checks_failed);
    tracing::info!("  Errors: {}", final_status.error_count);

    let elapsed = start_time.elapsed();
    tracing::info!("Total time: {:.2}s", elapsed.as_secs_f64());

    // Stop the pipeline
    pipeline.stop().await?;
    tracing::info!("Pipeline stopped");

    tracing::info!("Example completed successfully!");
    Ok(())
}
