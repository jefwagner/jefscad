use std::{
    fmt::{self, LowerExp},
    io::{Cursor, Write},
    str::FromStr,
};

use crate::Flint;

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
            if lb <= &ub_trunc {
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
}
