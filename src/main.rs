use std::process::exit;
use std::sync::RwLock;

use actix_web::{App, HttpServer};

use actix_web::web::Data;

use log::{error, info};

use opentelemetry::{global};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_sdk::propagation::TraceContextPropagator;

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
mod observation;

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

    let result = observation::init_tracer_provider(&app_config.otel_collector.oltp_endpoint);
    let tracer_provider = result.unwrap();
    global::set_tracer_provider(tracer_provider.clone());
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Initialize logs and save the logger_provider.
    let logger_provider = observation::init_logs(&app_config.otel_collector.oltp_endpoint).unwrap();

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
