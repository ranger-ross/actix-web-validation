use actix_web::{post, web::Json, App, HttpResponse, HttpServer, Responder};
use actix_web_validation::Validated;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Validate, Deserialize)]
struct Example {
    #[validate(length(min = 5))]
    name: String,
}

#[post("/")]
async fn post_hello(x: Validated<Json<Example>>) -> impl Responder {
    println!("{:#?}", x.0);

    HttpResponse::Ok().body("Hello world!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(post_hello))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
