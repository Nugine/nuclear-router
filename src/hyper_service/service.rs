use super::handler::{BoxHandler, Handler};
use super::{BoxError, BoxFuture, Request, Response};
use crate::router::OwnedCaptures;

use crate::http_router::{HttpRouter, Method};

use std::sync::Arc;
use std::task::{Context, Poll};

use hyper::service::Service;

#[derive(Debug)]
pub struct RouterService<H = BoxHandler> {
    router: HttpRouter<H>,
    default: H,
}

#[derive(Debug)]
pub struct SharedRouterService<H = BoxHandler>(Arc<RouterService<H>>);

impl<H> Clone for SharedRouterService<H> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<H> Service<Request> for RouterService<H>
where
    H: Handler + Send + Sync,
{
    type Response = Response;
    type Error = BoxError;
    type Future = BoxFuture<'static, Result<Response, BoxError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        RouterService::handle(self, req)
    }
}

impl<H> Service<Request> for SharedRouterService<H>
where
    H: Handler + Send + Sync,
{
    type Response = Response;
    type Error = BoxError;
    type Future = BoxFuture<'static, Result<Response, BoxError>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        RouterService::handle(&*self.0, req)
    }
}

impl<H> RouterService<H>
where
    H: Handler,
{
    fn handle(&self, req: Request) -> BoxFuture<'static, Result<Response, BoxError>> {
        let method = req.method();
        let path = req.uri().path();
        let (handler, params) = match self.router.find(method, path) {
            Some((h, caps)) => (h, OwnedCaptures::new(&caps)),
            None => (&self.default, OwnedCaptures::empty()),
        };
        Handler::call(handler, req, params)
    }

    pub fn new(default: H) -> Self {
        Self::from_router(HttpRouter::new(), default)
    }

    pub fn from_router(router: HttpRouter<H>, default: H) -> Self {
        Self { router, default }
    }

    pub fn into_shared(self) -> SharedRouterService<H> {
        SharedRouterService(Arc::new(self))
    }
}

impl HttpRouter<BoxHandler> {
    pub fn route(
        &mut self,
        method: Method,
        path: &str,
        h: impl Handler + Send + Sync + 'static,
    ) -> &mut Self {
        self.insert(method, path, Box::new(h))
    }

    pub fn with_default(self, default: impl Handler + Send + Sync + 'static) -> RouterService {
        RouterService::from_router(self, Box::new(default))
    }
}

macro_rules! define_method{
    ($name:tt,$method:tt) => {
        pub fn $name(&mut self,path: &str,h: impl Handler+Send+Sync+'static) -> &mut Self{
            self.route(Method::$method,path,h)
        }
    }
}

impl HttpRouter<BoxHandler> {
    define_method!(get, GET);
    define_method!(post, POST);
    define_method!(put, PUT);
    define_method!(delete, DELETE);
    define_method!(head, HEAD);
    define_method!(options, OPTIONS);
    define_method!(connect, CONNECT);
    define_method!(patch, PATCH);
    define_method!(trace, TRACE);
}
