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

    pub fn find<'a>(
        &'a self,
        path: &'a str,
    ) -> Option<(&'a T, impl Iterator<Item = (&'a str, &'a str)>)> {
        let mut captures: SmallVec<[(&'a str, &'a str); 8]> = SmallVec::new();
        let parts: SmallVec<[&str; 8]> = trim_fisrt_slash(path).split('/').collect();
        let data: &T = self.find_with_buf(path, &parts, &mut captures)?;
        Some((data, captures.into_iter()))
    }

    pub fn insert(&mut self, pattern: &str, data: T) {
        self.insert_endpoint(pattern, Either::A(data))
    }

    pub fn insert_regex(&mut self, pattern: Regex, data: T) {
        self.regexps.push((pattern, data))
    }

    pub fn nested(&mut self, prefix: &str, f: impl FnOnce(&mut Router<T>)) {
        let mut router = Self::new();
        f(&mut router);
        self.insert_endpoint(prefix, Either::B(router))
    }
}

impl<T> Router<T> {
    fn insert_endpoint(&mut self, pattern: &str, endpoint: Either<T, Router<T>>) {
        assert!(self.routes.len() < 512, "can not hold more than 512 routes");

        let mut parts: SmallVec<[&str; 8]> = trim_fisrt_slash(pattern).split('/').collect();

        let catch_all = match &endpoint {
            Either::A(_) => some_if(*parts.last().unwrap() == "**", || {
                parts.pop();
                parts.len()
            }),
            Either::B(_) => {
                assert!(
                    *parts.last().unwrap() != "**",
                    "\"**\" can not be used for router prefix: {:?}",
                    pattern
                );
                Some(parts.len())
            }
        };

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

        let mut captures: Vec<(String, usize)> = Vec::new();
        let id = self.routes.len();

        for (i, &part) in parts.iter().enumerate() {
            if part == "**" {
                panic!("\"**\" can only appear at end: {:?}", pattern);
            }
            if part.starts_with(':') {
                let name: &str = &part[1..];
                if name == "**" {
                    panic!("\"**\" can not be used for capture name: {:?}", pattern);
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
    router.nested("/user/:user_id", |user| {
        user.insert("post/:post_id", 1);
        user.insert("profile", 2);
        user.insert("file/**", 3);
        user.insert("", 4);
    });

    router.insert("explore", 5);
    router.insert("pan/**", 6);
    router.nested("pan", |pan| {
        pan.insert_regex(Regex::new(".*/(?P<name>.+)\\.php$").unwrap(), 7);
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

    for (url, data, captures) in cases.iter() {
        let ret = router.find(url).unwrap();
        assert_eq!(ret.0, data);
        let v: Vec<(&str, &str)> = ret.1.collect();
        assert_eq!(&v, captures);
    }
}
