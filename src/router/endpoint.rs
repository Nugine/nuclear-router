use super::Router;

#[derive(Debug)]
pub(super) enum Endpoint<T> {
    Data(T),
    Router(Router<T>),
}

impl<T> From<T> for Endpoint<T> {
    fn from(x: T) -> Self {
        Self::Data(x)
    }
}

impl<T> From<Router<T>> for Endpoint<T> {
    fn from(x: Router<T>) -> Self {
        Self::Router(x)
    }
}

impl<T> Endpoint<T> {
    #[inline]
    pub(super) fn is_router(&self) -> bool {
        match self {
            Self::Data(_) => false,
            Self::Router(_) => true,
        }
    }
}
