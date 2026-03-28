#![feature(portable_simd)]
#![feature(macro_metavar_expr)]

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

impl<'a, T> Flint<T> {
    /// Get a reference to the Flint object
    pub fn as_ref(&'a self) -> FlintRef<'a, T> {
        FlintRef {
            lb: &self.lb,
            ub: &self.ub,
        }
    }

    /// Get a mutable reference to the flint object
    pub fn as_mut(&'a mut self) -> FlintRef<'a, T> {
        FlintRef {
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
