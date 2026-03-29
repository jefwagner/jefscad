use num_traits::Float;
use std::cmp::{Ordering, PartialEq, PartialOrd};
use std::simd::prelude::*;

use crate::{Flint, FlintArray, FlintMut, FlintRef, FlintVec, FlintView, FlintViewMut};

// ----------------------
// Copmarisons for Flints
// ----------------------

impl<T, Rhs> PartialEq<Rhs> for Flint<T>
where
    T: Copy + PartialEq + PartialOrd,
    Rhs: Copy + Into<Flint<T>>,
{
    fn eq(&self, other: &Rhs) -> bool {
        let other: Flint<T> = (*other).into();
        self.ub >= other.lb && self.lb <= other.ub
    }
}

impl<T, Rhs> PartialOrd<Rhs> for Flint<T>
where
    T: Copy + PartialEq + PartialOrd + Float,
    Rhs: Copy + Into<Flint<T>>,
{
    fn partial_cmp(&self, other: &Rhs) -> Option<Ordering> {
        let other: Flint<T> = (*other).into();
        if self.lb.is_nan() || self.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
            Option::None
        } else if self.lb > other.ub {
            Some(Ordering::Greater)
        } else if self.ub < other.lb {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

// -------------------------
// Comparisons for FlintRefs
// -------------------------

impl<'a, T> PartialEq<FlintRef<'a, T>> for FlintRef<'a, T>
where
    T: Copy + PartialEq + PartialOrd,
{
    fn eq(&self, other: &FlintRef<T>) -> bool {
        *self.ub >= *other.lb && *self.lb <= *other.ub
    }
}

impl<'a, T> PartialOrd<FlintRef<'a, T>> for FlintRef<'a, T>
where
    T: Copy + PartialEq + PartialOrd + Float,
{
    fn partial_cmp(&self, other: &FlintRef<T>) -> Option<Ordering> {
        if self.lb.is_nan() || self.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
            None
        } else if *self.lb > *other.ub {
            Some(Ordering::Greater)
        } else if *self.ub < *other.lb {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

impl<'a, T, Rhs> PartialEq<Rhs> for FlintRef<'a, T>
where
    T: Copy + PartialEq + PartialOrd,
    Rhs: Copy + Into<Flint<T>>,
{
    fn eq(&self, other: &Rhs) -> bool {
        let other: Flint<T> = (*other).into();
        *self.ub >= other.lb && *self.lb <= other.ub
    }
}

impl<'a, T, Rhs> PartialOrd<Rhs> for FlintRef<'a, T>
where
    T: Copy + PartialEq + PartialOrd + Float,
    Rhs: Copy + Into<Flint<T>>,
{
    fn partial_cmp(&self, other: &Rhs) -> Option<Ordering> {
        let other: Flint<T> = (*other).into();
        if self.lb.is_nan() || self.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
            None
        } else if *self.lb > other.ub {
            Some(Ordering::Greater)
        } else if *self.ub < other.lb {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

// -------------------------
// Comparisons for FlintMuts
// -------------------------

impl<'a, T> PartialEq<FlintMut<'a, T>> for FlintMut<'a, T>
where
    T: Copy + PartialEq + PartialOrd,
{
    fn eq(&self, other: &FlintMut<T>) -> bool {
        *self.ub >= *other.lb && *self.lb <= *other.ub
    }
}

impl<'a, T> PartialOrd<FlintMut<'a, T>> for FlintMut<'a, T>
where
    T: Copy + PartialEq + PartialOrd + Float,
{
    fn partial_cmp(&self, other: &FlintMut<T>) -> Option<Ordering> {
        if self.lb.is_nan() || self.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
            None
        } else if *self.lb > *other.ub {
            Some(Ordering::Greater)
        } else if *self.ub < *other.lb {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

impl<'a, T, Rhs> PartialEq<Rhs> for FlintMut<'a, T>
where
    T: Copy + PartialEq + PartialOrd,
    Rhs: Copy + Into<Flint<T>>,
{
    fn eq(&self, other: &Rhs) -> bool {
        let other: Flint<T> = (*other).into();
        *self.ub >= other.lb && *self.lb <= other.ub
    }
}

impl<'a, T, Rhs> PartialOrd<Rhs> for FlintMut<'a, T>
where
    T: Copy + PartialEq + PartialOrd + Float,
    Rhs: Copy + Into<Flint<T>>,
{
    fn partial_cmp(&self, other: &Rhs) -> Option<Ordering> {
        let other: Flint<T> = (*other).into();
        if self.lb.is_nan() || self.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
            None
        } else if *self.lb > other.ub {
            Some(Ordering::Greater)
        } else if *self.ub < other.lb {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

// ---------------------------------------------------------
// Comparison between Flint and FlintRefs of same base types
// ---------------------------------------------------------

impl<'a, T> PartialEq<FlintRef<'a, T>> for Flint<T>
where
    T: Copy + PartialEq + PartialOrd,
{
    fn eq(&self, other: &FlintRef<T>) -> bool {
        self.ub >= *other.lb && self.lb <= *other.ub
    }
}

impl<'a, T> PartialOrd<FlintRef<'a, T>> for Flint<T>
where
    T: Copy + PartialEq + PartialOrd + Float,
{
    fn partial_cmp(&self, other: &FlintRef<T>) -> Option<Ordering> {
        if self.lb.is_nan() || self.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
            None
        } else if self.lb > *other.ub {
            Some(Ordering::Greater)
        } else if self.ub < *other.lb {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

// ---------------------------------------------------------
// Comparison between Flint and FlintMut of same base type
// (FlintMut is not Copy so the generic Rhs impl on Flint doesn't cover it)
// ---------------------------------------------------------

impl<'a, T> PartialEq<FlintMut<'a, T>> for Flint<T>
where
    T: Copy + PartialEq + PartialOrd,
{
    fn eq(&self, other: &FlintMut<T>) -> bool {
        self.ub >= *other.lb && self.lb <= *other.ub
    }
}

impl<'a, T> PartialOrd<FlintMut<'a, T>> for Flint<T>
where
    T: Copy + PartialEq + PartialOrd + Float,
{
    fn partial_cmp(&self, other: &FlintMut<T>) -> Option<Ordering> {
        if self.lb.is_nan() || self.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
            None
        } else if self.lb > *other.ub {
            Some(Ordering::Greater)
        } else if self.ub < *other.lb {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

// ----------------------------------------------------------
// Comparison between FlintRef and FlintMut of same base type
// (FlintMut is not Copy so the generic Rhs impl on FlintRef doesn't cover it)
// ----------------------------------------------------------

impl<'a, 'b, T> PartialEq<FlintMut<'b, T>> for FlintRef<'a, T>
where
    T: Copy + PartialEq + PartialOrd,
{
    fn eq(&self, other: &FlintMut<T>) -> bool {
        *self.ub >= *other.lb && *self.lb <= *other.ub
    }
}

impl<'a, 'b, T> PartialOrd<FlintMut<'b, T>> for FlintRef<'a, T>
where
    T: Copy + PartialEq + PartialOrd + Float,
{
    fn partial_cmp(&self, other: &FlintMut<T>) -> Option<Ordering> {
        if self.lb.is_nan() || self.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
            None
        } else if *self.lb > *other.ub {
            Some(Ordering::Greater)
        } else if *self.ub < *other.lb {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}
// FlintRef does not derive Copy, so the generic Rhs: Copy + Into<Flint<T>> impl
// on FlintMut does not cover it — explicit impls are needed.

impl<'a, 'b, T> PartialEq<FlintRef<'b, T>> for FlintMut<'a, T>
where
    T: Copy + PartialEq + PartialOrd,
{
    fn eq(&self, other: &FlintRef<T>) -> bool {
        *self.ub >= *other.lb && *self.lb <= *other.ub
    }
}

impl<'a, 'b, T> PartialOrd<FlintRef<'b, T>> for FlintMut<'a, T>
where
    T: Copy + PartialEq + PartialOrd + Float,
{
    fn partial_cmp(&self, other: &FlintRef<T>) -> Option<Ordering> {
        if self.lb.is_nan() || self.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
            None
        } else if *self.lb > *other.ub {
            Some(Ordering::Greater)
        } else if *self.ub < *other.lb {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

// --------------------------------------------------------------
// Comparisons between Flint and FlintRefs of differnt base types
// --------------------------------------------------------------

impl<'a> PartialEq<FlintRef<'a, f32>> for Flint<f64> {
    fn eq(&self, other: &FlintRef<f32>) -> bool {
        self.ub >= (*other.lb as f64) && self.lb <= (*other.ub as f64)
    }
}

impl<'a> PartialOrd<FlintRef<'a, f32>> for Flint<f64> {
    fn partial_cmp(&self, other: &FlintRef<f32>) -> Option<Ordering> {
        if self.lb.is_nan() || self.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
            None
        } else if self.lb > (*other.ub as f64) {
            Some(Ordering::Greater)
        } else if self.ub < (*other.lb as f64) {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

impl<'a> PartialEq<FlintRef<'a, f64>> for Flint<f32> {
    fn eq(&self, other: &FlintRef<f64>) -> bool {
        (self.ub as f64) >= *other.lb && (self.lb as f64) <= *other.ub
    }
}

impl<'a> PartialOrd<FlintRef<'a, f64>> for Flint<f32> {
    fn partial_cmp(&self, other: &FlintRef<f64>) -> Option<Ordering> {
        if self.lb.is_nan() || self.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
            None
        } else if (self.lb as f64) > *other.ub {
            Some(Ordering::Greater)
        } else if (self.ub as f64) < *other.lb {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

impl<'a> PartialEq<Flint<f64>> for FlintRef<'a, f32> {
    fn eq(&self, other: &Flint<f64>) -> bool {
        (*self.ub as f64) >= other.lb && (*self.lb as f64) <= other.ub
    }
}

impl<'a> PartialOrd<Flint<f64>> for FlintRef<'a, f32> {
    fn partial_cmp(&self, other: &Flint<f64>) -> Option<Ordering> {
        if self.lb.is_nan() || self.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
            None
        } else if (*self.lb as f64) > other.ub {
            Some(Ordering::Greater)
        } else if (*self.ub as f64) < other.lb {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

impl<'a> PartialEq<Flint<f32>> for FlintRef<'a, f64> {
    fn eq(&self, other: &Flint<f32>) -> bool {
        *self.ub >= (other.lb as f64) && *self.lb <= (other.ub as f64)
    }
}

impl<'a> PartialOrd<Flint<f32>> for FlintRef<'a, f64> {
    fn partial_cmp(&self, other: &Flint<f32>) -> Option<Ordering> {
        if self.lb.is_nan() || self.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
            None
        } else if *self.lb > (other.ub as f64) {
            Some(Ordering::Greater)
        } else if *self.ub < (other.lb as f64) {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

// ------------------------------------------------------------------
// Comparisons between Flint and FlintMut of different base types
// ------------------------------------------------------------------

impl<'a> PartialEq<FlintMut<'a, f32>> for Flint<f64> {
    fn eq(&self, other: &FlintMut<f32>) -> bool {
        self.ub >= (*other.lb as f64) && self.lb <= (*other.ub as f64)
    }
}

impl<'a> PartialOrd<FlintMut<'a, f32>> for Flint<f64> {
    fn partial_cmp(&self, other: &FlintMut<f32>) -> Option<Ordering> {
        if self.lb.is_nan() || self.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
            None
        } else if self.lb > (*other.ub as f64) {
            Some(Ordering::Greater)
        } else if self.ub < (*other.lb as f64) {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

impl<'a> PartialEq<FlintMut<'a, f64>> for Flint<f32> {
    fn eq(&self, other: &FlintMut<f64>) -> bool {
        (self.ub as f64) >= *other.lb && (self.lb as f64) <= *other.ub
    }
}

impl<'a> PartialOrd<FlintMut<'a, f64>> for Flint<f32> {
    fn partial_cmp(&self, other: &FlintMut<f64>) -> Option<Ordering> {
        if self.lb.is_nan() || self.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
            None
        } else if (self.lb as f64) > *other.ub {
            Some(Ordering::Greater)
        } else if (self.ub as f64) < *other.lb {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

impl<'a> PartialEq<Flint<f64>> for FlintMut<'a, f32> {
    fn eq(&self, other: &Flint<f64>) -> bool {
        (*self.ub as f64) >= other.lb && (*self.lb as f64) <= other.ub
    }
}

impl<'a> PartialOrd<Flint<f64>> for FlintMut<'a, f32> {
    fn partial_cmp(&self, other: &Flint<f64>) -> Option<Ordering> {
        if self.lb.is_nan() || self.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
            None
        } else if (*self.lb as f64) > other.ub {
            Some(Ordering::Greater)
        } else if (*self.ub as f64) < other.lb {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

impl<'a> PartialEq<Flint<f32>> for FlintMut<'a, f64> {
    fn eq(&self, other: &Flint<f32>) -> bool {
        *self.ub >= (other.lb as f64) && *self.lb <= (other.ub as f64)
    }
}

impl<'a> PartialOrd<Flint<f32>> for FlintMut<'a, f64> {
    fn partial_cmp(&self, other: &Flint<f32>) -> Option<Ordering> {
        if self.lb.is_nan() || self.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
            None
        } else if *self.lb > (other.ub as f64) {
            Some(Ordering::Greater)
        } else if *self.ub < (other.lb as f64) {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

// ---------------------------------------------------------------------
// Comparisons between types convertable to flint and flint and flintref
// ---------------------------------------------------------------------

macro_rules! impl_partial_cmp {
    ($flint_base:ty, $other_type:ty) => {
        impl PartialEq<Flint<$flint_base>> for $other_type {
            fn eq(&self, other: &Flint<$flint_base>) -> bool {
                let f: Flint<$flint_base> = (*self).into();
                f.ub >= other.lb && f.lb <= other.ub
            }
        }

        impl PartialOrd<Flint<$flint_base>> for $other_type {
            fn partial_cmp(&self, other: &Flint<$flint_base>) -> Option<Ordering> {
                let f: Flint<$flint_base> = (*self).into();
                if f.lb.is_nan() || f.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
                    None
                } else if f.lb > other.ub {
                    Some(Ordering::Greater)
                } else if f.ub < other.lb {
                    Some(Ordering::Less)
                } else {
                    Some(Ordering::Equal)
                }
            }
        }

        impl<'a> PartialEq<FlintRef<'a, $flint_base>> for $other_type {
            fn eq(&self, other: &FlintRef<$flint_base>) -> bool {
                let f: Flint<$flint_base> = (*self).into();
                f.ub >= *other.lb && f.lb <= *other.ub
            }
        }

        impl<'a> PartialOrd<FlintRef<'a, $flint_base>> for $other_type {
            fn partial_cmp(&self, other: &FlintRef<$flint_base>) -> Option<Ordering> {
                let f: Flint<$flint_base> = (*self).into();
                if f.lb.is_nan() || f.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
                    None
                } else if f.lb > *other.ub {
                    Some(Ordering::Greater)
                } else if f.ub < *other.lb {
                    Some(Ordering::Less)
                } else {
                    Some(Ordering::Equal)
                }
            }
        }

        impl<'a> PartialEq<FlintMut<'a, $flint_base>> for $other_type {
            fn eq(&self, other: &FlintMut<$flint_base>) -> bool {
                let f: Flint<$flint_base> = (*self).into();
                f.ub >= *other.lb && f.lb <= *other.ub
            }
        }

        impl<'a> PartialOrd<FlintMut<'a, $flint_base>> for $other_type {
            fn partial_cmp(&self, other: &FlintMut<$flint_base>) -> Option<Ordering> {
                let f: Flint<$flint_base> = (*self).into();
                if f.lb.is_nan() || f.ub.is_nan() || other.lb.is_nan() || other.ub.is_nan() {
                    None
                } else if f.lb > *other.ub {
                    Some(Ordering::Greater)
                } else if f.ub < *other.lb {
                    Some(Ordering::Less)
                } else {
                    Some(Ordering::Equal)
                }
            }
        }
    };
}

impl_partial_cmp!(f32, f32);
impl_partial_cmp!(f32, f64);
impl_partial_cmp!(f32, i8);
impl_partial_cmp!(f32, i16);
impl_partial_cmp!(f32, i32);
impl_partial_cmp!(f32, u8);
impl_partial_cmp!(f32, u16);
impl_partial_cmp!(f32, u32);
impl_partial_cmp!(f32, i64);
impl_partial_cmp!(f32, i128);
impl_partial_cmp!(f32, isize);
impl_partial_cmp!(f32, u64);
impl_partial_cmp!(f32, u128);
impl_partial_cmp!(f32, usize);

impl_partial_cmp!(f64, f32);
impl_partial_cmp!(f64, f64);
impl_partial_cmp!(f64, i8);
impl_partial_cmp!(f64, i16);
impl_partial_cmp!(f64, i32);
impl_partial_cmp!(f64, u8);
impl_partial_cmp!(f64, u16);
impl_partial_cmp!(f64, u32);
impl_partial_cmp!(f64, i64);
impl_partial_cmp!(f64, i128);
impl_partial_cmp!(f64, isize);
impl_partial_cmp!(f64, u64);
impl_partial_cmp!(f64, u128);
impl_partial_cmp!(f64, usize);

// ----------------------------------------------------
// Comparisons for FlintArray
// ----------------------------------------------------

impl<const N: usize> FlintArray<f32, N> {
    /// Returns an element-wise array indicating whether each interval pair overlaps.
    pub fn eq_intervals(&self, other: &Self) -> [bool; N] {
        let self_lb = Simd::<f32, N>::from_array(self.lb);
        let self_ub = Simd::<f32, N>::from_array(self.ub);
        let other_lb = Simd::<f32, N>::from_array(other.lb);
        let other_ub = Simd::<f32, N>::from_array(other.ub);
        (self_ub.simd_ge(other_lb) & self_lb.simd_le(other_ub)).to_array()
    }

    /// Returns an element-wise array indicating whether each self interval lies
    /// entirely below the corresponding other interval (no overlap, self upper < other lower).
    pub fn lt_intervals(&self, other: &Self) -> [bool; N] {
        Simd::<f32, N>::from_array(self.ub)
            .simd_lt(Simd::<f32, N>::from_array(other.lb))
            .to_array()
    }

    /// Returns an element-wise array indicating whether each self interval lies
    /// entirely above the corresponding other interval (no overlap, self lower > other upper).
    pub fn gt_intervals(&self, other: &Self) -> [bool; N] {
        Simd::<f32, N>::from_array(self.lb)
            .simd_gt(Simd::<f32, N>::from_array(other.ub))
            .to_array()
    }

    /// True iff every element pair overlaps.
    pub fn all_eq(&self, other: &Self) -> bool {
        self.eq_intervals(other).iter().all(|&b| b)
    }

    /// True iff every self interval lies entirely below the corresponding other interval.
    pub fn all_lt(&self, other: &Self) -> bool {
        self.lt_intervals(other).iter().all(|&b| b)
    }

    /// True iff every self interval lies entirely above the corresponding other interval.
    pub fn all_gt(&self, other: &Self) -> bool {
        self.gt_intervals(other).iter().all(|&b| b)
    }
}

impl<const N: usize> FlintArray<f64, N> {
    /// Returns an element-wise array indicating whether each interval pair overlaps.
    pub fn eq_intervals(&self, other: &Self) -> [bool; N] {
        let self_lb = Simd::<f64, N>::from_array(self.lb);
        let self_ub = Simd::<f64, N>::from_array(self.ub);
        let other_lb = Simd::<f64, N>::from_array(other.lb);
        let other_ub = Simd::<f64, N>::from_array(other.ub);
        (self_ub.simd_ge(other_lb) & self_lb.simd_le(other_ub)).to_array()
    }

    /// Returns an element-wise array indicating whether each self interval lies
    /// entirely below the corresponding other interval (no overlap, self upper < other lower).
    pub fn lt_intervals(&self, other: &Self) -> [bool; N] {
        Simd::<f64, N>::from_array(self.ub)
            .simd_lt(Simd::<f64, N>::from_array(other.lb))
            .to_array()
    }

    /// Returns an element-wise array indicating whether each self interval lies
    /// entirely above the corresponding other interval (no overlap, self lower > other upper).
    pub fn gt_intervals(&self, other: &Self) -> [bool; N] {
        Simd::<f64, N>::from_array(self.lb)
            .simd_gt(Simd::<f64, N>::from_array(other.ub))
            .to_array()
    }

    /// True iff every element pair overlaps.
    pub fn all_eq(&self, other: &Self) -> bool {
        self.eq_intervals(other).iter().all(|&b| b)
    }

    /// True iff every self interval lies entirely below the corresponding other interval.
    pub fn all_lt(&self, other: &Self) -> bool {
        self.lt_intervals(other).iter().all(|&b| b)
    }

    /// True iff every self interval lies entirely above the corresponding other interval.
    pub fn all_gt(&self, other: &Self) -> bool {
        self.gt_intervals(other).iter().all(|&b| b)
    }
}

// ---------------------------------------------------------------
// Comparisons for FlintVec and FlintView (chunked SIMD, lane = 8)
// ---------------------------------------------------------------

// Generates element-wise and aggregate comparison methods for both
// FlintVec<$T> and FlintView<'_, $T>.  Inner loops use $S8 (a Simd<$T, 8>
// alias) for SIMD throughput; a scalar tail handles any remainder.
macro_rules! impl_vec_view_cmp {
    ($T:ty, $S8:ty) => {
        impl FlintVec<$T> {
            /// Returns an element-wise vec indicating whether each interval pair overlaps.
            pub fn eq_intervals(&self, other: &Self) -> Vec<bool> {
                const L: usize = 8;
                let n = self.lb.len();
                let mut out = Vec::with_capacity(n);
                for i in 0..(n / L) {
                    let s = i * L;
                    out.extend_from_slice(
                        &(<$S8>::from_slice(&self.ub[s..]).simd_ge(<$S8>::from_slice(&other.lb[s..]))
                            & <$S8>::from_slice(&self.lb[s..]).simd_le(<$S8>::from_slice(&other.ub[s..])))
                        .to_array(),
                    );
                }
                for j in (n / L * L)..n {
                    out.push(self.ub[j] >= other.lb[j] && self.lb[j] <= other.ub[j]);
                }
                out
            }

            /// Returns an element-wise vec indicating whether each self interval lies
            /// entirely below the corresponding other interval.
            pub fn lt_intervals(&self, other: &Self) -> Vec<bool> {
                const L: usize = 8;
                let n = self.lb.len();
                let mut out = Vec::with_capacity(n);
                for i in 0..(n / L) {
                    let s = i * L;
                    out.extend_from_slice(
                        &<$S8>::from_slice(&self.ub[s..])
                            .simd_lt(<$S8>::from_slice(&other.lb[s..]))
                            .to_array(),
                    );
                }
                for j in (n / L * L)..n {
                    out.push(self.ub[j] < other.lb[j]);
                }
                out
            }

            /// Returns an element-wise vec indicating whether each self interval lies
            /// entirely above the corresponding other interval.
            pub fn gt_intervals(&self, other: &Self) -> Vec<bool> {
                const L: usize = 8;
                let n = self.lb.len();
                let mut out = Vec::with_capacity(n);
                for i in 0..(n / L) {
                    let s = i * L;
                    out.extend_from_slice(
                        &<$S8>::from_slice(&self.lb[s..])
                            .simd_gt(<$S8>::from_slice(&other.ub[s..]))
                            .to_array(),
                    );
                }
                for j in (n / L * L)..n {
                    out.push(self.lb[j] > other.ub[j]);
                }
                out
            }

            /// True iff every element pair overlaps.
            pub fn all_eq(&self, other: &Self) -> bool {
                self.eq_intervals(other).iter().all(|&b| b)
            }

            /// True iff every self interval lies entirely below the corresponding other interval.
            pub fn all_lt(&self, other: &Self) -> bool {
                self.lt_intervals(other).iter().all(|&b| b)
            }

            /// True iff every self interval lies entirely above the corresponding other interval.
            pub fn all_gt(&self, other: &Self) -> bool {
                self.gt_intervals(other).iter().all(|&b| b)
            }
        }

        impl<'a> FlintView<'a, $T> {
            /// Returns an element-wise vec indicating whether each interval pair overlaps.
            pub fn eq_intervals(&self, other: &Self) -> Vec<bool> {
                const L: usize = 8;
                let n = self.lb.len();
                let mut out = Vec::with_capacity(n);
                for i in 0..(n / L) {
                    let s = i * L;
                    out.extend_from_slice(
                        &(<$S8>::from_slice(&self.ub[s..]).simd_ge(<$S8>::from_slice(&other.lb[s..]))
                            & <$S8>::from_slice(&self.lb[s..]).simd_le(<$S8>::from_slice(&other.ub[s..])))
                        .to_array(),
                    );
                }
                for j in (n / L * L)..n {
                    out.push(self.ub[j] >= other.lb[j] && self.lb[j] <= other.ub[j]);
                }
                out
            }

            /// Returns an element-wise vec indicating whether each self interval lies
            /// entirely below the corresponding other interval.
            pub fn lt_intervals(&self, other: &Self) -> Vec<bool> {
                const L: usize = 8;
                let n = self.lb.len();
                let mut out = Vec::with_capacity(n);
                for i in 0..(n / L) {
                    let s = i * L;
                    out.extend_from_slice(
                        &<$S8>::from_slice(&self.ub[s..])
                            .simd_lt(<$S8>::from_slice(&other.lb[s..]))
                            .to_array(),
                    );
                }
                for j in (n / L * L)..n {
                    out.push(self.ub[j] < other.lb[j]);
                }
                out
            }

            /// Returns an element-wise vec indicating whether each self interval lies
            /// entirely above the corresponding other interval.
            pub fn gt_intervals(&self, other: &Self) -> Vec<bool> {
                const L: usize = 8;
                let n = self.lb.len();
                let mut out = Vec::with_capacity(n);
                for i in 0..(n / L) {
                    let s = i * L;
                    out.extend_from_slice(
                        &<$S8>::from_slice(&self.lb[s..])
                            .simd_gt(<$S8>::from_slice(&other.ub[s..]))
                            .to_array(),
                    );
                }
                for j in (n / L * L)..n {
                    out.push(self.lb[j] > other.ub[j]);
                }
                out
            }

            /// True iff every element pair overlaps.
            pub fn all_eq(&self, other: &Self) -> bool {
                self.eq_intervals(other).iter().all(|&b| b)
            }

            /// True iff every self interval lies entirely below the corresponding other interval.
            pub fn all_lt(&self, other: &Self) -> bool {
                self.lt_intervals(other).iter().all(|&b| b)
            }

            /// True iff every self interval lies entirely above the corresponding other interval.
            pub fn all_gt(&self, other: &Self) -> bool {
                self.gt_intervals(other).iter().all(|&b| b)
            }
        }

        impl<'a> FlintViewMut<'a, $T> {
            /// Returns an element-wise vec indicating whether each interval pair overlaps.
            pub fn eq_intervals(&self, other: &Self) -> Vec<bool> {
                const L: usize = 8;
                let n = self.lb.len();
                let mut out = Vec::with_capacity(n);
                for i in 0..(n / L) {
                    let s = i * L;
                    out.extend_from_slice(
                        &(<$S8>::from_slice(&self.ub[s..]).simd_ge(<$S8>::from_slice(&other.lb[s..]))
                            & <$S8>::from_slice(&self.lb[s..]).simd_le(<$S8>::from_slice(&other.ub[s..])))
                        .to_array(),
                    );
                }
                for j in (n / L * L)..n {
                    out.push(self.ub[j] >= other.lb[j] && self.lb[j] <= other.ub[j]);
                }
                out
            }

            /// Returns an element-wise vec indicating whether each self interval lies
            /// entirely below the corresponding other interval.
            pub fn lt_intervals(&self, other: &Self) -> Vec<bool> {
                const L: usize = 8;
                let n = self.lb.len();
                let mut out = Vec::with_capacity(n);
                for i in 0..(n / L) {
                    let s = i * L;
                    out.extend_from_slice(
                        &<$S8>::from_slice(&self.ub[s..])
                            .simd_lt(<$S8>::from_slice(&other.lb[s..]))
                            .to_array(),
                    );
                }
                for j in (n / L * L)..n {
                    out.push(self.ub[j] < other.lb[j]);
                }
                out
            }

            /// Returns an element-wise vec indicating whether each self interval lies
            /// entirely above the corresponding other interval.
            pub fn gt_intervals(&self, other: &Self) -> Vec<bool> {
                const L: usize = 8;
                let n = self.lb.len();
                let mut out = Vec::with_capacity(n);
                for i in 0..(n / L) {
                    let s = i * L;
                    out.extend_from_slice(
                        &<$S8>::from_slice(&self.lb[s..])
                            .simd_gt(<$S8>::from_slice(&other.ub[s..]))
                            .to_array(),
                    );
                }
                for j in (n / L * L)..n {
                    out.push(self.lb[j] > other.ub[j]);
                }
                out
            }

            /// True iff every element pair overlaps.
            pub fn all_eq(&self, other: &Self) -> bool {
                self.eq_intervals(other).iter().all(|&b| b)
            }

            /// True iff every self interval lies entirely below the corresponding other interval.
            pub fn all_lt(&self, other: &Self) -> bool {
                self.lt_intervals(other).iter().all(|&b| b)
            }

            /// True iff every self interval lies entirely above the corresponding other interval.
            pub fn all_gt(&self, other: &Self) -> bool {
                self.gt_intervals(other).iter().all(|&b| b)
            }
        }
    };
}

impl_vec_view_cmp!(f32, f32x8);
impl_vec_view_cmp!(f64, f64x8);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_equality_flint() {
        let fa: Flint<f64> = Flint { lb: 0.5, ub: 1.5 };
        let fb: Flint<f64> = Flint { lb: 0.25, ub: 0.75 };
        let fc: Flint<f64> = Flint { lb: 0.0, ub: 0.5 };
        let fd: Flint<f64> = Flint {
            lb: -0.25,
            ub: 0.25,
        };
        let fe: Flint<f64> = Flint {
            lb: f64::NAN,
            ub: f64::NAN,
        };
        // compare flints and refs
        assert_eq!(fa, fa);
        assert_eq!(fa, fb);
        assert_eq!(fa, fc);
        assert_eq!(fa, fa.as_ref());
        assert_eq!(fa.as_ref(), fb);
        assert_eq!(fa.as_ref(), fc.as_ref());
        assert_ne!(fa, fd);
        assert_ne!(fe, fe);
        assert_ne!(fa, fe);
        assert_ne!(fa, fd.as_ref());
        assert_ne!(fe.as_ref(), fe);
        assert_ne!(fa.as_ref(), fe.as_ref());
    }

    #[test]
    fn test_equality_nonflint() {
        let fa: Flint<f64> = Flint { lb: 0.5, ub: 1.5 };
        let fb: Flint<f64> = Flint { lb: 0.25, ub: 0.75 };
        let fc: Flint<f64> = Flint { lb: 0.0, ub: 0.5 };
        let fd: Flint<f64> = Flint {
            lb: -0.25,
            ub: 0.25,
        };
        let fe: Flint<f64> = Flint {
            lb: f64::NAN,
            ub: f64::NAN,
        };
        assert_eq!(1, fa);
        assert_eq!(fb, 0.5);
        assert_eq!(0.25_f32, fc);
        assert_ne!(fa, 0_u8);
        assert_ne!(fd, f32::NAN);
        assert_ne!(0.0, fe);
        assert_eq!(1, fa.as_ref());
        assert_eq!(fb.as_ref(), 0.5);
        assert_eq!(0.25_f32, fc.as_ref());
        assert_ne!(fa.as_ref(), 0_u8);
        assert_ne!(fd.as_ref(), f32::NAN);
        assert_ne!(0.0, fe.as_ref());
    }

    #[test]
    fn test_inequality_flint() {
        let fa: Flint<f64> = Flint { lb: 0.5, ub: 1.5 };
        let fb: Flint<f64> = Flint { lb: 0.25, ub: 0.75 };
        let fc: Flint<f64> = Flint { lb: 0.0, ub: 0.5 };
        let fd: Flint<f64> = Flint {
            lb: -0.25,
            ub: 0.25,
        };
        // overlap (so equals)
        assert!(!(fa > fb));
        assert!(!(fa < fb.as_ref()));
        assert!(!(fa.as_ref() > fc));
        assert!(!(fa.as_ref() < fc.as_ref()));
        // no overlap (so > or < depending on order)
        assert!(fa > fd);
        assert!(fd.as_ref() < fa);
        assert!(fa > fd.as_ref());
        assert!(fd.as_ref() < fa.as_ref());
    }

    #[test]
    fn test_inequality_nonflint() {
        let f: Flint<f64> = Flint { lb: 0.5, ub: 1.5 };
        assert!(f > 0.0);
        assert!(0 < f.as_ref());
        assert!(f.as_ref() > 0_u16);
        assert!(0.0_f32 < f);
    }

    // --- FlintRef vs FlintRef ordering ---

    #[test]
    fn test_inequality_ref_vs_ref() {
        let fa: Flint<f64> = Flint { lb: 0.5, ub: 1.5 };
        let fb: Flint<f64> = Flint { lb: 0.25, ub: 0.75 };
        let fd: Flint<f64> = Flint {
            lb: -0.25,
            ub: 0.25,
        };

        // overlapping: neither < nor >
        assert!(!(fa.as_ref() > fb.as_ref()));
        assert!(!(fa.as_ref() < fb.as_ref()));

        // non-overlapping: fa strictly above fd
        assert!(fa.as_ref() > fd.as_ref());
        assert!(fd.as_ref() < fa.as_ref());
    }

    // --- direct partial_cmp return value checks ---

    #[test]
    fn test_partial_cmp_values() {
        let fa: Flint<f64> = Flint { lb: 2.0, ub: 3.0 };
        let fb: Flint<f64> = Flint { lb: 0.0, ub: 1.0 };
        let fc: Flint<f64> = Flint { lb: 0.5, ub: 2.5 }; // overlaps fa
        let fnan: Flint<f64> = Flint {
            lb: f64::NAN,
            ub: f64::NAN,
        };

        // Some(Greater): fa entirely above fb
        assert_eq!(Some(Ordering::Greater), fa.partial_cmp(&fb));
        // Some(Less): fb entirely below fa
        assert_eq!(Some(Ordering::Less), fb.partial_cmp(&fa));
        // Some(Equal): fa and fc overlap
        assert_eq!(Some(Ordering::Equal), fa.partial_cmp(&fc));
        // None when either operand contains NaN
        assert_eq!(None, fa.partial_cmp(&fnan));
        assert_eq!(None, fnan.partial_cmp(&fa));
        assert_eq!(None, fnan.partial_cmp(&fnan));

        // same checks via FlintRef
        assert_eq!(
            Some(Ordering::Greater),
            fa.as_ref().partial_cmp(&fb.as_ref())
        );
        assert_eq!(Some(Ordering::Less), fb.as_ref().partial_cmp(&fa.as_ref()));
        assert_eq!(Some(Ordering::Equal), fa.as_ref().partial_cmp(&fc.as_ref()));
        assert_eq!(None, fa.as_ref().partial_cmp(&fnan.as_ref()));
        assert_eq!(None, fnan.as_ref().partial_cmp(&fa.as_ref()));

        // partial_cmp against a primitive
        assert_eq!(Some(Ordering::Greater), fa.partial_cmp(&0.0_f64));
        assert_eq!(Some(Ordering::Less), fb.partial_cmp(&10.0_f64));
        assert_eq!(None, fa.partial_cmp(&f64::NAN));
        assert_eq!(None, fa.as_ref().partial_cmp(&f64::NAN));
    }

    // --- touching intervals: equal at shared endpoint ---

    #[test]
    fn test_touching_intervals() {
        // [0.0, 1.0] and [1.0, 2.0] share the point 1.0 → equal (overlap)
        let fa: Flint<f64> = Flint { lb: 0.0, ub: 1.0 };
        let fb: Flint<f64> = Flint { lb: 1.0, ub: 2.0 };
        assert_eq!(fa, fb);
        assert_eq!(fa.as_ref(), fb.as_ref());
        // partial_cmp should return Some(Equal) for touching intervals
        assert_eq!(Some(Ordering::Equal), fa.partial_cmp(&fb));
        assert_eq!(Some(Ordering::Equal), fa.as_ref().partial_cmp(&fb.as_ref()));

        // [0.0, 0.9] and [1.0, 2.0] do NOT touch → not equal, fa < fb
        let fc: Flint<f64> = Flint { lb: 0.0, ub: 0.9 };
        assert_ne!(fc, fb);
        assert!(fc < fb);
        assert_eq!(Some(Ordering::Less), fc.partial_cmp(&fb));
    }

    // --- Flint<f32> comparisons ---

    #[test]
    fn test_equality_flint_f32() {
        let fa: Flint<f32> = Flint { lb: 0.5, ub: 1.5 };
        let fb: Flint<f32> = Flint { lb: 0.25, ub: 0.75 };
        let fc: Flint<f32> = Flint { lb: 2.0, ub: 3.0 };
        let fnan: Flint<f32> = Flint {
            lb: f32::NAN,
            ub: f32::NAN,
        };

        assert_eq!(fa, fb); // overlapping → equal
        assert_ne!(fa, fc); // disjoint → not equal
        assert_ne!(fnan, fnan); // NaN never equal
        assert_eq!(fa, fa.as_ref()); // owned == ref
        assert_eq!(fa.as_ref(), fb.as_ref()); // ref == ref

        // with primitives
        assert_eq!(fa, 1_i8);
        assert_eq!(1_u8, fa);
        assert_ne!(fa.as_ref(), 0_u8);
    }

    #[test]
    fn test_inequality_flint_f32() {
        let fa: Flint<f32> = Flint { lb: 2.0, ub: 3.0 };
        let fb: Flint<f32> = Flint { lb: 0.0, ub: 1.0 };

        assert!(fa > fb);
        assert!(fb < fa);
        assert!(fa.as_ref() > fb.as_ref());
        assert!(fb.as_ref() < fa.as_ref());
        assert_eq!(Some(Ordering::Greater), fa.partial_cmp(&fb));
        assert_eq!(Some(Ordering::Less), fb.partial_cmp(&fa));

        // with primitives
        assert!(fa > 0.0_f32);
        assert!(fa.as_ref() > 0_i8);
        assert_eq!(None, fa.partial_cmp(&f32::NAN));
    }

    // --- cross-type f32/f64 comparisons ---

    #[test]
    fn test_cross_type_cmp() {
        let f64val: Flint<f64> = Flint { lb: 0.5, ub: 1.5 };
        let f32val: Flint<f32> = Flint { lb: 0.25, ub: 0.75 };
        let f32hi: Flint<f32> = Flint { lb: 2.0, ub: 3.0 };
        let f32nan: Flint<f32> = Flint {
            lb: f32::NAN,
            ub: f32::NAN,
        };

        // Flint<f64> == FlintRef<f32>: overlapping
        assert_eq!(f64val, f32val.as_ref());
        // Flint<f64> != FlintRef<f32>: disjoint
        assert_ne!(f64val, f32hi.as_ref());
        // Flint<f64> ordering vs FlintRef<f32>
        assert!(f64val < f32hi.as_ref());
        assert_eq!(Some(Ordering::Less), f64val.partial_cmp(&f32hi.as_ref()));
        assert_eq!(None, f64val.partial_cmp(&f32nan.as_ref()));

        // Flint<f32> == FlintRef<f64>
        let f64hi: Flint<f64> = Flint { lb: 2.0, ub: 3.0 };
        assert_eq!(f32val, f64val.as_ref());
        assert_ne!(f32val, f64hi.as_ref());
        assert!(f32val < f64hi.as_ref());
        assert_eq!(Some(Ordering::Less), f32val.partial_cmp(&f64hi.as_ref()));

        // FlintRef<f32> vs Flint<f64>
        assert_eq!(f32val.as_ref(), f64val);
        assert!(f32hi.as_ref() > f64val);
        assert_eq!(Some(Ordering::Greater), f32hi.as_ref().partial_cmp(&f64val));
        assert_eq!(None, f32nan.as_ref().partial_cmp(&f64val));

        // FlintRef<f64> vs Flint<f32>
        assert_eq!(f64val.as_ref(), f32val);
        assert!(f64hi.as_ref() > f32val);
        assert_eq!(Some(Ordering::Greater), f64hi.as_ref().partial_cmp(&f32val));
    }

    // --- infinity comparisons ---

    #[test]
    fn test_infinity_cmp() {
        let finf: Flint<f64> = Flint {
            lb: f64::INFINITY,
            ub: f64::INFINITY,
        };
        let fninf: Flint<f64> = Flint {
            lb: f64::NEG_INFINITY,
            ub: f64::NEG_INFINITY,
        };
        let fa: Flint<f64> = Flint { lb: 0.5, ub: 1.5 };

        // +inf is greater than any finite interval
        assert!(finf > fa);
        assert!(fa < finf);
        assert_eq!(Some(Ordering::Greater), finf.partial_cmp(&fa));
        assert_eq!(Some(Ordering::Less), fa.partial_cmp(&finf));

        // -inf is less than any finite interval
        assert!(fninf < fa);
        assert!(fa > fninf);
        assert_eq!(Some(Ordering::Less), fninf.partial_cmp(&fa));
        assert_eq!(Some(Ordering::Greater), fa.partial_cmp(&fninf));

        // +inf vs -inf
        assert!(finf > fninf);
        assert_eq!(Some(Ordering::Greater), finf.partial_cmp(&fninf));

        // same via refs
        assert!(finf.as_ref() > fa.as_ref());
        assert!(fninf.as_ref() < fa.as_ref());
        assert_eq!(
            Some(Ordering::Greater),
            finf.as_ref().partial_cmp(&fa.as_ref())
        );
    }

    // ---- FlintArray comparisons ----

    #[test]
    fn test_array_cmp_flintarray_f32() {
        // a: [0,1], [2,3], [4,5], [6,7]
        let a: FlintArray<f32, 4> = FlintArray {
            lb: [0.0, 2.0, 4.0, 6.0],
            ub: [1.0, 3.0, 5.0, 7.0],
        };
        // b overlaps a at every element
        let b: FlintArray<f32, 4> = FlintArray {
            lb: [0.5, 2.5, 4.5, 6.5],
            ub: [1.5, 3.5, 5.5, 7.5],
        };
        // c is entirely above a at every element
        let c: FlintArray<f32, 4> = FlintArray {
            lb: [2.0, 4.0, 6.0, 8.0],
            ub: [3.0, 5.0, 7.0, 9.0],
        };
        // d is entirely below a at every element
        let d: FlintArray<f32, 4> = FlintArray {
            lb: [-3.0; 4],
            ub: [-2.0; 4],
        };
        // n has NaN in first element, otherwise matches a's intervals
        let n: FlintArray<f32, 4> = FlintArray {
            lb: [f32::NAN, 2.0, 4.0, 6.0],
            ub: [f32::NAN, 3.0, 5.0, 7.0],
        };
        // touch_a and touch_b share endpoint 1.0 → equal (overlap at a single point)
        let touch_a: FlintArray<f32, 4> = FlintArray {
            lb: [0.0; 4],
            ub: [1.0; 4],
        };
        let touch_b: FlintArray<f32, 4> = FlintArray {
            lb: [1.0; 4],
            ub: [2.0; 4],
        };

        // eq_intervals
        assert_eq!(a.eq_intervals(&a), [true; 4]);
        assert_eq!(a.eq_intervals(&b), [true; 4]);
        assert_eq!(a.eq_intervals(&c), [false; 4]);
        assert_eq!(a.eq_intervals(&d), [false; 4]);
        assert_eq!(a.eq_intervals(&n), [false, true, true, true]); // n[0] is NaN
        assert_eq!(touch_a.eq_intervals(&touch_b), [true; 4]); // touching = equal

        // lt_intervals: self.ub < other.lb
        assert_eq!(a.lt_intervals(&c), [true; 4]); // a entirely below c
        assert_eq!(c.lt_intervals(&a), [false; 4]); // c is above a, not below
        assert_eq!(a.lt_intervals(&b), [false; 4]); // overlap → not lt
        assert_eq!(n.lt_intervals(&c), [false, true, true, true]); // NaN → false

        // gt_intervals: self.lb > other.ub
        assert_eq!(c.gt_intervals(&a), [true; 4]); // c entirely above a
        assert_eq!(a.gt_intervals(&c), [false; 4]); // a is below c, not above
        assert_eq!(a.gt_intervals(&b), [false; 4]); // overlap → not gt
        assert_eq!(n.gt_intervals(&d), [false, true, true, true]); // NaN → false

        // all_* aggregates
        assert!(a.all_eq(&b));
        assert!(!a.all_eq(&c));
        assert!(a.all_lt(&c));
        assert!(!a.all_lt(&b)); // overlap → not all_lt
        assert!(c.all_gt(&a));
        assert!(!b.all_gt(&a)); // overlap → not all_gt
    }

    #[test]
    fn test_array_cmp_flintarray_f64() {
        let a: FlintArray<f64, 4> = FlintArray {
            lb: [0.0, 2.0, 4.0, 6.0],
            ub: [1.0, 3.0, 5.0, 7.0],
        };
        let b: FlintArray<f64, 4> = FlintArray {
            lb: [0.5, 2.5, 4.5, 6.5],
            ub: [1.5, 3.5, 5.5, 7.5],
        };
        let c: FlintArray<f64, 4> = FlintArray {
            lb: [2.0, 4.0, 6.0, 8.0],
            ub: [3.0, 5.0, 7.0, 9.0],
        };
        let nan: FlintArray<f64, 4> = FlintArray {
            lb: [f64::NAN, 2.0, 4.0, 6.0],
            ub: [f64::NAN, 3.0, 5.0, 7.0],
        };

        assert_eq!(a.eq_intervals(&b), [true; 4]);
        assert_eq!(a.eq_intervals(&c), [false; 4]);
        assert_eq!(a.eq_intervals(&nan), [false, true, true, true]);
        assert_eq!(a.lt_intervals(&c), [true; 4]);
        assert_eq!(c.gt_intervals(&a), [true; 4]);
        assert!(a.all_eq(&b));
        assert!(a.all_lt(&c));
        assert!(c.all_gt(&a));
        assert!(!a.all_lt(&b));
    }

    // ---- FlintVec comparisons ----

    #[test]
    fn test_array_cmp_flintvec_f32_len4() {
        let a = FlintVec::<f32> {
            lb: vec![0.0, 2.0, 4.0, 6.0],
            ub: vec![1.0, 3.0, 5.0, 7.0],
        };
        let b = FlintVec::<f32> {
            lb: vec![0.5, 2.5, 4.5, 6.5],
            ub: vec![1.5, 3.5, 5.5, 7.5],
        };
        let c = FlintVec::<f32> {
            lb: vec![2.0, 4.0, 6.0, 8.0],
            ub: vec![3.0, 5.0, 7.0, 9.0],
        };
        let d = FlintVec::<f32> {
            lb: vec![-3.0; 4],
            ub: vec![-2.0; 4],
        };

        assert_eq!(a.eq_intervals(&b), vec![true; 4]);
        assert_eq!(a.eq_intervals(&c), vec![false; 4]);
        assert_eq!(a.lt_intervals(&c), vec![true; 4]);
        assert_eq!(c.gt_intervals(&a), vec![true; 4]);
        assert_eq!(a.gt_intervals(&d), vec![true; 4]);
        assert!(a.all_eq(&b));
        assert!(!a.all_eq(&c));
        assert!(a.all_lt(&c));
        assert!(c.all_gt(&a));
    }

    #[test]
    fn test_array_cmp_flintvec_f32_len9() {
        // length 9 = 8 + 1, exercises the chunked-SIMD remainder path
        let a = FlintVec::<f32> {
            lb: vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0],
            ub: vec![1.0, 3.0, 5.0, 7.0, 9.0, 11.0, 13.0, 15.0, 17.0],
        };
        // b overlaps a at every element
        let b = FlintVec::<f32> {
            lb: vec![0.5, 2.5, 4.5, 6.5, 8.5, 10.5, 12.5, 14.5, 16.5],
            ub: vec![1.5, 3.5, 5.5, 7.5, 9.5, 11.5, 13.5, 15.5, 17.5],
        };
        // c is entirely above a
        let c = FlintVec::<f32> {
            lb: vec![2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0],
            ub: vec![3.0, 5.0, 7.0, 9.0, 11.0, 13.0, 15.0, 17.0, 19.0],
        };

        assert_eq!(a.eq_intervals(&b), vec![true; 9]);
        assert_eq!(a.eq_intervals(&c), vec![false; 9]);
        assert_eq!(a.lt_intervals(&c), vec![true; 9]);
        assert_eq!(c.gt_intervals(&a), vec![true; 9]);
        assert!(a.all_eq(&b));
        assert!(a.all_lt(&c));
        assert!(c.all_gt(&a));
    }

    // ---- FlintView comparisons ----

    #[test]
    fn test_array_cmp_flintview_f64() {
        let va = FlintVec::<f64> {
            lb: vec![0.0, 2.0, 4.0, 6.0],
            ub: vec![1.0, 3.0, 5.0, 7.0],
        };
        let vb = FlintVec::<f64> {
            lb: vec![0.5, 2.5, 4.5, 6.5],
            ub: vec![1.5, 3.5, 5.5, 7.5],
        };
        let vc = FlintVec::<f64> {
            lb: vec![2.0, 4.0, 6.0, 8.0],
            ub: vec![3.0, 5.0, 7.0, 9.0],
        };

        let a = FlintView::<f64> { lb: &va.lb, ub: &va.ub };
        let b = FlintView::<f64> { lb: &vb.lb, ub: &vb.ub };
        let c = FlintView::<f64> { lb: &vc.lb, ub: &vc.ub };

        assert_eq!(a.eq_intervals(&b), vec![true; 4]);
        assert_eq!(a.eq_intervals(&c), vec![false; 4]);
        assert_eq!(a.lt_intervals(&c), vec![true; 4]);
        assert_eq!(c.gt_intervals(&a), vec![true; 4]);
        assert!(a.all_eq(&b));
        assert!(a.all_lt(&c));
        assert!(c.all_gt(&a));
    }

    // ------------------------------------------------------------------
    // FlintMut scalar comparisons
    // ------------------------------------------------------------------

    #[test]
    fn test_cmp_flint_mut_basic() {
        // overlapping — equal
        let mut lb_a = 0.5_f64;
        let mut ub_a = 1.5_f64;
        let mut lb_b = 0.25_f64;
        let mut ub_b = 0.75_f64;
        let a = FlintMut { lb: &mut lb_a, ub: &mut ub_a };
        let b = FlintMut { lb: &mut lb_b, ub: &mut ub_b };
        assert!(a == b);
        assert!(!(a < b));
        assert!(!(a > b));
    }

    #[test]
    fn test_cmp_flint_mut_ordering() {
        // disjoint: a entirely above b
        let mut lb_a = 2.0_f64;
        let mut ub_a = 3.0_f64;
        let mut lb_b = 0.0_f64;
        let mut ub_b = 1.0_f64;
        let a = FlintMut { lb: &mut lb_a, ub: &mut ub_a };
        let b = FlintMut { lb: &mut lb_b, ub: &mut ub_b };
        assert!(a > b);
        assert!(b < a);
        assert!(!(a == b));
    }

    #[test]
    fn test_cmp_flint_mut_with_flint() {
        // FlintMut == Flint (generic Rhs covers Flint<T>)
        let fa = Flint::<f64> { lb: 0.5, ub: 1.5 };
        let mut lb = 0.25_f64;
        let mut ub = 0.75_f64;
        let m = FlintMut { lb: &mut lb, ub: &mut ub };
        assert!(m == fa); // generic Rhs impl

        // Flint == FlintMut (explicit impl since FlintMut is !Copy)
        assert!(fa == m);
    }

    #[test]
    fn test_cmp_flint_mut_with_flintref() {
        let lb_r = 0.5_f64;
        let ub_r = 1.5_f64;
        let r = FlintRef { lb: &lb_r, ub: &ub_r };

        let mut lb_m = 0.25_f64;
        let mut ub_m = 0.75_f64;
        let m = FlintMut { lb: &mut lb_m, ub: &mut ub_m };

        // FlintMut == FlintRef (generic Rhs covers FlintRef since it is Copy+Into)
        assert!(m == r);
        // FlintRef == FlintMut (explicit impl since FlintMut is !Copy)
        assert!(r == m);
    }

    #[test]
    fn test_cmp_flint_mut_nan() {
        let mut lb = f64::NAN;
        let mut ub = f64::NAN;
        let m = FlintMut { lb: &mut lb, ub: &mut ub };
        let f = Flint::<f64> { lb: 1.0, ub: 2.0 };
        assert!(m.partial_cmp(&f).is_none());
    }

    #[test]
    fn test_cmp_flint_mut_cross_precision() {
        // FlintMut<f32> vs Flint<f64>
        let mut lb32: f32 = 0.5;
        let mut ub32: f32 = 1.5;
        let m32 = FlintMut { lb: &mut lb32, ub: &mut ub32 };
        let f64 = Flint::<f64> { lb: 0.25, ub: 0.75 };
        assert!(m32 == f64);
        assert!(f64 == m32);

        // disjoint: FlintMut<f32> > Flint<f64>
        let mut lb32b: f32 = 5.0;
        let mut ub32b: f32 = 6.0;
        let m32b = FlintMut { lb: &mut lb32b, ub: &mut ub32b };
        let f64b = Flint::<f64> { lb: 1.0, ub: 2.0 };
        assert!(m32b > f64b);
        assert!(f64b < m32b);
    }

    #[test]
    fn test_cmp_flint_mut_with_primitive() {
        let mut lb = 0.5_f64;
        let mut ub = 1.5_f64;
        let m = FlintMut { lb: &mut lb, ub: &mut ub };
        // primitive on right: generic Rhs impl
        assert!(m == 1.0_f64);
        assert!(m == 1_i32);
        // primitive on left: impl_partial_cmp! macro
        assert!(1.0_f64 == m);
        assert!(1_i32 == m);
    }

    // ------------------------------------------------------------------
    // FlintViewMut array comparisons
    // ------------------------------------------------------------------

    #[test]
    fn test_cmp_view_mut_f32_eq() {
        let mut lb_a = [0.0_f32, 1.0, 2.0, 3.0];
        let mut ub_a = [1.0_f32, 2.0, 3.0, 4.0];
        let mut lb_b = [0.5_f32, 1.5, 2.5, 3.5];
        let mut ub_b = [1.5_f32, 2.5, 3.5, 4.5];
        let a = FlintViewMut { lb: &mut lb_a, ub: &mut ub_a };
        let b = FlintViewMut { lb: &mut lb_b, ub: &mut ub_b };
        assert_eq!(a.eq_intervals(&b), vec![true; 4]);
        assert!(a.all_eq(&b));
    }

    #[test]
    fn test_cmp_view_mut_f64_lt_gt() {
        let mut lb_a = [0.0_f64, 1.0, 2.0, 3.0];
        let mut ub_a = [0.5_f64, 1.5, 2.5, 3.5];
        let mut lb_b = [1.0_f64, 2.0, 3.0, 4.0];
        let mut ub_b = [1.5_f64, 2.5, 3.5, 4.5];
        let a = FlintViewMut { lb: &mut lb_a, ub: &mut ub_a };
        let b = FlintViewMut { lb: &mut lb_b, ub: &mut ub_b };
        assert_eq!(a.lt_intervals(&b), vec![true; 4]);
        assert_eq!(b.gt_intervals(&a), vec![true; 4]);
        assert!(a.all_lt(&b));
        assert!(b.all_gt(&a));
    }

    #[test]
    fn test_cmp_view_mut_matches_view() {
        // ViewMut and View over the same data should give identical results
        let lbs = [0.0_f64, 10.0, 20.0, 30.0];
        let ubs = [5.0_f64, 15.0, 25.0, 35.0];
        let lbs2 = [3.0_f64, 13.0, 23.0, 33.0];
        let ubs2 = [8.0_f64, 18.0, 28.0, 38.0];

        let va = FlintView { lb: &lbs, ub: &ubs };
        let vb = FlintView { lb: &lbs2, ub: &ubs2 };

        let mut lbm = lbs;
        let mut ubm = ubs;
        let mut lbm2 = lbs2;
        let mut ubm2 = ubs2;
        let vma = FlintViewMut { lb: &mut lbm, ub: &mut ubm };
        let vmb = FlintViewMut { lb: &mut lbm2, ub: &mut ubm2 };

        assert_eq!(va.eq_intervals(&vb), vma.eq_intervals(&vmb));
        assert_eq!(va.lt_intervals(&vb), vma.lt_intervals(&vmb));
        assert_eq!(va.gt_intervals(&vb), vma.gt_intervals(&vmb));
    }
}
