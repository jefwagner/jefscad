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
}
