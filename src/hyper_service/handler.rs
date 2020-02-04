use super::params::Params;
use super::{BoxError, BoxFuture, Future, Request, Response, StdError};

pub trait Handler {
    fn call(&self, req: Request, params: Params) -> BoxFuture<'static, Result<Response, BoxError>>;
}

pub type BoxHandler = Box<dyn Handler + Send + Sync>;

impl Handler for BoxHandler {
    fn call(&self, req: Request, params: Params) -> BoxFuture<'static, Result<Response, BoxError>> {
        Handler::call(&**self, req, params)
    }
}

impl<F, E, Fut> Handler for F
where
    F: Fn(Request, Params) -> Fut,
    E: StdError + Send + Sync + 'static,
    Fut: Future<Output = Result<Response, E>> + Send + 'static,
{
    fn call(&self, req: Request, params: Params) -> BoxFuture<'static, Result<Response, BoxError>> {
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
