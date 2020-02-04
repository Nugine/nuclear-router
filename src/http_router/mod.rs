#![forbid(unsafe_code)]

mod router;
mod router_macro;

pub use self::router::{HttpRouter, Method};
