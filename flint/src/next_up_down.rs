use std::simd::{Select, prelude::*};

/// Get the next representable floating point value in upward (positive) or downward (negative)
/// direction
pub trait NextUpDown {
    /// Next representable floating point value in upward (positive) direction
    fn nu(&self) -> Self;
    /// Next representable floating point value in downward (negative) direction
    fn nd(&self) -> Self;
}

#[allow(unused)]
const F32_SIGN_MASK: u32 = 0x8000_0000_u32;

impl NextUpDown for f32 {
    fn nu(&self) -> Self {
        // NaN and inf values are returned unchanged
        if !(self.is_finite()) {
            return *self;
        }
        // otherwise, we do bitwise manipulation
        let mut u = self.to_bits();
        // next-up for negativ zero, we replace neg zero with pos zero
        if u == F32_SIGN_MASK {
            u = 0;
        }
        // if positive (sign bit = 0), we increment, else decrement
        if (u & F32_SIGN_MASK) == 0 {
            u += 1;
        } else {
            u -= 1;
        }
        f32::from_bits(u)
    }

    fn nd(&self) -> Self {
        // NaN and inf values are returned unchanged
        if !(self.is_finite()) {
            return *self;
        }
        // otherwise do bitwise manipulation
        let mut u = self.to_bits();
        // next-down for pos zero, we replace pos zero with neg zero
        if u == 0 {
            u = F32_SIGN_MASK;
        }
        // if postive, we decrement, else increment
        if (u & F32_SIGN_MASK) == 0 {
            u -= 1;
        } else {
            u += 1;
        }
        f32::from_bits(u)
    }
}

impl<const N: usize> NextUpDown for Simd<f32, N> {
    fn nu(&self) -> Self {
        // convert ot the bit-rep
        let mut u = self.to_bits();
        // get mask for nan and inf values
        let zero_mask = !self.is_finite();
        // next_up needs positive zero, so replace neg zeros with pos zeros
        let pos_zero: Simd<u32, N> = Simd::splat(0);
        let neg_zero_mask = u.simd_ne(Simd::splat(0x8000_0000));
        u = neg_zero_mask.select(u, pos_zero);
        // Now we take our values and either increment, decrement, or leave unchanged
        // so lets define three constant vectors zero, inc, dec
        let zero: Simd<i32, N> = Simd::splat(0);
        let inc: Simd<i32, N> = Simd::splat(1);
        let dec: Simd<i32, N> = Simd::splat(-1);
        // look for negative values
        let sign_mask: Simd<u32, N> = Simd::splat(F32_SIGN_MASK);
        let neg_mask = (u & sign_mask).simd_eq(sign_mask);
        // set dec for neg values, inc otherwise, then set zero
        let mut offset = neg_mask.select(dec, inc);
        offset = zero_mask.select(zero, offset);
        // add the offset
        u = (u.cast::<i32>() + offset).cast::<u32>();
        // convert back to
        SimdFloat::from_bits(u)
    }

    fn nd(&self) -> Self {
        // convert ot the bit-rep
        let mut u = self.to_bits();
        // get mask for nan and inf values
        let zero_mask = !self.is_finite();
        // next_down needs negative zero, so replace pos zeros with net zeros
        let neg_zero: Simd<u32, N> = Simd::splat(0x8000_0000);
        let pos_zero_mask = u.simd_ne(Simd::splat(0));
        u = pos_zero_mask.select(u, neg_zero);
        // Now we take our values and either increment, decrement, or leave unchanged
        // so lets define three constant vectors zero, inc, dec
        let zero: Simd<i32, N> = Simd::splat(0);
        let inc: Simd<i32, N> = Simd::splat(1);
        let dec: Simd<i32, N> = Simd::splat(-1);
        // look for negative values
        let sign_mask: Simd<u32, N> = Simd::splat(F32_SIGN_MASK);
        let neg_mask = (u & sign_mask).simd_eq(sign_mask);
        // set inc for neg values, dec otherwise, then set zero
        let mut offset = neg_mask.select(inc, dec);
        offset = zero_mask.select(zero, offset);
        // add the offset
        u = (u.cast::<i32>() + offset).cast::<u32>();
        // convert back to
        SimdFloat::from_bits(u)
    }
}

impl<const N: usize> NextUpDown for [f32; N] {
    fn nu(&self) -> Self {
        let v: Simd<f32, N> = (*self).into();
        v.nu().to_array()
    }
    fn nd(&self) -> Self {
        let v: Simd<f32, N> = (*self).into();
        v.nd().to_array()
    }
}

impl NextUpDown for Vec<f32> {
    fn nu(&self) -> Self {
        const LANE_SIZE: usize = 8;
        let mut output = Vec::with_capacity(self.len());
        let mut chunks = self.chunks_exact(LANE_SIZE);
        for chunk in &mut chunks {
            let simd_chunk: Simd<f32, LANE_SIZE> = Simd::from_slice(chunk);
            output.extend_from_slice(simd_chunk.nu().as_array());
        }
        for val in chunks.remainder() {
            output.push(val.nu())
        }
        output
    }
    fn nd(&self) -> Self {
        const LANE_SIZE: usize = 8;
        let mut output = Vec::with_capacity(self.len());
        let mut chunks = self.chunks_exact(LANE_SIZE);
        for chunk in &mut chunks {
            let simd_chunk: Simd<f32, LANE_SIZE> = Simd::from_slice(chunk);
            output.extend_from_slice(simd_chunk.nd().as_array());
        }
        for val in chunks.remainder() {
            output.push(val.nd())
        }
        output
    }
}

#[allow(unused)]
const F64_SIGN_MASK: u64 = 0x8000_0000_0000_0000_u64;

impl NextUpDown for f64 {
    fn nu(&self) -> Self {
        // NaN and inf values are returned unchanged
        if !(self.is_finite()) {
            return *self;
        }
        // otherwise, we do bitwise manipulation
        let mut u = self.to_bits();
        // next-up for negativ zero, we replace neg zero with pos zero
        if u == F64_SIGN_MASK {
            u = 0;
        }
        // if positive (sign bit = 0), we increment, else decrement
        if (u & F64_SIGN_MASK) == 0 {
            u += 1;
        } else {
            u -= 1;
        }
        f64::from_bits(u)
    }

    fn nd(&self) -> Self {
        // NaN and inf values are returned unchanged
        if !(self.is_finite()) {
            return *self;
        }
        // otherwise do bitwise manipulation
        let mut u = self.to_bits();
        // next-down for pos zero, we replace pos zero with neg zero
        if u == 0 {
            u = F64_SIGN_MASK;
        }
        // if postive, we decrement, else increment
        if (u & F64_SIGN_MASK) == 0 {
            u -= 1;
        } else {
            u += 1;
        }
        f64::from_bits(u)
    }
}

impl<const N: usize> NextUpDown for Simd<f64, N> {
    fn nu(&self) -> Self {
        // convert ot the bit-rep
        let mut u = self.to_bits();
        // get mask for nan and inf values
        let zero_mask = !self.is_finite();
        // next_up needs positive zero, so replace neg zeros with pos zeros
        let pos_zero: Simd<u64, N> = Simd::splat(0);
        let neg_zero_mask = u.simd_ne(Simd::splat(0x8000_0000_0000_0000));
        u = neg_zero_mask.select(u, pos_zero);
        // Now we take our values and either increment, decrement, or leave unchanged
        // so lets define three constant vectors zero, inc, dec
        let zero: Simd<i64, N> = Simd::splat(0);
        let inc: Simd<i64, N> = Simd::splat(1);
        let dec: Simd<i64, N> = Simd::splat(-1);
        // look for negative values
        let sign_mask: Simd<u64, N> = Simd::splat(F64_SIGN_MASK);
        let neg_mask = (u & sign_mask).simd_eq(sign_mask);
        // set dec for neg values, inc otherwise, then set zero
        let mut offset = neg_mask.select(dec, inc);
        offset = zero_mask.select(zero, offset);
        // add the offset
        u = (u.cast::<i64>() + offset).cast::<u64>();
        // convert back to
        SimdFloat::from_bits(u)
    }

    fn nd(&self) -> Self {
        // convert ot the bit-rep
        let mut u = self.to_bits();
        // get mask for nan and inf values
        let zero_mask = !self.is_finite();
        // next_down needs negative zero, so replace pos zeros with net zeros
        let neg_zero: Simd<u64, N> = Simd::splat(0x8000_0000_0000_0000);
        let pos_zero_mask = u.simd_ne(Simd::splat(0));
        u = pos_zero_mask.select(u, neg_zero);
        // Now we take our values and either increment, decrement, or leave unchanged
        // so lets define three constant vectors zero, inc, dec
        let zero: Simd<i64, N> = Simd::splat(0);
        let inc: Simd<i64, N> = Simd::splat(1);
        let dec: Simd<i64, N> = Simd::splat(-1);
        // look for negative values
        let sign_mask: Simd<u64, N> = Simd::splat(F64_SIGN_MASK);
        let neg_mask = (u & sign_mask).simd_eq(sign_mask);
        // set inc for neg values, dec otherwise, then set zero
        let mut offset = neg_mask.select(inc, dec);
        offset = zero_mask.select(zero, offset);
        // add the offset
        u = (u.cast::<i64>() + offset).cast::<u64>();
        // convert back to
        SimdFloat::from_bits(u)
    }
}

impl<const N: usize> NextUpDown for [f64; N] {
    fn nu(&self) -> Self {
        let v: Simd<f64, N> = (*self).into();
        v.nu().to_array()
    }
    fn nd(&self) -> Self {
        let v: Simd<f64, N> = (*self).into();
        v.nd().to_array()
    }
}

impl NextUpDown for Vec<f64> {
    fn nu(&self) -> Self {
        const LANE_SIZE: usize = 8;
        let mut output = Vec::with_capacity(self.len());
        let mut chunks = self.chunks_exact(LANE_SIZE);
        for chunk in &mut chunks {
            let simd_chunk: Simd<f64, LANE_SIZE> = Simd::from_slice(chunk);
            output.extend_from_slice(simd_chunk.nu().as_array());
        }
        for val in chunks.remainder() {
            output.push(val.nu())
        }
        output
    }
    fn nd(&self) -> Self {
        const LANE_SIZE: usize = 8;
        let mut output = Vec::with_capacity(self.len());
        let mut chunks = self.chunks_exact(LANE_SIZE);
        for chunk in &mut chunks {
            let simd_chunk: Simd<f64, LANE_SIZE> = Simd::from_slice(chunk);
            output.extend_from_slice(simd_chunk.nd().as_array());
        }
        for val in chunks.remainder() {
            output.push(val.nd())
        }
        output
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn f32_nu_nd() {
        // validate next up and down at 1
        let a = 1.0_f32;
        assert_eq!(1.0000001, a.nu());
        assert_eq!(0.99999994, a.nd());
        // validate next up and down at -1
        let a = -1.0_f32;
        assert_eq!(-0.99999994, a.nu());
        assert_eq!(-1.0000001, a.nd());
        // validate nan stays nan
        let a = f32::NAN;
        assert!(a.nu().is_nan());
        assert!(a.nd().is_nan());
        // validate infinite stays infinite
        let a = f32::INFINITY;
        assert!(a.nu().is_infinite());
        assert!(a.nd().is_infinite());
        let a = f32::NEG_INFINITY;
        assert!(a.nu().is_infinite());
        assert!(a.nd().is_infinite());
        // show we go infinite at max value
        let a = f32::from_bits(0x7f7f_ffff);
        assert!(!(a.is_infinite()));
        assert!(a.nu().is_infinite());
        assert!(!(a.nd().is_infinite()));
        // show we go negative inf at li
        let a = f32::from_bits(0xff7f_ffff);
        assert!(!(a.is_infinite()));
        assert!(!(a.nu().is_infinite()));
        assert!(a.nd().is_infinite());
    }

    #[test]
    fn simd_f32_nu() {
        // check all special values
        let a = f32x8::from_array([
            f32::NEG_INFINITY,
            f32::from_bits(0xff7f_ffff),
            -1.0,
            -0.0,
            0.0,
            1.0,
            f32::from_bits(0x7f7f_ffff),
            f32::INFINITY,
        ]);
        let expect = f32x8::from_array([
            f32::NEG_INFINITY,
            -3.4028233e38,
            -0.99999994,
            1e-45,
            1e-45,
            1.0000001,
            f32::INFINITY,
            f32::INFINITY,
        ]);
        eprintln!("{:?}", a.nu());
        assert_eq!(expect, a.nu());
        // check NAN
        let b = f32x1::splat(f32::NAN);
        assert!(b.nu().is_nan().test(0));
    }

    #[test]
    fn simd_f32_nd() {
        // check all special values
        let a = f32x8::from_array([
            f32::NEG_INFINITY,
            f32::from_bits(0xff7f_ffff),
            -1.0,
            -0.0,
            0.0,
            1.0,
            f32::from_bits(0x7f7f_ffff),
            f32::INFINITY,
        ]);
        let expect = f32x8::from_array([
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
            -1.0000001,
            -1e-45,
            -1e-45,
            0.99999994,
            3.4028233e38,
            f32::INFINITY,
        ]);
        eprintln!("{:?}", a.nd());
        assert_eq!(expect, a.nd());
        // check NAN
        let b = f32x1::splat(f32::NAN);
        assert!(b.nd().is_nan().test(0));
    }

    #[test]
    fn array_f32_nu() {
        // check all special values
        let a = [
            f32::NEG_INFINITY,
            f32::from_bits(0xff7f_ffff),
            -1.0,
            -0.0,
            0.0,
            1.0,
            f32::from_bits(0x7f7f_ffff),
            f32::INFINITY,
        ];
        let expect = [
            f32::NEG_INFINITY,
            -3.4028233e38,
            -0.99999994,
            1e-45,
            1e-45,
            1.0000001,
            f32::INFINITY,
            f32::INFINITY,
        ];
        eprintln!("{:?}", a.nu());
        assert_eq!(expect, a.nu());
    }

    #[test]
    fn array_f32_nd() {
        // check all special values
        let a = [
            f32::NEG_INFINITY,
            f32::from_bits(0xff7f_ffff),
            -1.0,
            -0.0,
            0.0,
            1.0,
            f32::from_bits(0x7f7f_ffff),
            f32::INFINITY,
        ];
        let expect = [
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
            -1.0000001,
            -1e-45,
            -1e-45,
            0.99999994,
            3.4028233e38,
            f32::INFINITY,
        ];
        eprintln!("{:?}", a.nd());
        assert_eq!(expect, a.nd());
    }

    #[test]
    fn vec_f32_nd() {
        // check all special values
        let a = vec![
            f32::NEG_INFINITY,
            f32::from_bits(0xff7f_ffff),
            -1.0,
            -0.0,
            0.0,
            1.0,
            f32::from_bits(0x7f7f_ffff),
            f32::INFINITY,
        ];
        let expect = vec![
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
            -1.0000001,
            -1e-45,
            -1e-45,
            0.99999994,
            3.4028233e38,
            f32::INFINITY,
        ];
        eprintln!("{:?}", a.nd());
        assert_eq!(expect, a.nd());
    }

    #[test]
    fn f64_nu_nd() {
        // validate next up and down at 1
        let a = 1.0_f64;
        assert_eq!(1.0000000000000002, a.nu());
        assert_eq!(0.9999999999999999, a.nd());
        // validate next up and down at -1
        let a = -1.0_f64;
        assert_eq!(-0.9999999999999999, a.nu());
        assert_eq!(-1.0000000000000002, a.nd());
        // validate nan stays nan
        let a = f64::NAN;
        assert!(a.nu().is_nan());
        assert!(a.nd().is_nan());
        // validate infinite stays infinite
        let a = f64::INFINITY;
        assert!(a.nu().is_infinite());
        assert!(a.nd().is_infinite());
        let a = f64::NEG_INFINITY;
        assert!(a.nu().is_infinite());
        assert!(a.nd().is_infinite());
        // show we go infinite at max value
        let a = f64::from_bits(0x7fef_ffff_ffff_ffff);
        assert!(!(a.is_infinite()));
        assert!(a.nu().is_infinite());
        assert!(!(a.nd().is_infinite()));
        // show we go negative inf at li
        let a = f64::from_bits(0xffef_ffff_ffff_ffff);
        assert!(!(a.is_infinite()));
        assert!(!(a.nu().is_infinite()));
        assert!(a.nd().is_infinite());
    }

    #[test]
    fn simd_f64_nu() {
        // check all special values
        let a = f64x8::from_array([
            f64::NEG_INFINITY,
            f64::from_bits(0xffef_ffff_ffff_ffff),
            -1.0,
            -0.0,
            0.0,
            1.0,
            f64::from_bits(0x7fef_ffff_ffff_ffff),
            f64::INFINITY,
        ]);
        let expect = f64x8::from_array([
            f64::NEG_INFINITY,
            -1.7976931348623155e308,
            -0.9999999999999999,
            5e-324,
            5e-324,
            1.0000000000000002,
            f64::INFINITY,
            f64::INFINITY,
        ]);
        eprintln!("{:?}", a.nu());
        assert_eq!(expect, a.nu());
        // check NAN
        let b = f64x1::splat(f64::NAN);
        assert!(b.nu().is_nan().test(0));
    }

    #[test]
    fn simd_f64_nd() {
        // check all special values
        let a = f64x8::from_array([
            f64::NEG_INFINITY,
            f64::from_bits(0xffef_ffff_ffff_ffff),
            -1.0,
            -0.0,
            0.0,
            1.0,
            f64::from_bits(0x7fef_ffff_ffff_ffff),
            f64::INFINITY,
        ]);
        let expect = f64x8::from_array([
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
            -1.0000000000000002,
            -5e-324,
            -5e-324,
            0.9999999999999999,
            1.7976931348623155e308,
            f64::INFINITY,
        ]);
        eprintln!("{:?}", a.nd());
        assert_eq!(expect, a.nd());
        // check NAN
        let b = f64x1::splat(f64::NAN);
        assert!(b.nd().is_nan().test(0));
    }

    #[test]
    fn array_f64_nu() {
        // check all special values
        let a = [
            f64::NEG_INFINITY,
            f64::from_bits(0xffef_ffff_ffff_ffff),
            -1.0,
            -0.0,
            0.0,
            1.0,
            f64::from_bits(0x7fef_ffff_ffff_ffff),
            f64::INFINITY,
        ];
        let expect = [
            f64::NEG_INFINITY,
            -1.7976931348623155e308,
            -0.9999999999999999,
            5e-324,
            5e-324,
            1.0000000000000002,
            f64::INFINITY,
            f64::INFINITY,
        ];
        eprintln!("{:?}", a.nu());
        assert_eq!(expect, a.nu());
    }

    #[test]
    fn array_f64_nd() {
        // check all special values
        let a = [
            f64::NEG_INFINITY,
            f64::from_bits(0xffef_ffff_ffff_ffff),
            -1.0,
            -0.0,
            0.0,
            1.0,
            f64::from_bits(0x7fef_ffff_ffff_ffff),
            f64::INFINITY,
        ];
        let expect = [
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
            -1.0000000000000002,
            -5e-324,
            -5e-324,
            0.9999999999999999,
            1.7976931348623155e308,
            f64::INFINITY,
        ];
        eprintln!("{:?}", a.nd());
        assert_eq!(expect, a.nd());
    }

    #[test]
    fn vec_f64_nd() {
        // check all special values
        let a = vec![
            f64::NEG_INFINITY,
            f64::from_bits(0xffef_ffff_ffff_ffff),
            -1.0,
            -0.0,
            0.0,
            1.0,
            f64::from_bits(0x7fef_ffff_ffff_ffff),
            f64::INFINITY,
        ];
        let expect = vec![
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
            -1.0000000000000002,
            -5e-324,
            -5e-324,
            0.9999999999999999,
            1.7976931348623155e308,
            f64::INFINITY,
        ];
        eprintln!("{:?}", a.nd());
        assert_eq!(expect, a.nd());
    }
}
