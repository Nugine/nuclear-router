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

    pub fn find<'s, 'p, 't>(
        &'s self,
        method: &Method,
        path: &'p str,
    ) -> Option<(&'t T, Captures<'p>)>
    where
        's: 'p + 't,
    {
        self.method_map.get(method)?.find(path)
    }

    pub fn find_mut<'s, 'p, 't>(
        &'s mut self,
        method: &Method,
        path: &'p str,
    ) -> Option<(&'t mut T, Captures<'p>)>
    where
        's: 'p + 't,
    {
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

    pub fn insert_regexes<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = (Method, Regex, T)>,
    {
        let mut items: HashMap<Method, Vec<(Regex, T)>> = HashMap::new();
        for (m, r, t) in iter {
            items.entry(m).or_insert_with(Vec::new).push((r, t))
        }
        for (m, v) in items {
            self.access_router(m).insert_regexes(v);
        }
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

    pub fn insert_router(&mut self, prefix: &str, router: HttpRouter<T>) {
        for (method, router) in router.method_map {
            self.access_router(method).insert_router(prefix, router);
        }
    }

    pub fn try_insert_router(
        &mut self,
        prefix: &str,
        router: HttpRouter<T>,
    ) -> Result<&mut Self, RouterError> {
        for (method, router) in router.method_map {
            self.access_router(method)
                .try_insert_router(prefix, router)?;
        }
        Ok(self)
    }
}

impl<T> HttpRouter<T> {
    fn access_router(&mut self, method: Method) -> &mut Router<T> {
        self.method_map.entry(method).or_insert_with(Router::new)
    }
}
