use actix_web::{get, post, web::Json, App, HttpResponse, HttpServer, Responder};
use actix_web_validation::Validated;
use serde::Deserialize;

#[get("/")]
async fn hello(x: Validated<Json<u32>>) -> impl Responder {
    println!("{}", x.0);

    HttpResponse::Ok().body("Hello world!")
}

#[derive(Debug, Deserialize)]
struct Example {
    name: String,
}

#[post("/")]
// async fn post_hello(x: Json<Example>) -> impl Responder {
async fn post_hello(x: Validated<Json<Example>>) -> impl Responder {
    println!("{:#?}", x.0);

    HttpResponse::Ok().body("Hello world!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(hello).service(post_hello))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
