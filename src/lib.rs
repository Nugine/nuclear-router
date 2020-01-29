mod bitset;

pub mod router;
pub use self::router::{Router, RouterError};

#[cfg(feature = "http-router")]
pub mod http_router;
