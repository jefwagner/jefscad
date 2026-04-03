use num_traits::{Float, FloatConst};

use crate::next_up_down::NextUpDown;
use crate::{Flint, FlintArray, FlintVec, FlintView};

// -----------------------------------------------------------------------
// Internal helpers
// -----------------------------------------------------------------------

#[inline]
fn fmin4<T: PartialOrd + Copy>(a: T, b: T, c: T, d: T) -> T {
    let lo = if a <= b { a } else { b };
    let lo = if lo <= c { lo } else { c };
    if lo <= d { lo } else { d }
}

#[inline]
fn fmax4<T: PartialOrd + Copy>(a: T, b: T, c: T, d: T) -> T {
    let hi = if a >= b { a } else { b };
    let hi = if hi >= c { hi } else { c };
    if hi >= d { hi } else { d }
}

// -----------------------------------------------------------------------
// Scalar Flint<T> — standard math functions
// (no FloatConst required)
// -----------------------------------------------------------------------

impl<T> Flint<T>
where
    T: Float + NextUpDown + Copy,
{
    // --- Special value queries ---

    /// True if either bound is NaN.
    pub fn is_nan(&self) -> bool {
        self.lb.is_nan() || self.ub.is_nan()
    }

    /// True if either bound is infinite.
    pub fn is_infinite(&self) -> bool {
        self.lb.is_infinite() || self.ub.is_infinite()
    }

    /// True if both bounds are finite.
    pub fn is_finite(&self) -> bool {
        self.lb.is_finite() && self.ub.is_finite()
    }

    // --- Absolute value ---

    pub fn abs(&self) -> Flint<T> {
        if self.ub < T::zero() {
            // entirely negative: flip
            Flint { lb: -self.ub, ub: -self.lb }
        } else if self.lb < T::zero() {
            // straddles zero: lb = 0, ub = max of |lb|, ub
            let max_abs = if -self.lb > self.ub { -self.lb } else { self.ub };
            Flint { lb: T::zero(), ub: max_abs }
        } else {
            *self
        }
    }

    // --- Power / root functions ---

    /// Square root. Returns NaN interval for inputs entirely below 0.
    /// For inputs straddling 0, clamps lb to 0.
    pub fn sqrt(&self) -> Flint<T> {
        if self.ub < T::zero() {
            Flint { lb: T::nan(), ub: T::nan() }
        } else if self.lb < T::zero() {
            Flint { lb: T::zero(), ub: self.ub.sqrt().nu() }
        } else {
            Flint { lb: self.lb.sqrt().nd(), ub: self.ub.sqrt().nu() }
        }
    }

    /// Cube root. Monotonically increasing, defined for all reals.
    pub fn cbrt(&self) -> Flint<T> {
        Flint { lb: self.lb.cbrt().nd(), ub: self.ub.cbrt().nu() }
    }

    /// General power `self ^ exp`. Returns NaN if any corner product is NaN
    /// (e.g. negative base with non-integer exponent).
    pub fn powf<R: Into<Flint<T>>>(&self, exp: R) -> Flint<T> {
        let e = exp.into();
        let p00 = self.lb.powf(e.lb);
        let p01 = self.lb.powf(e.ub);
        let p10 = self.ub.powf(e.lb);
        let p11 = self.ub.powf(e.ub);
        if p00.is_nan() || p01.is_nan() || p10.is_nan() || p11.is_nan() {
            Flint { lb: T::nan(), ub: T::nan() }
        } else {
            Flint {
                lb: fmin4(p00, p01, p10, p11).nd(),
                ub: fmax4(p00, p01, p10, p11).nu(),
            }
        }
    }

    /// `sqrt(self² + other²)`. Handles sign correctly for all quadrants.
    pub fn hypot<R: Into<Flint<T>>>(&self, other: R) -> Flint<T> {
        let other = other.into();
        // For each argument find the value with minimum and maximum absolute value.
        let (f1_lo, f1_hi) = if self.lb < T::zero() {
            if self.ub < T::zero() {
                (-self.ub, -self.lb) // entirely negative
            } else {
                let max = if -self.lb > self.ub { -self.lb } else { self.ub };
                (T::zero(), max)
            }
        } else {
            (self.lb, self.ub)
        };
        let (f2_lo, f2_hi) = if other.lb < T::zero() {
            if other.ub < T::zero() {
                (-other.ub, -other.lb)
            } else {
                let max = if -other.lb > other.ub { -other.lb } else { other.ub };
                (T::zero(), max)
            }
        } else {
            (other.lb, other.ub)
        };
        let lo = f1_lo.hypot(f2_lo);
        // hypot(0,0) = 0 exactly; otherwise round outward
        let lo = if lo == T::zero() { T::zero() } else { lo.nd() };
        Flint { lb: lo, ub: f1_hi.hypot(f2_hi).nu() }
    }

    // --- Exponential functions (all monotonically increasing) ---

    pub fn exp(&self) -> Flint<T> {
        Flint { lb: self.lb.exp().nd(), ub: self.ub.exp().nu() }
    }

    pub fn exp2(&self) -> Flint<T> {
        Flint { lb: self.lb.exp2().nd(), ub: self.ub.exp2().nu() }
    }

    /// `e^x - 1`, accurate near zero.
    pub fn exp_m1(&self) -> Flint<T> {
        Flint { lb: self.lb.exp_m1().nd(), ub: self.ub.exp_m1().nu() }
    }

    // --- Log functions ---

    /// Natural log. Returns NaN for inputs entirely ≤ 0; -∞ lb for inputs straddling 0.
    pub fn ln(&self) -> Flint<T> {
        if self.ub < T::zero() {
            Flint { lb: T::nan(), ub: T::nan() }
        } else if self.lb < T::zero() {
            Flint { lb: T::neg_infinity(), ub: self.ub.ln().nu() }
        } else {
            Flint { lb: self.lb.ln().nd(), ub: self.ub.ln().nu() }
        }
    }

    /// log base 2.
    pub fn log2(&self) -> Flint<T> {
        if self.ub < T::zero() {
            Flint { lb: T::nan(), ub: T::nan() }
        } else if self.lb < T::zero() {
            Flint { lb: T::neg_infinity(), ub: self.ub.log2().nu() }
        } else {
            Flint { lb: self.lb.log2().nd(), ub: self.ub.log2().nu() }
        }
    }

    /// log base 10.
    pub fn log10(&self) -> Flint<T> {
        if self.ub < T::zero() {
            Flint { lb: T::nan(), ub: T::nan() }
        } else if self.lb < T::zero() {
            Flint { lb: T::neg_infinity(), ub: self.ub.log10().nu() }
        } else {
            Flint { lb: self.lb.log10().nd(), ub: self.ub.log10().nu() }
        }
    }

    /// `ln(1 + x)`, accurate near zero. Domain: (-1, ∞).
    pub fn ln_1p(&self) -> Flint<T> {
        if self.ub < -T::one() {
            Flint { lb: T::nan(), ub: T::nan() }
        } else if self.lb < -T::one() {
            Flint { lb: T::neg_infinity(), ub: self.ub.ln_1p().nu() }
        } else {
            Flint { lb: self.lb.ln_1p().nd(), ub: self.ub.ln_1p().nu() }
        }
    }

    /// Log with arbitrary interval base. Returns NaN if self is entirely negative
    /// or if the base produces NaN results.
    pub fn log<R: Into<Flint<T>>>(&self, base: R) -> Flint<T> {
        let b = base.into();
        let fnan = Flint { lb: T::nan(), ub: T::nan() };
        if self.ub < T::zero() {
            return fnan;
        }
        if self.lb < T::zero() {
            // interval straddles 0: lower bound is -∞
            let v1 = self.ub.log(b.lb);
            let v2 = self.ub.log(b.ub);
            if v1.is_nan() || v2.is_nan() {
                return fnan;
            }
            let hi = if v1 > v2 { v1 } else { v2 };
            Flint { lb: T::neg_infinity(), ub: hi.nu() }
        } else {
            let p00 = self.lb.log(b.lb);
            let p01 = self.lb.log(b.ub);
            let p10 = self.ub.log(b.lb);
            let p11 = self.ub.log(b.ub);
            if p00.is_nan() || p01.is_nan() || p10.is_nan() || p11.is_nan() {
                fnan
            } else {
                Flint {
                    lb: fmin4(p00, p01, p10, p11).nd(),
                    ub: fmax4(p00, p01, p10, p11).nu(),
                }
            }
        }
    }

    // --- Hyperbolic functions ---

    /// sinh: monotonically increasing.
    pub fn sinh(&self) -> Flint<T> {
        Flint { lb: self.lb.sinh().nd(), ub: self.ub.sinh().nu() }
    }

    /// cosh: minimum at x=0. If interval straddles 0, lb = 1.
    pub fn cosh(&self) -> Flint<T> {
        let c1 = self.lb.cosh();
        let c2 = self.ub.cosh();
        let (lo, hi) = if c1 < c2 { (c1, c2) } else { (c2, c1) };
        let lb = if self.lb <= T::zero() && self.ub >= T::zero() {
            T::one()
        } else {
            lo.nd()
        };
        Flint { lb, ub: hi.nu() }
    }

    /// tanh: monotonically increasing.
    pub fn tanh(&self) -> Flint<T> {
        Flint { lb: self.lb.tanh().nd(), ub: self.ub.tanh().nu() }
    }

    // --- Inverse hyperbolic ---

    /// asinh: monotonically increasing, domain all reals.
    pub fn asinh(&self) -> Flint<T> {
        Flint { lb: self.lb.asinh().nd(), ub: self.ub.asinh().nu() }
    }

    /// acosh: monotonically increasing on [1, ∞). Returns NaN for inputs entirely < 1.
    pub fn acosh(&self) -> Flint<T> {
        if self.ub < T::one() {
            Flint { lb: T::nan(), ub: T::nan() }
        } else if self.lb < T::one() {
            Flint { lb: T::zero(), ub: self.ub.acosh().nu() }
        } else {
            Flint { lb: self.lb.acosh().nd(), ub: self.ub.acosh().nu() }
        }
    }

    /// atanh: monotonically increasing on (-1, 1). Returns ±∞ at boundaries.
    pub fn atanh(&self) -> Flint<T> {
        if self.ub < -T::one() || self.lb > T::one() {
            Flint { lb: T::nan(), ub: T::nan() }
        } else {
            let lb = if self.lb < -T::one() { T::neg_infinity() } else { self.lb.atanh().nd() };
            let ub = if self.ub > T::one() { T::infinity() } else { self.ub.atanh().nu() };
            Flint { lb, ub }
        }
    }

    /// atan: monotonically increasing, defined for all reals.
    pub fn atan(&self) -> Flint<T> {
        Flint { lb: self.lb.atan().nd(), ub: self.ub.atan().nu() }
    }
}

// -----------------------------------------------------------------------
// Scalar Flint<T> — trig functions (require FloatConst for π constants)
// -----------------------------------------------------------------------

impl<T> Flint<T>
where
    T: Float + NextUpDown + Copy + FloatConst,
{
    /// sin: detects maxima (π/2 + 2kπ) and minima (3π/2 + 2kπ) within the interval.
    pub fn sin(&self) -> Flint<T> {
        let tau = T::TAU();
        let pi_2 = T::FRAC_PI_2();
        let three = T::one() + T::one() + T::one();
        let n = (self.lb / tau).floor();
        let da = self.lb - n * tau;
        let db = self.ub - n * tau;
        let s1 = self.lb.sin();
        let s2 = self.ub.sin();
        let (mut lo, mut hi) = if s1 < s2 { (s1.nd(), s2.nu()) } else { (s2.nd(), s1.nu()) };
        if da <= pi_2 && db > pi_2 {
            hi = T::one();
        } else if da <= three * pi_2 {
            if db > three * pi_2 { lo = -T::one(); }
            if db > T::from(5u8).unwrap() * pi_2 { hi = T::one(); }
        } else {
            if db > T::from(5u8).unwrap() * pi_2 { hi = T::one(); }
            if db > T::from(7u8).unwrap() * pi_2 { lo = -T::one(); }
        }
        Flint { lb: lo, ub: hi }
    }

    /// cos: detects maximum (0, 2kπ) and minimum (π + 2kπ) within the interval.
    pub fn cos(&self) -> Flint<T> {
        let tau = T::TAU();
        let pi = T::PI();
        let three = T::one() + T::one() + T::one();
        let n = (self.lb / tau).floor();
        let da = self.lb - n * tau;
        let db = self.ub - n * tau;
        let c1 = self.lb.cos();
        let c2 = self.ub.cos();
        let (mut lo, mut hi) = if c1 < c2 { (c1.nd(), c2.nu()) } else { (c2.nd(), c1.nu()) };
        if da <= pi && db >= pi {
            lo = -T::one();
            if db > tau { hi = T::one(); }
        } else {
            if db > tau { hi = T::one(); }
            if db > three * pi { lo = -T::one(); }
        }
        Flint { lb: lo, ub: hi }
    }

    /// tan: returns `[+∞, -∞]` (a conventional "all-spanning" sentinel) if the interval
    /// crosses a discontinuity at π/2 + kπ or is wider than π.
    pub fn tan(&self) -> Flint<T> {
        let pi = T::PI();
        let ta = self.lb.tan();
        let tb = self.ub.tan();
        if ta > tb || (self.ub - self.lb) > pi {
            Flint { lb: T::infinity(), ub: T::neg_infinity() }
        } else {
            Flint { lb: ta.nd(), ub: tb.nu() }
        }
    }

    /// asin: monotonically increasing on [-1, 1], range [-π/2, π/2].
    /// Clamps out-of-domain endpoints rather than returning NaN for partial overlaps.
    pub fn asin(&self) -> Flint<T> {
        let pi_2 = T::FRAC_PI_2();
        if self.ub < -T::one() || self.lb > T::one() {
            Flint { lb: T::nan(), ub: T::nan() }
        } else {
            let lb = if self.lb < -T::one() { -pi_2 } else { self.lb.asin().nd() };
            let ub = if self.ub > T::one() { pi_2 } else { self.ub.asin().nu() };
            Flint { lb, ub }
        }
    }

    /// acos: monotonically **decreasing** on [-1, 1], range [0, π].
    /// Note: lb of result = acos(ub of input), ub of result = acos(lb of input).
    pub fn acos(&self) -> Flint<T> {
        let pi = T::PI();
        if self.ub < -T::one() || self.lb > T::one() {
            Flint { lb: T::nan(), ub: T::nan() }
        } else {
            // acos decreases, so the output bounds are reversed relative to input.
            let ub = if self.lb < -T::one() { pi } else { self.lb.acos().nu() };
            let lb = if self.ub > T::one() { T::zero() } else { self.ub.acos().nd() };
            Flint { lb, ub }
        }
    }

    /// atan2(self, x): returns the angle in [-π, π]. Handles all quadrant combinations
    /// and the discontinuity at the negative real axis.
    pub fn atan2<R: Into<Flint<T>>>(&self, other: R) -> Flint<T> {
        let tau = T::TAU();
        let pi = T::PI();
        let y = self;
        let x: Flint<T> = other.into();
        let (a, b) = if y.lb > T::zero() {
            // y > 0
            if x.lb > T::zero() {
                (y.lb.atan2(x.ub), y.ub.atan2(x.lb))
            } else if x.ub > T::zero() {
                // x crosses 0, y > 0
                (y.lb.atan2(x.ub), y.lb.atan2(x.lb))
            } else {
                // x < 0, y > 0
                (y.ub.atan2(x.ub), y.lb.atan2(x.lb))
            }
        } else if y.ub > T::zero() {
            // y crosses 0
            if x.lb > T::zero() {
                (y.lb.atan2(x.lb), y.ub.atan2(x.lb))
            } else if x.ub > T::zero() {
                // x and y both cross 0: full circle
                return Flint { lb: -pi, ub: pi };
            } else {
                // x < 0, y crosses 0: interval crosses ±π branch cut
                (y.ub.atan2(x.ub), y.lb.atan2(x.ub) + tau)
            }
        } else {
            // y <= 0
            if x.lb > T::zero() {
                (y.lb.atan2(x.lb), y.ub.atan2(x.ub))
            } else if x.ub > T::zero() {
                // x crosses 0, y <= 0
                (y.ub.atan2(x.lb), y.ub.atan2(x.ub))
            } else {
                // x < 0, y <= 0
                (y.ub.atan2(x.lb), y.lb.atan2(x.ub))
            }
        };
        Flint { lb: a.nd(), ub: b.nu() }
    }
}

// -----------------------------------------------------------------------
// Macro: generate element-wise methods on FlintArray, FlintVec, FlintView
// -----------------------------------------------------------------------

macro_rules! impl_unary_for_arrays {
    // Without FloatConst
    ($func:ident) => {
        impl<T: Float + NextUpDown + Copy, const N: usize> FlintArray<T, N> {
            pub fn $func(&self) -> FlintArray<T, N> {
                let mut lb = [T::zero(); N];
                let mut ub = [T::zero(); N];
                for i in 0..N {
                    let r = Flint { lb: self.lb[i], ub: self.ub[i] }.$func();
                    lb[i] = r.lb;
                    ub[i] = r.ub;
                }
                FlintArray { lb, ub }
            }
        }
        impl<T: Float + NextUpDown + Copy> FlintVec<T> {
            pub fn $func(&self) -> FlintVec<T> {
                let mut lb = Vec::with_capacity(self.lb.len());
                let mut ub = Vec::with_capacity(self.ub.len());
                for i in 0..self.lb.len() {
                    let r = Flint { lb: self.lb[i], ub: self.ub[i] }.$func();
                    lb.push(r.lb);
                    ub.push(r.ub);
                }
                FlintVec { lb, ub }
            }
        }
        impl<'a, T: Float + NextUpDown + Copy> FlintView<'a, T> {
            pub fn $func(&self) -> FlintVec<T> {
                let mut lb = Vec::with_capacity(self.lb.len());
                let mut ub = Vec::with_capacity(self.ub.len());
                for i in 0..self.lb.len() {
                    let r = Flint { lb: self.lb[i], ub: self.ub[i] }.$func();
                    lb.push(r.lb);
                    ub.push(r.ub);
                }
                FlintVec { lb, ub }
            }
        }
    };
    // With FloatConst
    ($func:ident, floatconst) => {
        impl<T: Float + NextUpDown + Copy + FloatConst, const N: usize> FlintArray<T, N> {
            pub fn $func(&self) -> FlintArray<T, N> {
                let mut lb = [T::zero(); N];
                let mut ub = [T::zero(); N];
                for i in 0..N {
                    let r = Flint { lb: self.lb[i], ub: self.ub[i] }.$func();
                    lb[i] = r.lb;
                    ub[i] = r.ub;
                }
                FlintArray { lb, ub }
            }
        }
        impl<T: Float + NextUpDown + Copy + FloatConst> FlintVec<T> {
            pub fn $func(&self) -> FlintVec<T> {
                let mut lb = Vec::with_capacity(self.lb.len());
                let mut ub = Vec::with_capacity(self.ub.len());
                for i in 0..self.lb.len() {
                    let r = Flint { lb: self.lb[i], ub: self.ub[i] }.$func();
                    lb.push(r.lb);
                    ub.push(r.ub);
                }
                FlintVec { lb, ub }
            }
        }
        impl<'a, T: Float + NextUpDown + Copy + FloatConst> FlintView<'a, T> {
            pub fn $func(&self) -> FlintVec<T> {
                let mut lb = Vec::with_capacity(self.lb.len());
                let mut ub = Vec::with_capacity(self.ub.len());
                for i in 0..self.lb.len() {
                    let r = Flint { lb: self.lb[i], ub: self.ub[i] }.$func();
                    lb.push(r.lb);
                    ub.push(r.ub);
                }
                FlintVec { lb, ub }
            }
        }
    };
}

impl_unary_for_arrays!(abs);
impl_unary_for_arrays!(sqrt);
impl_unary_for_arrays!(cbrt);
impl_unary_for_arrays!(exp);
impl_unary_for_arrays!(exp2);
impl_unary_for_arrays!(exp_m1);
impl_unary_for_arrays!(ln);
impl_unary_for_arrays!(log2);
impl_unary_for_arrays!(log10);
impl_unary_for_arrays!(ln_1p);
impl_unary_for_arrays!(sinh);
impl_unary_for_arrays!(cosh);
impl_unary_for_arrays!(tanh);
impl_unary_for_arrays!(asinh);
impl_unary_for_arrays!(acosh);
impl_unary_for_arrays!(atanh);
impl_unary_for_arrays!(atan);
impl_unary_for_arrays!(sin, floatconst);
impl_unary_for_arrays!(cos, floatconst);
impl_unary_for_arrays!(tan, floatconst);
impl_unary_for_arrays!(asin, floatconst);
impl_unary_for_arrays!(acos, floatconst);

// -----------------------------------------------------------------------
// Binary functions on array types (scalar-broadcast second argument)
// -----------------------------------------------------------------------

impl<T: Float + NextUpDown + Copy, const N: usize> FlintArray<T, N> {
    pub fn powf<R: Into<Flint<T>> + Copy>(&self, exp: R) -> FlintArray<T, N> {
        let mut lb = [T::zero(); N];
        let mut ub = [T::zero(); N];
        for i in 0..N {
            let r = Flint { lb: self.lb[i], ub: self.ub[i] }.powf(exp);
            lb[i] = r.lb;
            ub[i] = r.ub;
        }
        FlintArray { lb, ub }
    }

    pub fn hypot<R: Into<Flint<T>> + Copy>(&self, other: R) -> FlintArray<T, N> {
        let mut lb = [T::zero(); N];
        let mut ub = [T::zero(); N];
        for i in 0..N {
            let r = Flint { lb: self.lb[i], ub: self.ub[i] }.hypot(other);
            lb[i] = r.lb;
            ub[i] = r.ub;
        }
        FlintArray { lb, ub }
    }

    pub fn log<R: Into<Flint<T>> + Copy>(&self, base: R) -> FlintArray<T, N> {
        let mut lb = [T::zero(); N];
        let mut ub = [T::zero(); N];
        for i in 0..N {
            let r = Flint { lb: self.lb[i], ub: self.ub[i] }.log(base);
            lb[i] = r.lb;
            ub[i] = r.ub;
        }
        FlintArray { lb, ub }
    }
}

impl<T: Float + NextUpDown + Copy + FloatConst, const N: usize> FlintArray<T, N> {
    pub fn atan2<R: Into<Flint<T>> + Copy>(&self, other: R) -> FlintArray<T, N> {
        let mut lb = [T::zero(); N];
        let mut ub = [T::zero(); N];
        for i in 0..N {
            let r = Flint { lb: self.lb[i], ub: self.ub[i] }.atan2(other);
            lb[i] = r.lb;
            ub[i] = r.ub;
        }
        FlintArray { lb, ub }
    }
}

impl<T: Float + NextUpDown + Copy> FlintVec<T> {
    pub fn powf<R: Into<Flint<T>> + Copy>(&self, exp: R) -> FlintVec<T> {
        let mut lb = Vec::with_capacity(self.lb.len());
        let mut ub = Vec::with_capacity(self.ub.len());
        for i in 0..self.lb.len() {
            let r = Flint { lb: self.lb[i], ub: self.ub[i] }.powf(exp);
            lb.push(r.lb);
            ub.push(r.ub);
        }
        FlintVec { lb, ub }
    }

    pub fn hypot<R: Into<Flint<T>> + Copy>(&self, other: R) -> FlintVec<T> {
        let mut lb = Vec::with_capacity(self.lb.len());
        let mut ub = Vec::with_capacity(self.ub.len());
        for i in 0..self.lb.len() {
            let r = Flint { lb: self.lb[i], ub: self.ub[i] }.hypot(other);
            lb.push(r.lb);
            ub.push(r.ub);
        }
        FlintVec { lb, ub }
    }

    pub fn log<R: Into<Flint<T>> + Copy>(&self, base: R) -> FlintVec<T> {
        let mut lb = Vec::with_capacity(self.lb.len());
        let mut ub = Vec::with_capacity(self.ub.len());
        for i in 0..self.lb.len() {
            let r = Flint { lb: self.lb[i], ub: self.ub[i] }.log(base);
            lb.push(r.lb);
            ub.push(r.ub);
        }
        FlintVec { lb, ub }
    }
}

impl<T: Float + NextUpDown + Copy + FloatConst> FlintVec<T> {
    pub fn atan2<R: Into<Flint<T>> + Copy>(&self, other: R) -> FlintVec<T> {
        let mut lb = Vec::with_capacity(self.lb.len());
        let mut ub = Vec::with_capacity(self.ub.len());
        for i in 0..self.lb.len() {
            let r = Flint { lb: self.lb[i], ub: self.ub[i] }.atan2(other);
            lb.push(r.lb);
            ub.push(r.ub);
        }
        FlintVec { lb, ub }
    }
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use crate::{flint64, flint64_arr, flint64_vec};
    use std::f64::consts;

    // Helper: check Flint<f64> contains a value.
    fn contains(f: Flint<f64>, v: f64) -> bool {
        f.lb <= v && v <= f.ub
    }

    // Helper: check Flint<f64> is a valid (non-inverted) interval.
    fn valid(f: Flint<f64>) -> bool {
        f.lb <= f.ub
    }

    // --- Special value queries ---

    #[test]
    fn test_special_values() {
        let f: Flint<f64> = 0.0_f64.into();
        assert!(!f.is_nan());
        assert!(f.is_finite());
        assert!(!f.is_infinite());

        let f: Flint<f64> = 1.0_f64.into();
        assert!(!f.is_nan());
        assert!(f.is_finite());
        assert!(!f.is_infinite());

        let f = Flint { lb: 0.0_f64, ub: f64::INFINITY };
        assert!(!f.is_nan());
        assert!(!f.is_finite());
        assert!(f.is_infinite());

        let f = Flint { lb: f64::NAN, ub: f64::NAN };
        assert!(f.is_nan());
        assert!(!f.is_finite());
        assert!(!f.is_infinite());
    }

    // --- abs ---

    #[test]
    fn test_abs() {
        // entirely negative
        let x: Flint<f64> = (-2_i32).into();
        let y = x.abs();
        assert!(contains(y, 2.0));
        assert_eq!(y.ub - y.lb, x.ub - x.lb); // same width

        // entirely positive
        let x: Flint<f64> = (2_i32).into();
        assert!(contains(x.abs(), 2.0));

        // straddles zero: lb = 0, ub = max(|lb|, ub)
        let x = Flint { lb: -1.0_f64, ub: 1.0_f64 };
        let y = x.abs();
        assert_eq!(0.0, y.lb);
        assert_eq!(1.0, y.ub);
    }

    // --- powf / sqrt / cbrt ---

    #[test]
    fn test_pow() {
        let x: Flint<f64> = 4_i32.into();
        assert!(contains(x.powf(2_i32), 16.0));
        assert!(contains(x.powf(0.5_f64), 2.0));
        // negative base, non-integer exp → NaN
        let x: Flint<f64> = (-1_i32).into();
        assert!(x.powf(0.5_f64).is_nan());
    }

    #[test]
    fn test_sqrt() {
        let x: Flint<f64> = 4_i32.into();
        let y = x.sqrt();
        assert!(y.ub > y.lb); // nonzero width from rounding
        assert!(contains(y, 2.0));

        let x = Flint { lb: -1.0_f64, ub: 4.0_f64 };
        let y = x.sqrt();
        assert_eq!(0.0, y.lb);
        assert!(contains(y, 2.0));

        let x: Flint<f64> = (-14_i32).into();
        assert!(x.sqrt().is_nan());
    }

    #[test]
    fn test_cbrt() {
        let x: Flint<f64> = 8_i32.into();
        let y = x.cbrt();
        assert!(y.ub > y.lb);
        assert!(contains(y, 2.0));
        // negative input: cbrt(-8) = -2
        let x: Flint<f64> = (-8_i32).into();
        assert!(contains(x.cbrt(), -2.0));
    }

    // --- hypot ---

    #[test]
    fn test_hypot() {
        let a: Flint<f64> = flint64!(0.6_f64);
        let b: Flint<f64> = flint64!(0.8_f64);
        assert!(contains(a.hypot(b), 1.0));
        assert!(contains((-a).hypot(b), 1.0));
        assert!(contains(a.hypot(-b), 1.0));
        assert!(contains((-a).hypot(-b), 1.0));

        // interval containing 0 in first arg: min is hypot(0,1)=1, we round down so lb < 1
        let a = Flint { lb: -1.0_f64, ub: 1.0_f64 };
        let c = a.hypot(1_i32);
        assert!(c.lb < 1.0);
        assert!(contains(c, consts::SQRT_2));

        // both args straddle zero: min is hypot(0,0) = 0
        let c = a.hypot(a);
        assert_eq!(0.0, c.lb);
        assert!(contains(c, consts::SQRT_2));
    }

    // --- exp family ---

    #[test]
    fn test_exp() {
        let x: Flint<f64> = 1_i32.into();
        assert!(contains(x.exp(), consts::E));
    }

    #[test]
    fn test_exp2() {
        let x: Flint<f64> = flint64!(0.5_f64);
        assert!(contains(x.exp2(), consts::SQRT_2));
    }

    #[test]
    fn test_exp_m1() {
        // For very small x, exp_m1(x) ≈ x + x²/2 (not exactly x).
        // Check that the result interval contains the actual float result.
        let v = 1.0e-15_f64.exp_m1();
        let x: Flint<f64> = flint64!(1.0e-15_f64);
        assert!(contains(x.exp_m1(), v));
    }

    // --- log family ---

    #[test]
    fn test_ln() {
        let x: Flint<f64> = flint64!(consts::E);
        assert!(contains(x.ln(), 1.0));
        // interval straddling 0: lower bound should be -∞
        let x = Flint { lb: -1.0e-8_f64, ub: 1.0e-8_f64 };
        assert!(x.ln().is_infinite());
        // entirely negative: NaN
        let x: Flint<f64> = (-1.0_f64).into();
        assert!(x.ln().is_nan());
    }

    #[test]
    fn test_log10() {
        let x: Flint<f64> = flint64!(1.0e-15_f64);
        assert!(contains(x.log10(), -15.0));
        let x = Flint { lb: -1.0e-8_f64, ub: 1.0e-8_f64 };
        assert!(x.log10().is_infinite());
        let x: Flint<f64> = (-1.0_f64).into();
        assert!(x.log10().is_nan());
    }

    #[test]
    fn test_log2() {
        let x: Flint<f64> = (1_i32 << 25_i32).into();
        assert!(contains(x.log2(), 25.0));
        let x = Flint { lb: -1.0e-8_f64, ub: 1.0e-8_f64 };
        assert!(x.log2().is_infinite());
        let x: Flint<f64> = (-1.0_f64).into();
        assert!(x.log2().is_nan());
    }

    #[test]
    fn test_ln_1p() {
        // ln_1p(x) < x for x > 0, so test containment of the actual float result.
        let v = 1.0e-15_f64.ln_1p();
        let x: Flint<f64> = flint64!(1.0e-15_f64);
        assert!(contains(x.ln_1p(), v));
        // ln(1 + (-1)) = ln(0) = -∞
        let x: Flint<f64> = (-1.0_f64).into();
        assert!(x.ln_1p().is_infinite());
        // ln(1 + x) for x < -1: NaN
        let x: Flint<f64> = (-2.0_f64).into();
        assert!(x.ln_1p().is_nan());
    }

    #[test]
    fn test_log() {
        // log(9, 3) = 2
        let x: Flint<f64> = 9_i32.into();
        assert!(contains(x.log(3_i32), 2.0));
        // entirely negative: NaN
        let x: Flint<f64> = (-1_i32).into();
        assert!(x.log(10_i32).is_nan());
        // negative base → NaN
        let x: Flint<f64> = 1_i32.into();
        assert!(x.log(-1_i32).is_nan());
    }

    // --- trig ---

    #[test]
    fn test_sin() {
        // sin(π) = 0
        let x: Flint<f64> = flint64!(consts::PI);
        assert!(contains(x.sin(), 0.0));
        // interval containing π/2 (maximum): ub must be 1
        let x = Flint { lb: 1.0_f64, ub: 2.0_f64 };
        let y = x.sin();
        assert_eq!(1.0, y.ub);
        assert!(contains(y, 1.0_f64.sin()));
        // interval containing 3π/2 (minimum): lb must be -1
        let x = Flint { lb: 4.0_f64, ub: 5.0_f64 };
        let y = x.sin();
        assert_eq!(-1.0, y.lb);
        // wide interval containing both extrema
        let x = Flint { lb: 4.0_f64, ub: 8.0_f64 };
        let y = x.sin();
        assert_eq!(-1.0, y.lb);
        assert_eq!(1.0, y.ub);
    }

    #[test]
    fn test_cos() {
        // cos(π/2) = 0
        let x: Flint<f64> = flint64!(consts::FRAC_PI_2);
        assert!(contains(x.cos(), 0.0));
        // interval crossing π (minimum): lb must be -1
        let x = Flint { lb: 3.0_f64, ub: 3.5_f64 };
        assert_eq!(-1.0, x.cos().lb);
        // interval crossing 0/2π (maximum): ub must be 1
        let x = Flint { lb: -0.1_f64, ub: 0.1_f64 };
        assert_eq!(1.0, x.cos().ub);
        // interval crossing both extrema
        let x = Flint { lb: 3.1_f64, ub: 6.3_f64 };
        let y = x.cos();
        assert_eq!(-1.0, y.lb);
        assert_eq!(1.0, y.ub);
    }

    #[test]
    fn test_tan() {
        // tan(π/4) = 1
        let x: Flint<f64> = flint64!(consts::FRAC_PI_4);
        assert!(contains(x.tan(), 1.0));
        // interval crossing discontinuity → infinite sentinel
        let x = Flint { lb: 1.5_f64, ub: 1.6_f64 };
        let y = x.tan();
        assert!(y.is_infinite());
    }

    // --- inverse trig ---

    #[test]
    fn test_asin() {
        assert!(contains(flint64!(0.5_f64).asin(), consts::FRAC_PI_2 / 3.0));
        // lb out of domain: clamp to -π/2
        let x = Flint { lb: -1.1_f64, ub: -0.9_f64 };
        assert_eq!(-consts::FRAC_PI_2, x.asin().lb);
        // ub out of domain: clamp to π/2
        let x = Flint { lb: 0.9_f64, ub: 1.1_f64 };
        assert_eq!(consts::FRAC_PI_2, x.asin().ub);
        // entirely out of domain: NaN
        assert!(flint64!(-1.1_f64).asin().is_nan());
        assert!(flint64!(1.1_f64).asin().is_nan());
    }

    #[test]
    fn test_acos() {
        // acos(0.5) = π/3
        assert!(contains(flint64!(0.5_f64).acos(), consts::FRAC_PI_3));
        // lb out of domain: ub clamps to π
        let x = Flint { lb: -1.1_f64, ub: -0.9_f64 };
        assert_eq!(consts::PI, x.acos().ub);
        // ub out of domain: lb clamps to 0
        let x = Flint { lb: 0.9_f64, ub: 1.1_f64 };
        assert_eq!(0.0, x.acos().lb);
        // entirely out of domain: NaN
        assert!(flint64!(-1.1_f64).acos().is_nan());
        assert!(flint64!(1.1_f64).acos().is_nan());
        // acos is decreasing: acos([0.3, 0.7]) = [acos(0.7), acos(0.3)]
        let x = Flint { lb: 0.3_f64, ub: 0.7_f64 };
        let y = x.acos();
        assert!(valid(y));
        assert!(contains(y, 0.3_f64.acos()));
        assert!(contains(y, 0.7_f64.acos()));
        assert!(y.lb < y.ub);
    }

    #[test]
    fn test_atan() {
        assert!(contains(flint64!(1_i32).atan(), consts::FRAC_PI_4));
    }

    #[test]
    fn test_atan2() {
        let zero = Flint { lb: -1.0e-8_f64, ub: 1.0e-8_f64 };
        let one: Flint<f64> = 1.0_f64.into();
        // positive x-axis
        assert!(contains(zero.atan2(one), 0.0));
        // Q1
        assert!(contains(one.atan2(one), consts::FRAC_PI_4));
        // positive y-axis
        assert!(contains(one.atan2(zero), consts::FRAC_PI_2));
        // Q2
        assert!(contains(one.atan2(-one), 3.0 * consts::FRAC_PI_4));
        // negative x-axis
        let z = zero.atan2(-one);
        assert!(valid(z) && contains(z, consts::PI));
        // Q3
        assert!(contains((-one).atan2(-one), -3.0 * consts::FRAC_PI_4));
        // negative y-axis
        assert!(contains((-one).atan2(zero), -consts::FRAC_PI_2));
        // Q4
        assert!(contains((-one).atan2(one), -consts::FRAC_PI_4));
        // origin: full circle
        let z = zero.atan2(zero);
        assert!(z.ub - z.lb >= consts::TAU);
    }

    // --- hyperbolic ---

    #[test]
    fn test_sinh() {
        let e = consts::E;
        let expected = 0.5 * (e - 1.0 / e);
        let x: Flint<f64> = 1_i32.into();
        assert!(contains(x.sinh(), expected));
        assert!(contains((-x).sinh(), -expected));
    }

    #[test]
    fn test_cosh() {
        let e = consts::E;
        let expected = 0.5 * (e + 1.0 / e);
        let x: Flint<f64> = 1_i32.into();
        assert!(contains(x.cosh(), expected));
        assert!(contains((-x).cosh(), expected)); // cosh is even
        // interval straddling 0: lb = 1 exactly
        let zero = Flint { lb: -1.0e-8_f64, ub: 1.0e-8_f64 };
        assert_eq!(1.0, zero.cosh().lb);
    }

    #[test]
    fn test_tanh() {
        let e = consts::E;
        let expected = (e - 1.0 / e) / (e + 1.0 / e);
        let x: Flint<f64> = 1_i32.into();
        assert!(contains(x.tanh(), expected));
        assert!(contains((-x).tanh(), -expected));
    }

    // --- inverse hyperbolic ---

    #[test]
    fn test_asinh() {
        let e = consts::E;
        let x: Flint<f64> = flint64!(0.5 * (e - 1.0 / e));
        assert!(contains(x.asinh(), 1.0));
    }

    #[test]
    fn test_acosh() {
        let e = consts::E;
        let x: Flint<f64> = flint64!(0.5 * (e + 1.0 / e));
        assert!(contains(x.acosh(), 1.0));
        // acosh(1) = 0
        assert!(contains(flint64!(1.0_f64).acosh(), 0.0));
        // acosh for x < 1: NaN
        assert!(flint64!(0.0_f64).acosh().is_nan());
    }

    #[test]
    fn test_atanh() {
        let e = consts::E;
        let x: Flint<f64> = flint64!((e - 1.0 / e) / (e + 1.0 / e));
        assert!(contains(x.atanh(), 1.0));
        // atanh(±1) = ±∞
        assert!(flint64!(1.0_f64).atanh().is_infinite());
        assert!(flint64!(-1.0_f64).atanh().is_infinite());
        // |x| > 1: NaN
        assert!(flint64!(1.1_f64).atanh().is_nan());
        assert!(flint64!(-1.1_f64).atanh().is_nan());
    }

    // --- array / vec smoke tests ---

    #[test]
    fn test_array_unary() {
        // FlintArray: abs element-wise
        let a: crate::FlintArray<f64, 4> = flint64_arr!(-2, -1, 1, 2);
        let b = a.abs();
        for i in 0..4 {
            assert!(b.lb[i] >= 0.0);
            assert!(b.lb[i] <= b.ub[i]);
        }
        // FlintArray: sqrt element-wise
        let a: crate::FlintArray<f64, 4> = flint64_arr!(1, 4, 9, 16);
        let b = a.sqrt();
        for (i, &expected) in [1.0, 2.0, 3.0, 4.0].iter().enumerate() {
            assert!(b.lb[i] <= expected && expected <= b.ub[i]);
        }
        // FlintArray: sin element-wise
        let a: crate::FlintArray<f64, 2> = flint64_arr!(0, 1);
        let b = a.sin();
        assert!(b.lb[0] <= 0.0 && 0.0 <= b.ub[0]);
        assert!(b.lb[1] <= 1.0_f64.sin() && 1.0_f64.sin() <= b.ub[1]);
    }

    #[test]
    fn test_vec_unary() {
        // FlintVec: exp element-wise
        let v = flint64_vec!(0, 1);
        let r = v.exp();
        assert!(r.lb[0] <= 1.0 && 1.0 <= r.ub[0]);
        assert!(r.lb[1] <= consts::E && consts::E <= r.ub[1]);
    }

    #[test]
    fn test_array_binary() {
        // FlintArray: powf with scalar exponent
        let a: crate::FlintArray<f64, 3> = flint64_arr!(1, 4, 9);
        let b = a.powf(0.5_f64);
        for (i, &expected) in [1.0, 2.0, 3.0].iter().enumerate() {
            assert!(b.lb[i] <= expected && expected <= b.ub[i]);
        }
        // FlintArray: atan2 with scalar x
        let y: crate::FlintArray<f64, 2> = flint64_arr!(1, -1);
        let r = y.atan2(1_i32);
        assert!(r.lb[0] <= consts::FRAC_PI_4 && consts::FRAC_PI_4 <= r.ub[0]);
        assert!(r.lb[1] <= -consts::FRAC_PI_4 && -consts::FRAC_PI_4 <= r.ub[1]);
    }
}
