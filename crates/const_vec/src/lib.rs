#![no_std]
#![feature(const_type_name)]

use const_panic::concat_panic;
use core::{
    any::type_name,
    fmt::{Debug, Formatter, Result},
    mem::MaybeUninit,
    ops::{Index, IndexMut},
    slice::from_raw_parts,
};

#[derive(Clone, Copy)]
pub struct ConstVec<T: Copy, const N: usize> {
    buffer: [MaybeUninit<T>; N],
    len: usize,
}

impl<T: Copy, const N: usize> ConstVec<T, N> {
    pub const fn new() -> Self {
        Self {
            buffer: [MaybeUninit::uninit(); N],
            len: 0,
        }
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    #[track_caller]
    pub const fn push(&mut self, item: T) -> &mut Self {
        let capacity = const { N };
        if self.len >= capacity {
            let type_name = type_name::<T>();
            concat_panic!(
                "\nReached ConstVec<",
                type_name,
                ", ",
                capacity,
                "> capacity"
            );
        }
        self.buffer[self.len] = MaybeUninit::new(item);
        self.len += 1;
        self
    }

    #[track_caller]
    pub const fn extend<const N2: usize>(&mut self, vec: ConstVec<T, N2>) -> &mut Self {
        let capacity = const { N };
        if self.len + vec.len > capacity {
            let type_name = type_name::<T>();
            concat_panic!(
                "\nCannot extend ",
                vec.len,
                " items into a ConstVec<",
                type_name,
                ", ",
                capacity,
                "> with ",
                self.len,
                " items, since that would exceed capacity"
            );
        }

        let mut i = 0;
        while i < vec.len {
            self.buffer[self.len + i] = vec.buffer[i];
            i += 1;
        }
        self.len += vec.len;
        self
    }

    #[track_caller]
    pub const fn from_slice(slice: &[T]) -> Self {
        let mut vec = Self::new();
        let capacity = const { N };

        if slice.len() > capacity {
            let type_name = type_name::<T>();
            concat_panic!(
                "\nSlice length exceeds ConstVec<",
                type_name,
                ", ",
                capacity,
                "> capacity"
            );
        }

        let mut i = 0;
        while i < slice.len() {
            vec.buffer[i] = MaybeUninit::new(slice[i]);
            i += 1;
        }
        vec.len = slice.len();
        vec
    }

    pub const fn into_slice(&self) -> &[T] {
        // SAFETY: We are certain that all items in the buffer up to length are initialized
        unsafe {
            let ptr = self.buffer.as_ptr() as *const T;
            from_raw_parts(ptr, self.len)
        }
    }

    pub const fn iter(&self) -> ConstVecIter<'_, T> {
        ConstVecIter {
            next: 0,
            slice: self.into_slice(),
        }
    }
}

pub struct ConstVecIter<'a, T> {
    next: usize,
    slice: &'a [T],
}

impl<'a, T> Iterator for ConstVecIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.slice.get(self.next);
        self.next += 1;
        current
    }
}

impl<T: Copy + Debug, const N: usize> Debug for ConstVec<T, N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T: Copy, const N: usize> Index<usize> for ConstVec<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.len {
            panic!("Index out of bounds");
        }
        // SAFETY: We are certain that all items in the buffer up to length are initialized
        unsafe { &*self.buffer[index].as_ptr() }
    }
}

impl<T: Copy, const N: usize> IndexMut<usize> for ConstVec<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.len {
            panic!("Index out of bounds");
        }
        // SAFETY: We are certain that all items in the buffer up to length are initialized
        unsafe { &mut *self.buffer[index].as_mut_ptr() }
    }
}

impl<T: Copy, const N: usize> Default for ConstVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    extern crate std;
    use std::panic::catch_unwind;

    #[test]
    fn into_slice() {
        let mut vec = ConstVec::<u8, 128>::from_slice(&[1, 2, 3]);
        assert!(vec.len() == 3);

        vec.push(4);
        assert_eq!(vec.len(), 4);
        assert_eq!(vec.into_slice(), &[1, 2, 3, 4]);
    }

    #[test]
    fn from_slice_panic() {
        let slice = &[1, 2, 3, 4, 5];
        let result = catch_unwind(|| {
            ConstVec::<u32, 4>::from_slice(slice);
        });
        assert!(result.is_err());
    }

    #[test]
    fn length_panic() {
        let mut vec = ConstVec::<u32, 2>::new();

        vec.push(1);
        vec.push(2);

        let result = catch_unwind(|| {
            let mut vec = vec;
            vec.push(3);
        });
        assert!(result.is_err());
    }

    #[test]
    fn append() {
        let mut vec = ConstVec::<u32, 4>::from_slice(&[1, 2]);
        vec.extend(ConstVec::<u32, 10>::from_slice(&[3, 4]));
        assert_eq!(vec.into_slice(), &[1, 2, 3, 4]);
    }

    #[test]
    fn append_panic() {
        let vec1 = ConstVec::<u32, 4>::from_slice(&[1, 2]);
        let vec2 = ConstVec::<u32, 3>::from_slice(&[3, 4, 5]);

        let result = catch_unwind(|| {
            let mut vec1 = vec1;
            vec1.extend(vec2);
        });
        assert!(result.is_err());
    }

    #[test]
    fn get_index() {
        let vec = ConstVec::<u32, 4>::from_slice(&[1, 2, 3, 4]);
        assert_eq!(vec[0], 1);
        assert_eq!(vec[1], 2);
        assert_eq!(vec[2], 3);
        assert_eq!(vec[3], 4);
    }

    #[test]
    fn get_index_panic() {
        let vec = ConstVec::<u32, 4>::from_slice(&[1, 2, 3, 4]);
        let result = catch_unwind(|| {
            let _ = vec[4];
        });
        assert!(result.is_err());
    }

    #[test]
    fn set_index() {
        let mut vec = ConstVec::<u32, 4>::from_slice(&[1, 2, 3, 4]);
        vec[0] = 5;
        vec[1] = 6;
        vec[2] = 7;
        vec[3] = 8;

        assert_eq!(vec.into_slice(), &[5, 6, 7, 8]);
    }

    #[test]
    fn set_index_panic() {
        let vec = ConstVec::<u32, 4>::from_slice(&[1, 2, 3, 4]);
        let result = catch_unwind(|| {
            let mut vec = vec;
            vec[4] = 5;
        });
        assert!(result.is_err());
    }
}
