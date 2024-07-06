//! Validation for the [garde](https://docs.rs/garde/latest/garde) crate.
//! Requires the `garde` feature flag

use ::garde::Validate;
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
            .map(|(path, error)| format!("{}: {}", path.to_string(), error.message().to_string()))
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

pub trait GardeErrorHandlerExt {
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
