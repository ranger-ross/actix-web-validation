/// To run this example use ``cargo r --example validator_simple`
///
/// Once the server is running you can test with
/// ```
/// curl -X POST localhost:8080/example --json '{"name": "foo"}'
/// ```
///
/// Changing the length of the name should result to more than 4 chars should result in HTTP 200
///
use actix_web::{post, web::Json, App, HttpResponse, HttpServer, Responder};
// NOTE:Generally, you will want to use `actix_web_validation::Validated` instead.
//      This import is due to a feature flag limitation in cargo examples
use actix_web_validation::validator::Validated;
use serde::{Deserialize, Serialize};
use validator::Validate;

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
