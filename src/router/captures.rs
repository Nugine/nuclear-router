#![forbid(unsafe_code)]

use std::fmt::{self, Debug};
use std::ops::Deref;
use std::str::FromStr;

use smallvec::SmallVec;

pub struct Captures<'a> {
    path: &'a str,
    buf: SmallVec<[(&'a str, &'a str); 8]>,
}

impl Captures<'_> {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.buf
            .iter()
            .find_map(|&(k, v)| if name == k { Some(v) } else { None })
    }

    pub fn parse<T: FromStr>(&self, name: &str) -> Option<Result<T, T::Err>> {
        self.get(name).map(T::from_str)
    }
}

impl<'a> Deref for Captures<'a> {
    type Target = [(&'a str, &'a str)];
    fn deref(&self) -> &Self::Target {
        &*self.buf
    }
}

impl<'a> Captures<'a> {
    pub(super) fn new(path: &'a str) -> Self {
        Self {
            path,
            buf: SmallVec::new(),
        }
    }

    #[inline(always)]
    pub(crate) fn buffer(&mut self) -> &mut SmallVec<[(&'a str, &'a str); 8]> {
        &mut self.buf
    }

    #[inline(always)]
    pub(crate) fn path(&self)->&'a str{
        self.path
    }
}

impl Debug for Captures<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Captures")
            .field("path", &self.path)
            .field("buf", &self.buf.as_slice())
            .finish()
    }
}
