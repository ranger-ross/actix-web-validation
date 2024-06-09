//! Error declaration.
use std::sync::Arc;

use actix_web::dev::{ServiceFactory, ServiceRequest};
use actix_web::http::StatusCode;
use actix_web::{App, HttpRequest, HttpResponse, ResponseError};
use thiserror::Error;
use validator::{ValidationError, ValidationErrors, ValidationErrorsKind};

#[derive(Error, Debug)]
pub enum Error {
    #[error("Validation error: {0}")]
    Validate(#[from] validator::ValidationErrors),
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(StatusCode::BAD_REQUEST).body(match self {
            Self::Validate(e) => {
                format!(
                    "Validation errors in fields:\n{}",
                    flatten_errors(e)
                        .iter()
                        .map(|(_, field, err)| { format!("\t{}: {}", field, err) })
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
        })
    }
}

/// Helper function for error extraction and formatting.
/// Return Vec of tuples where first element is full field path (separated by dot)
/// and second is error.
#[inline]
pub fn flatten_errors(errors: &ValidationErrors) -> Vec<(u16, String, &ValidationError)> {
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

pub type ErrHandler =
    Arc<dyn Fn(validator::ValidationErrors, &HttpRequest) -> actix_web::Error + Send + Sync>;

pub struct ValidationErrorHandler {
    pub handler: ErrHandler,
}

pub trait ValidationErrorHandlerExt {
    fn validation_error_handler(self, handler: ErrHandler) -> Self;
}

impl<T> ValidationErrorHandlerExt for App<T>
where
    T: ServiceFactory<ServiceRequest, Config = (), Error = actix_web::Error, InitError = ()>,
{
    fn validation_error_handler(self, handler: ErrHandler) -> Self {
        self.app_data(ValidationErrorHandler { handler })
    }
}
