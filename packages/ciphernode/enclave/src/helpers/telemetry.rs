use anyhow::Result;
use config::AppConfig;
use opentelemetry::trace::{Tracer, TracerProvider as _};
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
pub fn setup_tracing(_config: &AppConfig, log_level: Level) -> Result<()> {
    // Initialize OTLP exporter using gRPC (Tonic)
    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()?;
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
        .build();
    let _ = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(otlp_exporter)
        .build();

    let tracer = provider.tracer("readme_example");
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry().with(telemetry).init();

    Ok(())
}
