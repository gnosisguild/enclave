use anyhow::Result;
use config::AppConfig;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing::Level;
use tracing_subscriber::layer::{Layer, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;

pub fn setup_tracing(config: &AppConfig, log_level: Level) -> Result<()> {
    let name = config.name();
    let maybe_otel_endpoint = config.otel();
    let subscriber = tracing_subscriber::registry().with(
        tracing_subscriber::fmt::layer().without_time().with_filter(
            tracing_subscriber::filter::LevelFilter::from_level(log_level),
        ),
    );

    match maybe_otel_endpoint {
        Some(endpoint) => {
            let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .with_endpoint(endpoint.to_string())
                .build()?;

            let provider = SdkTracerProvider::builder()
                .with_batch_exporter(otlp_exporter)
                .build();

            let tracer = provider.tracer(name.unwrap_or("default-name".to_string()));
            let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

            subscriber.with(telemetry).init();
        }
        None => {
            subscriber.init();
        }
    }

    Ok(())
}
