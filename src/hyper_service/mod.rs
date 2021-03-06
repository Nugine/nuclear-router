mod handler;
mod service;
mod service_macro;

pub use self::handler::Handler;
pub use self::service::{RouterService, SharedRouterService};

use std::error::Error as StdError;
use std::future::Future;
use std::pin::Pin;

type Request = hyper::Request<hyper::Body>;
type Response = hyper::Response<hyper::Body>;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
type BoxError = Box<dyn StdError + Send + Sync>;
