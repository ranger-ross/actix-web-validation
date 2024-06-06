use actix_web::FromRequest;
use futures_core::ready;
use std::{future::Future, pin::Pin, task::Poll};

pub struct Validated<T>(pub T);

pub struct ValidatedFut<T: FromRequest> {
    fut: <T as FromRequest>::Future,
}

impl<T> Future for ValidatedFut<T>
where
    T: FromRequest,
    T::Future: Unpin,
{
    type Output = Result<Validated<T>, actix_web::Error>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
        //
        let res = ready!(Pin::new(&mut this.fut).poll(cx));

        let res = match res {
            Ok(data) => {
                // TODO: Do validation here ...

                Ok(Validated(data))
            }
            Err(_) => todo!(), // TODO: Handle errors
        };

        Poll::Ready(res)
    }
}

impl<T> FromRequest for Validated<T>
where
    T: FromRequest,
    T::Future: Unpin,
{
    type Error = actix_web::Error; // TODO: Better errors

    type Future = ValidatedFut<T>;

    fn from_request(
        req: &actix_web::HttpRequest,
        payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let fut = T::from_request(req, payload);

        ValidatedFut { fut }
    }
}
