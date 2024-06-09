use std::sync::Arc;

use actix_web::HttpRequest;
use actix_web::{post, web::Json, App, HttpResponse, HttpServer, Responder};
use actix_web_validation::Validated;
use actix_web_validation::ValidationErrorHandlerExt;
use derive_more::{Display, Error};
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Validate, Deserialize)]
struct Example {
    #[validate(length(min = 5))]
    name: String,
}

#[post("/")]
async fn post_hello(x: Validated<Json<Example>>) -> impl Responder {
    let x = x.into_inner().into_inner();

    println!("{:#?}", x);

    HttpResponse::Ok().body("Hello world!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(post_hello)
            .validation_error_handler(Arc::new(handle_validation_errors))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

#[derive(Debug, Display, Error)]
#[display(fmt = "my error: {}", error)]
struct MyValError {
    error: String,
}

impl actix_web::error::ResponseError for MyValError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::BAD_REQUEST
    }
}

fn handle_validation_errors(
    _errors: validator::ValidationErrors,
    _req: &HttpRequest,
) -> actix_web::Error {
    MyValError {
        error: "this is an example error".to_string(),
    }
    .into()
}
