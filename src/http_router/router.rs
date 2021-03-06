use crate::router::{Captures, Router, RouterError};

use std::collections::HashMap;

pub use http::Method;

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

    pub fn find<'p, 's: 'p>(
        &'s self,
        method: &Method,
        path: &'p str,
    ) -> Option<(&'s T, Captures<'p>)> {
        self.method_map.get(method)?.find(path)
    }

    pub fn find_mut<'p, 's: 'p>(
        &'s mut self,
        method: &Method,
        path: &'p str,
    ) -> Option<(&'s mut T, Captures<'p>)> {
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
