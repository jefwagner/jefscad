use crate::{next_up_down::NextUpDown, Flint, FlintMut, FlintRef};
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

impl<'a, T> From<FlintMut<'a, T>> for Flint<T>
where
    T: Copy,
{
    fn from(f: FlintMut<T>) -> Self {
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

// -------------------------------------------
// From [f32; N] / [f64; N] -> FlintArray<T,N>
// -------------------------------------------

impl<const N: usize> From<[f32; N]> for FlintArray<f32, N> {
    fn from(arr: [f32; N]) -> Self {
        let mut lb = [0.0_f32; N];
        let mut ub = [0.0_f32; N];
        for i in 0..N {
            let f = Flint::from(arr[i]);
            lb[i] = f.lb;
            ub[i] = f.ub;
        }
        FlintArray { lb, ub }
    }
}

impl<const N: usize> From<[f64; N]> for FlintArray<f64, N> {
    fn from(arr: [f64; N]) -> Self {
        let mut lb = [0.0_f64; N];
        let mut ub = [0.0_f64; N];
        for i in 0..N {
            let f = Flint::from(arr[i]);
            lb[i] = f.lb;
            ub[i] = f.ub;
        }
        FlintArray { lb, ub }
    }
}

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
        // f64 -> Flint<f64>
        let two_ulp_f64 = 3.3306690738754696e-16;
        let f: Flint<f64> = (1.0_f64).into();
        assert!(f.lb < 1.0 && 1.0 < f.ub);
        assert_eq!(two_ulp_f64, width(f));
        // f32 -> Flint<f64>: widened to f64 after taking nd/nu on the f32
        let two_ulp_f32 = 1.7881393432617188e-7;
        let f: Flint<f64> = (1.0_f32).into();
        assert!(f.lb < 1.0 && 1.0 < f.ub);
        assert_eq!(two_ulp_f32, width(f));
        // f32 -> Flint<f32>
        let f: Flint<f32> = (1.0_f32).into();
        assert!(f.lb < 1.0 && 1.0 < f.ub);
        assert_eq!(two_ulp_f32 as f32, width(f));
        // f64 -> Flint<f32>: cast to f32 first, then take nd/nu on the f32
        let f: Flint<f32> = (1.0_f64).into();
        assert!(f.lb < 1.0_f32 && 1.0_f32 < f.ub);
        assert_eq!(two_ulp_f32 as f32, width(f));

        // special values: NaN propagates to both bounds
        let f: Flint<f64> = f64::NAN.into();
        assert!(f.lb.is_nan() && f.ub.is_nan());
        let f: Flint<f32> = f32::NAN.into();
        assert!(f.lb.is_nan() && f.ub.is_nan());

        // special values: infinity is left unchanged by nd/nu
        let f: Flint<f64> = f64::INFINITY.into();
        assert!(f.lb.is_infinite() && f.ub.is_infinite());
        assert!(f.lb > 0.0 && f.ub > 0.0);
        let f: Flint<f64> = f64::NEG_INFINITY.into();
        assert!(f.lb.is_infinite() && f.ub.is_infinite());
        assert!(f.lb < 0.0 && f.ub < 0.0);
        let f: Flint<f32> = f32::INFINITY.into();
        assert!(f.lb.is_infinite() && f.ub.is_infinite());
        let f: Flint<f32> = f32::NEG_INFINITY.into();
        assert!(f.lb.is_infinite() && f.ub.is_infinite());

        // special values: zero — nd/nu straddle zero with subnormals
        let f: Flint<f64> = (0.0_f64).into();
        assert!(f.lb < 0.0 && 0.0 < f.ub);
        let f: Flint<f32> = (0.0_f32).into();
        assert!(f.lb < 0.0 && 0.0 < f.ub);
        // negative zero is identical to positive zero for conversion purposes
        let f_pos: Flint<f64> = (0.0_f64).into();
        let f_neg: Flint<f64> = (-0.0_f64).into();
        assert_eq!(f_pos.lb, f_neg.lb);
        assert_eq!(f_pos.ub, f_neg.ub);
    }

    #[test]
    fn test_from_small_int() {
        // check that small ints always give exact (zero-width) intervals
        // --- Flint<f32> ---
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

        // --- Flint<f64> ---
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

        // negative small integers must also be exact
        let f: Flint<f32> = (-1_i8).into();
        assert_eq!(-1.0, f.lb);
        assert_eq!(-1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f32> = (-1_i16).into();
        assert_eq!(-1.0, f.lb);
        assert_eq!(-1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f64> = (-1_i8).into();
        assert_eq!(-1.0, f.lb);
        assert_eq!(-1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f64> = (-1_i16).into();
        assert_eq!(-1.0, f.lb);
        assert_eq!(-1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f64> = (-1_i32).into();
        assert_eq!(-1.0, f.lb);
        assert_eq!(-1.0, f.ub);
        assert_eq!(0.0, width(f));

        // zero is always exact
        let f: Flint<f32> = (0_i8).into();
        assert_eq!(0.0, f.lb);
        assert_eq!(0.0, f.ub);
        let f: Flint<f64> = (0_u8).into();
        assert_eq!(0.0, f.lb);
        assert_eq!(0.0, f.ub);
    }

    #[test]
    fn test_from_large_int() {
        // --- Flint<f32>: signed large ints ---
        // exact for small values
        let f: Flint<f32> = (1_i32).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f32> = (1_i64).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f32> = (1_i128).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        // non-exact past the boundary
        let f: Flint<f32> = (MAX_F32_EXACT_INT + 1).into();
        assert_ne!(0.0, width(f));
        let f: Flint<f32> = (-MAX_F32_EXACT_INT - 1).into();
        assert_ne!(0.0, width(f));
        let f: Flint<f32> = ((MAX_F32_EXACT_INT + 1) as i64).into();
        assert_ne!(0.0, width(f));
        let f: Flint<f32> = ((-MAX_F32_EXACT_INT - 1) as i64).into();
        assert_ne!(0.0, width(f));
        let f: Flint<f32> = ((MAX_F32_EXACT_INT + 1) as i128).into();
        assert_ne!(0.0, width(f));
        let f: Flint<f32> = ((-MAX_F32_EXACT_INT - 1) as i128).into();
        assert_ne!(0.0, width(f));

        // --- Flint<f32>: isize/usize ---
        let f: Flint<f32> = (1_isize).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f32> = ((MAX_F32_EXACT_INT + 1) as isize).into();
        assert_ne!(0.0, width(f));
        let f: Flint<f32> = (1_usize).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f32> = ((MAX_F32_EXACT_INT as usize) + 1).into();
        assert_ne!(0.0, width(f));

        // --- Flint<f32>: unsigned large ints ---
        let f: Flint<f32> = (1_u32).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f32> = ((MAX_F32_EXACT_INT as u32) + 1).into();
        assert_ne!(0.0, width(f));
        let f: Flint<f32> = (1_u64).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f32> = ((MAX_F32_EXACT_INT as u64) + 1).into();
        assert_ne!(0.0, width(f));
        let f: Flint<f32> = (1_u128).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f32> = ((MAX_F32_EXACT_INT as u128) + 1).into();
        assert_ne!(0.0, width(f));

        // --- Flint<f64>: signed large ints ---
        // exact for small values
        let f: Flint<f64> = (1_i64).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f64> = (1_i128).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        // non-exact past the boundary
        let f: Flint<f64> = (MAX_F64_EXACT_INT + 1).into();
        assert_ne!(0.0, width(f));
        let f: Flint<f64> = (-MAX_F64_EXACT_INT - 1).into();
        assert_ne!(0.0, width(f));
        let f: Flint<f64> = ((MAX_F64_EXACT_INT + 1) as i128).into();
        assert_ne!(0.0, width(f));
        let f: Flint<f64> = ((-MAX_F64_EXACT_INT - 1) as i128).into();
        assert_ne!(0.0, width(f));

        // --- Flint<f64>: isize/usize ---
        let f: Flint<f64> = (1_isize).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f64> = ((MAX_F64_EXACT_INT + 1) as isize).into();
        assert_ne!(0.0, width(f));
        let f: Flint<f64> = (1_usize).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f64> = ((MAX_F64_EXACT_INT as usize) + 1).into();
        assert_ne!(0.0, width(f));

        // --- Flint<f64>: unsigned large ints ---
        let f: Flint<f64> = (1_u64).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f64> = ((MAX_F64_EXACT_INT as u64) + 1).into();
        assert_ne!(0.0, width(f));
        let f: Flint<f64> = (1_u128).into();
        assert_eq!(1.0, f.lb);
        assert_eq!(1.0, f.ub);
        assert_eq!(0.0, width(f));
        let f: Flint<f64> = ((MAX_F64_EXACT_INT as u128) + 1).into();
        assert_ne!(0.0, width(f));
    }

    #[test]
    fn test_flint_macros() {
        let two_ulp_f64 = 3.3306690738754696e-16;
        let two_ulp_f32 = 1.7881393432617188e-7_f64;

        // flint64!(f64) — standard case
        let f = flint64!(1.0_f64);
        assert!(f.lb < 1.0 && 1.0 < f.ub);
        assert_eq!(two_ulp_f64, width(f));

        // flint64!(f32) — widens f32 to Flint<f64>
        let f = flint64!(1.0_f32);
        assert!(f.lb < 1.0 && 1.0 < f.ub);
        assert_eq!(two_ulp_f32, width(f));

        // flint32!(f32) — standard case
        let f = flint32!(1.0_f32);
        assert!(f.lb < 1.0_f32 && 1.0_f32 < f.ub);
        assert_eq!(two_ulp_f32 as f32, width(f));

        // flint32!(f64) — narrows f64 to Flint<f32>
        let f = flint32!(1.0_f64);
        assert!(f.lb < 1.0_f32 && 1.0_f32 < f.ub);
        assert_eq!(two_ulp_f32 as f32, width(f));

        // macros with integer inputs — must produce exact zero-width intervals
        let f = flint32!(1_i8);
        assert_eq!(0.0, width(f));
        assert_eq!(1.0_f32, f.lb);
        let f = flint64!(1_i8);
        assert_eq!(0.0, width(f));
        assert_eq!(1.0_f64, f.lb);
        let f = flint32!(-42_i16);
        assert_eq!(0.0, width(f));
        assert_eq!(-42.0_f32, f.lb);
        let f = flint64!(255_u8);
        assert_eq!(0.0, width(f));
        assert_eq!(255.0_f64, f.lb);
    }

    #[test]
    fn test_from_float_array() {
        // From<[f32; N]> — each element gets nd/nu expansion
        let arr: FlintArray<f32, 3> = [1.0_f32, 0.2_f32, -1.0_f32].into();
        for i in 0..3 {
            assert!(arr.lb[i] <= arr.ub[i], "lb must be <= ub at index {i}");
        }
        // 1.0 is exactly representable in f32 — interval straddles it
        assert!(arr.lb[0] < 1.0_f32 && 1.0_f32 < arr.ub[0]);
        // 0.2 is not exactly representable
        assert!(arr.lb[1] < 0.2_f32 && 0.2_f32 < arr.ub[1]);
        // negative value: lb < -1.0 < ub
        assert!(arr.lb[2] < -1.0_f32 && -1.0_f32 < arr.ub[2]);

        // From<[f64; N]> — same guarantees
        let arr: FlintArray<f64, 2> = [1.0_f64, 0.1_f64].into();
        assert!(arr.lb[0] < 1.0_f64 && 1.0_f64 < arr.ub[0]);
        assert!(arr.lb[1] < 0.1_f64 && 0.1_f64 < arr.ub[1]);

        // integer-valued floats: exact representation — lb < val < ub still holds
        // (nd/nu always expand by at least 1 ULP, so lb < exact < ub)
        let arr: FlintArray<f32, 1> = [3.0_f32].into();
        assert!(arr.lb[0] < 3.0_f32 && 3.0_f32 < arr.ub[0]);
    }

    #[test]
    fn test_array() {
        // flint32_arr! with integer inputs — all exact
        let farr = flint32_arr!(1, 0, 0, 0);
        assert_eq!([1.0, 0.0, 0.0, 0.0], farr.lb);
        assert_eq!([1.0, 0.0, 0.0, 0.0], farr.ub);

        // flint64_arr! with integer inputs — all exact
        let farr = flint64_arr!(1, 0, 0, 0);
        assert_eq!([1.0_f64, 0.0, 0.0, 0.0], farr.lb);
        assert_eq!([1.0_f64, 0.0, 0.0, 0.0], farr.ub);

        // flint32_arr! with float inputs — lb < value < ub for non-exact floats
        let farr = flint32_arr!(1.0_f32, -1.0_f32);
        assert!(farr.lb[0] < 1.0_f32 && 1.0_f32 < farr.ub[0]);
        assert!(farr.lb[1] < -1.0_f32 && -1.0_f32 < farr.ub[1]);

        // flint64_arr! with float inputs
        let farr = flint64_arr!(1.0_f64, -1.0_f64);
        assert!(farr.lb[0] < 1.0_f64 && 1.0_f64 < farr.ub[0]);
        assert!(farr.lb[1] < -1.0_f64 && -1.0_f64 < farr.ub[1]);

        // flint32_arr! with mixed types (int and float)
        let farr = flint32_arr!(2_i32, 1.0_f32);
        assert_eq!(2.0_f32, farr.lb[0]); // small int: exact
        assert_eq!(2.0_f32, farr.ub[0]);
        assert!(farr.lb[1] < 1.0_f32 && 1.0_f32 < farr.ub[1]); // float: interval

        // flint64_arr! with mixed types
        let farr = flint64_arr!(2_i32, 1.0_f64);
        assert_eq!(2.0_f64, farr.lb[0]);
        assert_eq!(2.0_f64, farr.ub[0]);
        assert!(farr.lb[1] < 1.0_f64 && 1.0_f64 < farr.ub[1]);
    }

    #[test]
    fn test_vec() {
        // flint32_vec! with mixed integer and float inputs
        let fvec = flint32_vec![-2, -1, 1.0, 2.0];
        assert_eq!(vec![-2.0, -1.0, 0.99999994, 1.9999999], fvec.lb);
        assert_eq!(vec![-2.0, -1.0, 1.0000001, 2.0000002], fvec.ub);

        // flint64_vec! with integer inputs — all exact
        let fvec = flint64_vec![1, 2, 3];
        assert_eq!(vec![1.0_f64, 2.0, 3.0], fvec.lb);
        assert_eq!(vec![1.0_f64, 2.0, 3.0], fvec.ub);

        // flint64_vec! with float inputs — lb < value < ub
        let fvec = flint64_vec![1.0_f64, -1.0_f64];
        assert!(fvec.lb[0] < 1.0_f64 && 1.0_f64 < fvec.ub[0]);
        assert!(fvec.lb[1] < -1.0_f64 && -1.0_f64 < fvec.ub[1]);

        // flint64_vec! with mixed types
        let fvec = flint64_vec![1_i32, 1.0_f64];
        assert_eq!(1.0_f64, fvec.lb[0]); // exact integer
        assert_eq!(1.0_f64, fvec.ub[0]);
        assert!(fvec.lb[1] < 1.0_f64 && 1.0_f64 < fvec.ub[1]); // float interval
    }

    #[test]
    fn test_from_flint_mut() {
        // From<FlintMut> should produce the same Flint as to_owned()
        let mut lb = 1.0_f32.nd();
        let mut ub = 1.0_f32.nu();
        let m = FlintMut {
            lb: &mut lb,
            ub: &mut ub,
        };
        let owned: Flint<f32> = m.into();
        assert!(owned.lb < 1.0_f32 && 1.0_f32 < owned.ub);

        // f64 variant
        let mut lb = 2.5_f64.nd();
        let mut ub = 2.5_f64.nu();
        let m = FlintMut {
            lb: &mut lb,
            ub: &mut ub,
        };
        let owned: Flint<f64> = m.into();
        assert!(owned.lb < 2.5_f64 && 2.5_f64 < owned.ub);
    }
}
