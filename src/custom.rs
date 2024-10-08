//! Validation with using an external library for validation.
//! Requires the `custom` feature flag
//!
//! For usage examples, see the documentation for [`Validated`]
//!

use crate::validated_definition;
use actix_web::dev::{ServiceFactory, ServiceRequest};
use actix_web::http::StatusCode;
use actix_web::FromRequest;
use actix_web::{App, HttpRequest, HttpResponse, ResponseError};
use std::fmt::Display;
use std::future::Future;
use std::sync::Arc;
use std::{fmt::Debug, ops::Deref, pin::Pin, task::Poll};
use thiserror::Error;

/// A trait that can be implemented to provide validation logic.
pub trait Validate {
    fn validate(&self) -> Result<(), Vec<ValidationError>>;
}

/// A validation error
#[derive(Debug)]
pub struct ValidationError {
    message: String,
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// A validated extactor.
///
/// This type will run any validations on the inner extractors.
///
/// ```
/// use actix_web::{post, web::{self, Json}, App};
/// use serde::Deserialize;
/// use actix_web_validation::custom::{Validated, Validate, ValidationError};
///
/// #[derive(Debug, Deserialize)]
/// struct Info {
///     username: String,
/// }
///
/// impl Validate for Info {
///     fn validate(&self) -> Result<(), Vec<ValidationError>> {
///         // Do validation logic here...
///         Ok(())
///     }
/// }
///
/// #[post("/")]
/// async fn index(info: Validated<Json<Info>>) -> String {
///     format!("Welcome {}!", info.username)
/// }
/// ```
pub struct Validated<T>(pub T);

validated_definition!();

pub struct ValidatedFut<T: FromRequest> {
    req: actix_web::HttpRequest,
    fut: <T as FromRequest>::Future,
    error_handler: Option<ValidationErrHandler>,
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
        let Poll::Ready(res) = Pin::new(&mut this.fut).poll(cx) else {
            return std::task::Poll::Pending;
        };

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
            .app_data::<ValidationErrorHandler>()
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
    errors: Vec<ValidationError>,
}

impl From<Vec<ValidationError>> for Error {
    fn from(value: Vec<ValidationError>) -> Self {
        Self { errors: value }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.errors
                .iter()
                .map(|e| e.message.as_ref())
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(StatusCode::BAD_REQUEST).body(format!(
            "Validation errors in fields:\n{}",
            &self
                .errors
                .iter()
                .map(|err| { format!("\t{}", err) })
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }
}

pub type ValidationErrHandler =
    Arc<dyn Fn(Vec<ValidationError>, &HttpRequest) -> actix_web::Error + Send + Sync>;

struct ValidationErrorHandler {
    handler: ValidationErrHandler,
}

/// Extension trait to provide a convenience method for adding custom error handler
pub trait ValidationErrorHandlerExt {
    /// Add a custom error handler for garde validated requests
    fn validation_error_handler(self, handler: ValidationErrHandler) -> Self;
}

impl<T> ValidationErrorHandlerExt for App<T>
where
    T: ServiceFactory<ServiceRequest, Config = (), Error = actix_web::Error, InitError = ()>,
{
    fn validation_error_handler(self, handler: ValidationErrHandler) -> Self {
        self.app_data(ValidationErrorHandler { handler })
    }
}

impl ValidationErrorHandlerExt for &mut actix_web::web::ServiceConfig {
    fn validation_error_handler(self, handler: ValidationErrHandler) -> Self {
        self.app_data(ValidationErrorHandler { handler })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use actix_web::web::Bytes;
    use actix_web::{http::header::ContentType, post, test, web::Json, App, Responder};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize)]
    struct ExamplePayload {
        name: String,
    }

    impl Validate for ExamplePayload {
        fn validate(&self) -> Result<(), Vec<ValidationError>> {
            if self.name.len() > 4 {
                Ok(())
            } else {
                Err(vec![ValidationError {
                    message: "name not long enough".to_string(),
                }])
            }
        }
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
            Bytes::from_static(b"Validation errors in fields:\n\tname not long enough")
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

    fn error_handler(errors: Vec<ValidationError>, _: &HttpRequest) -> actix_web::Error {
        CustomErrorResponse {
            custom_message: "My custom message".to_string(),
            errors: errors.iter().map(|err| err.message.clone()).collect(),
        }
        .into()
    }

    #[actix_web::test]
    async fn should_use_allow_custom_error_responses() {
        let app = test::init_service(
            App::new()
                .service(endpoint)
                .validation_error_handler(Arc::new(error_handler)),
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
            Bytes::from_static(
                b"{\"custom_message\":\"My custom message\",\"errors\":[\"name not long enough\"]}"
            )
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
