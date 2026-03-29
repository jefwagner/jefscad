#![feature(portable_simd)]
#![feature(macro_metavar_expr)]

use core::ops::Range;
use std::simd::prelude::*;

// Module for getting next representable floating point value
mod next_up_down;
// use next_up_down::NextUpDown;
// Module for printing flints
mod fmt;
// Module for creating flints from other types
mod conv;
// Module for comparisons
mod cmp;
// Module for arithmetic operators
mod arith;

pub fn add<const N: usize>(a: Simd<f32, N>, b: Simd<f32, N>) -> Simd<f32, N> {
    a + b
}

pub fn uiadd<const N: usize>(a: Simd<u8, N>, b: Simd<i8, N>) -> Simd<u8, N> {
    let a: Simd<i8, N> = a.cast::<i8>();
    (a + b).cast::<u8>()
}

/// A Rounded floating point interval or Flint type
///
/// This type is generic on the underlying float type: f32 or f64
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Flint<T> {
    lb: T,
    ub: T,
}

/// A Rounded floating point interval
///
/// This type has the interval endpoints as references and can act as a single
/// 'scalar' value reference from a FlintArray or FlintVec.
#[repr(C)]
#[derive(Debug)]
pub struct FlintRef<'a, T> {
    lb: &'a T,
    ub: &'a T,
}

/// A mutable reference to a Flint
#[repr(C)]
#[derive(Debug)]
pub struct FlintMut<'a, T> {
    lb: &'a mut T,
    ub: &'a mut T,
}

/// An owned array of rounded floating point intervals
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FlintArray<T, const N: usize> {
    lb: [T; N],
    ub: [T; N],
}

/// An owned dynamically sized vector of floating point intervals
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FlintVec<T> {
    lb: Vec<T>,
    ub: Vec<T>,
}

/// A dynamically sized view (slice) of floating point intervals
///
/// The lower and upper bound slices may be borrowed from a FlintArray or FlintVec.
#[repr(C)]
#[derive(Debug)]
pub struct FlintView<'a, T> {
    lb: &'a [T],
    ub: &'a [T],
}

/// A mutable slice of a floating point intervals
#[repr(C)]
#[derive(Debug)]
pub struct FlintViewMut<'a, T> {
    lb: &'a mut [T],
    ub: &'a mut [T],
}

impl<'a, T> Flint<T> {
    /// Get a reference to the Flint object
    pub fn as_ref(&'a self) -> FlintRef<'a, T> {
        FlintRef {
            lb: &self.lb,
            ub: &self.ub,
        }
    }

    /// Get a mutable reference to the flint object
    pub fn as_mut(&'a mut self) -> FlintMut<'a, T> {
        FlintMut {
            lb: &mut self.lb,
            ub: &mut self.ub,
        }
    }
}

impl<'a, T> FlintRef<'a, T>
where
    T: Copy,
{
    /// Turn a reference into a new flint
    pub fn to_owned(&self) -> Flint<T> {
        Flint {
            lb: *self.lb,
            ub: *self.ub,
        }
    }
}

impl<'a, T> FlintMut<'a, T>
where
    T: Copy,
{
    pub fn to_owned(&self) -> Flint<T> {
        Flint {
            lb: *self.lb,
            ub: *self.ub,
        }
    }
}

pub trait FlintSoA<T> {
    fn parts(&self) -> (&[T], &[T]);

    fn get(&self, i: usize) -> Option<FlintRef<'_, T>> {
        let (lb, ub) = self.parts();
        Some(FlintRef {
            lb: lb.get(i)?,
            ub: ub.get(i)?,
        })
    }

    fn slice(&self, r: core::ops::Range<usize>) -> FlintView<'_, T> {
        let (lb, ub) = self.parts();
        FlintView {
            lb: &lb[r.clone()],
            ub: &ub[r],
        }
    }
}

pub trait FlintSoAMut<T> {
    fn parts_mut(&mut self) -> (&mut [T], &mut [T]);

    fn get_mut(&mut self, i: usize) -> Option<FlintMut<'_, T>> {
        let (lb, ub) = self.parts_mut();
        Some(FlintMut {
            lb: lb.get_mut(i)?,
            ub: ub.get_mut(i)?,
        })
    }

    fn slice_mut(&mut self, r: core::ops::Range<usize>) -> FlintViewMut<'_, T> {
        let (lb, ub) = self.parts_mut();
        FlintViewMut {
            lb: &mut lb[r.clone()],
            ub: &mut ub[r],
        }
    }
}

impl<T, const N: usize> FlintSoA<T> for FlintArray<T, N> {
    fn parts(&self) -> (&[T], &[T]) {
        (&self.lb[..], &self.ub[..])
    }
}
impl<T, const N: usize> FlintSoAMut<T> for FlintArray<T, N> {
    fn parts_mut(&mut self) -> (&mut [T], &mut [T]) {
        (&mut self.lb[..], &mut self.ub[..])
    }
}

impl<T> FlintSoA<T> for FlintVec<T> {
    fn parts(&self) -> (&[T], &[T]) {
        (&self.lb[..], &self.ub[..])
    }
}
impl<T> FlintSoAMut<T> for FlintVec<T> {
    fn parts_mut(&mut self) -> (&mut [T], &mut [T]) {
        (&mut self.lb[..], &mut self.ub[..])
    }
}

impl<'a, T> FlintSoA<T> for FlintView<'a, T> {
    fn parts(&self) -> (&[T], &[T]) {
        (self.lb, self.ub)
    }
}

impl<'a, T> FlintSoA<T> for FlintViewMut<'a, T> {
    fn parts(&self) -> (&[T], &[T]) {
        (self.lb, self.ub)
    }
}
impl<'a, T> FlintSoAMut<T> for FlintViewMut<'a, T> {
    fn parts_mut(&mut self) -> (&mut [T], &mut [T]) {
        (self.lb, self.ub)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simd_it_works() {
        let a = f32x4::splat(10.0);
        let b = f32x4::from_array([1.0, 2.0, 3.0, 4.0]);
        let result = add(a, b);
        eprintln!("{result:?}");
        assert_eq!(result, f32x4::from_array([11.0, 12.0, 13.0, 14.0]));
    }

    #[test]
    fn test_add_sub_uint() {
        let a = u8x8::from_array([128, 128, 0, 0, 240, 240, 240, 240]);
        let b = i8x8::from_array([-1, 0, 1, 0, -1, 0, 1, 0]);
        let result = uiadd(a, b);
        eprintln!("{result:?}");
        assert_eq!(
            result,
            u8x8::from_array([127, 128, 1, 0, 239, 240, 241, 240])
        );
    }
}
