use super::captures::Captures;
use super::error::RouterError;
use super::Router;

impl<T> Router<T> {
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            routes: Vec::new(),
            endpoints: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.segments.clear();
        self.routes.clear();
        self.endpoints.clear();
    }

    pub fn find<'p, 's: 'p>(&'s self, path: &'p str) -> Option<(&'s T, Captures<'p>)> {
        let mut captures = Captures::new(path);
        let data = self.real_find(path, &mut captures.buffer())?;
        Some((data, captures))
    }

    pub fn find_mut<'p, 's: 'p>(&'s mut self, path: &'p str) -> Option<(&'s mut T, Captures<'p>)> {
        let mut captures = Captures::new(path);
        let data = self.real_find_mut(path, &mut captures.buffer())?;
        Some((data, captures))
    }

    pub fn insert(&mut self, pattern: &str, data: T) -> &mut Self {
        if let Err(e) = self.insert_endpoint(pattern, data.into()) {
            panic!("{}: pattern = {:?}", e, pattern);
        }
        self
    }

    pub fn try_insert(&mut self, pattern: &str, data: T) -> Result<&mut Self, RouterError> {
        match self.insert_endpoint(pattern, data.into()) {
            Ok(()) => Ok(self),
            Err(msg) => Err(RouterError::new(msg)),
        }
    }

    pub fn insert_router(&mut self, prefix: &str, router: Router<T>) -> &mut Self {
        if let Err(e) = self.insert_endpoint(prefix, router.into()) {
            panic!("{}: pattern = {:?}", e, prefix);
        }
        self
    }

    pub fn try_insert_router(
        &mut self,
        prefix: &str,
        router: Router<T>,
    ) -> Result<&mut Self, RouterError> {
        match self.insert_endpoint(prefix, router.into()) {
            Ok(()) => Ok(self),
            Err(msg) => Err(RouterError::new(msg)),
        }
    }

    pub fn nest(&mut self, prefix: &str, f: impl FnOnce(&mut Router<T>)) -> &mut Self {
        let mut router = Self::new();
        f(&mut router);
        self.insert_router(prefix, router)
    }

    pub fn try_nest(
        &mut self,
        prefix: &str,
        f: impl FnOnce(&mut Router<T>),
    ) -> Result<&mut Self, RouterError> {
        let mut router = Self::new();
        f(&mut router);
        self.try_insert_router(prefix, router)
    }
}
