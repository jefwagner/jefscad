use std::{
    fmt::{self, LowerExp},
    io::{Cursor, Write},
    str::FromStr,
};

use crate::{Flint, FlintArray, FlintMut, FlintRef, FlintVec, FlintView, FlintViewMut};

/// Format a sequence of interval pairs as `[v0, v1, ...]` where each element is
/// formatted using `Flint::Display`. Used by all array/vec/view Display impls.
fn fmt_interval_slice<T>(lb: &[T], ub: &[T], f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    T: num_traits::Float + ryu::Float + Copy + GenNumConsts + LowerExp + FloatStringParts,
{
    write!(f, "[")?;
    let mut first = true;
    for (l, u) in lb.iter().zip(ub.iter()) {
        if !first {
            write!(f, ", ")?;
        }
        first = false;
        fmt::Display::fmt(&Flint { lb: *l, ub: *u }, f)?;
    }
    write!(f, "]")
}

impl<T, const N: usize> fmt::Display for FlintArray<T, N>
where
    T: num_traits::Float + ryu::Float + Copy + GenNumConsts + LowerExp + FloatStringParts,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_interval_slice(&self.lb, &self.ub, f)
    }
}

impl<T> fmt::Display for FlintVec<T>
where
    T: num_traits::Float + ryu::Float + Copy + GenNumConsts + LowerExp + FloatStringParts,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_interval_slice(&self.lb, &self.ub, f)
    }
}

impl<'a, T> fmt::Display for FlintView<'a, T>
where
    T: num_traits::Float + ryu::Float + Copy + GenNumConsts + LowerExp + FloatStringParts,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_interval_slice(self.lb, self.ub, f)
    }
}

trait GenNumConsts {
    const ZERO: Self;
    const HALF: Self;
}

impl GenNumConsts for f32 {
    const ZERO: Self = 0.0;
    const HALF: Self = 0.5;
}

impl GenNumConsts for f64 {
    const ZERO: Self = 0.0;
    const HALF: Self = 0.5;
}

trait FloatStringParts {
    /// Turn a float number into integer part, fractional part, and exponent as strings
    fn to_str_parts<'a, 'b>(&'a self, buf: &'b mut [u8]) -> (&'b str, &'b str, &'b str)
    where
        Self: LowerExp,
    {
        let mut cursor = std::io::Cursor::new(buf);
        write!(cursor, "{:.17e}", self).expect("Failed writing to buf");
        let buf_slice = &cursor.into_inner()[..];
        let buf_str: &str = unsafe { std::str::from_utf8_unchecked(buf_slice) };
        let mut buf_iter = buf_str.split(".");
        let int_part = buf_iter.next().unwrap();
        let rest = buf_iter.next().expect("No . in string");
        let mut rest_iter = rest.split("e");
        let frac = rest_iter.next().unwrap();
        let exp = rest_iter
            .next()
            .expect("No e in string")
            .trim_end_matches(|c: char| c == '\0' || c.is_whitespace());
        (int_part, frac, exp)
    }

    // Create a float from integer part, fractional part, and exponent as strings
    fn from_str_parts(int_part: &str, frac: &str, exp: &str) -> Self;
}

impl FloatStringParts for f32 {
    fn from_str_parts(int_part: &str, frac: &str, exp: &str) -> Self {
        let mut buf = [0_u8; 32];
        let mut cursor = Cursor::new(&mut buf[..]);
        write!(cursor, "{int_part}.{frac}e{exp}").expect("Failed combining string");
        let buf_slice = &cursor.get_ref()[..];
        let num_str: &str = unsafe { std::str::from_utf8_unchecked(buf_slice) };
        let num_str = num_str.trim_end_matches(|c: char| c == '\0' || c.is_whitespace());
        f32::from_str(num_str).expect("Failed parse combined string")
    }
}

impl FloatStringParts for f64 {
    fn from_str_parts(int_part: &str, frac: &str, exp: &str) -> Self {
        let mut buf = [0_u8; 32];
        let mut cursor = Cursor::new(&mut buf[..]);
        write!(cursor, "{int_part}.{frac}e{exp}").expect("Failed combining string");
        let buf_slice = &cursor.get_ref()[..];
        let num_str: &str = unsafe { std::str::from_utf8_unchecked(buf_slice) };
        let num_str = num_str.trim_end_matches(|c: char| c == '\0' || c.is_whitespace());
        // eprintln!("num_str[{}]={}", num_str.len(), num_str);
        f64::from_str(num_str.trim()).expect("Failed parse combined string")
    }
}

/// Display for FlintRef delegates to Flint::Display by copying through the references.
/// T must be Copy (true for f32 and f64), so no allocation is needed.
impl<'a, T> fmt::Display for FlintRef<'a, T>
where
    T: num_traits::Float + ryu::Float + Copy + GenNumConsts + LowerExp + FloatStringParts,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Flint {
            lb: *self.lb,
            ub: *self.ub,
        }
        .fmt(f)
    }
}

/// Display for FlintMut delegates to Flint::Display identically to FlintRef.
impl<'a, T> fmt::Display for FlintMut<'a, T>
where
    T: num_traits::Float + ryu::Float + Copy + GenNumConsts + LowerExp + FloatStringParts,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Flint {
            lb: *self.lb,
            ub: *self.ub,
        }
        .fmt(f)
    }
}

/// Display for FlintViewMut delegates to fmt_interval_slice identically to FlintView.
impl<'a, T> fmt::Display for FlintViewMut<'a, T>
where
    T: num_traits::Float + ryu::Float + Copy + GenNumConsts + LowerExp + FloatStringParts,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_interval_slice(self.lb, self.ub, f)
    }
}

// Debug printing is derived to show the full struct
impl<T> fmt::Display for Flint<T>
where
    T: num_traits::Float + ryu::Float + GenNumConsts + LowerExp + FloatStringParts,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Flint { lb, ub } = self;
        // special cases
        if lb.is_nan() || ub.is_nan() {
            return write!(f, "NaN");
        }
        if lb.is_infinite() || ub.is_infinite() {
            return write!(f, "+/-inf");
        }
        if *lb <= T::ZERO && T::ZERO <= *ub {
            return write!(f, "0");
        }
        // get the string representaiton of the upper bound
        let mut buf = [0_u8; 32];
        let (ub_int, ub_frac, ub_exp) = ub.to_str_parts(&mut buf[..]);
        for i in 0..ub_frac.len() {
            let ub_trunc: T = FloatStringParts::from_str_parts(ub_int, &ub_frac[0..i], ub_exp);
            // ub_trunc must lie within [lb, ub]: for positive numbers truncation
            // always moves toward zero (smaller), so ub_trunc <= ub is automatic.
            // For negative numbers truncation moves toward zero (larger/less negative),
            // so ub_trunc can exceed ub — we must check both bounds.
            if lb <= &ub_trunc && &ub_trunc <= ub {
                let mut buf = ryu::Buffer::new();
                return write!(f, "{}", buf.format(ub_trunc));
            }
        }
        // fall back to full midpoint
        let mut buf = ryu::Buffer::new();
        let midpoint = T::HALF * (self.lb + self.ub);
        write!(f, "{}", buf.format(midpoint))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug() {
        let f = Flint {
            lb: 0.0_f32,
            ub: 1.0_f32,
        };
        assert_eq!("Flint { lb: 0.0, ub: 1.0 }", format!("{f:?}"));
    }

    #[test]
    fn test_fmt() {
        let f = Flint {
            lb: 1.555_f32,
            ub: 1.565_f32,
        };
        assert_eq!("1.56", format!("{f}"));
    }

    // Regression: negative intervals were printed incorrectly because truncating
    // fractional digits on a negative number moves it toward zero (i.e. above ub),
    // so the `lb <= ub_trunc` check alone is not sufficient — `ub_trunc <= ub`
    // must also be verified.
    #[test]
    fn test_fmt_negative() {
        // Simple negative interval: [-1.6, -1.55]
        let f = Flint {
            lb: -1.6_f64,
            ub: -1.55_f64,
        };
        let s = format!("{f}");
        let v: f64 = s.parse().expect("output should be a valid float");
        assert!(
            -1.6_f64 <= v && v <= -1.55_f64,
            "output {s} not in [{}, {}]",
            -1.6_f64,
            -1.55_f64
        );
        // Shortest repr inside the interval is "-1.6" (ub itself) or "-1.55"; "-1" is wrong.
        assert_ne!(
            s, "-1",
            "formatter emitted integer truncation of negative ub"
        );
    }

    // Regression: narrow negative f64 interval where ub is just above -1.6.
    // The 17-digit expansion of ub truncates to "-1" at i=0 which is outside the interval.
    #[test]
    fn test_fmt_negative_narrow() {
        let ub = -1.5999999999999999_f64; // f64 just above -1.6 (closer to zero)
        let lb = -1.601_f64;
        let f = Flint { lb, ub };
        let s = format!("{f}");
        let v: f64 = s.parse().expect("output should be a valid float");
        assert!(
            lb <= v && v <= ub,
            "output {s} ({v:.17e}) not in [{lb:.17e}, {ub:.17e}]"
        );
    }

    // --- FlintRef Display ---
    // FlintRef::Display must produce identical output to Flint::Display for all cases.

    #[test]
    fn test_fmt_ref_basic() {
        let lb = 1.555_f32;
        let ub = 1.565_f32;
        let r = FlintRef { lb: &lb, ub: &ub };
        assert_eq!("1.56", format!("{r}"));
    }

    #[test]
    fn test_fmt_ref_matches_owned() {
        // For every case the ref output must exactly equal the owned output.
        let cases: &[(f64, f64)] = &[
            (1.555, 1.565),                         // normal positive
            (-1.6, -1.55),                          // normal negative
            (0.0, 0.0),                             // zero interval
            (-0.5, 0.5),                            // straddles zero
            (f64::NAN, f64::NAN),                   // NaN
            (f64::NEG_INFINITY, f64::NEG_INFINITY), // infinite
        ];
        for &(lb, ub) in cases {
            let owned = Flint { lb, ub };
            let r = FlintRef { lb: &lb, ub: &ub };
            assert_eq!(
                format!("{owned}"),
                format!("{r}"),
                "mismatch for lb={lb} ub={ub}"
            );
        }
    }

    #[test]
    fn test_fmt_ref_negative_narrow() {
        // Regression case: narrow negative interval must not truncate to integer part.
        let ub = -1.5999999999999999_f64;
        let lb = -1.601_f64;
        let r = FlintRef { lb: &lb, ub: &ub };
        let s = format!("{r}");
        let v: f64 = s.parse().expect("output should be a valid float");
        assert!(
            lb <= v && v <= ub,
            "output {s} ({v:.17e}) not in [{lb:.17e}, {ub:.17e}]"
        );
    }

    // --- FlintArray Display ---

    #[test]
    fn test_fmt_array_basic() {
        // Intervals chosen so the shortest in-range repr is unambiguous:
        //   [0.999, 1.001] -> "1.0"   (i=0 truncation "1.e0" = 1.0_f64; ryu emits "1.0")
        //   [2.49,  2.51 ] -> "2.5"   (i=1 truncation "2.5e0" is in range)
        //   [3.141, 3.142] -> "3.141" (need 4 sig figs to land in range)
        //   [-0.5,  0.5  ] -> "0"     (straddles zero)
        let a = FlintArray::<f64, 4> {
            lb: [0.999, 2.49, 3.141, -0.5],
            ub: [1.001, 2.51, 3.142, 0.5],
        };
        assert_eq!("[1.0, 2.5, 3.141, 0]", format!("{a}"));
    }

    #[test]
    fn test_fmt_array_single() {
        let a = FlintArray::<f64, 1> {
            lb: [1.555],
            ub: [1.565],
        };
        assert_eq!("[1.56]", format!("{a}"));
    }

    #[test]
    fn test_fmt_array_empty() {
        let a = FlintArray::<f32, 0> { lb: [], ub: [] };
        assert_eq!("[]", format!("{a}"));
    }

    // --- FlintVec Display ---

    #[test]
    fn test_fmt_vec_basic() {
        let v = FlintVec::<f64> {
            lb: vec![0.999, 2.49, 3.141, -0.5],
            ub: vec![1.001, 2.51, 3.142, 0.5],
        };
        assert_eq!("[1.0, 2.5, 3.141, 0]", format!("{v}"));
    }

    #[test]
    fn test_fmt_vec_single() {
        let v = FlintVec::<f64> {
            lb: vec![1.555],
            ub: vec![1.565],
        };
        assert_eq!("[1.56]", format!("{v}"));
    }

    #[test]
    fn test_fmt_vec_empty() {
        let v = FlintVec::<f64> {
            lb: vec![],
            ub: vec![],
        };
        assert_eq!("[]", format!("{v}"));
    }

    // --- FlintView Display ---

    #[test]
    fn test_fmt_view_basic() {
        let lb = [0.999_f64, 2.49, 3.141, -0.5];
        let ub = [1.001_f64, 2.51, 3.142, 0.5];
        let view = FlintView { lb: &lb, ub: &ub };
        assert_eq!("[1.0, 2.5, 3.141, 0]", format!("{view}"));
    }

    #[test]
    fn test_fmt_view_single() {
        let lb = [1.555_f64];
        let ub = [1.565_f64];
        let view = FlintView { lb: &lb, ub: &ub };
        assert_eq!("[1.56]", format!("{view}"));
    }

    #[test]
    fn test_fmt_view_empty() {
        let lb: [f64; 0] = [];
        let ub: [f64; 0] = [];
        let view = FlintView { lb: &lb, ub: &ub };
        assert_eq!("[]", format!("{view}"));
    }

    // --- FlintMut Display ---

    #[test]
    fn test_fmt_mut_basic() {
        let mut lb = 1.555_f32;
        let mut ub = 1.565_f32;
        let m = FlintMut {
            lb: &mut lb,
            ub: &mut ub,
        };
        assert_eq!("1.56", format!("{m}"));
    }

    #[test]
    fn test_fmt_mut_matches_owned() {
        let cases: &[(f64, f64)] = &[
            (1.555, 1.565),
            (-1.6, -1.55),
            (0.0, 0.0),
            (-0.5, 0.5),
        ];
        for &(lbv, ubv) in cases {
            let owned = Flint { lb: lbv, ub: ubv };
            let mut lb = lbv;
            let mut ub = ubv;
            let m = FlintMut {
                lb: &mut lb,
                ub: &mut ub,
            };
            assert_eq!(
                format!("{owned}"),
                format!("{m}"),
                "mismatch for lb={lbv} ub={ubv}"
            );
        }
    }

    #[test]
    fn test_fmt_mut_negative_narrow() {
        let ubv = -1.5999999999999999_f64;
        let lbv = -1.601_f64;
        let mut lb = lbv;
        let mut ub = ubv;
        let m = FlintMut {
            lb: &mut lb,
            ub: &mut ub,
        };
        let s = format!("{m}");
        let v: f64 = s.parse().expect("output should be a valid float");
        assert!(
            lbv <= v && v <= ubv,
            "output {s} ({v:.17e}) not in [{lbv:.17e}, {ubv:.17e}]"
        );
    }

    // --- FlintViewMut Display ---

    #[test]
    fn test_fmt_view_mut_basic() {
        let mut lb = [0.999_f64, 2.49, 3.141, -0.5];
        let mut ub = [1.001_f64, 2.51, 3.142, 0.5];
        let view = FlintViewMut {
            lb: &mut lb,
            ub: &mut ub,
        };
        assert_eq!("[1.0, 2.5, 3.141, 0]", format!("{view}"));
    }

    #[test]
    fn test_fmt_view_mut_single() {
        let mut lb = [1.555_f64];
        let mut ub = [1.565_f64];
        let view = FlintViewMut {
            lb: &mut lb,
            ub: &mut ub,
        };
        assert_eq!("[1.56]", format!("{view}"));
    }

    #[test]
    fn test_fmt_view_mut_empty() {
        let mut lb: [f64; 0] = [];
        let mut ub: [f64; 0] = [];
        let view = FlintViewMut {
            lb: &mut lb,
            ub: &mut ub,
        };
        assert_eq!("[]", format!("{view}"));
    }

    #[test]
    fn test_fmt_view_mut_matches_view() {
        let lbs = [0.999_f64, 2.49, -1.6, -0.5];
        let ubs = [1.001_f64, 2.51, -1.55, 0.5];
        let view = FlintView::<f64> { lb: &lbs, ub: &ubs };
        let mut lb_m = lbs;
        let mut ub_m = ubs;
        let view_mut = FlintViewMut {
            lb: &mut lb_m,
            ub: &mut ub_m,
        };
        assert_eq!(format!("{view}"), format!("{view_mut}"));
    }

    // --- Consistency: FlintArray, FlintVec, FlintView produce identical output ---

    #[test]
    fn test_fmt_array_types_consistent() {
        let lbs = [0.999_f64, 2.49, -1.6, -0.5];
        let ubs = [1.001_f64, 2.51, -1.55, 0.5];
        let arr = FlintArray::<f64, 4> { lb: lbs, ub: ubs };
        let vec = FlintVec::<f64> {
            lb: lbs.to_vec(),
            ub: ubs.to_vec(),
        };
        let view = FlintView::<f64> { lb: &lbs, ub: &ubs };
        let arr_s = format!("{arr}");
        let vec_s = format!("{vec}");
        let view_s = format!("{view}");
        assert_eq!(arr_s, vec_s, "FlintArray and FlintVec output differ");
        assert_eq!(arr_s, view_s, "FlintArray and FlintView output differ");
    }
}
