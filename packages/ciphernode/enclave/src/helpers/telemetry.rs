use anyhow::Result;
use config::AppConfig;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub fn setup_tracing(config: &AppConfig, log_level: Level) -> Result<()> {
    let name = config.name();
    let maybe_otel_endpoint = config.otel();

    match maybe_otel_endpoint {
        Some(endpoint) => {
            let parsed_endpoint = format!(
                // Add http protocol
                "http://{}",
                // ensure we are using localhost instead of 127.0.0.1
                // socket address needs to use a specific IP
                // we might be able to get around this by just passing in a free string but we
                // loose input validation
                endpoint.to_string().replace("127.0.0.1", "localhost")
            );

            let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .with_endpoint(parsed_endpoint)
                .with_protocol(Protocol::Grpc)
                .build()?;

            let resource = Resource::builder()
                .with_service_name(name.unwrap_or("default-name".to_string()))
                .build();

            let provider = SdkTracerProvider::builder()
                .with_batch_exporter(otlp_exporter)
                .with_resource(resource)
                .build();

            let tracer = provider.tracer("enclave");
            let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer())
                .with(telemetry)
                .with(tracing_subscriber::filter::LevelFilter::from_level(
                    log_level,
                ))
                .init();
        }
        None => {
            // TODO: we might be able to dedupe this with above but there were
            //       issues with telemetry so have left this like so for now
            tracing_subscriber::registry()
                .with(tracing_subscriber::fmt::layer())
                .with(tracing_subscriber::filter::LevelFilter::from_level(
                    log_level,
                ))
                .init();
        }
    }

    Ok(())
}
