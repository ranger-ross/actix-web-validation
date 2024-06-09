use actix_web::FromRequest;
use error::{ErrHandler, ValidationErrorHandler};
use futures_core::ready;
use std::{fmt::Debug, future::Future, ops::Deref, pin::Pin, task::Poll};
use validator::Validate;

mod error;

pub use error::ValidationErrorHandlerExt;

pub struct Validated<T>(pub T);

pub struct ValidatedFut<T: FromRequest> {
    req: actix_web::HttpRequest,
    fut: <T as FromRequest>::Future,
    error_handler: Option<ErrHandler>,
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
                        let err: crate::error::Error = e.into();
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
