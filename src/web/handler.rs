use super::{FromRequest, IntoResponse, Request, Response};
use crate::extract::{FromRequestParts, IntoRejectionResponse};
use std::{future::Future, pin::Pin};

/// Shared asynchronous handler contract. Function handlers support zero to
/// sixteen arguments, evaluated left-to-right. Only the last can read the body.
pub trait Handler<T, S>: Clone + Send + Sync + 'static {
    type Future: Future<Output = Response> + Send + 'static;
    fn call(self, request: Request, state: S) -> Self::Future;
}

impl<H, F, R, S> Handler<((),), S> for H
where
    H: FnOnce() -> F + Clone + Send + Sync + 'static,
    F: Future<Output = R> + Send + 'static,
    R: IntoResponse,
{
    type Future = Pin<Box<dyn Future<Output = Response> + Send>>;
    fn call(self, _: Request, _: S) -> Self::Future {
        Box::pin(async move { self().await.into_response() })
    }
}

macro_rules! handler {
    ([$($head:ident),*], $last:ident) => {
        #[allow(non_snake_case)]
        impl<H, F, R, S, M, $($head,)* $last> Handler<(M, $($head,)* $last), S> for H
        where H: FnOnce($($head,)* $last) -> F + Clone + Send + Sync + 'static,
              F: Future<Output = R> + Send + 'static, R: IntoResponse,
              S: Send + Sync + 'static,
              $($head: FromRequestParts<S> + Send,)*
              $last: FromRequest<S, M> + Send,
        {
            type Future = Pin<Box<dyn Future<Output = Response> + Send>>;
            fn call(self, request: Request, state: S) -> Self::Future {
                Box::pin(async move {
                    #[allow(unused_mut)]
                    let (mut parts, body) = request.into_parts();
                    $(let $head = match $head::from_request_parts(&mut parts, &state).await {
                        Ok(value) => value,
                        Err(error) => return error.into_rejection_response().into_response(),
                    };)*
                    let $last = match $last::from_request(Request::from_parts(parts, body), &state).await {
                        Ok(value) => value,
                        Err(error) => return error.into_rejection_response().into_response(),
                    };
                    self($($head,)* $last).await.into_response()
                })
            }
        }
    };
}

handler!([], A);
handler!([A], B);
handler!([A, B], C);
handler!([A, B, C], D);
handler!([A, B, C, D], E);
handler!([A, B, C, D, E], F0);
handler!([A, B, C, D, E, F0], G);
handler!([A, B, C, D, E, F0, G], I);
handler!([A, B, C, D, E, F0, G, I], J);
handler!([A, B, C, D, E, F0, G, I, J], K);
handler!([A, B, C, D, E, F0, G, I, J, K], L);
handler!([A, B, C, D, E, F0, G, I, J, K, L], N);
handler!([A, B, C, D, E, F0, G, I, J, K, L, N], O);
handler!([A, B, C, D, E, F0, G, I, J, K, L, N, O], P);
handler!([A, B, C, D, E, F0, G, I, J, K, L, N, O, P], Q);
handler!([A, B, C, D, E, F0, G, I, J, K, L, N, O, P, Q], U);
