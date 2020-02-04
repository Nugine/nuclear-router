use super::handler::{BoxHandler, Handler};
use super::params::Params;
use super::{BoxError, BoxFuture, Request, Response};

use crate::http_router::{HttpRouter, Method};

use std::task::{Context, Poll};

use hyper::service::Service;

pub struct RouterService<H = BoxHandler> {
    router: HttpRouter<H>,
    default: H,
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

impl<H> Service<Request> for &'_ RouterService<H>
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

impl<H> RouterService<H>
where
    H: Handler,
{
    fn handle(&self, req: Request) -> BoxFuture<'static, Result<Response, BoxError>> {
        let method = req.method();
        let path = req.uri().path();
        let (handler, params) = match self.router.find(method, path) {
            Some((h, caps)) => (h, Params::new(path, &caps)),
            None => (&self.default, Params::empty()),
        };
        Handler::call(handler, req, params)
    }

    pub fn new(default: H) -> Self {
        Self::from_router(HttpRouter::new(), default)
    }

    pub fn from_router(router: HttpRouter<H>, default: H) -> Self {
        Self { router, default }
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

#[macro_export]
macro_rules! router_service {
    {$($method:tt $pattern:expr => $data:expr),+ ; _ => $default:expr} => {{
        let mut __router = $crate::http_router::HttpRouter::new();
        $(router_service!(@entry __router, $method, $pattern, $data);)+
        __router.with_default($default)
    }};

    {$($method:tt $pattern:expr => $data:expr),+} => {{
        let mut __router = $crate::http_router::HttpRouter::new();
        $(router_service!(@entry __router, $method, $pattern, $data);)+
        __router
    }};

    {@entry $router:expr, @, $prefix:expr, $sub_router:expr} => {
        $router.nest($prefix, |__r| *__r = $sub_router)
    };
    {@entry $router:expr, GET, $pattern:expr, $data:expr} => {
        $router.route($crate::http_router::Method::GET, $pattern, $data)
    };
    {@entry $router:expr, POST, $pattern:expr, $data:expr} => {
        $router.route($crate::http_router::Method::POST, $pattern, $data)
    };
    {@entry $router:expr, PUT, $pattern:expr, $data:expr} => {
        $router.route($crate::http_router::Method::PUT, $pattern, $data)
    };
    {@entry $router:expr, DELETE, $pattern:expr, $data:expr} => {
        $router.route($crate::http_router::Method::DELETE, $pattern, $data)
    };
    {@entry $router:expr, HEAD, $pattern:expr, $data:expr} => {
        $router.route($crate::http_router::Method::HEAD, $pattern, $data)
    };
    {@entry $router:expr, OPTIONS, $pattern:expr, $data:expr} => {
        $router.route($crate::http_router::Method::OPTIONS, $pattern, $data)
    };
    {@entry $router:expr, CONNECT, $pattern:expr, $data:expr} => {
        $router.route($crate::http_router::Method::CONNECT, $pattern, $data)
    };
    {@entry $router:expr, PATCH, $pattern:expr, $data:expr} => {
        $router.route($crate::http_router::Method::PATCH, $pattern, $data)
    };
    {@entry $router:expr, TRACE, $pattern:expr, $data:expr} => {
        $router.route($crate::http_router::Method::TRACE, $pattern, $data)
    };
}
