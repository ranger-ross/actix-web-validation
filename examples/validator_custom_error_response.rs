#![cfg(not(test))]

use actix_web::{
    post, web::Json, App, HttpRequest, HttpResponse, HttpServer, Responder, ResponseError,
};
use actix_web_validation::{validator::ValidatorErrorHandlerExt, Validated};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

/// To run this example use `cargo r --example validator_custom_error_response --features validator`
///
/// Once the server is running you can test with
/// ```
/// curl -X POST localhost:8080/example --json '{"name": "foo"}'
/// ```
///
/// Changing the length of the name should result to more than 4 chars should result in HTTP 200
///

#[derive(Debug, Serialize, Deserialize, Validate)]
struct Example {
    #[validate(length(min = 5))]
    name: String,
}

#[post("/example")]
async fn example(Validated(Json(payload)): Validated<Json<Example>>) -> impl Responder {
    println!("Got validated payload {:#?}", payload);

    HttpResponse::Ok().body(format!("Hello {}", payload.name))
}

#[derive(Debug, Serialize, Display)]
#[display(fmt = "My custom error. This is just an example from the derive_more crate")]
struct CustomErrorResponse {
    custom_message: String,
    errors: Vec<String>,
}

impl ResponseError for CustomErrorResponse {
    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::BAD_REQUEST
    }

    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code()).body(serde_json::to_string(self).unwrap())
    }
}

fn error_handler(errors: ::validator::ValidationErrors, _: &HttpRequest) -> actix_web::Error {
    CustomErrorResponse {
        custom_message: "My custom message".to_string(),
        errors: errors
            .errors()
            .iter()
            .map(|(err, _)| err.to_string())
            .collect(),
    }
    .into()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(example)
            .validator_error_handler(Arc::new(error_handler))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
