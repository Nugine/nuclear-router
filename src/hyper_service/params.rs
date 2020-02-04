use crate::router::Captures;

use std::iter::{DoubleEndedIterator, ExactSizeIterator, FusedIterator};
use std::str::FromStr;

pub struct Params {
    path: Option<String>,
    offset: Vec<(String, usize, usize)>, // (name, start, end)
}

impl Params {
    pub fn get(&self, name: &str) -> Option<&str> {
        let path = self.path.as_ref()?;
        self.offset
            .iter()
            .find_map(|&(ref n, s, e)| some_if(n == name, || &path[s..e]))
    }

    pub fn parse<T: FromStr>(&self, name: &str) -> Option<Result<T, T::Err>> {
        self.get(name).map(T::from_str)
    }

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            path: self.path.as_deref(),
            offset: self.offset.iter(),
        }
    }
}

impl IntoIterator for Params {
    type Item = (String, String);
    type IntoIter = IntoIter;
    fn into_iter(self) -> IntoIter {
        IntoIter {
            path: self.path,
            offset: self.offset.into_iter(),
        }
    }
}

impl Params {
    pub(super) fn empty() -> Self {
        Self {
            path: None,
            offset: Vec::new(),
        }
    }

    pub(super) fn new<'a>(path: &'a str, caps: &Captures<'a>) -> Self {
        let mut offset: Vec<(String, usize, usize)> = Vec::with_capacity(caps.len());
        let base = path.as_ptr() as usize;
        offset.extend(caps.iter().map(|&(name, value)| {
            let name = name.to_owned();
            let start = (value.as_ptr() as usize) - base;
            let end = start + value.len();
            (name, start, end)
        }));
        let path = some_if(!offset.is_empty(), || path.to_owned());
        Self { path, offset }
    }
}

#[inline(always)]
fn some_if<T>(cond: bool, f: impl FnOnce() -> T) -> Option<T> {
    if cond {
        Some(f())
    } else {
        None
    }
}

pub struct Iter<'a> {
    path: Option<&'a str>,
    offset: std::slice::Iter<'a, (String, usize, usize)>,
}

pub struct IntoIter {
    path: Option<String>,
    offset: std::vec::IntoIter<(String, usize, usize)>,
}

macro_rules! delegate {
    (iter,$method:tt) => {
        fn $method(&mut self) -> Option<Self::Item> {
            let &(ref n, s, e) = self.offset.$method()?;
            let path = self.path.unwrap();
            Some((n.as_str(), &path[s..e]))
        }
    };

    (into_iter, $method:tt) => {
        fn $method(&mut self) -> Option<Self::Item> {
            let (n, s, e) = self.offset.$method()?;
            let path = self.path.as_ref().unwrap();
            Some((n, path[s..e].to_owned()))
        }
    };

    (size_hint ) => {
        fn size_hint(&self) -> (usize, Option<usize>) {
        self.offset.size_hint()
    }};

    (len) => {
        fn len(&self) -> usize {
            self.offset.len()
        }
    };
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a str, &'a str);
    delegate!(iter, next);
    delegate!(size_hint);
}

impl DoubleEndedIterator for Iter<'_> {
    delegate!(iter, next_back);
}

impl Iterator for IntoIter {
    type Item = (String, String);
    delegate!(into_iter, next);
    delegate!(size_hint);
}

impl DoubleEndedIterator for IntoIter {
    delegate!(into_iter, next_back);
}

impl FusedIterator for Iter<'_> {}
impl FusedIterator for IntoIter {}

impl ExactSizeIterator for Iter<'_> {
    delegate!(len);
}

impl ExactSizeIterator for IntoIter {
    delegate!(len);
}
