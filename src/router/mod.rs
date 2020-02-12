mod captures;
mod core;
mod endpoint;
mod error;
mod imp;
mod owned_captures;

pub use self::captures::Captures;
pub use self::error::RouterError;
pub use self::owned_captures::OwnedCaptures;

use self::endpoint::Endpoint;
use crate::bitset::FixedBitSet;
use crate::strmap::StrMap;

#[derive(Debug, Default)]
pub struct Router<T> {
    segments: Vec<Segment>,
    routes: Vec<Route>,
    endpoints: Vec<Endpoint<T>>,
}

type Bits = u128;

#[derive(Debug)]
struct Segment {
    static_map: StrMap<FixedBitSet<Bits>>,
    dynamic: FixedBitSet<Bits>,
    wildcard: FixedBitSet<Bits>,
    num_mask: FixedBitSet<Bits>,
}

#[derive(Debug)]
struct Route {
    segment_num: usize,
    rank: u64,
    wildcard: Option<Box<str>>,
    captures: Vec<(Box<str>, usize)>,
    nested: bool,
}
