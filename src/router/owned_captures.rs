use crate::router::Captures;

use std::iter::{DoubleEndedIterator, ExactSizeIterator, FusedIterator};
use std::str::FromStr;

#[derive(Debug)]
pub struct OwnedCaptures {
    path: Option<Box<str>>,
    offset: Vec<(Box<str>, usize, usize)>, // (name, start, end)
}

impl OwnedCaptures {
    pub fn empty() -> Self {
        Self {
            path: None,
            offset: Vec::new(),
        }
    }

    pub fn new(caps: &Captures<'_>) -> Self {
        let mut offset: Vec<(Box<str>, usize, usize)> = Vec::with_capacity(caps.len());
        let base = caps.path().as_ptr() as usize;
        offset.extend(caps.iter().map(|&(name, value)| {
            let name = name.into();
            let start = (value.as_ptr() as usize) - base;
            let end = start + value.len();
            (name, start, end)
        }));
        let path = some_if(!offset.is_empty(), || caps.path().into());
        Self { path, offset }
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        let path = self.path.as_ref()?;
        self.offset
            .iter()
            .find_map(|&(ref n, s, e)| some_if(&**n == name, || &path[s..e]))
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

#[inline(always)]
fn some_if<T>(cond: bool, f: impl FnOnce() -> T) -> Option<T> {
    if cond {
        Some(f())
    } else {
        None
    }
}

impl IntoIterator for OwnedCaptures {
    type Item = (String, String);
    type IntoIter = IntoIter;
    fn into_iter(self) -> IntoIter {
        IntoIter {
            path: self.path,
            offset: self.offset.into_iter(),
        }
    }
}

#[derive(Debug)]
pub struct Iter<'a> {
    path: Option<&'a str>,
    offset: std::slice::Iter<'a, (Box<str>, usize, usize)>,
}

#[derive(Debug)]
pub struct IntoIter {
    path: Option<Box<str>>,
    offset: std::vec::IntoIter<(Box<str>, usize, usize)>,
}

macro_rules! delegate {
    (iter,$method:tt) => {
        fn $method(&mut self) -> Option<Self::Item> {
            let &(ref n, s, e) = self.offset.$method()?;
            let path = self.path.unwrap();
            Some((&**n, &path[s..e]))
        }
    };

    (into_iter, $method:tt) => {
        fn $method(&mut self) -> Option<Self::Item> {
            let (n, s, e) = self.offset.$method()?;
            let path = self.path.as_ref().unwrap();
            Some((n.into(), path[s..e].to_owned()))
        }
    };

    (size_hint) => {
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.offset.size_hint()
        }
    };

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
