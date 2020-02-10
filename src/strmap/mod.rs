#![allow(unsafe_code)]

use std::cmp::Ordering;

#[derive(Debug, Default)]
pub struct StrMap<T> {
    keys: Vec<Box<[u8]>>,
    values: Vec<T>,
}

impl<T> StrMap<T> {
    pub fn new() -> Self {
        Self {
            keys: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn find(&self, key: &str) -> Option<&T> {
        match self.find_index(key.as_bytes()) {
            Ok(i) => Some(unsafe { self.values.get_unchecked(i) }),
            Err(_) => None,
        }
    }

    pub fn find_mut_with(&mut self, key: &str, f: impl FnOnce() -> T) -> &mut T {
        let i = match self.find_index(key.as_bytes()) {
            Ok(i) => i,
            Err(i) => {
                let val = f();
                self.values.insert(i, val);
                self.keys.insert(i, key.as_bytes().into());
                i
            }
        };
        unsafe { self.values.get_unchecked_mut(i) }
    }

    fn find_index(&self, key: &[u8]) -> Result<usize, usize> {
        let keys: &[Box<[u8]>] = &self.keys;

        let mut l: usize = 0;
        let mut r: usize = keys.len();

        while l < r {
            let mid = l + (r - l) / 2;
            let m = unsafe { &**keys.get_unchecked(mid) };
            match m.cmp(key) {
                Ordering::Less => l = mid + 1,
                Ordering::Equal => return Ok(mid),
                Ordering::Greater => r = mid,
            }
        }
        let target: &[u8] = match keys.get(l) {
            Some(t) => &**t,
            None => return Err(keys.len()),
        };
        match target.cmp(key) {
            Ordering::Less => Err(l + 1),
            Ordering::Equal => Ok(l),
            Ordering::Greater => Err(l),
        }
    }
}
