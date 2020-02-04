#![warn(
    nonstandard_style,
    rust_2018_idioms,
    future_incompatible,
    missing_debug_implementations
)]

mod bitset;

pub mod router;
pub use crate::router::{Captures, Router, RouterError};

macro_rules! cfg_feature{
    ($feature:literal; $($item:item)*)=>{
        $(
            #[cfg(feature = $feature)]
            $item
        )*
    }
}

cfg_feature! {
    "http-router";
    pub mod http_router;
    pub use crate::http_router::{HttpRouter, Method};
}

cfg_feature! {
    "hyper-service";
    pub mod hyper_service;
    pub use crate::hyper_service::{Params, RouterService, Handler};
}
