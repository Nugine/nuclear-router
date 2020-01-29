#![forbid(unsafe_code)]

use crate::router::{Router, RouterError};

use std::collections::HashMap;

pub use http::Method;
use regex::Regex;

#[derive(Debug, Default)]
pub struct HttpRouter<T> {
    method_map: HashMap<Method, Router<T>>,
}

#[derive(Debug)]
pub enum Endpoint<T> {
    Data(T),
    Router(HttpRouter<T>),
}

impl<T> From<T> for Endpoint<T> {
    fn from(x: T) -> Self {
        Self::Data(x)
    }
}

impl<T> From<HttpRouter<T>> for Endpoint<T> {
    fn from(x: HttpRouter<T>) -> Self {
        Self::Router(x)
    }
}

impl<T> HttpRouter<T> {
    pub fn new() -> Self {
        Self {
            method_map: HashMap::new(),
        }
    }

    pub fn find<'a>(
        &'a self,
        method: &Method,
        path: &'a str,
    ) -> Option<(&'a T, impl Iterator<Item = (&'a str, &'a str)>)> {
        self.method_map.get(method)?.find(path)
    }

    pub fn insert(
        &mut self,
        method: Method,
        pattern: &str,
        endpoint: impl Into<Endpoint<T>>,
    ) -> &mut Self {
        match endpoint.into() {
            Endpoint::Router(router) => {
                self.insert_router(pattern, router);
            }
            Endpoint::Data(data) => {
                self.access_router(method).insert(pattern, data);
            }
        }
        self
    }

    pub fn try_insert(
        &mut self,
        method: Method,
        pattern: &str,
        endpoint: impl Into<Endpoint<T>>,
    ) -> Result<&mut Self, RouterError> {
        match endpoint.into() {
            Endpoint::Router(router) => {
                self.try_insert_router(pattern, router)?;
            }
            Endpoint::Data(data) => {
                self.access_router(method).try_insert(pattern, data)?;
            }
        }
        Ok(self)
    }

    pub fn insert_regex(&mut self, method: Method, pattern: Regex, data: T) -> &mut Self {
        self.access_router(method).insert_regex(pattern, data);
        self
    }

    pub fn nest(&mut self, prefix: &str, f: impl FnOnce(&mut HttpRouter<T>)) -> &mut Self {
        let mut sub_router = Self::new();
        f(&mut sub_router);

        for (method, router) in sub_router.method_map {
            self.access_router(method).nest(prefix, |r| *r = router);
        }

        self
    }

    pub fn try_nest(
        &mut self,
        prefix: &str,
        f: impl FnOnce(&mut HttpRouter<T>),
    ) -> Result<&mut Self, RouterError> {
        let mut sub_router = Self::new();
        f(&mut sub_router);
        self.insert_router(prefix, sub_router);
        Ok(self)
    }
}

impl<T> HttpRouter<T> {
    fn access_router(&mut self, method: Method) -> &mut Router<T> {
        self.method_map.entry(method).or_insert_with(Router::new)
    }

    fn try_insert_router(
        &mut self,
        prefix: &str,
        router: HttpRouter<T>,
    ) -> Result<(), RouterError> {
        for (method, router) in router.method_map {
            self.access_router(method)
                .try_nest(prefix, |r| *r = router)?;
        }
        Ok(())
    }

    fn insert_router(&mut self, prefix: &str, router: HttpRouter<T>) {
        for (method, router) in router.method_map {
            self.access_router(method).nest(prefix, |r| *r = router);
        }
    }
}

#[macro_export]
macro_rules! http_router {
    {$($method:tt $pattern:expr => $data:expr),+} => {{
        let mut __router = $crate::http_router::HttpRouter::new();
        $(http_router!(@ __router, $method, $pattern,$data);)+
        __router
    }};

    {@ $router:expr, GET, $pattern:expr, $data:expr} => {
        $router.insert($crate::http_router::Method::GET, $pattern, $data)
    };
    {@ $router:expr, POST, $pattern:expr, $data:expr} => {
        $router.insert($crate::http_router::Method::POST, $pattern, $data)
    };
    {@ $router:expr, PUT, $pattern:expr, $data:expr} => {
        $router.insert($crate::http_router::Method::PUT, $pattern, $data)
    };
    {@ $router:expr, DELETE, $pattern:expr, $data:expr} => {
        $router.insert($crate::http_router::Method::DELETE, $pattern, $data)
    };
    {@ $router:expr, HEAD, $pattern:expr, $data:expr} => {
        $router.insert($crate::http_router::Method::HEAD, $pattern, $data)
    };
    {@ $router:expr, OPTIONS, $pattern:expr, $data:expr} => {
        $router.insert($crate::http_router::Method::OPTIONS, $pattern, $data)
    };
    {@ $router:expr, CONNECT, $pattern:expr, $data:expr} => {
        $router.insert($crate::http_router::Method::CONNECT, $pattern, $data)
    };
    {@ $router:expr, PATCH, $pattern:expr, $data:expr} => {
        $router.insert($crate::http_router::Method::PATCH, $pattern, $data)
    };
    {@ $router:expr, TRACE, $pattern:expr, $data:expr} => {
        $router.insert($crate::http_router::Method::TRACE, $pattern, $data)
    };
}

#[test]
fn test_macro() {
    let router: HttpRouter<i32> = http_router! {
        GET "/u/:uid/p/:pid" => 1i32,
        POST "/u/:uid/p" => 2,
        GET "/v1" => http_router!{
            GET "/info" => 3_i32,
            POST "/info" => 4
        },
        HEAD "**" => 5
    };

    assert_eq!(*router.find(&Method::GET, "/u/asd/p/qwe").unwrap().0, 1);
    assert_eq!(*router.find(&Method::POST, "/u/asd/p").unwrap().0, 2);
    assert_eq!(*router.find(&Method::GET, "/v1/info").unwrap().0, 3);
    assert_eq!(*router.find(&Method::POST, "/v1/info").unwrap().0, 4);
    assert_eq!(*router.find(&Method::HEAD, "/home/asd").unwrap().0, 5);
}
