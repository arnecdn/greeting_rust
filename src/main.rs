use std::{env, fs};
use std::future::Future;
use std::process::exit;
use std::sync::RwLock;

use actix_web::{App, HttpServer};
use actix_web::web::Data;
use serde::Deserialize;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use greeting::{api, service};

use crate::greeting::repository::SqliteStudentRepository;
use crate::greeting::service::GreetingService;

mod greeting;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    #[derive(OpenApi)]
    #[openapi(
        info(description = "Greeting Api description"),
        paths(api::greet, api::list_greetings),
        components(schemas(api::GreetingDto))
    )]
    struct ApiDoc;
    let server_congig = match fs::read_to_string("./res/server.toml") {
        Ok(c) => c,
        Err(_) => exit(1),
    };

    let app_config: AppConfig = toml::from_str(&server_congig).unwrap_or_else(|e| {
        println!("{}", e.message());
        exit(1)
    });

    let repo = match SqliteStudentRepository::new(&app_config.postgres.database_url).await {
        Ok(r) => r,
        Err(e) => {
            println!("{:?}", e);
            exit(1)
        }
    };
    //Need explicit type in order to enforce type restrictions with dynamoc trait object allocation
    let service_impl = service::GreetingServiceImpl::new(repo);
    let svc: Data<RwLock<Box<dyn GreetingService + Sync + Send>>> = Data::new(RwLock::new(
        Box::new(service_impl),
    ));

    env::set_var("RUST_LOG", "debug");
    env_logger::init();

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
    .await
}
#[derive(Deserialize)]
struct AppConfig {
    kafka: KafkaConfig,
    postgres: DbConfig,
}
#[derive(Deserialize)]
struct KafkaConfig {
    broker: String,
    topic: String,
    message_timeout_ms: i32,
    enable_idempotence: bool,
    processing_guarantee: String,
}
#[derive(Deserialize)]
struct DbConfig {
    user: String,
    password: String,
    database_url: String,
}
