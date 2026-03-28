use num_traits::Float;
use std::cmp::{Ordering, PartialEq, PartialOrd};

use crate::{Flint, FlintRef};

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
}
