# Actix Web Validation

Request validation for actix-web.


## Supported Validation Libraries

* [validator](https://github.com/Keats/validator)
* [garde](https://github.com/jprochazk/garde)



## Usage


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

## Limitations

TODO


## Motivations

TODO
