#![forbid(unsafe_code)]

use crate::router::{Captures, Router, RouterError};

use std::collections::HashMap;

pub use http::Method;
use regex::Regex;

#[derive(Debug, Default)]
pub struct HttpRouter<T> {
    method_map: HashMap<Method, Router<T>>,
}

impl<T> HttpRouter<T> {
    pub fn new() -> Self {
        Self {
            method_map: HashMap::new(),
        }
    }

    pub fn find<'a>(&'a self, method: &Method, path: &'a str) -> Option<(&'a T, Captures<'a>)> {
        self.method_map.get(method)?.find(path)
    }

    pub fn find_mut<'a>(
        &'a mut self,
        method: &Method,
        path: &'a str,
    ) -> Option<(&'a mut T, Captures<'a>)> {
        self.method_map.get_mut(method)?.find_mut(path)
    }

    pub fn insert(&mut self, method: Method, pattern: &str, data: T) -> &mut Self {
        self.access_router(method).insert(pattern, data);
        self
    }

    pub fn try_insert(
        &mut self,
        method: Method,
        pattern: &str,
        data: T,
    ) -> Result<&mut Self, RouterError> {
        self.access_router(method).try_insert(pattern, data)?;
        Ok(self)
    }

    pub fn insert_regex(&mut self, method: Method, pattern: Regex, data: T) -> &mut Self {
        self.access_router(method).insert_regex(pattern, data);
        self
    }

    pub fn nest(&mut self, prefix: &str, f: impl FnOnce(&mut HttpRouter<T>)) -> &mut Self {
        let mut sub_router = Self::new();
        f(&mut sub_router);
        self.insert_router(prefix, sub_router);
        self
    }

    pub fn try_nest(
        &mut self,
        prefix: &str,
        f: impl FnOnce(&mut HttpRouter<T>),
    ) -> Result<&mut Self, RouterError> {
        let mut sub_router = Self::new();
        f(&mut sub_router);
        self.try_insert_router(prefix, sub_router)?;
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
        $(http_router!(@entry __router, $method, $pattern, $data);)+
        __router
    }};

    {@entry $router:expr, @, $prefix:expr, $sub_router:expr} => {
        $router.nest($prefix, |__r| *__r = $sub_router)
    };
    {@entry $router:expr, GET, $pattern:expr, $data:expr} => {
        $router.insert($crate::http_router::Method::GET, $pattern, $data)
    };
    {@entry $router:expr, POST, $pattern:expr, $data:expr} => {
        $router.insert($crate::http_router::Method::POST, $pattern, $data)
    };
    {@entry $router:expr, PUT, $pattern:expr, $data:expr} => {
        $router.insert($crate::http_router::Method::PUT, $pattern, $data)
    };
    {@entry $router:expr, DELETE, $pattern:expr, $data:expr} => {
        $router.insert($crate::http_router::Method::DELETE, $pattern, $data)
    };
    {@entry $router:expr, HEAD, $pattern:expr, $data:expr} => {
        $router.insert($crate::http_router::Method::HEAD, $pattern, $data)
    };
    {@entry $router:expr, OPTIONS, $pattern:expr, $data:expr} => {
        $router.insert($crate::http_router::Method::OPTIONS, $pattern, $data)
    };
    {@entry $router:expr, CONNECT, $pattern:expr, $data:expr} => {
        $router.insert($crate::http_router::Method::CONNECT, $pattern, $data)
    };
    {@entry $router:expr, PATCH, $pattern:expr, $data:expr} => {
        $router.insert($crate::http_router::Method::PATCH, $pattern, $data)
    };
    {@entry $router:expr, TRACE, $pattern:expr, $data:expr} => {
        $router.insert($crate::http_router::Method::TRACE, $pattern, $data)
    };
}
