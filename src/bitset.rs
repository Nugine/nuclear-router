#![allow(unsafe_code)]

use std::{mem, slice};

pub unsafe trait BitStorage: Sized {
    fn bit_size() -> usize {
        mem::size_of::<Self>() * 8
    }

    fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self as *const _ as *const u8, Self::bit_size() / 8) }
    }
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self as *mut _ as *mut u8, Self::bit_size() / 8) }
    }
}

unsafe impl BitStorage for [u128; 4] {}
unsafe impl BitStorage for u128 {}

#[derive(Debug, Clone)]
pub struct FixedBitSet<S: BitStorage> {
    buf: S,
}

impl<S: BitStorage> FixedBitSet<S> {
    pub fn zero() -> Self {
        Self {
            buf: unsafe { mem::zeroed() },
        }
    }

    pub fn one() -> Self {
        let mut set = Self {
            buf: unsafe { mem::MaybeUninit::uninit().assume_init() },
        };
        set.buf
            .as_bytes_mut()
            .iter_mut()
            .for_each(|x| *x = u8::max_value());
        set
    }

    pub fn intersect_with(&mut self, other: &Self) {
        self.buf
            .as_bytes_mut()
            .iter_mut()
            .zip(other.buf.as_bytes().iter())
            .for_each(|(lhs, rhs)| *lhs &= rhs)
    }

    pub fn get_inner(&self) -> &S {
        &self.buf
    }

    pub fn set(&mut self, index: usize, bit: bool) {
        let idx = index / 8;
        let offset: u8 = (index % 8) as _;
        let mask = (bit as u8) << offset;
        let bytes = self.buf.as_bytes_mut();
        let pos: &mut u8 = match bytes.get_mut(idx) {
            Some(pos) => pos,
            None => panic!(
                "bitset index out of bound: index = {}, bound = {}",
                index,
                S::bit_size()
            ),
        };
        *pos |= mask
    }

    pub fn is_zero(&self) -> bool {
        self.buf.as_bytes().iter().all(|&x| x == 0)
    }

    pub fn iter_ones(&self) -> impl Iterator<Item = usize> + '_ {
        self.buf
            .as_bytes()
            .iter()
            .flat_map(|&x| crate::table::TABLE[x as usize])
            .cloned()
    }
}
