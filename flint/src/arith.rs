use num_traits::Float;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};
use std::simd::prelude::*;

use crate::next_up_down::NextUpDown;
use crate::{Flint, FlintArray, FlintMut, FlintRef, FlintVec, FlintView, FlintViewMut};

// -----------------------------------------------------------------------
// AsFlintSlice — internal borrow trait for FlintVec / FlintView ops,
// avoids cloning the entire vec when the rhs is a FlintView.
// -----------------------------------------------------------------------

trait AsFlintSlice<T> {
    fn lb_slice(&self) -> &[T];
    fn ub_slice(&self) -> &[T];
}

impl<T> AsFlintSlice<T> for FlintVec<T> {
    fn lb_slice(&self) -> &[T] {
        &self.lb
    }
    fn ub_slice(&self) -> &[T] {
        &self.ub
    }
}

impl<'a, T> AsFlintSlice<T> for FlintView<'a, T> {
    fn lb_slice(&self) -> &[T] {
        self.lb
    }
    fn ub_slice(&self) -> &[T] {
        self.ub
    }
}

impl<'a, T> AsFlintSlice<T> for FlintViewMut<'a, T> {
    fn lb_slice(&self) -> &[T] {
        self.lb
    }
    fn ub_slice(&self) -> &[T] {
        self.ub
    }
}

// -----------------------------------------------------------------------
// Unary negation
// IEEE 754 negation is exact for all finite values and ±infinity, so
// bounds are simply swapped without any ULP expansion.
// -----------------------------------------------------------------------

impl<T> Neg for Flint<T>
where
    T: Float,
{
    type Output = Flint<T>;
    /// Negate the interval by swapping and negating the bounds.
    fn neg(self) -> Flint<T> {
        Flint {
            lb: -self.ub,
            ub: -self.lb,
        }
    }
}

impl<'a, T> Neg for FlintRef<'a, T>
where
    T: Float,
{
    type Output = Flint<T>;
    /// Negate the interval by swapping and negating the bounds.
    fn neg(self) -> Flint<T> {
        Flint {
            lb: -*self.ub,
            ub: -*self.lb,
        }
    }
}

// -----------------------------------------------------------------------
// Addition
// result.lb = (self.lb + rhs.lb).nd()
// result.ub = (self.ub + rhs.ub).nu()
// -----------------------------------------------------------------------

impl<T, Rhs> Add<Rhs> for Flint<T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    type Output = Flint<T>;
    /// Add two intervals; expands the result by 1 ULP outward.
    fn add(self, rhs: Rhs) -> Flint<T> {
        let rhs: Flint<T> = rhs.into();
        Flint {
            lb: (self.lb + rhs.lb).nd(),
            ub: (self.ub + rhs.ub).nu(),
        }
    }
}

impl<'a, T, Rhs> Add<Rhs> for FlintRef<'a, T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    type Output = Flint<T>;
    /// Add two intervals; expands the result by 1 ULP outward.
    fn add(self, rhs: Rhs) -> Flint<T> {
        self.to_owned() + rhs
    }
}

impl<T, Rhs> AddAssign<Rhs> for Flint<T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    fn add_assign(&mut self, rhs: Rhs) {
        *self = *self + rhs;
    }
}

// -----------------------------------------------------------------------
// Subtraction
// result.lb = (self.lb - rhs.ub).nd()   // smallest possible difference
// result.ub = (self.ub - rhs.lb).nu()   // largest possible difference
// -----------------------------------------------------------------------

impl<T, Rhs> Sub<Rhs> for Flint<T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    type Output = Flint<T>;
    /// Subtract two intervals; expands the result by 1 ULP outward.
    fn sub(self, rhs: Rhs) -> Flint<T> {
        let rhs: Flint<T> = rhs.into();
        Flint {
            lb: (self.lb - rhs.ub).nd(),
            ub: (self.ub - rhs.lb).nu(),
        }
    }
}

impl<'a, T, Rhs> Sub<Rhs> for FlintRef<'a, T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    type Output = Flint<T>;
    /// Subtract two intervals; expands the result by 1 ULP outward.
    fn sub(self, rhs: Rhs) -> Flint<T> {
        self.to_owned() - rhs
    }
}

impl<T, Rhs> SubAssign<Rhs> for Flint<T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    fn sub_assign(&mut self, rhs: Rhs) {
        *self = *self - rhs;
    }
}

// -----------------------------------------------------------------------
// Multiplication
// Evaluates all four boundary products to handle sign changes correctly:
//   p = [self.lb*rhs.lb, self.lb*rhs.ub, self.ub*rhs.lb, self.ub*rhs.ub]
//   result.lb = min(p).nd()
//   result.ub = max(p).nu()
// -----------------------------------------------------------------------

impl<T, Rhs> Mul<Rhs> for Flint<T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    type Output = Flint<T>;
    /// Multiply two intervals using all four boundary products.
    fn mul(self, rhs: Rhs) -> Flint<T> {
        let rhs: Flint<T> = rhs.into();
        let p1 = self.lb * rhs.lb;
        let p2 = self.lb * rhs.ub;
        let p3 = self.ub * rhs.lb;
        let p4 = self.ub * rhs.ub;
        let lo = p1.min(p2).min(p3).min(p4);
        let hi = p1.max(p2).max(p3).max(p4);
        Flint {
            lb: lo.nd(),
            ub: hi.nu(),
        }
    }
}

impl<'a, T, Rhs> Mul<Rhs> for FlintRef<'a, T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    type Output = Flint<T>;
    /// Multiply two intervals using all four boundary products.
    fn mul(self, rhs: Rhs) -> Flint<T> {
        self.to_owned() * rhs
    }
}

impl<T, Rhs> MulAssign<Rhs> for Flint<T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    fn mul_assign(&mut self, rhs: Rhs) {
        *self = *self * rhs;
    }
}

// -----------------------------------------------------------------------
// Division
// Evaluates all four boundary quotients (same pattern as multiplication).
// Division by a zero-straddling interval yields ±infinity naturally via
// IEEE 754; no special casing is required.
// -----------------------------------------------------------------------

impl<T, Rhs> Div<Rhs> for Flint<T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    type Output = Flint<T>;
    /// Divide two intervals using all four boundary quotients.
    fn div(self, rhs: Rhs) -> Flint<T> {
        let rhs: Flint<T> = rhs.into();
        let q1 = self.lb / rhs.lb;
        let q2 = self.lb / rhs.ub;
        let q3 = self.ub / rhs.lb;
        let q4 = self.ub / rhs.ub;
        let lo = q1.min(q2).min(q3).min(q4);
        let hi = q1.max(q2).max(q3).max(q4);
        Flint {
            lb: lo.nd(),
            ub: hi.nu(),
        }
    }
}

impl<'a, T, Rhs> Div<Rhs> for FlintRef<'a, T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    type Output = Flint<T>;
    /// Divide two intervals using all four boundary quotients.
    fn div(self, rhs: Rhs) -> Flint<T> {
        self.to_owned() / rhs
    }
}

impl<T, Rhs> DivAssign<Rhs> for Flint<T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    fn div_assign(&mut self, rhs: Rhs) {
        *self = *self / rhs;
    }
}

// -----------------------------------------------------------------------
// FlintMut arithmetic
// Non-assign ops convert to owned and delegate; assign ops mutate the
// underlying floats through the &'a mut T fields.
// -----------------------------------------------------------------------

impl<'a, T> Neg for FlintMut<'a, T>
where
    T: Float,
{
    type Output = Flint<T>;
    fn neg(self) -> Flint<T> {
        -self.to_owned()
    }
}

impl<'a, T, Rhs> Add<Rhs> for FlintMut<'a, T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    type Output = Flint<T>;
    fn add(self, rhs: Rhs) -> Flint<T> {
        self.to_owned() + rhs
    }
}

impl<'a, T, Rhs> AddAssign<Rhs> for FlintMut<'a, T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    fn add_assign(&mut self, rhs: Rhs) {
        let result = self.to_owned() + rhs;
        *self.lb = result.lb;
        *self.ub = result.ub;
    }
}

impl<'a, T, Rhs> Sub<Rhs> for FlintMut<'a, T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    type Output = Flint<T>;
    fn sub(self, rhs: Rhs) -> Flint<T> {
        self.to_owned() - rhs
    }
}

impl<'a, T, Rhs> SubAssign<Rhs> for FlintMut<'a, T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    fn sub_assign(&mut self, rhs: Rhs) {
        let result = self.to_owned() - rhs;
        *self.lb = result.lb;
        *self.ub = result.ub;
    }
}

impl<'a, T, Rhs> Mul<Rhs> for FlintMut<'a, T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    type Output = Flint<T>;
    fn mul(self, rhs: Rhs) -> Flint<T> {
        self.to_owned() * rhs
    }
}

impl<'a, T, Rhs> MulAssign<Rhs> for FlintMut<'a, T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    fn mul_assign(&mut self, rhs: Rhs) {
        let result = self.to_owned() * rhs;
        *self.lb = result.lb;
        *self.ub = result.ub;
    }
}

impl<'a, T, Rhs> Div<Rhs> for FlintMut<'a, T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    type Output = Flint<T>;
    fn div(self, rhs: Rhs) -> Flint<T> {
        self.to_owned() / rhs
    }
}

impl<'a, T, Rhs> DivAssign<Rhs> for FlintMut<'a, T>
where
    T: Float + NextUpDown,
    Rhs: Into<Flint<T>>,
{
    fn div_assign(&mut self, rhs: Rhs) {
        let result = self.to_owned() / rhs;
        *self.lb = result.lb;
        *self.ub = result.ub;
    }
}

// -----------------------------------------------------------------------
// FlintArray arithmetic
// All ops are element-wise; multiplication and division use the four-
// boundary min/max pattern via SIMD simd_min/simd_max.
// -----------------------------------------------------------------------

impl<const N: usize> Neg for FlintArray<f32, N> {
    type Output = FlintArray<f32, N>;
    fn neg(self) -> FlintArray<f32, N> {
        let lb = -Simd::<f32, N>::from_array(self.ub);
        let ub = -Simd::<f32, N>::from_array(self.lb);
        FlintArray {
            lb: lb.to_array(),
            ub: ub.to_array(),
        }
    }
}

impl<const N: usize> Neg for FlintArray<f64, N> {
    type Output = FlintArray<f64, N>;
    fn neg(self) -> FlintArray<f64, N> {
        let lb = -Simd::<f64, N>::from_array(self.ub);
        let ub = -Simd::<f64, N>::from_array(self.lb);
        FlintArray {
            lb: lb.to_array(),
            ub: ub.to_array(),
        }
    }
}

macro_rules! impl_array_arith {
    ($T:ty) => {
        impl<const N: usize, Rhs> Add<Rhs> for FlintArray<$T, N>
        where
            Rhs: Into<FlintArray<$T, N>>,
        {
            type Output = FlintArray<$T, N>;
            fn add(self, rhs: Rhs) -> FlintArray<$T, N> {
                let rhs = rhs.into();
                let slb = Simd::<$T, N>::from_array(self.lb);
                let sub = Simd::<$T, N>::from_array(self.ub);
                let rlb = Simd::<$T, N>::from_array(rhs.lb);
                let rub = Simd::<$T, N>::from_array(rhs.ub);
                FlintArray {
                    lb: (slb + rlb).nd().to_array(),
                    ub: (sub + rub).nu().to_array(),
                }
            }
        }

        impl<const N: usize, Rhs> AddAssign<Rhs> for FlintArray<$T, N>
        where
            Rhs: Into<FlintArray<$T, N>>,
        {
            fn add_assign(&mut self, rhs: Rhs) {
                *self = *self + rhs;
            }
        }

        impl<const N: usize, Rhs> Sub<Rhs> for FlintArray<$T, N>
        where
            Rhs: Into<FlintArray<$T, N>>,
        {
            type Output = FlintArray<$T, N>;
            fn sub(self, rhs: Rhs) -> FlintArray<$T, N> {
                let rhs = rhs.into();
                let slb = Simd::<$T, N>::from_array(self.lb);
                let sub = Simd::<$T, N>::from_array(self.ub);
                let rlb = Simd::<$T, N>::from_array(rhs.lb);
                let rub = Simd::<$T, N>::from_array(rhs.ub);
                FlintArray {
                    lb: (slb - rub).nd().to_array(),
                    ub: (sub - rlb).nu().to_array(),
                }
            }
        }

        impl<const N: usize, Rhs> SubAssign<Rhs> for FlintArray<$T, N>
        where
            Rhs: Into<FlintArray<$T, N>>,
        {
            fn sub_assign(&mut self, rhs: Rhs) {
                *self = *self - rhs;
            }
        }

        impl<const N: usize, Rhs> Mul<Rhs> for FlintArray<$T, N>
        where
            Rhs: Into<FlintArray<$T, N>>,
        {
            type Output = FlintArray<$T, N>;
            fn mul(self, rhs: Rhs) -> FlintArray<$T, N> {
                let rhs = rhs.into();
                let slb = Simd::<$T, N>::from_array(self.lb);
                let sub = Simd::<$T, N>::from_array(self.ub);
                let rlb = Simd::<$T, N>::from_array(rhs.lb);
                let rub = Simd::<$T, N>::from_array(rhs.ub);
                let p1 = slb * rlb;
                let p2 = slb * rub;
                let p3 = sub * rlb;
                let p4 = sub * rub;
                let lo = p1.simd_min(p2).simd_min(p3).simd_min(p4);
                let hi = p1.simd_max(p2).simd_max(p3).simd_max(p4);
                FlintArray {
                    lb: lo.nd().to_array(),
                    ub: hi.nu().to_array(),
                }
            }
        }

        impl<const N: usize, Rhs> MulAssign<Rhs> for FlintArray<$T, N>
        where
            Rhs: Into<FlintArray<$T, N>>,
        {
            fn mul_assign(&mut self, rhs: Rhs) {
                *self = *self * rhs;
            }
        }

        impl<const N: usize, Rhs> Div<Rhs> for FlintArray<$T, N>
        where
            Rhs: Into<FlintArray<$T, N>>,
        {
            type Output = FlintArray<$T, N>;
            fn div(self, rhs: Rhs) -> FlintArray<$T, N> {
                let rhs = rhs.into();
                let slb = Simd::<$T, N>::from_array(self.lb);
                let sub = Simd::<$T, N>::from_array(self.ub);
                let rlb = Simd::<$T, N>::from_array(rhs.lb);
                let rub = Simd::<$T, N>::from_array(rhs.ub);
                let q1 = slb / rlb;
                let q2 = slb / rub;
                let q3 = sub / rlb;
                let q4 = sub / rub;
                let lo = q1.simd_min(q2).simd_min(q3).simd_min(q4);
                let hi = q1.simd_max(q2).simd_max(q3).simd_max(q4);
                FlintArray {
                    lb: lo.nd().to_array(),
                    ub: hi.nu().to_array(),
                }
            }
        }

        impl<const N: usize, Rhs> DivAssign<Rhs> for FlintArray<$T, N>
        where
            Rhs: Into<FlintArray<$T, N>>,
        {
            fn div_assign(&mut self, rhs: Rhs) {
                *self = *self / rhs;
            }
        }
    };
}

impl_array_arith!(f32);
impl_array_arith!(f64);

// -----------------------------------------------------------------------
// FlintVec / FlintView arithmetic
// Chunked SIMD (lane = 8) with scalar fallback for the remainder.
// FlintView binary ops return FlintVec (owned) since slices are immutable.
// FlintView has no assign ops.
// -----------------------------------------------------------------------

macro_rules! impl_vec_view_arith {
    ($T:ty, $S8:ty) => {
        impl Neg for FlintVec<$T> {
            type Output = FlintVec<$T>;
            fn neg(self) -> FlintVec<$T> {
                const L: usize = 8;
                let n = self.lb.len();
                let mut lb = Vec::with_capacity(n);
                let mut ub = Vec::with_capacity(n);
                let chunks = n / L;
                for i in 0..chunks {
                    let s = i * L;
                    let neg_lb = -<$S8>::from_slice(&self.ub[s..]);
                    let neg_ub = -<$S8>::from_slice(&self.lb[s..]);
                    lb.extend_from_slice(&neg_lb.to_array());
                    ub.extend_from_slice(&neg_ub.to_array());
                }
                for j in (chunks * L)..n {
                    lb.push(-self.ub[j]);
                    ub.push(-self.lb[j]);
                }
                FlintVec { lb, ub }
            }
        }

        impl<Rhs: AsFlintSlice<$T>> Add<Rhs> for FlintVec<$T> {
            type Output = FlintVec<$T>;
            fn add(self, rhs: Rhs) -> FlintVec<$T> {
                const L: usize = 8;
                let n = self.lb.len();
                let rlb = rhs.lb_slice();
                let rub = rhs.ub_slice();
                let mut lb = Vec::with_capacity(n);
                let mut ub = Vec::with_capacity(n);
                let chunks = n / L;
                for i in 0..chunks {
                    let s = i * L;
                    let lo = <$S8>::from_slice(&self.lb[s..]) + <$S8>::from_slice(&rlb[s..]);
                    let hi = <$S8>::from_slice(&self.ub[s..]) + <$S8>::from_slice(&rub[s..]);
                    lb.extend_from_slice(&lo.nd().to_array());
                    ub.extend_from_slice(&hi.nu().to_array());
                }
                for j in (chunks * L)..n {
                    lb.push((self.lb[j] + rlb[j]).nd());
                    ub.push((self.ub[j] + rub[j]).nu());
                }
                FlintVec { lb, ub }
            }
        }

        impl<Rhs: AsFlintSlice<$T>> AddAssign<Rhs> for FlintVec<$T> {
            fn add_assign(&mut self, rhs: Rhs) {
                // replace self by consuming it; safe since self is &mut
                let owned = std::mem::replace(self, FlintVec { lb: vec![], ub: vec![] });
                *self = owned + rhs;
            }
        }

        impl<Rhs: AsFlintSlice<$T>> Sub<Rhs> for FlintVec<$T> {
            type Output = FlintVec<$T>;
            fn sub(self, rhs: Rhs) -> FlintVec<$T> {
                const L: usize = 8;
                let n = self.lb.len();
                let rlb = rhs.lb_slice();
                let rub = rhs.ub_slice();
                let mut lb = Vec::with_capacity(n);
                let mut ub = Vec::with_capacity(n);
                let chunks = n / L;
                for i in 0..chunks {
                    let s = i * L;
                    let lo = <$S8>::from_slice(&self.lb[s..]) - <$S8>::from_slice(&rub[s..]);
                    let hi = <$S8>::from_slice(&self.ub[s..]) - <$S8>::from_slice(&rlb[s..]);
                    lb.extend_from_slice(&lo.nd().to_array());
                    ub.extend_from_slice(&hi.nu().to_array());
                }
                for j in (chunks * L)..n {
                    lb.push((self.lb[j] - rub[j]).nd());
                    ub.push((self.ub[j] - rlb[j]).nu());
                }
                FlintVec { lb, ub }
            }
        }

        impl<Rhs: AsFlintSlice<$T>> SubAssign<Rhs> for FlintVec<$T> {
            fn sub_assign(&mut self, rhs: Rhs) {
                let owned = std::mem::replace(self, FlintVec { lb: vec![], ub: vec![] });
                *self = owned - rhs;
            }
        }

        impl<Rhs: AsFlintSlice<$T>> Mul<Rhs> for FlintVec<$T> {
            type Output = FlintVec<$T>;
            fn mul(self, rhs: Rhs) -> FlintVec<$T> {
                const L: usize = 8;
                let n = self.lb.len();
                let rlb = rhs.lb_slice();
                let rub = rhs.ub_slice();
                let mut lb = Vec::with_capacity(n);
                let mut ub = Vec::with_capacity(n);
                let chunks = n / L;
                for i in 0..chunks {
                    let s = i * L;
                    let slb = <$S8>::from_slice(&self.lb[s..]);
                    let sub = <$S8>::from_slice(&self.ub[s..]);
                    let rl = <$S8>::from_slice(&rlb[s..]);
                    let ru = <$S8>::from_slice(&rub[s..]);
                    let p1 = slb * rl;
                    let p2 = slb * ru;
                    let p3 = sub * rl;
                    let p4 = sub * ru;
                    let lo = p1.simd_min(p2).simd_min(p3).simd_min(p4);
                    let hi = p1.simd_max(p2).simd_max(p3).simd_max(p4);
                    lb.extend_from_slice(&lo.nd().to_array());
                    ub.extend_from_slice(&hi.nu().to_array());
                }
                for j in (chunks * L)..n {
                    let p1 = self.lb[j] * rlb[j];
                    let p2 = self.lb[j] * rub[j];
                    let p3 = self.ub[j] * rlb[j];
                    let p4 = self.ub[j] * rub[j];
                    lb.push(p1.min(p2).min(p3).min(p4).nd());
                    ub.push(p1.max(p2).max(p3).max(p4).nu());
                }
                FlintVec { lb, ub }
            }
        }

        impl<Rhs: AsFlintSlice<$T>> MulAssign<Rhs> for FlintVec<$T> {
            fn mul_assign(&mut self, rhs: Rhs) {
                let owned = std::mem::replace(self, FlintVec { lb: vec![], ub: vec![] });
                *self = owned * rhs;
            }
        }

        impl<Rhs: AsFlintSlice<$T>> Div<Rhs> for FlintVec<$T> {
            type Output = FlintVec<$T>;
            fn div(self, rhs: Rhs) -> FlintVec<$T> {
                const L: usize = 8;
                let n = self.lb.len();
                let rlb = rhs.lb_slice();
                let rub = rhs.ub_slice();
                let mut lb = Vec::with_capacity(n);
                let mut ub = Vec::with_capacity(n);
                let chunks = n / L;
                for i in 0..chunks {
                    let s = i * L;
                    let slb = <$S8>::from_slice(&self.lb[s..]);
                    let sub = <$S8>::from_slice(&self.ub[s..]);
                    let rl = <$S8>::from_slice(&rlb[s..]);
                    let ru = <$S8>::from_slice(&rub[s..]);
                    let q1 = slb / rl;
                    let q2 = slb / ru;
                    let q3 = sub / rl;
                    let q4 = sub / ru;
                    let lo = q1.simd_min(q2).simd_min(q3).simd_min(q4);
                    let hi = q1.simd_max(q2).simd_max(q3).simd_max(q4);
                    lb.extend_from_slice(&lo.nd().to_array());
                    ub.extend_from_slice(&hi.nu().to_array());
                }
                for j in (chunks * L)..n {
                    let q1 = self.lb[j] / rlb[j];
                    let q2 = self.lb[j] / rub[j];
                    let q3 = self.ub[j] / rlb[j];
                    let q4 = self.ub[j] / rub[j];
                    lb.push(q1.min(q2).min(q3).min(q4).nd());
                    ub.push(q1.max(q2).max(q3).max(q4).nu());
                }
                FlintVec { lb, ub }
            }
        }

        impl<Rhs: AsFlintSlice<$T>> DivAssign<Rhs> for FlintVec<$T> {
            fn div_assign(&mut self, rhs: Rhs) {
                let owned = std::mem::replace(self, FlintVec { lb: vec![], ub: vec![] });
                *self = owned / rhs;
            }
        }

        impl<'a> Neg for FlintView<'a, $T> {
            type Output = FlintVec<$T>;
            fn neg(self) -> FlintVec<$T> {
                const L: usize = 8;
                let n = self.lb.len();
                let mut lb = Vec::with_capacity(n);
                let mut ub = Vec::with_capacity(n);
                let chunks = n / L;
                for i in 0..chunks {
                    let s = i * L;
                    let neg_lb = -<$S8>::from_slice(&self.ub[s..]);
                    let neg_ub = -<$S8>::from_slice(&self.lb[s..]);
                    lb.extend_from_slice(&neg_lb.to_array());
                    ub.extend_from_slice(&neg_ub.to_array());
                }
                for j in (chunks * L)..n {
                    lb.push(-self.ub[j]);
                    ub.push(-self.lb[j]);
                }
                FlintVec { lb, ub }
            }
        }

        impl<'a, Rhs: AsFlintSlice<$T>> Add<Rhs> for FlintView<'a, $T> {
            type Output = FlintVec<$T>;
            fn add(self, rhs: Rhs) -> FlintVec<$T> {
                const L: usize = 8;
                let n = self.lb.len();
                let rlb = rhs.lb_slice();
                let rub = rhs.ub_slice();
                let mut lb = Vec::with_capacity(n);
                let mut ub = Vec::with_capacity(n);
                let chunks = n / L;
                for i in 0..chunks {
                    let s = i * L;
                    let lo = <$S8>::from_slice(&self.lb[s..]) + <$S8>::from_slice(&rlb[s..]);
                    let hi = <$S8>::from_slice(&self.ub[s..]) + <$S8>::from_slice(&rub[s..]);
                    lb.extend_from_slice(&lo.nd().to_array());
                    ub.extend_from_slice(&hi.nu().to_array());
                }
                for j in (chunks * L)..n {
                    lb.push((self.lb[j] + rlb[j]).nd());
                    ub.push((self.ub[j] + rub[j]).nu());
                }
                FlintVec { lb, ub }
            }
        }

        impl<'a, Rhs: AsFlintSlice<$T>> Sub<Rhs> for FlintView<'a, $T> {
            type Output = FlintVec<$T>;
            fn sub(self, rhs: Rhs) -> FlintVec<$T> {
                const L: usize = 8;
                let n = self.lb.len();
                let rlb = rhs.lb_slice();
                let rub = rhs.ub_slice();
                let mut lb = Vec::with_capacity(n);
                let mut ub = Vec::with_capacity(n);
                let chunks = n / L;
                for i in 0..chunks {
                    let s = i * L;
                    let lo = <$S8>::from_slice(&self.lb[s..]) - <$S8>::from_slice(&rub[s..]);
                    let hi = <$S8>::from_slice(&self.ub[s..]) - <$S8>::from_slice(&rlb[s..]);
                    lb.extend_from_slice(&lo.nd().to_array());
                    ub.extend_from_slice(&hi.nu().to_array());
                }
                for j in (chunks * L)..n {
                    lb.push((self.lb[j] - rub[j]).nd());
                    ub.push((self.ub[j] - rlb[j]).nu());
                }
                FlintVec { lb, ub }
            }
        }

        impl<'a, Rhs: AsFlintSlice<$T>> Mul<Rhs> for FlintView<'a, $T> {
            type Output = FlintVec<$T>;
            fn mul(self, rhs: Rhs) -> FlintVec<$T> {
                const L: usize = 8;
                let n = self.lb.len();
                let rlb = rhs.lb_slice();
                let rub = rhs.ub_slice();
                let mut lb = Vec::with_capacity(n);
                let mut ub = Vec::with_capacity(n);
                let chunks = n / L;
                for i in 0..chunks {
                    let s = i * L;
                    let slb = <$S8>::from_slice(&self.lb[s..]);
                    let sub = <$S8>::from_slice(&self.ub[s..]);
                    let rl = <$S8>::from_slice(&rlb[s..]);
                    let ru = <$S8>::from_slice(&rub[s..]);
                    let p1 = slb * rl;
                    let p2 = slb * ru;
                    let p3 = sub * rl;
                    let p4 = sub * ru;
                    let lo = p1.simd_min(p2).simd_min(p3).simd_min(p4);
                    let hi = p1.simd_max(p2).simd_max(p3).simd_max(p4);
                    lb.extend_from_slice(&lo.nd().to_array());
                    ub.extend_from_slice(&hi.nu().to_array());
                }
                for j in (chunks * L)..n {
                    let p1 = self.lb[j] * rlb[j];
                    let p2 = self.lb[j] * rub[j];
                    let p3 = self.ub[j] * rlb[j];
                    let p4 = self.ub[j] * rub[j];
                    lb.push(p1.min(p2).min(p3).min(p4).nd());
                    ub.push(p1.max(p2).max(p3).max(p4).nu());
                }
                FlintVec { lb, ub }
            }
        }

        impl<'a, Rhs: AsFlintSlice<$T>> Div<Rhs> for FlintView<'a, $T> {
            type Output = FlintVec<$T>;
            fn div(self, rhs: Rhs) -> FlintVec<$T> {
                const L: usize = 8;
                let n = self.lb.len();
                let rlb = rhs.lb_slice();
                let rub = rhs.ub_slice();
                let mut lb = Vec::with_capacity(n);
                let mut ub = Vec::with_capacity(n);
                let chunks = n / L;
                for i in 0..chunks {
                    let s = i * L;
                    let slb = <$S8>::from_slice(&self.lb[s..]);
                    let sub = <$S8>::from_slice(&self.ub[s..]);
                    let rl = <$S8>::from_slice(&rlb[s..]);
                    let ru = <$S8>::from_slice(&rub[s..]);
                    let q1 = slb / rl;
                    let q2 = slb / ru;
                    let q3 = sub / rl;
                    let q4 = sub / ru;
                    let lo = q1.simd_min(q2).simd_min(q3).simd_min(q4);
                    let hi = q1.simd_max(q2).simd_max(q3).simd_max(q4);
                    lb.extend_from_slice(&lo.nd().to_array());
                    ub.extend_from_slice(&hi.nu().to_array());
                }
                for j in (chunks * L)..n {
                    let q1 = self.lb[j] / rlb[j];
                    let q2 = self.lb[j] / rub[j];
                    let q3 = self.ub[j] / rlb[j];
                    let q4 = self.ub[j] / rub[j];
                    lb.push(q1.min(q2).min(q3).min(q4).nd());
                    ub.push(q1.max(q2).max(q3).max(q4).nu());
                }
                FlintVec { lb, ub }
            }
        }

        // -----------------------------------------------------------------------
        // FlintViewMut arithmetic
        // Non-assign ops delegate to FlintView (returning FlintVec).
        // Assign ops mutate the underlying slices in place using the same chunked
        // SIMD pattern as FlintVec.
        // -----------------------------------------------------------------------

        impl<'a> Neg for FlintViewMut<'a, $T> {
            type Output = FlintVec<$T>;
            fn neg(self) -> FlintVec<$T> {
                -(FlintView::<$T> { lb: self.lb, ub: self.ub })
            }
        }

        impl<'a, Rhs: AsFlintSlice<$T>> Add<Rhs> for FlintViewMut<'a, $T> {
            type Output = FlintVec<$T>;
            fn add(self, rhs: Rhs) -> FlintVec<$T> {
                FlintView::<$T> { lb: self.lb, ub: self.ub } + rhs
            }
        }

        impl<Rhs: AsFlintSlice<$T>> AddAssign<Rhs> for FlintViewMut<'_, $T> {
            fn add_assign(&mut self, rhs: Rhs) {
                const L: usize = 8;
                let n = self.lb.len();
                let rlb = rhs.lb_slice();
                let rub = rhs.ub_slice();
                let chunks = n / L;
                for i in 0..chunks {
                    let s = i * L;
                    let lo = (<$S8>::from_slice(&self.lb[s..]) + <$S8>::from_slice(&rlb[s..])).nd();
                    let hi = (<$S8>::from_slice(&self.ub[s..]) + <$S8>::from_slice(&rub[s..])).nu();
                    self.lb[s..s + L].copy_from_slice(&lo.to_array());
                    self.ub[s..s + L].copy_from_slice(&hi.to_array());
                }
                for j in (chunks * L)..n {
                    self.lb[j] = (self.lb[j] + rlb[j]).nd();
                    self.ub[j] = (self.ub[j] + rub[j]).nu();
                }
            }
        }

        impl<'a, Rhs: AsFlintSlice<$T>> Sub<Rhs> for FlintViewMut<'a, $T> {
            type Output = FlintVec<$T>;
            fn sub(self, rhs: Rhs) -> FlintVec<$T> {
                FlintView::<$T> { lb: self.lb, ub: self.ub } - rhs
            }
        }

        impl<Rhs: AsFlintSlice<$T>> SubAssign<Rhs> for FlintViewMut<'_, $T> {
            fn sub_assign(&mut self, rhs: Rhs) {
                const L: usize = 8;
                let n = self.lb.len();
                let rlb = rhs.lb_slice();
                let rub = rhs.ub_slice();
                let chunks = n / L;
                for i in 0..chunks {
                    let s = i * L;
                    let lo = (<$S8>::from_slice(&self.lb[s..]) - <$S8>::from_slice(&rub[s..])).nd();
                    let hi = (<$S8>::from_slice(&self.ub[s..]) - <$S8>::from_slice(&rlb[s..])).nu();
                    self.lb[s..s + L].copy_from_slice(&lo.to_array());
                    self.ub[s..s + L].copy_from_slice(&hi.to_array());
                }
                for j in (chunks * L)..n {
                    self.lb[j] = (self.lb[j] - rub[j]).nd();
                    self.ub[j] = (self.ub[j] - rlb[j]).nu();
                }
            }
        }

        impl<'a, Rhs: AsFlintSlice<$T>> Mul<Rhs> for FlintViewMut<'a, $T> {
            type Output = FlintVec<$T>;
            fn mul(self, rhs: Rhs) -> FlintVec<$T> {
                FlintView::<$T> { lb: self.lb, ub: self.ub } * rhs
            }
        }

        impl<Rhs: AsFlintSlice<$T>> MulAssign<Rhs> for FlintViewMut<'_, $T> {
            fn mul_assign(&mut self, rhs: Rhs) {
                const L: usize = 8;
                let n = self.lb.len();
                let rlb = rhs.lb_slice();
                let rub = rhs.ub_slice();
                let chunks = n / L;
                for i in 0..chunks {
                    let s = i * L;
                    let slb = <$S8>::from_slice(&self.lb[s..]);
                    let sub = <$S8>::from_slice(&self.ub[s..]);
                    let rl = <$S8>::from_slice(&rlb[s..]);
                    let ru = <$S8>::from_slice(&rub[s..]);
                    let p1 = slb * rl;
                    let p2 = slb * ru;
                    let p3 = sub * rl;
                    let p4 = sub * ru;
                    let lo = p1.simd_min(p2).simd_min(p3).simd_min(p4).nd();
                    let hi = p1.simd_max(p2).simd_max(p3).simd_max(p4).nu();
                    self.lb[s..s + L].copy_from_slice(&lo.to_array());
                    self.ub[s..s + L].copy_from_slice(&hi.to_array());
                }
                for j in (chunks * L)..n {
                    let p1 = self.lb[j] * rlb[j];
                    let p2 = self.lb[j] * rub[j];
                    let p3 = self.ub[j] * rlb[j];
                    let p4 = self.ub[j] * rub[j];
                    self.lb[j] = p1.min(p2).min(p3).min(p4).nd();
                    self.ub[j] = p1.max(p2).max(p3).max(p4).nu();
                }
            }
        }

        impl<'a, Rhs: AsFlintSlice<$T>> Div<Rhs> for FlintViewMut<'a, $T> {
            type Output = FlintVec<$T>;
            fn div(self, rhs: Rhs) -> FlintVec<$T> {
                FlintView::<$T> { lb: self.lb, ub: self.ub } / rhs
            }
        }

        impl<Rhs: AsFlintSlice<$T>> DivAssign<Rhs> for FlintViewMut<'_, $T> {
            fn div_assign(&mut self, rhs: Rhs) {
                const L: usize = 8;
                let n = self.lb.len();
                let rlb = rhs.lb_slice();
                let rub = rhs.ub_slice();
                let chunks = n / L;
                for i in 0..chunks {
                    let s = i * L;
                    let slb = <$S8>::from_slice(&self.lb[s..]);
                    let sub = <$S8>::from_slice(&self.ub[s..]);
                    let rl = <$S8>::from_slice(&rlb[s..]);
                    let ru = <$S8>::from_slice(&rub[s..]);
                    let q1 = slb / rl;
                    let q2 = slb / ru;
                    let q3 = sub / rl;
                    let q4 = sub / ru;
                    let lo = q1.simd_min(q2).simd_min(q3).simd_min(q4).nd();
                    let hi = q1.simd_max(q2).simd_max(q3).simd_max(q4).nu();
                    self.lb[s..s + L].copy_from_slice(&lo.to_array());
                    self.ub[s..s + L].copy_from_slice(&hi.to_array());
                }
                for j in (chunks * L)..n {
                    let q1 = self.lb[j] / rlb[j];
                    let q2 = self.lb[j] / rub[j];
                    let q3 = self.ub[j] / rlb[j];
                    let q4 = self.ub[j] / rub[j];
                    self.lb[j] = q1.min(q2).min(q3).min(q4).nd();
                    self.ub[j] = q1.max(q2).max(q3).max(q4).nu();
                }
            }
        }
    };
}

impl_vec_view_arith!(f32, f32x8);
impl_vec_view_arith!(f64, f64x8);

#[cfg(test)]
mod test {
    use super::*;
    use crate::{flint32, flint32_arr, flint32_vec, flint64, flint64_arr, flint64_vec};

    fn width<T: std::ops::Sub<Output = T>>(f: Flint<T>) -> T {
        f.ub - f.lb
    }

    // ---- Negation ----

    #[test]
    fn test_neg_flint() {
        let a: Flint<f64> = Flint { lb: 1.0, ub: 2.0 };
        let neg_a = -a;
        assert_eq!(neg_a.lb, -2.0_f64);
        assert_eq!(neg_a.ub, -1.0_f64);
    }

    #[test]
    fn test_neg_flintref() {
        let a: Flint<f64> = Flint { lb: 1.0, ub: 2.0 };
        let neg_a = -a.as_ref();
        assert_eq!(neg_a.lb, -2.0_f64);
        assert_eq!(neg_a.ub, -1.0_f64);
    }

    #[test]
    fn test_neg_straddle_zero() {
        // negating a zero-straddling interval returns the same interval
        let z: Flint<f32> = Flint { lb: -1.0, ub: 1.0 };
        let neg_z = -z;
        assert_eq!(neg_z.lb, -1.0_f32);
        assert_eq!(neg_z.ub, 1.0_f32);
    }

    #[test]
    fn test_neg_negative_interval() {
        let n: Flint<f64> = Flint { lb: -5.0, ub: -3.0 };
        let neg_n = -n;
        assert_eq!(neg_n.lb, 3.0_f64);
        assert_eq!(neg_n.ub, 5.0_f64);
    }

    // ---- Addition ----

    #[test]
    fn test_add_exact_integers() {
        // integer intervals within exact representation range are zero-width;
        // addition of exact values stays exact (up to ULP rounding at boundaries)
        let sum = flint64!(1_i32) + flint64!(2_i32);
        assert_eq!(sum, 3_i32);
    }

    #[test]
    fn test_add_interval_growth() {
        // each addition widens the interval by at least 1 ULP on each side
        let w0 = width(flint64!(0.2_f64));
        let w1 = width(flint64!(0.2_f64) + flint64!(0.2_f64));
        assert!(w1 > w0, "interval should grow after addition");
    }

    #[test]
    fn test_add_captures_0p2_x3_equals_0p6() {
        // Regardless of native float behavior, the Flint interval for
        // 0.2 + 0.2 + 0.2 must overlap the exact value 0.6.
        let sum = flint32!(0.2_f32) + flint32!(0.2_f32) + flint32!(0.2_f32);
        assert_eq!(sum, 0.6_f32);
    }

    #[test]
    fn test_add_interval_contains_exact_result_f64() {
        // Classic f64 pitfall: 0.1 + 0.2 is not exactly 0.3 in IEEE 754.
        assert_ne!(0.1_f64 + 0.2_f64, 0.3_f64);
        // With Flint, the interval for 0.1 + 0.2 must contain 0.3.
        let sum = flint64!(0.1_f64) + flint64!(0.2_f64);
        assert_eq!(sum, 0.3_f64);
    }

    #[test]
    fn test_add_flintref_rhs() {
        let a = flint64!(1.0_f64);
        let b = flint64!(2.0_f64);
        assert_eq!(a + b.as_ref(), 3.0_f64);
    }

    #[test]
    fn test_add_assign() {
        let mut x = flint32!(1.0_f32);
        x += flint32!(2.0_f32);
        assert_eq!(x, 3.0_f32);
    }

    // ---- Subtraction ----

    #[test]
    fn test_sub_floats() {
        let diff = flint32!(0.6_f32) - flint32!(0.2_f32);
        assert_eq!(diff, 0.4_f32);
    }

    #[test]
    fn test_sub_exact_integers() {
        let diff = flint64!(5_i32) - flint64!(3_i32);
        assert_eq!(diff, 2_i32);
    }

    #[test]
    fn test_sub_assign() {
        let mut x = flint64!(5.0_f64);
        x -= flint64!(3.0_f64);
        assert_eq!(x, 2.0_f64);
    }

    // ---- Multiplication ----

    #[test]
    fn test_mul_positive() {
        let p = flint64!(3.0_f64) * flint64!(4.0_f64);
        assert_eq!(p, 12.0_f64);
    }

    #[test]
    fn test_mul_sign_change() {
        // [-2, 3] * [1, 2]: products are -4, -2, 3, 6 → result straddles [-4, 6]
        let a: Flint<f64> = Flint { lb: -2.0, ub: 3.0 };
        let b: Flint<f64> = Flint { lb: 1.0, ub: 2.0 };
        let p = a * b;
        assert!(p.lb <= -4.0_f64, "lb should be ≤ -4.0 (got {})", p.lb);
        assert!(p.ub >= 6.0_f64, "ub should be ≥ 6.0 (got {})", p.ub);
    }

    #[test]
    fn test_mul_assign() {
        let mut x = flint32!(3.0_f32);
        x *= flint32!(4.0_f32);
        assert_eq!(x, 12.0_f32);
    }

    // ---- Division ----

    #[test]
    fn test_div_third() {
        // 1/3 is not exactly representable; the interval must contain the true value
        let q = flint64!(1.0_f64) / flint64!(3.0_f64);
        assert_eq!(q, 1.0_f64 / 3.0_f64);
    }

    #[test]
    fn test_div_exact() {
        let q = flint64!(6_i32) / flint64!(2_i32);
        assert_eq!(q, 3_i32);
    }

    #[test]
    fn test_div_assign() {
        let mut x = flint32!(6.0_f32);
        x /= flint32!(2.0_f32);
        assert_eq!(x, 3.0_f32);
    }

    // ---- FlintArray ----

    fn arr_width_f32<const N: usize>(a: FlintArray<f32, N>) -> [f32; N] {
        let mut w = [0.0_f32; N];
        for i in 0..N {
            w[i] = a.ub[i] - a.lb[i];
        }
        w
    }

    #[test]
    fn test_array_neg() {
        let a = flint32_arr!(1.0_f32, -2.0_f32, 3.0_f32);
        let neg_a = -a;
        // lb should be ≤ -original_ub and ub ≥ -original_lb
        assert!(neg_a.lb[0] <= -a.ub[0] && neg_a.ub[0] >= -a.lb[0]);
        assert!(neg_a.lb[1] <= -a.ub[1] && neg_a.ub[1] >= -a.lb[1]);
        assert!(neg_a.lb[2] <= -a.ub[2] && neg_a.ub[2] >= -a.lb[2]);
    }

    #[test]
    fn test_array_add_exact_integers() {
        let a = flint32_arr!(1_i32, 2_i32, 3_i32);
        let b = flint32_arr!(4_i32, 5_i32, 6_i32);
        let sum = a + b;
        // exact integer addition must contain the true result
        assert!(sum.lb[0] <= 5.0_f32 && 5.0_f32 <= sum.ub[0]);
        assert!(sum.lb[1] <= 7.0_f32 && 7.0_f32 <= sum.ub[1]);
        assert!(sum.lb[2] <= 9.0_f32 && 9.0_f32 <= sum.ub[2]);
    }

    #[test]
    fn test_array_add_interval_growth() {
        // 0.2 + 0.2 interval must widen vs a single 0.2 interval
        let w0 = arr_width_f32(flint32_arr!(0.2_f32))[0];
        let w1 = arr_width_f32(flint32_arr!(0.2_f32) + flint32_arr!(0.2_f32))[0];
        assert!(w1 > w0, "array interval should grow after addition");
    }

    #[test]
    fn test_array_add_captures_0p1_plus_0p2_f64() {
        assert_ne!(0.1_f64 + 0.2_f64, 0.3_f64);
        let a = flint64_arr!(0.1_f64, 0.1_f64);
        let b = flint64_arr!(0.2_f64, 0.2_f64);
        let sum = a + b;
        assert!(sum.lb[0] <= 0.3_f64 && 0.3_f64 <= sum.ub[0]);
    }

    #[test]
    fn test_array_sub() {
        let a = flint32_arr!(0.6_f32, 1.0_f32);
        let b = flint32_arr!(0.2_f32, 0.5_f32);
        let diff = a - b;
        assert!(diff.lb[0] <= 0.4_f32 && 0.4_f32 <= diff.ub[0]);
        assert!(diff.lb[1] <= 0.5_f32 && 0.5_f32 <= diff.ub[1]);
    }

    #[test]
    fn test_array_mul_sign_change() {
        // element 0: [-2,3] * [1,2] → lb ≤ -4, ub ≥ 6
        let a = FlintArray::<f64, 1> { lb: [-2.0], ub: [3.0] };
        let b = FlintArray::<f64, 1> { lb: [1.0], ub: [2.0] };
        let p = a * b;
        assert!(p.lb[0] <= -4.0_f64, "lb should be ≤ -4.0 (got {})", p.lb[0]);
        assert!(p.ub[0] >= 6.0_f64, "ub should be ≥ 6.0 (got {})", p.ub[0]);
    }

    #[test]
    fn test_array_mul_positive() {
        let a = flint64_arr!(3.0_f64, 2.0_f64);
        let b = flint64_arr!(4.0_f64, 5.0_f64);
        let p = a * b;
        assert!(p.lb[0] <= 12.0_f64 && 12.0_f64 <= p.ub[0]);
        assert!(p.lb[1] <= 10.0_f64 && 10.0_f64 <= p.ub[1]);
    }

    #[test]
    fn test_array_div_third() {
        let a = flint64_arr!(1.0_f64, 2.0_f64);
        let b = flint64_arr!(3.0_f64, 4.0_f64);
        let q = a / b;
        assert!(q.lb[0] <= 1.0_f64 / 3.0_f64 && 1.0_f64 / 3.0_f64 <= q.ub[0]);
        assert!(q.lb[1] <= 0.5_f64 && 0.5_f64 <= q.ub[1]);
    }

    #[test]
    fn test_array_add_assign() {
        let mut a = flint32_arr!(1.0_f32, 2.0_f32);
        a += flint32_arr!(3.0_f32, 4.0_f32);
        assert!(a.lb[0] <= 4.0_f32 && 4.0_f32 <= a.ub[0]);
        assert!(a.lb[1] <= 6.0_f32 && 6.0_f32 <= a.ub[1]);
    }

    #[test]
    fn test_array_rhs_from_raw_array() {
        // Rhs: Into<FlintArray<f32,2>> should accept [f32; 2]
        let a = flint32_arr!(1.0_f32, 2.0_f32);
        let sum = a + [3.0_f32, 4.0_f32];
        assert!(sum.lb[0] <= 4.0_f32 && 4.0_f32 <= sum.ub[0]);
        assert!(sum.lb[1] <= 6.0_f32 && 6.0_f32 <= sum.ub[1]);
    }

    // ---- FlintVec ----

    #[test]
    fn test_vec_neg() {
        let v = flint32_vec![1.0_f32, -2.0_f32, 3.0_f32];
        let neg_v = -v.clone();
        for i in 0..3 {
            assert!(neg_v.lb[i] <= -v.ub[i] && neg_v.ub[i] >= -v.lb[i]);
        }
    }

    #[test]
    fn test_vec_add_exact_integers() {
        let a = flint32_vec![1, 2, 3];
        let b = flint32_vec![4, 5, 6];
        let sum = a + b;
        assert!(sum.lb[0] <= 5.0_f32 && 5.0_f32 <= sum.ub[0]);
        assert!(sum.lb[1] <= 7.0_f32 && 7.0_f32 <= sum.ub[1]);
        assert!(sum.lb[2] <= 9.0_f32 && 9.0_f32 <= sum.ub[2]);
    }

    #[test]
    fn test_vec_add_interval_growth() {
        let w0 = flint64_vec![0.2_f64].ub[0] - flint64_vec![0.2_f64].lb[0];
        let a = flint64_vec![0.2_f64];
        let b = flint64_vec![0.2_f64];
        let sum = a + b;
        let w1 = sum.ub[0] - sum.lb[0];
        assert!(w1 > w0, "vec interval should grow after addition");
    }

    #[test]
    fn test_vec_mul_sign_change() {
        let a = FlintVec::<f64> { lb: vec![-2.0], ub: vec![3.0] };
        let b = FlintVec::<f64> { lb: vec![1.0], ub: vec![2.0] };
        let p = a * b;
        assert!(p.lb[0] <= -4.0_f64);
        assert!(p.ub[0] >= 6.0_f64);
    }

    #[test]
    fn test_vec_div_third() {
        let a = flint64_vec![1.0_f64];
        let b = flint64_vec![3.0_f64];
        let q = a / b;
        assert!(q.lb[0] <= 1.0_f64 / 3.0_f64 && 1.0_f64 / 3.0_f64 <= q.ub[0]);
    }

    #[test]
    fn test_vec_add_assign() {
        let mut a = flint32_vec![1.0_f32, 2.0_f32];
        let b = flint32_vec![3.0_f32, 4.0_f32];
        a += b;
        assert!(a.lb[0] <= 4.0_f32 && 4.0_f32 <= a.ub[0]);
        assert!(a.lb[1] <= 6.0_f32 && 6.0_f32 <= a.ub[1]);
    }

    #[test]
    fn test_vec_chunked_remainder_length_9() {
        // length 9 = one full chunk of 8 + one scalar remainder
        let a = flint64_vec![1.0_f64, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let b = flint64_vec![2.0_f64, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0];
        let sum = a + b;
        assert_eq!(sum.lb.len(), 9);
        for i in 0..9 {
            assert!(sum.lb[i] <= 3.0_f64 && 3.0_f64 <= sum.ub[i]);
        }
    }

    // ---- FlintView as RHS ----

    #[test]
    fn test_vec_add_view_rhs() {
        let a = flint32_vec![1.0_f32, 2.0_f32];
        let b = flint32_vec![3.0_f32, 4.0_f32];
        let view = FlintView { lb: &b.lb, ub: &b.ub };
        let sum = a + view;
        assert!(sum.lb[0] <= 4.0_f32 && 4.0_f32 <= sum.ub[0]);
        assert!(sum.lb[1] <= 6.0_f32 && 6.0_f32 <= sum.ub[1]);
    }

    #[test]
    fn test_view_neg() {
        let v = flint32_vec![1.0_f32, -2.0_f32];
        let view = FlintView { lb: &v.lb, ub: &v.ub };
        let neg_view = -view;
        assert!(neg_view.lb[0] <= -v.ub[0] && neg_view.ub[0] >= -v.lb[0]);
        assert!(neg_view.lb[1] <= -v.ub[1] && neg_view.ub[1] >= -v.lb[1]);
    }

    #[test]
    fn test_view_add() {
        let a = flint64_vec![1.0_f64, 2.0_f64];
        let b = flint64_vec![3.0_f64, 4.0_f64];
        let va = FlintView { lb: &a.lb, ub: &a.ub };
        let vb = FlintView { lb: &b.lb, ub: &b.ub };
        let sum = va + vb;
        assert!(sum.lb[0] <= 4.0_f64 && 4.0_f64 <= sum.ub[0]);
        assert!(sum.lb[1] <= 6.0_f64 && 6.0_f64 <= sum.ub[1]);
    }

    // ---- FlintMut scalar arithmetic ----

    #[test]
    fn test_mut_neg() {
        let a = flint64!(2.0_f64);
        let mut lb = a.lb;
        let mut ub = a.ub;
        let m = FlintMut { lb: &mut lb, ub: &mut ub };
        let neg = -m;
        assert!(neg.lb <= -2.0_f64 && -2.0_f64 <= neg.ub);
        assert!(neg.lb < 0.0 && neg.ub < 0.0);
    }

    #[test]
    fn test_mut_add_sub() {
        let a = flint64!(1.0_f64);
        let b = flint64!(2.0_f64);
        let mut lb = a.lb;
        let mut ub = a.ub;
        let m = FlintMut { lb: &mut lb, ub: &mut ub };
        let sum = m + b;
        assert!(sum.lb <= 3.0_f64 && 3.0_f64 <= sum.ub);

        let mut lb2 = a.lb;
        let mut ub2 = a.ub;
        let m2 = FlintMut { lb: &mut lb2, ub: &mut ub2 };
        let diff = m2 - flint64!(0.5_f64);
        assert!(diff.lb <= 0.5_f64 && 0.5_f64 <= diff.ub);
    }

    #[test]
    fn test_mut_mul_div() {
        let a = flint64!(3.0_f64);
        let mut lb = a.lb;
        let mut ub = a.ub;
        let m = FlintMut { lb: &mut lb, ub: &mut ub };
        let prod = m * flint64!(2.0_f64);
        assert!(prod.lb <= 6.0_f64 && 6.0_f64 <= prod.ub);

        let mut lb2 = a.lb;
        let mut ub2 = a.ub;
        let m2 = FlintMut { lb: &mut lb2, ub: &mut ub2 };
        let quot = m2 / flint64!(2.0_f64);
        assert!(quot.lb <= 1.5_f64 && 1.5_f64 <= quot.ub);
    }

    #[test]
    fn test_mut_assign_ops() {
        let a = flint64!(1.0_f64);
        let mut lb = a.lb;
        let mut ub = a.ub;

        // add_assign
        {
            let mut m = FlintMut { lb: &mut lb, ub: &mut ub };
            m += flint64!(2.0_f64);
        }
        assert!(lb <= 3.0_f64 && 3.0_f64 <= ub);

        // sub_assign
        {
            let mut m = FlintMut { lb: &mut lb, ub: &mut ub };
            m -= flint64!(1.0_f64);
        }
        assert!(lb <= 2.0_f64 && 2.0_f64 <= ub);

        // mul_assign
        {
            let mut m = FlintMut { lb: &mut lb, ub: &mut ub };
            m *= flint64!(3.0_f64);
        }
        assert!(lb <= 6.0_f64 && 6.0_f64 <= ub);

        // div_assign
        {
            let mut m = FlintMut { lb: &mut lb, ub: &mut ub };
            m /= flint64!(2.0_f64);
        }
        assert!(lb <= 3.0_f64 && 3.0_f64 <= ub);
    }

    #[test]
    fn test_mut_assign_writes_through() {
        // Verify the assign op actually mutates the underlying floats,
        // not just local copies.
        let mut val_lb = 1.0_f32.nd();
        let mut val_ub = 1.0_f32.nu();
        let original_lb = val_lb;
        {
            let mut m = FlintMut { lb: &mut val_lb, ub: &mut val_ub };
            m += flint32!(1.0_f32);
        }
        assert!(val_lb != original_lb, "lb should have changed after add_assign");
        assert!(val_lb <= 2.0_f32 && 2.0_f32 <= val_ub);
    }

    // ---- FlintViewMut array arithmetic ----

    #[test]
    fn test_view_mut_neg() {
        let v = flint64_vec![1.0_f64, -2.0_f64];
        let mut lb = v.lb.clone();
        let mut ub = v.ub.clone();
        let vm = FlintViewMut { lb: &mut lb, ub: &mut ub };
        let neg = -vm;
        assert!(neg.lb[0] <= -1.0_f64 && -1.0_f64 <= neg.ub[0]);
        assert!(neg.lb[1] <= 2.0_f64 && 2.0_f64 <= neg.ub[1]);
    }

    #[test]
    fn test_view_mut_add_matches_view() {
        // FlintViewMut + rhs should give the same result as FlintView + rhs
        let a = flint64_vec![1.0_f64, 2.0_f64, 3.0_f64];
        let b = flint64_vec![4.0_f64, 5.0_f64, 6.0_f64];

        let va = FlintView { lb: &a.lb, ub: &a.ub };
        let vb = FlintView { lb: &b.lb, ub: &b.ub };
        let expected = va + vb;

        let mut lb_m = a.lb.clone();
        let mut ub_m = a.ub.clone();
        let vma = FlintViewMut { lb: &mut lb_m, ub: &mut ub_m };
        let vb2 = FlintView { lb: &b.lb, ub: &b.ub };
        let result = vma + vb2;

        assert_eq!(expected.lb, result.lb);
        assert_eq!(expected.ub, result.ub);
    }

    #[test]
    fn test_view_mut_add_assign() {
        let a = flint64_vec![1.0_f64, 2.0_f64, 3.0_f64];
        let b = flint64_vec![4.0_f64, 5.0_f64, 6.0_f64];
        let mut lb = a.lb.clone();
        let mut ub = a.ub.clone();
        {
            let mut vm = FlintViewMut { lb: &mut lb, ub: &mut ub };
            vm += FlintView { lb: &b.lb, ub: &b.ub };
        }
        assert!(lb[0] <= 5.0_f64 && 5.0_f64 <= ub[0]);
        assert!(lb[1] <= 7.0_f64 && 7.0_f64 <= ub[1]);
        assert!(lb[2] <= 9.0_f64 && 9.0_f64 <= ub[2]);
    }

    #[test]
    fn test_view_mut_sub_assign() {
        let a = flint64_vec![5.0_f64, 6.0_f64];
        let b = flint64_vec![1.0_f64, 2.0_f64];
        let mut lb = a.lb.clone();
        let mut ub = a.ub.clone();
        {
            let mut vm = FlintViewMut { lb: &mut lb, ub: &mut ub };
            vm -= FlintView { lb: &b.lb, ub: &b.ub };
        }
        assert!(lb[0] <= 4.0_f64 && 4.0_f64 <= ub[0]);
        assert!(lb[1] <= 4.0_f64 && 4.0_f64 <= ub[1]);
    }

    #[test]
    fn test_view_mut_mul_assign() {
        let a = flint64_vec![2.0_f64, 3.0_f64];
        let b = flint64_vec![4.0_f64, 5.0_f64];
        let mut lb = a.lb.clone();
        let mut ub = a.ub.clone();
        {
            let mut vm = FlintViewMut { lb: &mut lb, ub: &mut ub };
            vm *= FlintView { lb: &b.lb, ub: &b.ub };
        }
        assert!(lb[0] <= 8.0_f64 && 8.0_f64 <= ub[0]);
        assert!(lb[1] <= 15.0_f64 && 15.0_f64 <= ub[1]);
    }

    #[test]
    fn test_view_mut_div_assign() {
        let a = flint64_vec![6.0_f64, 9.0_f64];
        let b = flint64_vec![2.0_f64, 3.0_f64];
        let mut lb = a.lb.clone();
        let mut ub = a.ub.clone();
        {
            let mut vm = FlintViewMut { lb: &mut lb, ub: &mut ub };
            vm /= FlintView { lb: &b.lb, ub: &b.ub };
        }
        assert!(lb[0] <= 3.0_f64 && 3.0_f64 <= ub[0]);
        assert!(lb[1] <= 3.0_f64 && 3.0_f64 <= ub[1]);
    }

    #[test]
    fn test_view_mut_assign_writes_through() {
        // Verify the assign op mutates the underlying vec data
        let a = flint32_vec![1.0_f32, 2.0_f32, 3.0_f32];
        let b = flint32_vec![1.0_f32, 1.0_f32, 1.0_f32];
        let mut lb = a.lb.clone();
        let mut ub = a.ub.clone();
        let original_lb0 = lb[0];
        {
            let mut vm = FlintViewMut { lb: &mut lb, ub: &mut ub };
            vm += FlintView { lb: &b.lb, ub: &b.ub };
        }
        assert!(lb[0] != original_lb0, "lb[0] should have changed");
        assert!(lb[0] <= 2.0_f32 && 2.0_f32 <= ub[0]);
        assert!(lb[1] <= 3.0_f32 && 3.0_f32 <= ub[1]);
        assert!(lb[2] <= 4.0_f32 && 4.0_f32 <= ub[2]);
    }
}
