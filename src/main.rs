use std::process::exit;
use std::sync::RwLock;

use actix_web::{App, HttpServer};

use actix_web::web::Data;

use log::{error, info, Level};
use once_cell::sync::Lazy;
use opentelemetry::{global, KeyValue};
use opentelemetry::logs::LogError;
use opentelemetry::propagation::Injector;
use opentelemetry::trace::TraceError;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::logs::LoggerProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::{Resource, runtime};
use opentelemetry_sdk::trace::{Config, TracerProvider};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use rdkafka::message::{Headers, OwnedHeaders};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use greeting::{api, service};
use settings::Settings;

use crate::greeting::kafka_producer::KafkaGreetingRepository;
use crate::greeting::service::GreetingService;

mod greeting;
mod settings;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    #[derive(OpenApi)]
    #[openapi(
        info(description = "Greeting Api description"),
        paths(api::greet, api::list_greetings),
        components(schemas(api::GreetingDto))
    )]

    struct ApiDoc;
    let app_config = Settings::new();

    let result = init_tracer_provider(&app_config.otel_collector.oltp_endpoint);
    let tracer_provider = result.unwrap();
    global::set_tracer_provider(tracer_provider.clone());
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Initialize logs and save the logger_provider.
    let logger_provider = init_logs(&app_config.otel_collector.oltp_endpoint).unwrap();

    // Create a new OpenTelemetryTracingBridge using the above LoggerProvider.
    let layer = OpenTelemetryTracingBridge::new(&logger_provider);
    tracing_subscriber::registry()
        .with(layer)
        .init();

    info!("Starting server");

    let repo = match KafkaGreetingRepository::new(app_config.kafka, "producer_1") {
        Ok(r) => r,
        Err(e) => {
            error!("{:?}", e);
            exit(1)
        }
    };

    //Need explicit type in order to enforce type restrictions with dynamoc trait object allocation
    let service_impl = service::GreetingServiceImpl::new(repo);
    let svc: Data<RwLock<Box<dyn GreetingService + Sync + Send>>> =
        Data::new(RwLock::new(Box::new(service_impl)));

    HttpServer::new(move || {
        App::new()
            .app_data(svc.clone())
            .service(api::greet)
            .service(api::list_greetings)
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", ApiDoc::openapi()),
            )
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await.expect("Error stopping server");
    global::shutdown_tracer_provider();
    logger_provider.shutdown().expect("Failed shutting down loggprovider");
    Ok(())
}
static RESOURCE: Lazy<Resource> = Lazy::new(|| {
    Resource::new(vec![KeyValue::new(
        opentelemetry_semantic_conventions::resource::SERVICE_NAME,
        "greeting_rust",
    )])
});

fn init_logs(otlp_endpoint: &str) -> Result<LoggerProvider, LogError> {
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

fn init_tracer_provider(otlp_endpoint: &str) -> Result<TracerProvider, TraceError> {
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
