#![allow(unsafe_code)]

use super::table::TABLE;
use std::{mem, slice};

pub unsafe trait BitStorage: Sized {
    fn bit_size() -> usize {
        mem::size_of::<Self>() * 8
    }

    fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self as *const _ as *const u8, mem::size_of::<Self>()) }
    }
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self as *mut _ as *mut u8, mem::size_of::<Self>()) }
    }
}

unsafe impl BitStorage for u128 {}
unsafe impl BitStorage for u64 {}
unsafe impl BitStorage for u32 {}
unsafe impl BitStorage for u16 {}
unsafe impl BitStorage for u8 {}

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
}

impl<S: BitStorage> FixedBitSet<S> {
    pub fn intersect_with(&mut self, other: &Self) {
        self.buf
            .as_bytes_mut()
            .iter_mut()
            .zip(other.buf.as_bytes().iter())
            .for_each(|(lhs, rhs)| *lhs &= rhs)
    }

    pub fn union_with(&mut self, other: &Self) {
        self.buf
            .as_bytes_mut()
            .iter_mut()
            .zip(other.buf.as_bytes().iter())
            .for_each(|(lhs, rhs)| *lhs |= rhs)
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

    pub fn iter_ones(&self) -> impl Iterator<Item = usize> + '_ {
        self.buf
            .as_bytes()
            .iter()
            .enumerate()
            .flat_map(|(i, &x)| TABLE[x as usize].iter().map(move |&j| i * 8 + j))
    }
}
