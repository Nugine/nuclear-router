#![allow(unsafe_code)]

use super::endpoint::Endpoint;
use super::{Bits, Route, Router, Segment};

use crate::bitset::{BitStorage, FixedBitSet};
use crate::strmap::StrMap;

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
        let (wildcard_base, num_base) = match segments.last() {
            Some(s) => (s.wildcard.clone(), s.num_mask.clone()),
            None => (FixedBitSet::zero(), FixedBitSet::zero()),
        };

        segments.resize_with(num, || Segment {
            static_map: StrMap::new(),
            dynamic: wildcard_base.clone(),
            wildcard: wildcard_base.clone(),
            num_mask: num_base.clone(),
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
        // safety: pattern.len() >= 1
        let pattern = unsafe { pattern.get_unchecked(1..) };

        if self.routes.len() >= Bits::bit_size() {
            return Err("a single router can not hold more than 128 routes");
        }

        let mut parts: SmallVec<[&str; 8]> = pattern.split(SLASH).collect();

        if parts.len() > 64 {
            return Err("a single router can not hold more than 64 segments");
        }

        let nested = endpoint.is_router();

        let wildcard: Option<Box<str>> = {
            // safety: parts.len() >= 1
            let last = unsafe { parts.get_unchecked(parts.len() - 1) };

            if last.starts_with(STAR) {
                // safety: last.len() >= 1
                let last: Box<str> = unsafe { last.get_unchecked(1..) }.into();
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
                // safety: part.len() >= 1
                let name: Box<str> = unsafe { part.get_unchecked(1..) }.into();
                captures.push((name, i));
            } else {
                rank |= 1;
            }
        }

        let check_collision = || {
            if self.routes.is_empty() {
                return false;
            }
            let mut enable_mask: FixedBitSet<Bits> = FixedBitSet::one();
            for (part, s) in parts.iter().cloned().zip(self.segments.iter()) {
                let mut e = s.dynamic.clone();
                if !part.starts_with(COLON) {
                    if let Some(m) = s.static_map.find(part) {
                        e.union_with(m);
                    }
                }
                enable_mask.intersect_with(&e);
            }
            // safety: FixedBitSet<u128>.iter_ones(), i in 0..128, and routes.len() < 128
            let mut iter = enable_mask
                .iter_ones()
                .map(|i| unsafe { self.routes.get_unchecked(i) });

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
                    .find_mut_with(part, FixedBitSet::zero)
                    .set(id, true)
            }
        }

        if nested | wildcard.is_some() {
            let pos = parts.len();
            // safety: parts.len() <= segment_num <= self.segments.len()
            let segs = unsafe { self.segments.get_unchecked_mut(pos..) };
            for s in segs {
                s.dynamic.set(id, true);
                s.wildcard.set(id, true);
            }
        }

        {
            let pos = segment_num - 1;
            let segs = unsafe { self.segments.get_unchecked_mut(pos..) };
            for s in segs {
                s.num_mask.set(id, true);
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
    pub(super) fn real_find<'p, 's: 'p>(
        &'s self,
        path: &'p str,
        captures: &mut SmallKvBuffer<'p>,
    ) -> Option<&'s T> {
        let parts: SmallVec<[&str; 8]> = trim_first_slash(path).split(SLASH).collect();
        self.find_with_parts(path, &parts, captures)
            .map(|p| unsafe { &*p.as_ptr() })
    }

    pub(super) fn real_find_mut<'p, 's: 'p>(
        &'s self,
        path: &'p str,
        captures: &mut SmallKvBuffer<'p>,
    ) -> Option<&'s mut T> {
        let parts: SmallVec<[&str; 8]> = trim_first_slash(path).split(SLASH).collect();
        self.find_with_parts(path, &parts, captures)
            .map(|p| unsafe { &mut *p.as_ptr() })
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

        let mut enable_mask: FixedBitSet<Bits> = FixedBitSet::one();

        for (part, s) in parts.iter().cloned().zip(self.segments.iter()) {
            let mut e = s.dynamic.clone();
            if let Some(m) = s.static_map.find(part) {
                e.union_with(m);
            }
            enable_mask.intersect_with(&e);
        }
        if parts.len() > self.segments.len() {
            // safety: self.routes is not empty so that self.segments is not empty
            let last_wildcard = unsafe {
                &self
                    .segments
                    .get_unchecked(self.segments.len() - 1)
                    .wildcard
            };
            enable_mask.intersect_with(last_wildcard);
        }
        if parts.len() - 1 < self.segments.len() {
            let s = unsafe { self.segments.get_unchecked(parts.len() - 1) };
            enable_mask.intersect_with(&s.num_mask);
        }

        let base_ptr = self.routes.as_ptr();
        let mut iter = enable_mask
            .iter_ones()
            .map(|i| unsafe { &*base_ptr.add(i) });

        let route: &Route = {
            let mut ans = iter.next()?;
            for r in iter {
                if r.segment_num == ans.segment_num && r.rank > ans.rank {
                    ans = r;
                }
                if r.segment_num > ans.segment_num {
                    ans = r;
                }
            }
            ans
        };

        for &(ref name, i) in route.captures.iter() {
            // safety: i < route.segment_num <= parts.len()
            captures.push((&**name, unsafe { parts.get_unchecked(i) }));
        }
        if let Some(ref name) = route.wildcard {
            // safety: parts and path point to the same str, and path is the base ptr
            let offset =
                (calc_offset(path, parts[route.segment_num - 1]) as usize).saturating_sub(1);
            captures.push((&**name, unsafe { path.get_unchecked(offset..) }));
        }

        let idx = unsafe { offset_from(route, self.routes.as_ptr()) };
        let endpoint = unsafe { self.endpoints.get_unchecked(idx) };
        match endpoint {
            Endpoint::Data(t) => Some(NonNull::from(t)),
            Endpoint::Router(r) => {
                let parts = unsafe { parts.get_unchecked((route.segment_num - 1)..) };
                let offset = (calc_offset(path, parts[0]) as usize).saturating_sub(1);
                let path = unsafe { path.get_unchecked(offset..) };
                r.find_with_parts(path, parts, captures)
            }
        }
    }
}

#[inline(always)]
fn trim_first_slash(s: &str) -> &str {
    if s.starts_with(SLASH) {
        unsafe { s.get_unchecked(1..) }
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

// safety: caller should ensure that size_of::<T>() != 0 and ptr >= src
#[inline(always)]
unsafe fn offset_from<T>(ptr: *const T, src: *const T) -> usize {
    assert!(std::mem::size_of::<T>() != 0);
    (ptr as usize).wrapping_sub(src as usize) / std::mem::size_of::<T>()
}
