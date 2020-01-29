#![deny(unsafe_code)]

mod bitset;
mod table;

use crate::bitset::{BitStorage, FixedBitSet};
use crate::table::TABLE;

use std::collections::HashMap;

use regex::Regex;
use smallvec::SmallVec;

#[derive(Debug, Default)]
pub struct Router<T> {
    segments: Vec<Segment>,
    routes: Vec<Route<T>>,
    regexps: Vec<(Regex, T)>,
}

#[derive(Debug, thiserror::Error)]
#[error("{msg}")]
pub struct RouterError {
    msg: &'static str,
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
    captures: Vec<(String, usize)>,
    catch_all: Option<usize>,
    endpoint: Endpoint<T>,
}

#[derive(Debug)]
enum Endpoint<T> {
    Data(T),
    Router((Router<T>, usize)),
}

enum Either<A, B> {
    A(A),
    B(B),
}

impl<T> Router<T> {
    pub fn new() -> Self {
        Self {
            segments: vec![],
            routes: vec![],
            regexps: vec![],
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
        let mut captures: SmallVec<[(&'a str, &'a str); 8]> = SmallVec::new();
        let parts: SmallVec<[&str; 8]> = trim_fisrt_slash(path).split('/').collect();
        let data: &T = self.find_with_buf(path, &parts, &mut captures)?;
        Some((data, captures.into_iter()))
    }

    pub fn insert(&mut self, pattern: &str, data: T) -> &mut Self {
        if let Err(e) = self.insert_endpoint(pattern, Either::A(data)) {
            panic!("{}: pattern = {:?}", e, pattern);
        }
        self
    }

    pub fn try_insert(&mut self, pattern: &str, data: T) -> Result<&mut Self, RouterError> {
        match self.insert_endpoint(pattern, Either::A(data)) {
            Ok(()) => Ok(self),
            Err(msg) => Err(RouterError { msg }),
        }
    }

    pub fn insert_regex(&mut self, pattern: Regex, data: T) -> &mut Self {
        self.regexps.push((pattern, data));
        self
    }

    pub fn nested(&mut self, prefix: &str, f: impl FnOnce(&mut Router<T>)) -> &mut Self {
        let mut router = Self::new();
        f(&mut router);
        if let Err(e) = self.insert_endpoint(prefix, Either::B(router)) {
            panic!("{}: pattern = {:?}", e, prefix);
        }
        self
    }

    pub fn try_nested(
        &mut self,
        prefix: &str,
        f: impl FnOnce(&mut Router<T>),
    ) -> Result<&mut Self, RouterError> {
        let mut router = Self::new();
        f(&mut router);
        match self.insert_endpoint(prefix, Either::B(router)) {
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
            Endpoint::Router((ref router, j)) => parts
                .get(catch_from)
                .map(|p| (offset(pattern, p) as usize).saturating_sub(1))
                .map(|offset| router.check_collision(&pattern[offset..], &parts[j..], catch_all))
                .unwrap_or(false),
        })
    }

    fn insert_endpoint(
        &mut self,
        pattern: &str,
        endpoint: Either<T, Router<T>>,
    ) -> Result<(), &'static str> {
        if self.routes.len() >= 512 {
            return Err("can not hold more than 512 routes");
        }

        let mut parts: SmallVec<[&str; 8]> = trim_fisrt_slash(pattern).split('/').collect();

        let nested = match &endpoint {
            Either::A(_) => false,
            Either::B(_) => true,
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

        let endpoint = match endpoint {
            Either::A(data) => Endpoint::Data(data),
            Either::B(router) => Endpoint::Router((router, parts.len())),
        };

        self.routes.push(Route {
            captures,
            catch_all,
            endpoint,
        });

        Ok(())
    }

    fn find_with_buf<'a>(
        &'a self,
        path: &'a str,
        parts: &[&'a str],
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

        let mut ones_buf: SmallVec<[usize; 8]> = SmallVec::new();

        if !self.segments.is_empty() {
            let mut bitset: FixedBitSet<BitArray> = FixedBitSet::one();

            for (i, &part) in parts.iter().enumerate() {
                let segment: &Segment = match self.segments.get(i) {
                    Some(s) => s,
                    None => break,
                };
                let mask = segment.static_map.get(part).unwrap_or(&segment.dynamic);
                bitset.intersect_with(mask);
            }

            for (i, u) in bitset.get_inner().iter().enumerate() {
                if *u != 0 {
                    for (j, x) in u.as_bytes().iter().enumerate() {
                        if *x != 0 {
                            for k in TABLE[*x as usize] {
                                ones_buf.push(i * 128 + j * 8 + k)
                            }
                        }
                    }
                }
            }
        }

        for route in ones_buf.iter().map(|&i| &self.routes[i]) {
            match route.endpoint {
                Endpoint::Router((ref router, i)) => {
                    for (name, i) in &route.captures {
                        captures.push((name.as_str(), parts[*i]))
                    }
                    let offset: usize = match parts.get(i) {
                        Some(p) => (offset(path, p) as usize).saturating_sub(1),
                        None => continue,
                    };
                    let ret = router.find_with_buf(&path[offset..], &parts[i..], captures);
                    if ret.is_some() {
                        return ret;
                    }
                }
                Endpoint::Data(_) => continue,
            }
        }

        for route in ones_buf.iter().map(|&i| &self.routes[i]) {
            match route.endpoint {
                Endpoint::Data(ref t) => {
                    for (name, i) in &route.captures {
                        captures.push((name.as_str(), parts[*i]))
                    }
                    if let Some(i) = route.catch_all {
                        let offset: usize = match parts.get(i) {
                            Some(p) => (offset(path, p) as usize).saturating_sub(1),
                            None => continue,
                        };
                        captures.push(("**", &path[offset..]));
                    }

                    return Some(t);
                }
                Endpoint::Router(_) => continue,
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
fn offset(src: &str, dst: &str) -> isize {
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

#[test]
fn test_simple() {
    let mut router: Router<usize> = Router::new();
    router
        .nested("/user/:user_id", |user| {
            user.insert("post/:post_id", 1)
                .insert("profile", 2)
                .insert("file/**", 3)
                .insert("", 4);
        })
        .insert("explore", 5)
        .nested("pan", |pan| {
            pan.insert("**", 6)
                .insert_regex(Regex::new(".*/(?P<name>.+)\\.php$").unwrap(), 7);
        });

    let cases: &[(_, _, &[(&str, &str)])] = &[
        (
            "/user/asd/post/123",
            1,
            &[("user_id", "asd"), ("post_id", "123")],
        ),
        ("/user/asd/profile", 2, &[("user_id", "asd")]),
        (
            "/user/asd/file/home/asd/.bashrc",
            3,
            &[("user_id", "asd"), ("**", "/home/asd/.bashrc")],
        ),
        ("/user/asd/", 4, &[("user_id", "asd")]),
        ("/explore", 5, &[]),
        ("/pan/home/asd", 6, &[("**", "/home/asd")]),
        ("/pan/phpinfo.php", 7, &[("name", "phpinfo")]),
    ];

    for (url, data, captures) in cases.iter().skip(5) {
        dbg!((url, data));
        let ret = router.find(url).unwrap();
        assert_eq!(ret.0, data);
        let v: Vec<(&str, &str)> = ret.1.collect();
        assert_eq!(&v, captures);
    }
}

#[test]
fn test_collision() {
    let mut router: Router<usize> = Router::new();
    router.try_insert("/u/:id/p/:id", 1).unwrap();
    router.try_insert("/u/:uid/p/:pid", 2).unwrap_err();

    let mut router: Router<usize> = Router::new();
    router.try_insert("/application/c/:a", 1).unwrap();
    router.try_insert("/application/b", 2).unwrap();
    router.try_insert("/application/b/:id", 3).unwrap();

    let mut router: Router<usize> = Router::new();
    router.try_insert("/application/**", 1).unwrap();
    router
        .try_nested("/application", |r| {
            r.insert("**", 2);
        })
        .unwrap_err();
}
