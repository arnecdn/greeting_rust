use std::collections::HashMap;
use std::sync::RwLock;

use actix_web::{get, HttpResponse, post, ResponseError, web};
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;

use chrono::{DateTime, Utc};
use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::{Validate, ValidationErrors};
use validator_derive::Validate;

use crate::greeting::api::ApiError::{ApplicationError, BadClientData};
use crate::greeting::repository::GreetingRepositoryInMemory;
use crate::greeting::service::{Greeting, GreetingService, GreetingServiceImpl, ServiceError};

#[utoipa::path(
    get,
    path = "/greeting",
    responses(
        (status = 200, description = "Greetings", body = GreetingDto),
        (status = NOT_FOUND, description = "Greetings was not found")
    )
    )]
#[get("/greeting")]
pub async fn list_greetings(
    data: web::Data< RwLock<GreetingServiceImpl<GreetingRepositoryInMemory>>>,
) -> Result<HttpResponse, ApiError> {
    if let Ok( read_guard) = data.read(){
        let greetings = read_guard.all_greetings()?
            .iter().map(|f|GreetingDto::from(f.clone())).collect::<Vec<_>>();
        return Ok(HttpResponse::Ok().json(greetings));
    }
    Err(ApiError::Error)

}

#[utoipa::path(
    post,
    path = "/greeting",
    responses(
        (status = 201, description = "Greeting successfully stored", body = GreetingDto),
        (status = NOT_FOUND, description = "Resource not found")
    ),
    )]
#[post("/greeting")]
pub async fn greet(
    mut data: web::Data< RwLock<GreetingServiceImpl<GreetingRepositoryInMemory>>>,
    greeting: web::Json<GreetingDto>,
) -> Result<HttpResponse, ApiError> {
    greeting.validate()?;

    data.write().unwrap().receive_greeting(Greeting::from(greeting.0))?;
    Ok(HttpResponse::Ok().body(""))
}

#[derive(Debug, Display, Error)]
pub enum ApiError {
    BadClientData(ValidationErrors),
    ApplicationError(ServiceError),
    Error
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::json())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            BadClientData(_) => StatusCode::BAD_REQUEST,
            ApplicationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

impl From<ValidationErrors> for ApiError {
    fn from(value: ValidationErrors) -> Self {
        BadClientData(value)
    }
}

impl From<ServiceError> for ApiError {
    fn from(value: ServiceError) -> Self {
        ApplicationError(value)
    }
}

#[derive(Validate, Serialize, Deserialize, Clone, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GreetingDto {
    #[validate(length(min = 1, max = 20))]
    to: String,
    #[validate(length(min = 1, max = 20))]
    from: String,
    #[validate(length(min = 1, max = 50))]
    heading: String,
    #[validate(length(min = 1, max = 50))]
    message: String,
    #[schema(value_type = String, format = DateTime)]
    created: DateTime<Utc>,
}

impl From<GreetingDto> for Greeting{
    fn from(greeting: GreetingDto) -> Self {
        Greeting{
            to: greeting.to.clone(),
            from: greeting.from.clone(),
            heading: greeting.heading.clone(),
            message: greeting.message.clone(),
            created: greeting.created,
        }
    }
}
impl From<Greeting> for GreetingDto {
    fn from(greeting: Greeting) -> Self {
        GreetingDto{
            to: greeting.to.clone(),
            from: greeting.from.clone(),
            heading: greeting.heading.clone(),
            message: greeting.message.clone(),
            created: greeting.created,
        }
    }
}


#[cfg(test)]
mod test {
    use std::vec;
    use actix_web::test;

    use super::*;

    #[actix_web::test]
    async fn test_read_greeting() {
        // let  repo = HashMap::new();
        #[derive(Clone)]
        struct Me;

        impl  GreetingService for Me{
            fn receive_greeting(&mut self, greeting: Greeting) -> Result<(), ServiceError>{
                Ok(())
            }
            fn all_greetings(&self) -> Result<Vec<Greeting>, ServiceError>{
                return Ok(vec![Greeting::new(String::from(""),String::from(""),String::from(""),String::from(""))]);
            }
        };




        let app = test::init_service(
            actix_web::App::new()
                .app_data(Me{}.clone())
                .service(list_greetings)

        ).await;

        let req = test::TestRequest::get()
            .uri("/greeting")
            .insert_header(actix_web::http::header::ContentType::json())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    // #[actix_web::test]
    // async fn test_store_greeting() {
    //     let app = test::init_service(crate::test_app!(HashMap::new(), greet)).await;
    //
    //     let req = test::TestRequest::post()
    //         .uri("/greeting")
    //         .insert_header(actix_web::http::header::ContentType::json())
    //         .set_json(GreetingDto {
    //             to: String::from("test"),
    //             from: String::from("testa"),
    //             heading: String::from("Merry Christmas"),
    //             message: String::from("Happy new year"),
    //             created: DateTime::default(),
    //         })
    //         .to_request();
    //
    //     let resp = test::call_service(&app, req).await;
    //     assert!(resp.status().is_success());
    // }

    // #[actix_web::test]
    // async fn test_invalid_greeting() {
    //     let app = test::init_service(crate::test_app!(HashMap::new(), greet)).await;
    //
    //     let req = test::TestRequest::post()
    //         .uri("/greeting")
    //         .insert_header(actix_web::http::header::ContentType::json())
    //         .set_json(GreetingDto {
    //             to: String::from("testtesttesttesttesttesttesttest"),
    //             from: String::from("testa"),
    //             heading: String::from("Merry Christmas"),
    //             message: String::from("Happy new year"),
    //             created: DateTime::default(),
    //         })
    //         .to_request();
    //
    //     let resp = test::call_service(&app, req).await;
    //     assert!(resp.status().is_client_error());
    //     println!("{:?}", resp.response().body());
    // }
}
//
// #[macro_export]
// macro_rules! test_app {
//     ($data:expr,$service:expr) => {{
//           let s = ||-> dyn GreetingService{
//             fn receive_greeting(&mut self,  greeting: Greeting) -> Result<(), ServiceError>{
//                 Ok(ok)
//             }
//             fn all_greetings(&self) -> Result<Vec<Greeting>, ServiceError>{
//                 return Ok(())
//             }
//         };
//         let greeting_store: actix_web::web::Data<RwLock<HashMap<usize, GreetingDto>>> =
//             actix_web::web::Data::new(RwLock::new($data));
//
//         actix_web::App::new()
//             .app_data(s.clone())
//             .service($service)
//     }};
// }
