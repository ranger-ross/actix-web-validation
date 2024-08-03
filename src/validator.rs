//! Validation for the [validator](https://docs.rs/validator/latest/validator) crate.
//! Requires the `validator` feature flag
//!
//! Validator is a popular validation library for Rust.
//!
//! You will need to import the validator crate in your `Cargo.toml`.
//!
//! ```toml
//! [dependencies]
//! validator = { version = "0.0.0", features = ["derive"] }
//! actix-web-validation = { version = "0.0.0", features = ["validator"]}
//! ```
//!
//! For usage examples, see the documentation for [`Validated`]
//!

use ::validator::Validate;
use actix_web::dev::{ServiceFactory, ServiceRequest};
use actix_web::http::StatusCode;
use actix_web::FromRequest;
use actix_web::{App, HttpRequest, HttpResponse, ResponseError};
use futures_core::ready;
use futures_core::Future;
use std::fmt::Display;
use std::sync::Arc;
use std::{fmt::Debug, ops::Deref, pin::Pin, task::Poll};
use thiserror::Error;
use validator::{ValidationError, ValidationErrors, ValidationErrorsKind};

/// A validated extactor.
///
/// This type will run any validations on the inner extractors.
///
/// ```
/// use actix_web::{post, web::{self, Json}, App};
/// use serde::Deserialize;
/// use validator::Validate;
/// use actix_web_validation::validator::Validated;
///
/// #[derive(Debug, Deserialize, Validate)]
/// struct Info {
///     #[validate(length(min = 5))]
///     username: String,
/// }
///
/// #[post("/")]
/// async fn index(info: Validated<Json<Info>>) -> String {
///     format!("Welcome {}!", info.username)
/// }
/// ```
pub struct Validated<T>(pub T);

impl<T> Validated<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Validated<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Validated<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Debug for Validated<T>
where
    T: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Validated").field(&self.0).finish()
    }
}

/// Future that extracts and validates actix requests using the Actix Web [`FromRequest`] trait
///
/// End users of this library should not need to use this directly for most usecases
pub struct ValidatedFut<T: FromRequest> {
    req: actix_web::HttpRequest,
    fut: <T as FromRequest>::Future,
    error_handler: Option<ValidatorErrHandler>,
}
impl<T> Future for ValidatedFut<T>
where
    T: FromRequest + Debug + Deref,
    T::Future: Unpin,
    T::Target: Validate,
{
    type Output = Result<Validated<T>, actix_web::Error>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
        let res = ready!(Pin::new(&mut this.fut).poll(cx));

        let res = match res {
            Ok(data) => {
                if let Err(e) = data.validate() {
                    if let Some(error_handler) = &this.error_handler {
                        Err((*error_handler)(e, &this.req))
                    } else {
                        let err: Error = e.into();
                        Err(err.into())
                    }
                } else {
                    Ok(Validated(data))
                }
            }
            Err(e) => Err(e.into()),
        };

        Poll::Ready(res)
    }
}

impl<T> FromRequest for Validated<T>
where
    T: FromRequest + Debug + Deref,
    T::Future: Unpin,
    T::Target: Validate,
{
    type Error = actix_web::Error;

    type Future = ValidatedFut<T>;

    fn from_request(
        req: &actix_web::HttpRequest,
        payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let error_handler = req
            .app_data::<ValidatorErrorHandler>()
            .map(|h| h.handler.clone());

        let fut = T::from_request(req, payload);

        ValidatedFut {
            fut,
            error_handler,
            req: req.clone(),
        }
    }
}

#[derive(Error, Debug)]
struct Error {
    errors: validator::ValidationErrors,
}

impl From<validator::ValidationErrors> for Error {
    fn from(value: validator::ValidationErrors) -> Self {
        Self { errors: value }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.errors)
    }
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(StatusCode::BAD_REQUEST).body(format!(
            "Validation errors in fields:\n{}",
            flatten_errors(&self.errors)
                .iter()
                .map(|(_, field, err)| { format!("\t{}: {}", field, err) })
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }
}

/// Helper function for error extraction and formatting.
/// Return Vec of tuples where first element is full field path (separated by dot)
/// and second is error.
#[inline]
fn flatten_errors(errors: &ValidationErrors) -> Vec<(u16, String, &ValidationError)> {
    _flatten_errors(errors, None, None)
}

#[inline]
fn _flatten_errors(
    errors: &ValidationErrors,
    path: Option<String>,
    indent: Option<u16>,
) -> Vec<(u16, String, &ValidationError)> {
    errors
        .errors()
        .iter()
        .flat_map(|(&field, err)| {
            let indent = indent.unwrap_or(0);
            let actual_path = path
                .as_ref()
                .map(|path| [path.as_str(), field].join("."))
                .unwrap_or_else(|| field.to_owned());
            match err {
                ValidationErrorsKind::Field(field_errors) => field_errors
                    .iter()
                    .map(|error| (indent, actual_path.clone(), error))
                    .collect::<Vec<_>>(),
                ValidationErrorsKind::List(list_error) => list_error
                    .iter()
                    .flat_map(|(index, errors)| {
                        let actual_path = format!("{}[{}]", actual_path.as_str(), index);
                        _flatten_errors(errors, Some(actual_path), Some(indent + 1))
                    })
                    .collect::<Vec<_>>(),
                ValidationErrorsKind::Struct(struct_errors) => {
                    _flatten_errors(struct_errors, Some(actual_path), Some(indent + 1))
                }
            }
        })
        .collect::<Vec<_>>()
}

pub type ValidatorErrHandler =
    Arc<dyn Fn(validator::ValidationErrors, &HttpRequest) -> actix_web::Error + Send + Sync>;

struct ValidatorErrorHandler {
    handler: ValidatorErrHandler,
}

/// Extension trait to provide a convenience method for adding custom error handler
pub trait ValidatorErrorHandlerExt {
    /// Add a custom error handler for validator validated requests
    fn validator_error_handler(self, handler: ValidatorErrHandler) -> Self;
}

impl<T> ValidatorErrorHandlerExt for App<T>
where
    T: ServiceFactory<ServiceRequest, Config = (), Error = actix_web::Error, InitError = ()>,
{
    fn validator_error_handler(self, handler: ValidatorErrHandler) -> Self {
        self.app_data(ValidatorErrorHandler { handler })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use actix_web::web::Bytes;
    use actix_web::{http::header::ContentType, post, test, web::Json, App, Responder};
    use serde::{Deserialize, Serialize};
    use validator::Validate;

    #[derive(Debug, Deserialize, Serialize, Validate)]
    struct ExamplePayload {
        #[validate(length(min = 5))]
        name: String,
    }

    #[post("/")]
    async fn endpoint(v: Validated<Json<ExamplePayload>>) -> impl Responder {
        assert!(v.name.len() > 4);
        HttpResponse::Ok().body(())
    }

    #[actix_web::test]
    async fn should_validate_simple() {
        let app = test::init_service(App::new().service(endpoint)).await;

        // Valid request
        let req = test::TestRequest::post()
            .uri("/")
            .insert_header(ContentType::plaintext())
            .set_json(ExamplePayload {
                name: "123456".to_string(),
            })
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 200);

        // Invalid request
        let req = test::TestRequest::post()
            .uri("/")
            .insert_header(ContentType::plaintext())
            .set_json(ExamplePayload {
                name: "1234".to_string(),
            })
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), 400);
    }

    // TODO: This test is unstable because the error or appears to be non-dermimistic
    #[ignore]
    #[actix_web::test]
    async fn should_respond_with_errors_correctly() {
        let app = test::init_service(App::new().service(endpoint)).await;

        // Invalid request
        let req = test::TestRequest::post()
            .uri("/")
            .insert_header(ContentType::plaintext())
            .set_json(ExamplePayload {
                name: "1234".to_string(),
            })
            .to_request();
        let result = test::call_and_read_body(&app, req).await;
        assert_eq!(
            result,
            Bytes::from_static(b"Validation errors in fields:\n\tname: Validation error: length [{\"min\": Number(5), \"value\": String(\"1234\")}]")
        );
    }

    #[derive(Debug, Serialize, Error)]
    struct CustomErrorResponse {
        custom_message: String,
        errors: Vec<String>,
    }

    impl Display for CustomErrorResponse {
        fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            unimplemented!()
        }
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

    #[actix_web::test]
    async fn should_use_allow_custom_error_responses() {
        let app = test::init_service(
            App::new()
                .service(endpoint)
                .validator_error_handler(Arc::new(error_handler)),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/")
            .insert_header(ContentType::plaintext())
            .set_json(ExamplePayload {
                name: "1234".to_string(),
            })
            .to_request();
        let result = test::call_and_read_body(&app, req).await;
        assert_eq!(
            result,
            Bytes::from_static(b"{\"custom_message\":\"My custom message\",\"errors\":[\"name\"]}")
        );
    }

    #[test]
    async fn debug_for_validated_should_work() {
        let v = Validated(ExamplePayload {
            name: "abcde".to_string(),
        });

        assert_eq!(
            "Validated(ExamplePayload { name: \"abcde\" })",
            format!("{v:?}")
        );
    }
}
