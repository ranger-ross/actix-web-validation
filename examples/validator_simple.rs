use actix_web::{post, web::Json, App, HttpResponse, HttpServer, Responder};
use actix_web_validation::Validated;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// To run this example use ``cargo r --example validator_simple --features validator`
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(example))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
