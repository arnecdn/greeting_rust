use once_cell::sync::Lazy;
use opentelemetry::{global, KeyValue};
use opentelemetry::logs::LogError;
use opentelemetry::propagation::Injector;
use opentelemetry::trace::TraceError;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::logs::LoggerProvider;
use opentelemetry_sdk::{Resource, runtime};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{Config, TracerProvider};
use rdkafka::message::{Headers, OwnedHeaders};

static RESOURCE: Lazy<Resource> = Lazy::new(|| {
    Resource::new(vec![KeyValue::new(
        opentelemetry_semantic_conventions::resource::SERVICE_NAME,
        "greeting_rust",
    )])
});

pub(crate) fn init_logs(otlp_endpoint: &str) -> Result<LoggerProvider, LogError> {
    opentelemetry_otlp::new_pipeline()
        .logging()
        .with_resource(RESOURCE.clone())
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otlp_endpoint),
        )
        .install_batch(runtime::Tokio)
}

pub(crate) fn init_tracer_provider(otlp_endpoint: &str) -> Result<TracerProvider, TraceError> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(otlp_endpoint),
        )
        .with_trace_config(Config::default().with_resource(RESOURCE.clone()))
        .install_batch(runtime::Tokio)
}

pub struct HeaderInjector<'a>(pub &'a mut OwnedHeaders);

impl <'a>Injector for HeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        let mut new = OwnedHeaders::new().insert(rdkafka::message::Header {
            key,
            value: Some(&value),
        });

        for header in self.0.iter() {
            let s = String::from_utf8(header.value.unwrap().to_vec()).unwrap();
            new = new.insert(rdkafka::message::Header { key: header.key, value: Some(&s) });
        }

        self.0.clone_from(&new);
    }
}
