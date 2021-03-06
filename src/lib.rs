#![warn(
    nonstandard_style,
    rust_2018_idioms,
    future_incompatible,
    missing_debug_implementations
)]
#![deny(unsafe_code)]

mod bitset;
mod strmap;

mod router;
pub use crate::router::{Captures, OwnedCaptures, Router, RouterError};

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
    mod http_router;
    pub use crate::http_router::{HttpRouter, Method};
}

cfg_feature! {
    "hyper-service";
    mod hyper_service;
    pub use crate::hyper_service::{RouterService, Handler, SharedRouterService};
}
