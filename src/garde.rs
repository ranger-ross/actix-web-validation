//! Validation for the [garde](https://docs.rs/garde/latest/garde) crate.
//! Requires the `garde` feature flag
//!
//! Garde is a popular validation library for Rust.
//!
//! You will need to import the garde crate in your `Cargo.toml`.
//!
//! ```toml
//! [dependencies]
//! garde = { version = "0.0.0", features = ["derive"] }
//! actix-web-validation = { version = "0.0.0", features = ["garde"]}
//! ```
//!
//! For usage examples, see the documentation for [`Validated`]
//!

use crate::validated_definition;
use ::garde::Validate;
use actix_web::dev::{ServiceFactory, ServiceRequest};
use actix_web::http::StatusCode;
use actix_web::FromRequest;
use actix_web::{App, HttpRequest, HttpResponse, ResponseError};
use std::fmt::Display;
use std::future::Future;
use std::sync::Arc;
use std::{fmt::Debug, ops::Deref, pin::Pin, task::Poll};
use thiserror::Error;

/// A validated extactor.
///
/// This type will run any validations on the inner extractors.
///
/// ```
/// use actix_web::{post, web::{self, Json}, App};
/// use serde::Deserialize;
/// use garde::Validate;
/// use actix_web_validation::garde::Validated;
///
/// #[derive(Debug, Deserialize, Validate)]
/// struct Info {
///     #[garde(length(min = 3))]
///     username: String,
/// }
///
/// #[post("/")]
/// async fn index(info: Validated<Json<Info>>) -> String {
///     format!("Welcome {}!", info.username)
/// }
/// ```
pub struct Validated<T>(pub T);

validated_definition!();

/// Future that extracts and validates actix requests using the Actix Web [`FromRequest`] trait
///
/// End users of this library should not need to use this directly for most usecases
pub struct ValidatedFut<T: FromRequest> {
    req: actix_web::HttpRequest,
    fut: <T as FromRequest>::Future,
    error_handler: Option<GardeErrHandler>,
}

impl<T> Future for ValidatedFut<T>
where
    T: FromRequest + Debug + Deref,
    T::Future: Unpin,
    T::Target: Validate,
    <T::Target as garde::Validate>::Context: Default,
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
    <T::Target as garde::Validate>::Context: Default,
{
    type Error = actix_web::Error;

    type Future = ValidatedFut<T>;

    fn from_request(
        req: &actix_web::HttpRequest,
        payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let error_handler = req
            .app_data::<GardeErrorHandler>()
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
    report: garde::Report,
}

impl From<garde::Report> for Error {
    fn from(value: garde::Report) -> Self {
        Self { report: value }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.report)
    }
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        let message = self
            .report
            .iter()
            .map(|(path, error)| format!("{path}: {}", error.message()))
            .collect::<Vec<_>>()
            .join("\n");

        HttpResponse::build(StatusCode::BAD_REQUEST)
            .body(format!("Validation errors in fields:\n{}", message))
    }
}

pub type GardeErrHandler =
    Arc<dyn Fn(garde::Report, &HttpRequest) -> actix_web::Error + Send + Sync>;

struct GardeErrorHandler {
    handler: GardeErrHandler,
}

/// Extension trait to provide a convenience method for adding custom error handler
pub trait GardeErrorHandlerExt {
    /// Add a custom error handler for garde validated requests
    fn garde_error_handler(self, handler: GardeErrHandler) -> Self;
}

impl<T> GardeErrorHandlerExt for App<T>
where
    T: ServiceFactory<ServiceRequest, Config = (), Error = actix_web::Error, InitError = ()>,
{
    fn garde_error_handler(self, handler: GardeErrHandler) -> Self {
        self.app_data(GardeErrorHandler { handler })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use actix_web::web::Bytes;
    use actix_web::{http::header::ContentType, post, test, web::Json, App, Responder};
    use garde::Validate;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize, Validate)]
    struct ExamplePayload {
        #[garde(length(min = 5))]
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
            Bytes::from_static(b"Validation errors in fields:\nname: length is lower than 5")
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

    fn error_handler(errors: ::garde::Report, _: &HttpRequest) -> actix_web::Error {
        CustomErrorResponse {
            custom_message: "My custom message".to_string(),
            errors: errors.iter().map(|(_, err)| err.to_string()).collect(),
        }
        .into()
    }

    #[actix_web::test]
    async fn should_use_allow_custom_error_responses() {
        let app = test::init_service(
            App::new()
                .service(endpoint)
                .garde_error_handler(Arc::new(error_handler)),
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
            Bytes::from_static(b"{\"custom_message\":\"My custom message\",\"errors\":[\"length is lower than 5\"]}")
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
