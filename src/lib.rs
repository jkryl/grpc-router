// The router is implemented as a struct with hardcoded number of services
// it can proxy (two). The only overhead compared to a configuration without
// the router is that http responses from grpc services must be move to a
// boxed value on heap so that we can return abstract data type using
// virtual dispatch table on response body unifying grpc responses from all
// possible grpc services.

extern crate bytes;
extern crate futures;
extern crate http;
extern crate tower_grpc;
extern crate tower_hyper;
extern crate tower_service;

use bytes::{Bytes, IntoBuf};
use futures::{
    future::{self, FutureResult},
    Async,
    Future,
    Poll,
};
use http::Request;
use std::boxed::Box;
use tower_grpc::{codegen::server::grpc::Never, Body, BoxBody, Status};
use tower_service::Service;

type BytesBuf = <Bytes as IntoBuf>::Buf;

#[derive(Debug, Clone)]
pub struct Router2<A, B> {
    url_prefix: String,
    svc: A,
    default_svc: B,
}

impl<A, B> Router2<A, B> {
    pub fn new(prefix: &str, svc: A, default_svc: B) -> Self {
        Self {
            url_prefix: prefix.to_string(),
            svc,
            default_svc,
        }
    }
}

impl<A, B, U, V> Service<Request<BoxBody>> for Router2<A, B>
where
    A: Service<Request<BoxBody>, Error = Never, Response = http::Response<U>>,
    B: Service<Request<BoxBody>, Error = Never, Response = http::Response<V>>,
    <A as Service<Request<BoxBody>>>::Future: Send + 'static,
    <B as Service<Request<BoxBody>>>::Future: Send + 'static,
    U: Body<Data = BytesBuf, Error = Status> + Send + 'static,
    V: Body<Data = BytesBuf, Error = Status> + Send + 'static,
{
    type Response = http::Response<BoxBody>;
    type Error = Never;
    type Future =
        Box<dyn Future<Item = Self::Response, Error = Self::Error> + Send>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(().into())
    }

    fn call(&mut self, request: Request<BoxBody>) -> Self::Future {
        let path = request.uri().path().to_string();

        if path.starts_with(&self.url_prefix) {
            Box::new(self.svc.call(request).map(|resp| {
                let (head, body) = resp.into_parts();
                let boxed_body = BoxBody::new(Box::new(body));
                http::Response::from_parts(head, boxed_body)
            }))
        } else {
            Box::new(self.default_svc.call(request).map(|resp| {
                let (head, body) = resp.into_parts();
                let boxed_body = BoxBody::new(Box::new(body));
                http::Response::from_parts(head, boxed_body)
            }))
        }
    }
}

impl<A, B> Service<()> for Router2<A, B>
where
    A: Clone,
    B: Clone,
{
    type Response = Self;
    type Error = Never;
    type Future = FutureResult<Self::Response, Self::Error>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(Async::Ready(()))
    }

    fn call(&mut self, _target: ()) -> Self::Future {
        future::ok(self.clone())
    }
}

impl<A, B, U, V> Service<Request<tower_hyper::Body>> for Router2<A, B>
where
    A: Service<Request<BoxBody>, Error = Never, Response = http::Response<U>>,
    B: Service<Request<BoxBody>, Error = Never, Response = http::Response<V>>,
    <A as Service<Request<BoxBody>>>::Future: Send + 'static,
    <B as Service<Request<BoxBody>>>::Future: Send + 'static,
    U: Body<Data = BytesBuf, Error = Status> + Send + 'static,
    V: Body<Data = BytesBuf, Error = Status> + Send + 'static,
{
    type Response = <Self as Service<Request<BoxBody>>>::Response;
    type Error = <Self as Service<Request<BoxBody>>>::Error;
    type Future = <Self as Service<Request<BoxBody>>>::Future;

    fn poll_ready(&mut self) -> futures::Poll<(), Self::Error> {
        Service::<Request<BoxBody>>::poll_ready(self)
    }

    fn call(
        &mut self,
        request: http::Request<tower_hyper::Body>,
    ) -> Self::Future {
        let request = request.map(BoxBody::map_from);
        Service::<Request<BoxBody>>::call(self, request)
    }
}
