mod captures;
mod core;
mod endpoint;
mod error;
mod imp;

pub use self::captures::Captures;
pub use self::error::RouterError;

use self::endpoint::Endpoint;
use crate::bitset::FixedBitSet;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Router<T> {
    segments: Vec<Segment>,
    routes: Vec<Route>,
    endpoints: Vec<Endpoint<T>>,
}

#[derive(Debug)]
struct Segment {
    static_map: HashMap<Box<str>, FixedBitSet<u128>>,
    dynamic: FixedBitSet<u128>,
    wildcard: FixedBitSet<u128>,
}

#[derive(Debug)]
struct Route {
    segment_num: usize,
    rank: u64,
    wildcard: Option<Box<str>>,
    captures: Vec<(Box<str>, usize)>,
    nested: bool,
}
