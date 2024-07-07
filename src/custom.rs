//! Validation with using an external library for validation.
//! Requires the `custom` feature flag

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

/// A trait that be implmented to provide validation logic.
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

pub trait ValidationErrorHandlerExt {
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

#[cfg(test)]
mod test {
    use super::*;
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

    #[actix_web::test]
    async fn should_validate_simple() {
        #[post("/")]
        async fn endpoint(v: Validated<Json<ExamplePayload>>) -> impl Responder {
            assert!(v.name.len() > 4);
            HttpResponse::Ok().body(())
        }

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
}
