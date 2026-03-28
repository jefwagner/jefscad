use crate::{next_up_down::NextUpDown, Flint, FlintRef};
#[allow(unused_imports)]
use crate::{FlintArray, FlintVec};

// -----------------
// From ref -> flint
// -----------------

impl<'a, T> From<FlintRef<'a, T>> for Flint<T>
where
    T: Copy,
{
    fn from(f: FlintRef<T>) -> Self {
        f.to_owned()
    }
}

// -------------------
// From float -> flint
// -------------------

impl From<f32> for Flint<f32> {
    fn from(f: f32) -> Self {
        Flint {
            lb: f.nd(),
            ub: f.nu(),
        }
    }
}

impl From<f64> for Flint<f32> {
    fn from(f: f64) -> Self {
        Flint {
            lb: (f as f32).nd(),
            ub: (f as f32).nu(),
        }
    }
}

impl From<f32> for Flint<f64> {
    fn from(f: f32) -> Self {
        Flint {
            lb: f.nd() as f64,
            ub: f.nu() as f64,
        }
    }
}

impl From<f64> for Flint<f64> {
    fn from(f: f64) -> Self {
        Flint {
            lb: f.nd(),
            ub: f.nu(),
        }
    }
}

// -----------------------------
// conversion from  int -> flint
// -----------------------------

const MAX_F32_EXACT_INT: i32 = 16_777_216_i32;
const MAX_F64_EXACT_INT: i64 = 9_007_199_254_740_992_i64;

macro_rules! impl_small_int {
    ($flint_type:ty, $sm_int:ty) => {
        impl From<$sm_int> for Flint<$flint_type> {
            fn from(i: $sm_int) -> Self {
                Flint {
                    lb: (i as $flint_type),
                    ub: (i as $flint_type),
                }
            }
        }
    };
}

macro_rules! impl_large_int {
    (f32, $lg_int:ty) => {
        impl From<$lg_int> for Flint<f32> {
            fn from(i: $lg_int) -> Self {
                if i > (MAX_F32_EXACT_INT as $lg_int) || i < -(MAX_F32_EXACT_INT as $lg_int) {
                    Flint {
                        lb: (i as f32).nd(),
                        ub: (i as f32).nu(),
                    }
                } else {
                    Flint {
                        lb: (i as f32),
                        ub: (i as f32),
                    }
                }
            }
        }
    };
    (f64, $lg_int:ty) => {
        impl From<$lg_int> for Flint<f64> {
            fn from(i: $lg_int) -> Self {
                if i > (MAX_F64_EXACT_INT as $lg_int) || i < -(MAX_F64_EXACT_INT as $lg_int) {
                    Flint {
                        lb: (i as f64).nd(),
                        ub: (i as f64).nu(),
                    }
                } else {
                    Flint {
                        lb: (i as f64),
                        ub: (i as f64),
                    }
                }
            }
        }
    };
}

macro_rules! impl_large_uint {
    (f32, $lg_int:ty) => {
        impl From<$lg_int> for Flint<f32> {
            fn from(i: $lg_int) -> Self {
                if i > (MAX_F32_EXACT_INT as $lg_int) {
                    Flint {
                        lb: (i as f32).nd(),
                        ub: (i as f32).nu(),
                    }
                } else {
                    Flint {
                        lb: (i as f32),
                        ub: (i as f32),
                    }
                }
            }
        }
    };
    (f64, $lg_int:ty) => {
        impl From<$lg_int> for Flint<f64> {
            fn from(i: $lg_int) -> Self {
                if i > (MAX_F64_EXACT_INT as $lg_int) {
                    Flint {
                        lb: (i as f64).nd(),
                        ub: (i as f64).nu(),
                    }
                } else {
                    Flint {
                        lb: (i as f64),
                        ub: (i as f64),
                    }
                }
            }
        }
    };
}

impl_small_int!(f32, i8);
impl_small_int!(f32, i16);
impl_large_int!(f32, i32);
impl_small_int!(f32, u8);
impl_small_int!(f32, u16);
impl_large_uint!(f32, u32);
impl_large_int!(f32, i64);
impl_large_int!(f32, i128);
impl_large_int!(f32, isize);
impl_large_uint!(f32, u64);
impl_large_uint!(f32, u128);
impl_large_uint!(f32, usize);

impl_small_int!(f64, i8);
impl_small_int!(f64, i16);
impl_small_int!(f64, i32);
impl_small_int!(f64, u8);
impl_small_int!(f64, u16);
impl_small_int!(f64, u32);
impl_large_int!(f64, i64);
impl_large_int!(f64, i128);
impl_large_int!(f64, isize);
impl_large_uint!(f64, u64);
impl_large_uint!(f64, u128);
impl_large_uint!(f64, usize);

// --------------------------------------------
// macro for simplfied flint and array creation
// --------------------------------------------

#[macro_export]
macro_rules! flint32 {
    ( $x:expr ) => {
        Flint::<f32>::from($x)
    };
}

#[macro_export]
macro_rules! flint64 {
    ( $x:expr ) => {
        Flint::<f64>::from($x)
    };
}

#[macro_export]
macro_rules! flint32_arr {
    ( $( $x:expr ),* ) => {
        {
            let mut lb = [ 0.0_f32; ${count($x)} ];
            let mut ub = [ 0.0_f32; ${count($x)} ];
            $(
                let f: Flint<f32> = Flint::from($x);
                lb[${index()}] = f.lb;
                ub[${index()}] = f.ub;
            )*
            FlintArray { lb, ub }
        }
    }
}

#[macro_export]
macro_rules! flint64_arr {
    ( $( $x:expr ),* ) => {
        {
            let mut lb = [ 0.0; ${count($x)} ];
            let mut ub = [ 0.0; ${count($x)} ];
            $(
                let f: Flint<f64> = Flint::from($x);
                lb[${index()}] = f.lb;
                ub[${index()}] = f.ub;
            )*
            FlintArray { lb, ub }
        }
    }
}

#[macro_export]
macro_rules! flint32_vec {
    ( $( $x:expr ),* ) => {
        {
            let mut fv = FlintVec{ lb: vec![], ub: vec![] };
            $(
                let f: Flint<f32> = Flint::from($x);
                fv.lb.push(f.lb);
                fv.ub.push(f.ub);
            )*
            fv
        }
    }
}

#[macro_export]
macro_rules! flint64_vec {
    ( $( $x:expr ),* ) => {
        {
            let mut fv = FlintVec{ lb: vec![], ub: vec![] };
            $(
                let f: Flint<f64> = Flint::from($x);
                fv.lb.push(f.lb);
                fv.ub.push(f.ub);
            )*
            fv
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::ops::Sub;

    fn width<T>(f: Flint<T>) -> T
    where
        T: Sub<Output = T>,
    {
        f.ub - f.lb
    }

    #[test]
    fn test_from_float() {
        // check the value around 1.0
        // first for an f64
        let two_ulp = 3.3306690738754696e-16;
        let f: Flint<f64> = (1.0).into();
        assert!(f.lb < 1.0 && 1.0 < f.ub);
        assert_eq!(two_ulp, width(f));
        // second for an f32
        let two_ulp = 1.7881393432617188e-7;
        let f: Flint<f64> = (1.0_f32).into();
        assert!(f.lb < 1.0 && 1.0 < f.ub);
        assert_eq!(two_ulp, width(f));
        let f: Flint<f32> = (1.0).into();
        assert!(f.lb < 1.0 && 1.0 < f.ub);
        assert_eq!(two_ulp as f32, width(f));
    }

    #[test]
    fn test_from_small_int() {
        // check that small ints always give exact values
        let f: Flint<f32> = (1_i8).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f32> = (1_i16).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f32> = (1_u8).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f32> = (1_u16).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));

        let f: Flint<f64> = (1_i8).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f64> = (1_i16).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f64> = (1_i32).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f64> = (1_u8).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f64> = (1_u16).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f64> = (1_u32).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
    }

    #[test]
    fn test_from_large_int() {
        // f32
        let f: Flint<f32> = (1_i32).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f32> = (1_i64).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f32> = (MAX_F32_EXACT_INT + 1).into();
        assert_ne!(0.0, width(f));
        let f: Flint<f32> = (-MAX_F32_EXACT_INT - 1).into();
        assert_ne!(0.0, width(f));
        let f: Flint<f32> = ((MAX_F32_EXACT_INT + 1) as i64).into();
        assert_ne!(0.0, width(f));
        let f: Flint<f32> = ((-MAX_F32_EXACT_INT - 1) as i64).into();
        assert_ne!(0.0, width(f));

        // f64
        let f: Flint<f64> = (1_i64).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f64> = (1_i128).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f64> = (MAX_F64_EXACT_INT + 1).into();
        assert_ne!(0.0, width(f));
        let f: Flint<f64> = (-MAX_F64_EXACT_INT - 1).into();
        assert_ne!(0.0, width(f));
        let f: Flint<f64> = ((MAX_F64_EXACT_INT + 1) as i128).into();
        assert_ne!(0.0, width(f));
        let f: Flint<f64> = ((-MAX_F64_EXACT_INT - 1) as i128).into();
        assert_ne!(0.0, width(f));
    }

    #[test]
    fn test_flint_macros() {
        // first for an f64
        let two_ulp = 3.3306690738754696e-16;
        let f = flint64!(1.0);
        assert!(f.lb < 1.0 && 1.0 < f.ub);
        assert_eq!(two_ulp, width(f));
        // second for an f32
        let two_ulp = 1.7881393432617188e-7;
        let f = flint64!(1.0_f32);
        assert!(f.lb < 1.0 && 1.0 < f.ub);
        assert_eq!(two_ulp, width(f));
        let f = flint32!(1.0);
        assert!(f.lb < 1.0 && 1.0 < f.ub);
        assert_eq!(two_ulp as f32, width(f));
    }

    #[test]
    fn test_array() {
        let farr = flint32_arr!(1, 0, 0, 0);
        assert_eq!([1.0, 0.0, 0.0, 0.0], farr.lb);
        assert_eq!([1.0, 0.0, 0.0, 0.0], farr.ub);
    }

    #[test]
    fn test_vec() {
        let fvec = flint32_vec![-2, -1, 1.0, 2.0];
        assert_eq!(vec![-2.0, -1.0, 0.99999994, 1.9999999], fvec.lb);
        assert_eq!(vec![-2.0, -1.0, 1.0000001, 2.0000002], fvec.ub);
    }
}
