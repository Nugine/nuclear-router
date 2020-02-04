#![forbid(unsafe_code)]

use std::fmt::{self, Debug};
use std::ops::Deref;
use std::str::FromStr;

use smallvec::SmallVec;

pub struct Captures<'a> {
    pub(super) buf: SmallVec<[(&'a str, &'a str); 8]>,
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

impl Captures<'_> {
    pub(super) fn new() -> Self {
        Self {
            buf: SmallVec::new(),
        }
    }
}

impl Debug for Captures<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as Deref>::Target::fmt(self, f)
    }
}
