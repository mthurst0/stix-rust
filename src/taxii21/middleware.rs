use lazy_static::lazy_static;
use regex::Regex;
use std::future::{ready, Ready};

use actix_web::{
    body::EitherBody,
    dev::{self, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;

pub struct CheckAcceptHeader;

impl<S, B> Transform<S, ServiceRequest> for CheckAcceptHeader
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = CheckAcceptHeaderMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(CheckAcceptHeaderMiddleware { service }))
    }
}
pub struct CheckAcceptHeaderMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for CheckAcceptHeaderMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    dev::forward_ready!(service);

    fn call(&self, request: ServiceRequest) -> Self::Future {
        lazy_static! {
            static ref RE: Regex =
                Regex::new(r"^application/taxii\+json(;version=(\d\.\d))?$").unwrap();
        }
        let valid_accept_header = match request.headers().get("accept") {
            Some(v) => match v.to_str() {
                // TODO: extract version -- return NotAcceptable if the version isn't what
                // we expect.
                Ok(v) => RE.is_match(v),
                Err(err) => false,
            },
            None => false,
        };
        if !valid_accept_header {
            let (request, _pl) = request.into_parts();
            let response = HttpResponse::BadRequest()
                .finish()
                // constructed responses map to "right" body
                .map_into_right_body();

            return Box::pin(async { Ok(ServiceResponse::new(request, response)) });
        }

        let res = self.service.call(request);
        Box::pin(async move {
            // forwarded responses map to "left" body
            res.await.map(ServiceResponse::map_into_left_body)
        })
    }
}
