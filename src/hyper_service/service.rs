use super::params::Params;
use crate::http_router::{HttpRouter, Method};

use std::error::Error as StdError;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use hyper::service::Service;

type Request = hyper::Request<hyper::Body>;
type Response = hyper::Response<hyper::Body>;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
type BoxError = Box<dyn StdError + Send + Sync>;

pub trait Handler {
    fn call(
        &mut self,
        req: Request,
        params: Params,
    ) -> BoxFuture<'static, Result<Response, BoxError>>;
}

impl Handler for Box<dyn Handler+Send+Sync>{
    fn call(
        &mut self,
        req: Request,
        params: Params,
    ) -> BoxFuture<'static, Result<Response, BoxError>>{
        Handler::call(&mut **self,req,params)
    }
}

impl<F, E, Fut> Handler for F
where
    F: FnMut(Request, Params) -> Fut,
    E: StdError + Send + Sync + 'static,
    Fut: Future<Output = Result<Response, E>> + Send + 'static,
{
    fn call(
        &mut self,
        req: Request,
        params: Params,
    ) -> BoxFuture<'static, Result<Response, BoxError>> {
        let fut = (self)(req, params);
        Box::pin(async move {
            let ret = fut.await;
            match ret {
                Ok(r) => Ok(r),
                Err(e) => Err(Box::new(e) as BoxError),
            }
        })
    }
}

type BoxHandler = Box<dyn Handler + Send + Sync>;

pub struct RouterService<H = BoxHandler > {
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
        let method = req.method();
        let path = req.uri().path();
        let (handler, params) = match self.router.find_mut(method, path) {
            Some((h, caps)) => (h, Params::new(path, &caps)),
            None => (&mut self.default, Params::empty()),
        };

        handler.call(req, params)
    }
}

impl<H> RouterService<H> {
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

    pub fn with_default(self,default: impl Handler+Send+Sync+'static)->RouterService{
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
