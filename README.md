# Actix Web Validation

Request validation for actix-web.


## Supported Validation Libraries

* [validator](https://github.com/Keats/validator)
* [garde](https://github.com/jprochazk/garde)



## Usage

<details>

<summary>Using Validator</summary>

```rs
use actix_web_validation::validator::Validate;


#[derive(Debug, Validate, Deserialize)]
struct Example {
    #[validate(length(min = 5))]
    name: String,
}

#[post("/")]
async fn hello(Validated(Json(payload)): Validated<Json<Example>>) -> impl Responder {
    HttpResponse::Ok().body(payload)
}

```

</details>



<details>

<summary>Using Garde</summary>

```rs
use actix_web_validation::garde::Validate;

#[derive(Debug, Validate, Deserialize)]
struct Example {
    #[validate(length(min = 5))]
    name: String,
}

#[post("/")]
async fn hello(Validated(Json(payload)): Validated<Json<Example>>) -> impl Responder {
    HttpResponse::Ok().body(payload)
}
```

</details>




