#![allow(unsafe_code)]

use super::endpoint::Endpoint;
use super::{Route, Router, Segment};

use crate::bitset::FixedBitSet;

use std::collections::HashMap;
use std::ptr::NonNull;

use smallvec::SmallVec;

type SmallKvBuffer<'a> = SmallVec<[(&'a str, &'a str); 8]>;

const STAR: char = '*';
const COLON: char = ':';
const SLASH: char = '/';

impl<T> Router<T> {
    fn check_parts(parts: &[&str]) -> Result<(), &'static str> {
        for &part in parts {
            if part.starts_with(STAR) {
                return Err("wildcard pattern can only appear at end");
            }
            if part.starts_with(COLON) && part.len() == 1 {
                return Err("capture name can not be empty");
            }
        }
        Ok(())
    }

    fn extend_segments(segments: &mut Vec<Segment>, num: usize) {
        let base = match segments.last() {
            Some(s) => s.wildcard.clone(),
            None => FixedBitSet::zero(),
        };

        segments.resize_with(num, || Segment {
            static_map: HashMap::new(),
            dynamic: base.clone(),
            wildcard: base.clone(),
        });
    }

    pub(super) fn insert_endpoint(
        &mut self,
        pattern: &str,
        endpoint: Endpoint<T>,
    ) -> Result<(), &'static str> {
        if !pattern.starts_with(SLASH) {
            return Err("pattern must start with '/'");
        }
        let pattern = &pattern[1..];

        if self.routes.len() >= 128 {
            return Err("a single router can not hold more than 128 routes");
        }

        let mut parts: SmallVec<[&str; 8]> = pattern.split(SLASH).collect();

        if parts.len() > 64 {
            return Err("a single router can not hold more than 64 segments");
        }

        let nested = endpoint.is_router();

        let wildcard: Option<Box<str>> = {
            let last = *parts.last().unwrap();
            if last.starts_with(STAR) {
                let last: Box<str> = last[1..].into();
                if last.is_empty() {
                    return Err("capture name can not be empty");
                }
                if nested {
                    return Err("wildcard pattern can not be used for router prefix");
                }
                parts.pop();
                Some(last)
            } else {
                None
            }
        };

        Self::check_parts(&parts)?;

        let segment_num = parts.len() + usize::from(nested | wildcard.is_some());

        let mut captures: Vec<(Box<str>, usize)> = Vec::new();
        let mut rank: u64 = 0;

        for (i, &part) in parts.iter().enumerate() {
            rank <<= 1;
            if part.starts_with(COLON) {
                captures.push((part[1..].into(), i));
            } else {
                rank |= 1;
            }
        }

        let check_collision = || {
            if self.routes.is_empty() {
                return false;
            }
            let mut enable_mask: FixedBitSet<u128> = FixedBitSet::one();
            for (part, s) in parts.iter().cloned().zip(self.segments.iter()) {
                let mut e = s.dynamic.clone();
                if !part.starts_with(COLON) {
                    if let Some(m) = s.static_map.get(part) {
                        e.union_with(m);
                    }
                }
                enable_mask.intersect_with(&e);
            }
            let mut iter = enable_mask.iter_ones().map(|i| &self.routes[i]);
            iter.any(|route: &Route| -> bool {
                if route.nested {
                    return nested || segment_num >= route.segment_num;
                }
                if nested {
                    return route.segment_num >= segment_num;
                }
                let same = !(route.wildcard.is_some() ^ wildcard.is_some());
                same && rank == route.rank
            })
        };

        if check_collision() {
            return Err("pattern collision occured");
        }

        if segment_num > self.segments.len() {
            Self::extend_segments(&mut self.segments, segment_num);
        }

        let id = self.routes.len();

        for (part, s) in parts.iter().cloned().zip(self.segments.iter_mut()) {
            if part.starts_with(COLON) {
                s.dynamic.set(id, true)
            } else {
                s.static_map
                    .entry(part.into())
                    .or_insert_with(FixedBitSet::zero)
                    .set(id, true)
            }
        }

        if nested | wildcard.is_some() {
            let pos = parts.len();
            for s in self.segments[pos..].iter_mut() {
                s.dynamic.set(id, true);
                s.wildcard.set(id, true);
            }
        }

        self.endpoints.push(endpoint);
        self.routes.push(Route {
            segment_num,
            rank,
            captures,
            wildcard,
            nested,
        });

        Ok(())
    }
}

impl<T> Router<T> {
    pub(super) fn find_ptr<'p, 's: 'p>(
        &'s self,
        path: &'p str,
        captures: &mut SmallKvBuffer<'p>,
    ) -> Option<NonNull<T>> {
        let parts: SmallVec<[&str; 8]> = trim_first_slash(path).split(SLASH).collect();
        self.find_with_parts(path, &parts, captures)
    }

    fn find_with_parts<'p, 's: 'p>(
        &'s self,
        path: &'p str,
        parts: &[&'p str],
        captures: &mut SmallKvBuffer<'p>,
    ) -> Option<NonNull<T>> {
        if self.routes.is_empty() {
            return None;
        }

        let mut enable_mask: FixedBitSet<u128> = FixedBitSet::one();

        for (part, s) in parts.iter().cloned().zip(self.segments.iter()) {
            let mut e = s.dynamic.clone();
            if let Some(m) = s.static_map.get(part) {
                e.union_with(m);
            }
            enable_mask.intersect_with(&e);
        }
        if parts.len() > self.segments.len() {
            let last_wildcard = &self.segments.last().unwrap().wildcard;
            enable_mask.intersect_with(last_wildcard);
        }

        let idx = enable_mask
            .iter_ones()
            .filter(|&i| self.routes[i].segment_num <= parts.len())
            .max_by(|&i, &j| {
                let lhs = &self.routes[i];
                let rhs = &self.routes[j];
                if lhs.segment_num != rhs.segment_num {
                    return lhs.segment_num.cmp(&rhs.segment_num);
                }
                lhs.rank.cmp(&rhs.rank)
            })?;

        let route = &self.routes[idx];
        for &(ref name, i) in route.captures.iter() {
            captures.push((&**name, parts[i]));
        }
        if let Some(ref name) = route.wildcard {
            let offset =
                (calc_offset(path, parts[route.segment_num - 1]) as usize).saturating_sub(1);
            captures.push((&**name, &path[offset..]));
        }

        let endpoint = &self.endpoints[idx];
        match endpoint {
            Endpoint::Data(t) => Some(NonNull::from(t)),
            Endpoint::Router(r) => {
                let parts = &parts[(route.segment_num - 1)..];
                let offset = (calc_offset(path, parts[0]) as usize).saturating_sub(1);
                let path = &path[offset..];
                r.find_with_parts(path, parts, captures)
            }
        }
    }
}

#[inline(always)]
fn trim_first_slash(s: &str) -> &str {
    if s.starts_with(SLASH) {
        &s[1..]
    } else {
        s
    }
}

#[inline(always)]
fn calc_offset(src: &str, dst: &str) -> isize {
    let p2 = dst.as_ptr() as isize;
    let p1 = src.as_ptr() as isize;
    p2 - p1
}
