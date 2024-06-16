# Actix Web Validation

> [!WARNING]  
> This library is currently in early access and may recieve breaking changes. Use at your own risk.

Request validation for actix-web.

## Supported Validation Libraries

* [validator](https://github.com/Keats/validator)
* [garde](https://github.com/jprochazk/garde)


## Usage

Any type that implments the Actix [`FromRequest`](https://docs.rs/actix-web/latest/actix_web/trait.FromRequest.html) trait can be automatically validated.


```toml
# Cargo.toml
actix-web-validation = { version = "0.0.0", features = ["validator"]}
# or 
actix-web-validation = { version = "0.0.0", features = ["garde"]}
```

```rs
use actix_web_validation::Validated;

use validator::Validate;
// or use garde::Validate;

// Do validation using your validation library
#[derive(Debug, Validate, Deserialize)]
struct Example {
    #[validate(length(min = 3))]
    name: String,
}

// Wrap your Actix extractor with `Validated` to automatically run validation
#[post("/")]
async fn hello(Validated(Json(payload)): Validated<Json<Example>>) -> impl Responder {
    HttpResponse::Ok().body(format!("Hello {}", payload.name))
}
```

## Custom Errors

Custom error responses can achieved by providing an error handler.

Below is an example custom error response that responds with JSON
```rs
#[derive(Debug, Serialize, Error)]
struct CustomErrorResponse {
    custom_message: String,
    errors: Vec<String>,
}

// impl Display for CustomErrorResponse { ... }

impl ResponseError for CustomErrorResponse {
    fn status_code(&self) -> actix_web::http::StatusCode {
        actix_web::http::StatusCode::BAD_REQUEST
    }

    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code()).body(serde_json::to_string(self).unwrap())
    }
}
```

Below is an example for the `validator` crate

```rs
fn error_handler(errors: ::validator::ValidationErrors, req: &HttpRequest) -> actix_web::Error {
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
            .validator_error_handler(Arc::new(error_handler))
            // ....
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
```

Below is an example for the `garde` crate

```rs
fn error_handler(errors: ::garde::Report, req: &HttpRequest) -> actix_web::Error {
    CustomErrorResponse {
        custom_message: "My custom message".to_string(),
        errors: errors.iter().map(|(_, err)| err.to_string()).collect(),
    }
    .into()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .garde_error_handler(Arc::new(error_handler))
            // ....
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
```

## Limitations

Usage of `actix_web_validation::Validated` is limited to one validation implmentation at a time. 
If you need to multiple validator, you can disable the `exclusive` feature flag (part of the default feature flags). Disabling will remove `actix_web_validation::Validated` but allow using the library implmentations directly (ie. `actix_web_validation::validator::Validated`, `actix_web_validation::garde::Validated`).

## Motivations

This library is heavily inspired by [Spring Validation](https://docs.spring.io/spring-framework/reference/core/validation/beanvalidation.html) and [actix-web-validator](https://crates.io/crates/actix-web-validator). 

The actix-web-validator is great but there are a few pain points I would like to address with this library.
- More explict validation by using the `Validated` extractor to reduce the risk of using the wrong `Json`/`Query`/ect extractor by mistake.
- Provide a common interface for validation libraries that can be extended as the Rust ecosystem evolves.

