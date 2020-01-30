#![forbid(unsafe_code)]

use crate::bitset::{BitStorage, FixedBitSet, TABLE};

use std::collections::HashMap;

use regex::Regex;
use smallvec::SmallVec;

#[derive(Debug, Default)]
pub struct Router<T> {
    min_segments: Option<usize>,
    segments: Vec<Segment>,
    routes: Vec<Route<T>>,
    regexps: Vec<(Regex, T)>,
}

#[derive(Debug, thiserror::Error)]
#[error("{msg}")]
pub struct RouterError {
    msg: &'static str,
}

#[derive(Debug)]
pub enum Endpoint<T> {
    Data(T),
    Router(Router<T>),
}

type BitArray = [u128; 4];

#[derive(Debug)]
struct Segment {
    static_map: HashMap<String, FixedBitSet<BitArray>>,
    dynamic: FixedBitSet<BitArray>,
    catch_all: FixedBitSet<BitArray>,
}

#[derive(Debug)]
struct Route<T> {
    min_segments: usize,
    catch_all: Option<usize>,
    captures: Vec<(String, usize)>,
    endpoint: Endpoint<T>,
}

impl<T> From<T> for Endpoint<T> {
    fn from(x: T) -> Self {
        Self::Data(x)
    }
}

impl<T> From<Router<T>> for Endpoint<T> {
    fn from(x: Router<T>) -> Self {
        Self::Router(x)
    }
}

impl<T> Router<T> {
    pub fn new() -> Self {
        Self {
            segments: vec![],
            routes: vec![],
            regexps: vec![],
            min_segments: None,
        }
    }

    pub fn clear(&mut self) {
        self.segments.clear();
        self.routes.clear();
        self.regexps.clear();
    }

    pub fn find<'a>(
        &'a self,
        path: &'a str,
    ) -> Option<(&'a T, impl Iterator<Item = (&'a str, &'a str)>)> {
        let min_segments = self.min_segments?;
        let parts: SmallVec<[&str; 8]> = trim_fisrt_slash(path).split('/').collect();
        if parts.len() < min_segments {
            return None;
        }
        let mut captures: SmallVec<[(&'a str, &'a str); 8]> = SmallVec::new();
        let data: &T = self.find_with_buf(path, &parts, &mut captures)?;
        Some((data, captures.into_iter()))
    }

    pub fn insert(&mut self, pattern: &str, endpoint: impl Into<Endpoint<T>>) -> &mut Self {
        if let Err(e) = self.insert_endpoint(pattern, endpoint.into()) {
            panic!("{}: pattern = {:?}", e, pattern);
        }
        self
    }

    pub fn try_insert(
        &mut self,
        pattern: &str,
        endpoint: impl Into<Endpoint<T>>,
    ) -> Result<&mut Self, RouterError> {
        match self.insert_endpoint(pattern, endpoint.into()) {
            Ok(()) => Ok(self),
            Err(msg) => Err(RouterError { msg }),
        }
    }

    pub fn insert_regex(&mut self, pattern: Regex, data: T) -> &mut Self {
        self.regexps.push((pattern, data));
        self
    }

    pub fn nest(&mut self, prefix: &str, f: impl FnOnce(&mut Router<T>)) -> &mut Self {
        let mut router = Self::new();
        f(&mut router);
        if let Err(e) = self.insert_endpoint(prefix, Endpoint::Router(router)) {
            panic!("{}: pattern = {:?}", e, prefix);
        }
        self
    }

    pub fn try_nest(
        &mut self,
        prefix: &str,
        f: impl FnOnce(&mut Router<T>),
    ) -> Result<&mut Self, RouterError> {
        let mut router = Self::new();
        f(&mut router);
        match self.insert_endpoint(prefix, Endpoint::Router(router)) {
            Ok(()) => Ok(self),
            Err(msg) => Err(RouterError { msg }),
        }
    }
}

impl<T> Router<T> {
    fn check_collision(&self, pattern: &str, parts: &[&str], catch_all: Option<usize>) -> bool {
        if self.routes.is_empty() {
            return false;
        }

        let mut bitset: FixedBitSet<BitArray> = FixedBitSet::one();

        for (i, &part) in parts.iter().enumerate() {
            let get_mask = || -> Option<&_> {
                let s = self.segments.get(i)?;
                if part.starts_with(':') {
                    Some(&s.dynamic)
                } else {
                    s.static_map.get(part)
                }
            };
            match get_mask() {
                Some(mask) => bitset.intersect_with(mask),
                None => return false,
            }
        }

        let catch_from = match catch_all {
            None => return !bitset.is_zero(),
            Some(i) => i,
        };

        let mut iter = bitset.iter_ones().map(|i| &self.routes[i]);

        iter.any(|route: &Route<T>| match route.endpoint {
            Endpoint::Data(_) => route.catch_all.map(|j| catch_from == j).unwrap_or(true),
            Endpoint::Router(ref router) => parts
                .get(catch_from)
                .map(|p| (calc_offset(pattern, p) as usize).saturating_sub(1))
                .map(|offset| {
                    router.check_collision(
                        &pattern[offset..],
                        &parts[route.catch_all.unwrap()..],
                        catch_all,
                    )
                })
                .unwrap_or(false),
        })
    }

    fn insert_endpoint(
        &mut self,
        pattern: &str,
        endpoint: Endpoint<T>,
    ) -> Result<(), &'static str> {
        if self.routes.len() >= 512 {
            return Err("can not hold more than 512 routes");
        }

        let mut parts: SmallVec<[&str; 8]> = trim_fisrt_slash(pattern).split('/').collect();

        let nested = match &endpoint {
            Endpoint::Data(_) => false,
            Endpoint::Router(_) => true,
        };

        let catch = *parts.last().unwrap() == "**";

        if nested && catch {
            return Err("\"**\" can not be used for router prefix");
        }

        let catch_all = if nested {
            Some(parts.len())
        } else {
            some_if(catch, || parts.len() - 1)
        };

        if self.check_collision(pattern, &parts, catch_all) {
            return Err("pattern collision occurred");
        }

        let last_len = self.segments.len();

        for _ in self.segments.len()..parts.len() {
            let catch_all = if last_len > 0 {
                self.segments[last_len - 1].catch_all.clone()
            } else {
                FixedBitSet::zero()
            };

            self.segments.push(Segment {
                static_map: HashMap::new(),
                dynamic: catch_all.clone(),
                catch_all,
            });
        }

        let min_segments = parts.len();
        self.min_segments = match self.min_segments {
            Some(m) => Some(m.min(min_segments)),
            None => Some(min_segments),
        };

        if catch {
            parts.pop();
        }

        let mut captures: Vec<(String, usize)> = Vec::new();
        let id = self.routes.len();

        for (i, &part) in parts.iter().enumerate() {
            if part == "**" {
                return Err("\"**\" can only appear at end");
            }
            if part.starts_with(':') {
                let name: &str = &part[1..];
                if name == "**" {
                    return Err("\"**\" can not be used for capture name");
                }
                captures.push((name.to_owned(), i));
                self.segments[i].dynamic.set(id, true);
            } else {
                let bitset = self.segments[i]
                    .static_map
                    .entry(part.to_owned())
                    .or_insert_with(FixedBitSet::zero);
                bitset.set(id, true);
            }
        }

        if let Some(pos) = catch_all {
            for i in pos..self.segments.len() {
                let s = &mut self.segments[i];
                s.dynamic.set(id, true);
                s.catch_all.set(id, true);
            }
        }

        self.routes.push(Route {
            min_segments,
            captures,
            catch_all,
            endpoint,
        });

        Ok(())
    }

    fn find_regex<'a>(
        &'a self,
        path: &'a str,
        captures: &mut SmallVec<[(&'a str, &'a str); 8]>,
    ) -> Option<&T> {
        for (regex, data) in &self.regexps {
            if let Some(caps) = regex.captures(path) {
                for name in regex.capture_names().flatten() {
                    let text = caps.name(name).unwrap().as_str();
                    captures.push((name, text))
                }
                return Some(data);
            }
        }
        None
    }

    fn find_with_buf<'a>(
        &'a self,
        path: &'a str,
        parts: &[&'a str],
        captures: &mut SmallVec<[(&'a str, &'a str); 8]>,
    ) -> Option<&T> {
        let ret = self.find_regex(path, captures);
        if ret.is_some() {
            return ret;
        }

        if self.segments.is_empty() {
            return None;
        }

        let mut router_routes: SmallVec<[&'a Route<T>; 8]> = SmallVec::new();
        let mut data_routes: SmallVec<[&'a Route<T>; 8]> = SmallVec::new();

        let mut bitset: FixedBitSet<BitArray> = FixedBitSet::one();

        for (i, &part) in parts.iter().enumerate() {
            match self.segments.get(i) {
                Some(s) => {
                    let mask = s.static_map.get(part).unwrap_or(&s.dynamic);
                    bitset.intersect_with(mask);
                }
                None => {
                    let mask = &self.segments.last().unwrap().catch_all;
                    bitset.intersect_with(mask);
                    break;
                }
            };
        }

        for (i, u) in bitset.get_inner().iter().enumerate() {
            if *u != 0 {
                for (j, x) in u.as_bytes().iter().enumerate() {
                    if *x != 0 {
                        for k in TABLE[*x as usize] {
                            let route = &self.routes[i * 128 + j * 8 + k];
                            match route.endpoint {
                                Endpoint::Data(_) => data_routes.push(route),
                                Endpoint::Router(_) => router_routes.push(route),
                            }
                        }
                    }
                }
            }
        }

        let offset = |catch_from: usize| {
            parts
                .get(catch_from)
                .map(|p| (calc_offset(path, p) as usize).saturating_sub(1))
        };

        for &route in &router_routes {
            match route.endpoint {
                Endpoint::Router(ref router) => {
                    if route.min_segments > parts.len() {
                        continue;
                    }
                    for &(ref name, i) in &route.captures {
                        captures.push((name.as_str(), parts[i]))
                    }
                    let catch_from = route.catch_all.unwrap();
                    let offset: usize = match offset(catch_from) {
                        Some(o) => o,
                        None => continue,
                    };
                    let sub_parts = &parts[catch_from..];
                    match router.min_segments {
                        None => continue,
                        Some(m) => {
                            if m > sub_parts.len() {
                                continue;
                            }
                        }
                    }
                    let ret = router.find_with_buf(&path[offset..], sub_parts, captures);
                    if ret.is_some() {
                        return ret;
                    }
                }
                _ => unreachable!(),
            }
        }

        for &route in &data_routes {
            match route.endpoint {
                Endpoint::Data(ref t) => {
                    if route.min_segments > parts.len() {
                        continue;
                    }
                    for &(ref name, i) in &route.captures {
                        captures.push((name.as_str(), parts[i]))
                    }
                    if let Some(catch_from) = route.catch_all {
                        let offset: usize = match offset(catch_from) {
                            Some(o) => o,
                            None => continue,
                        };
                        captures.push(("**", &path[offset..]));
                    }
                    return Some(t);
                }
                _ => unreachable!(),
            }
        }

        None
    }
}

#[inline]
fn trim_fisrt_slash(s: &str) -> &str {
    if s.starts_with('/') {
        &s[1..]
    } else {
        s
    }
}

#[inline]
fn calc_offset(src: &str, dst: &str) -> isize {
    let p2 = dst.as_ptr() as isize;
    let p1 = src.as_ptr() as isize;
    p2 - p1
}

#[inline]
fn some_if<T>(cond: bool, f: impl FnOnce() -> T) -> Option<T> {
    if cond {
        Some(f())
    } else {
        None
    }
}
