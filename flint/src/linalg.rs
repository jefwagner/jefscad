use num_traits::Float;

use crate::next_up_down::NextUpDown;
use crate::{Flint, FlintArray, FlintSoA, FlintVec};

// -----------------------------------------------------------------------
// Internal helper: 3×3 determinant from nine interval values (row-major).
// -----------------------------------------------------------------------

fn det3x3<T>(m: [Flint<T>; 9]) -> Flint<T>
where
    T: Float + NextUpDown + Copy,
{
    let [m00, m01, m02, m10, m11, m12, m20, m21, m22] = m;
    m00 * (m11 * m22 - m12 * m21)
        - m01 * (m10 * m22 - m12 * m20)
        + m02 * (m10 * m21 - m11 * m20)
}

// -----------------------------------------------------------------------
// FlintArray<T, 16> — 4×4 matrix methods
// -----------------------------------------------------------------------

impl<T> FlintArray<T, 16>
where
    T: Float + NextUpDown + Copy,
{
    /// Multiply two 4×4 interval matrices (row-major).
    ///
    /// Result element `(i, j)` = sum over k of `self(i, k) * rhs(k, j)`.
    pub fn mat_mul(&self, rhs: &FlintArray<T, 16>) -> FlintArray<T, 16> {
        let zero = Flint { lb: T::zero(), ub: T::zero() };
        let mut lb = [T::zero(); 16];
        let mut ub = [T::zero(); 16];
        for i in 0..4 {
            for j in 0..4 {
                let mut acc = zero;
                for k in 0..4 {
                    let a = Flint { lb: self.lb[i * 4 + k], ub: self.ub[i * 4 + k] };
                    let b = Flint { lb: rhs.lb[k * 4 + j], ub: rhs.ub[k * 4 + j] };
                    acc = acc + a * b;
                }
                lb[i * 4 + j] = acc.lb;
                ub[i * 4 + j] = acc.ub;
            }
        }
        FlintArray { lb, ub }
    }

    /// Apply this 4×4 matrix to a single 4-element column vector.
    ///
    /// # Panics
    /// Panics if `vec` does not contain exactly 4 elements.
    pub fn apply<V: FlintSoA<T>>(&self, vec: &V) -> FlintArray<T, 4> {
        let (vlb, vub) = vec.parts();
        assert!(
            vlb.len() == 4,
            "apply: vector must have exactly 4 elements, got {}",
            vlb.len()
        );
        let zero = Flint { lb: T::zero(), ub: T::zero() };
        let mut lb = [T::zero(); 4];
        let mut ub = [T::zero(); 4];
        for i in 0..4 {
            let mut acc = zero;
            for k in 0..4 {
                let a = Flint { lb: self.lb[i * 4 + k], ub: self.ub[i * 4 + k] };
                let b = Flint { lb: vlb[k], ub: vub[k] };
                acc = acc + a * b;
            }
            lb[i] = acc.lb;
            ub[i] = acc.ub;
        }
        FlintArray { lb, ub }
    }

    /// Apply this 4×4 matrix to a batch of 4-element column vectors.
    ///
    /// The input is a flat SoA with `4N` elements. Vectors are stored as rows:
    /// elements `[4i, 4i+1, 4i+2, 4i+3]` form vector `i`. The output is a
    /// `FlintVec<T>` of the same length with the same row layout. An empty
    /// input returns an empty `FlintVec`.
    ///
    /// # Panics
    /// Panics if the input length is not divisible by 4.
    pub fn apply_batch<V: FlintSoA<T>>(&self, cols: &V) -> FlintVec<T> {
        let (clb, cub) = cols.parts();
        assert!(
            clb.len() % 4 == 0,
            "apply_batch: input length {} is not divisible by 4",
            clb.len()
        );
        let n = clb.len() / 4;
        let zero = Flint { lb: T::zero(), ub: T::zero() };
        let mut out_lb = Vec::with_capacity(clb.len());
        let mut out_ub = Vec::with_capacity(clb.len());
        for r in 0..n {
            for i in 0..4 {
                let mut acc = zero;
                for k in 0..4 {
                    let a = Flint { lb: self.lb[i * 4 + k], ub: self.ub[i * 4 + k] };
                    let b = Flint { lb: clb[r * 4 + k], ub: cub[r * 4 + k] };
                    acc = acc + a * b;
                }
                out_lb.push(acc.lb);
                out_ub.push(acc.ub);
            }
        }
        FlintVec { lb: out_lb, ub: out_ub }
    }

    /// Compute the determinant of the upper-left 3×3 submatrix.
    pub fn det3(&self) -> Flint<T> {
        let m = |r: usize, c: usize| Flint { lb: self.lb[r * 4 + c], ub: self.ub[r * 4 + c] };
        det3x3([
            m(0, 0), m(0, 1), m(0, 2),
            m(1, 0), m(1, 1), m(1, 2),
            m(2, 0), m(2, 1), m(2, 2),
        ])
    }

    /// Compute the 4×4 determinant via cofactor expansion along the first row.
    pub fn det4(&self) -> Flint<T> {
        let m = |r: usize, c: usize| Flint { lb: self.lb[r * 4 + c], ub: self.ub[r * 4 + c] };
        let c00 = det3x3([
            m(1, 1), m(1, 2), m(1, 3),
            m(2, 1), m(2, 2), m(2, 3),
            m(3, 1), m(3, 2), m(3, 3),
        ]);
        let c01 = det3x3([
            m(1, 0), m(1, 2), m(1, 3),
            m(2, 0), m(2, 2), m(2, 3),
            m(3, 0), m(3, 2), m(3, 3),
        ]);
        let c02 = det3x3([
            m(1, 0), m(1, 1), m(1, 3),
            m(2, 0), m(2, 1), m(2, 3),
            m(3, 0), m(3, 1), m(3, 3),
        ]);
        let c03 = det3x3([
            m(1, 0), m(1, 1), m(1, 2),
            m(2, 0), m(2, 1), m(2, 2),
            m(3, 0), m(3, 1), m(3, 2),
        ]);
        m(0, 0) * c00 - m(0, 1) * c01 + m(0, 2) * c02 - m(0, 3) * c03
    }
}

// -----------------------------------------------------------------------
// FlintArray<T, 4> — 4-vector methods
// -----------------------------------------------------------------------

impl<T> FlintArray<T, 4>
where
    T: Float + NextUpDown + Copy,
{
    /// Compute the dot product of this 4-vector with another 4-element SoA.
    ///
    /// # Panics
    /// Panics if `rhs` does not contain exactly 4 elements.
    pub fn dot<V: FlintSoA<T>>(&self, rhs: &V) -> Flint<T> {
        let (rlb, rub) = rhs.parts();
        assert!(
            rlb.len() == 4,
            "dot: rhs must have exactly 4 elements, got {}",
            rlb.len()
        );
        let zero = Flint { lb: T::zero(), ub: T::zero() };
        let mut acc = zero;
        for i in 0..4 {
            let a = Flint { lb: self.lb[i], ub: self.ub[i] };
            let b = Flint { lb: rlb[i], ub: rub[i] };
            acc = acc + a * b;
        }
        acc
    }

    /// Compute dot products of this 4-vector with a batch of 4-element vectors.
    ///
    /// The input is a flat SoA with `4N` elements. Vectors are stored as rows:
    /// elements `[4i, 4i+1, 4i+2, 4i+3]` form vector `i`. Returns a `FlintVec<T>`
    /// of N scalar dot products. An empty input returns an empty `FlintVec`.
    ///
    /// # Panics
    /// Panics if the input length is not divisible by 4.
    pub fn dot_batch<V: FlintSoA<T>>(&self, cols: &V) -> FlintVec<T> {
        let (clb, cub) = cols.parts();
        assert!(
            clb.len() % 4 == 0,
            "dot_batch: input length {} is not divisible by 4",
            clb.len()
        );
        let n = clb.len() / 4;
        let zero = Flint { lb: T::zero(), ub: T::zero() };
        let mut out_lb = Vec::with_capacity(n);
        let mut out_ub = Vec::with_capacity(n);
        for r in 0..n {
            let mut acc = zero;
            for i in 0..4 {
                let a = Flint { lb: self.lb[i], ub: self.ub[i] };
                let b = Flint { lb: clb[r * 4 + i], ub: cub[r * 4 + i] };
                acc = acc + a * b;
            }
            out_lb.push(acc.lb);
            out_ub.push(acc.ub);
        }
        FlintVec { lb: out_lb, ub: out_ub }
    }
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use crate::{flint64_arr, flint64_vec};

    // Check that every element of a FlintArray<f64, N> contains the expected value.
    fn arr_contains<const N: usize>(arr: &FlintArray<f64, N>, expected: &[f64; N]) {
        for i in 0..N {
            assert!(
                arr.lb[i] <= expected[i] && expected[i] <= arr.ub[i],
                "element {i}: expected {} not in [{}, {}]",
                expected[i], arr.lb[i], arr.ub[i]
            );
        }
    }

    // Check that every element of a FlintVec<f64> contains the expected value.
    fn vec_contains(v: &FlintVec<f64>, expected: &[f64]) {
        assert_eq!(v.lb.len(), expected.len(), "length mismatch");
        for i in 0..expected.len() {
            assert!(
                v.lb[i] <= expected[i] && expected[i] <= v.ub[i],
                "element {i}: expected {} not in [{}, {}]",
                expected[i], v.lb[i], v.ub[i]
            );
        }
    }

    // Check that a scalar Flint<f64> contains the expected value.
    fn scalar_contains(f: Flint<f64>, expected: f64) {
        assert!(
            f.lb <= expected && expected <= f.ub,
            "expected {expected} not in [{}, {}]",
            f.lb, f.ub
        );
    }

    // --- mat_mul ---

    #[test]
    fn mat_mul_identity_left() {
        let m: FlintArray<f64, 16> =
            flint64_arr!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
        let id: FlintArray<f64, 16> =
            flint64_arr!(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1);
        let result = id.mat_mul(&m);
        arr_contains(&result, &[1., 2., 3., 4., 5., 6., 7., 8., 9., 10., 11., 12., 13., 14., 15., 16.]);
    }

    #[test]
    fn mat_mul_identity_right() {
        let m: FlintArray<f64, 16> =
            flint64_arr!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
        let id: FlintArray<f64, 16> =
            flint64_arr!(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1);
        let result = m.mat_mul(&id);
        arr_contains(&result, &[1., 2., 3., 4., 5., 6., 7., 8., 9., 10., 11., 12., 13., 14., 15., 16.]);
    }

    #[test]
    fn mat_mul_known() {
        // 90° CCW around Z times 90° CW around Z = identity
        let ccw: FlintArray<f64, 16> =
            flint64_arr!(0, -1, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1);
        let cw: FlintArray<f64, 16> =
            flint64_arr!(0, 1, 0, 0, -1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1);
        let result = ccw.mat_mul(&cw);
        arr_contains(&result, &[1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.]);
    }

    #[test]
    fn mat_mul_non_commutative() {
        // A = scale(2,3,5,1),  B = translate(1,2,3)
        // A*B = [2,0,0,2, 0,3,0,6, 0,0,5,15, 0,0,0,1]
        // B*A = [2,0,0,1, 0,3,0,2, 0,0,5,3,  0,0,0,1]
        let a: FlintArray<f64, 16> =
            flint64_arr!(2, 0, 0, 0, 0, 3, 0, 0, 0, 0, 5, 0, 0, 0, 0, 1);
        let b: FlintArray<f64, 16> =
            flint64_arr!(1, 0, 0, 1, 0, 1, 0, 2, 0, 0, 1, 3, 0, 0, 0, 1);
        let ab = a.mat_mul(&b);
        let ba = b.mat_mul(&a);
        arr_contains(&ab, &[2., 0., 0., 2., 0., 3., 0., 6., 0., 0., 5., 15., 0., 0., 0., 1.]);
        arr_contains(&ba, &[2., 0., 0., 1., 0., 3., 0., 2., 0., 0., 5., 3., 0., 0., 0., 1.]);
    }

    // --- apply ---

    #[test]
    fn apply_identity() {
        let id: FlintArray<f64, 16> =
            flint64_arr!(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1);
        let v: FlintArray<f64, 4> = flint64_arr!(1, 2, 3, 1);
        let result = id.apply(&v);
        arr_contains(&result, &[1., 2., 3., 1.]);
    }

    #[test]
    fn apply_scale() {
        let s: FlintArray<f64, 16> =
            flint64_arr!(2, 0, 0, 0, 0, 3, 0, 0, 0, 0, 5, 0, 0, 0, 0, 1);
        let v: FlintArray<f64, 4> = flint64_arr!(1, 2, 3, 1);
        let result = s.apply(&v);
        arr_contains(&result, &[2., 6., 15., 1.]);
    }

    #[test]
    fn apply_translation() {
        // translate by (10, 20, 30)
        let t: FlintArray<f64, 16> =
            flint64_arr!(1, 0, 0, 10, 0, 1, 0, 20, 0, 0, 1, 30, 0, 0, 0, 1);
        let v: FlintArray<f64, 4> = flint64_arr!(1, 2, 3, 1);
        let result = t.apply(&v);
        arr_contains(&result, &[11., 22., 33., 1.]);
    }

    #[test]
    #[should_panic]
    fn apply_panic_wrong_len() {
        let id: FlintArray<f64, 16> =
            flint64_arr!(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1);
        let v = flint64_vec!(1, 2, 3); // 3 elements — not 4
        let _ = id.apply(&v);
    }

    // --- apply_batch ---

    #[test]
    fn apply_batch_identity() {
        let id: FlintArray<f64, 16> =
            flint64_arr!(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1);
        // two row-vectors: [1,2,3,1] and [5,6,7,1]
        let cols = flint64_vec!(1, 2, 3, 1, 5, 6, 7, 1);
        let result = id.apply_batch(&cols);
        vec_contains(&result, &[1., 2., 3., 1., 5., 6., 7., 1.]);
    }

    #[test]
    fn apply_batch_scale() {
        let s: FlintArray<f64, 16> =
            flint64_arr!(2, 0, 0, 0, 0, 3, 0, 0, 0, 0, 5, 0, 0, 0, 0, 1);
        let cols = flint64_vec!(1, 2, 3, 1, 5, 6, 7, 1);
        let result = s.apply_batch(&cols);
        vec_contains(&result, &[2., 6., 15., 1., 10., 18., 35., 1.]);
    }

    #[test]
    fn apply_batch_empty() {
        let id: FlintArray<f64, 16> =
            flint64_arr!(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1);
        let cols = FlintVec::<f64> { lb: vec![], ub: vec![] };
        let result = id.apply_batch(&cols);
        assert_eq!(result.lb.len(), 0);
        assert_eq!(result.ub.len(), 0);
    }

    #[test]
    #[should_panic]
    fn apply_batch_panic_bad_len() {
        let id: FlintArray<f64, 16> =
            flint64_arr!(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1);
        let cols = flint64_vec!(1, 2, 3, 4, 5); // 5 elements — not divisible by 4
        let _ = id.apply_batch(&cols);
    }

    // --- det3 ---

    #[test]
    fn det3_identity() {
        let id: FlintArray<f64, 16> =
            flint64_arr!(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1);
        scalar_contains(id.det3(), 1.0);
    }

    #[test]
    fn det3_diagonal() {
        // upper-left diag(2, 3, 5) → det3 = 30
        let m: FlintArray<f64, 16> =
            flint64_arr!(2, 0, 0, 0, 0, 3, 0, 0, 0, 0, 5, 0, 0, 0, 0, 1);
        scalar_contains(m.det3(), 30.0);
    }

    #[test]
    fn det3_singular() {
        // zero first row in upper 3×3 → det3 = 0
        let m: FlintArray<f64, 16> =
            flint64_arr!(0, 0, 0, 5, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1);
        scalar_contains(m.det3(), 0.0);
    }

    #[test]
    fn det3_reflection() {
        // negate x axis → det3 = -1
        let m: FlintArray<f64, 16> =
            flint64_arr!(-1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1);
        scalar_contains(m.det3(), -1.0);
    }

    // --- det4 ---

    #[test]
    fn det4_identity() {
        let id: FlintArray<f64, 16> =
            flint64_arr!(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1);
        scalar_contains(id.det4(), 1.0);
    }

    #[test]
    fn det4_diagonal() {
        // diag(1, 2, 3, 4) → det4 = 24
        let m: FlintArray<f64, 16> =
            flint64_arr!(1, 0, 0, 0, 0, 2, 0, 0, 0, 0, 3, 0, 0, 0, 0, 4);
        scalar_contains(m.det4(), 24.0);
    }

    #[test]
    fn det4_singular() {
        // zero second row → det4 = 0
        let m: FlintArray<f64, 16> =
            flint64_arr!(1, 2, 3, 4, 0, 0, 0, 0, 9, 10, 11, 12, 13, 14, 15, 16);
        scalar_contains(m.det4(), 0.0);
    }

    #[test]
    fn det4_affine() {
        // 90° rotation around Z + translation — det4 = 1
        let m: FlintArray<f64, 16> =
            flint64_arr!(0, -1, 0, 5, 1, 0, 0, 3, 0, 0, 1, 2, 0, 0, 0, 1);
        scalar_contains(m.det4(), 1.0);
    }

    // --- dot ---

    #[test]
    fn dot_unit() {
        let a: FlintArray<f64, 4> = flint64_arr!(1, 0, 0, 0);
        let b: FlintArray<f64, 4> = flint64_arr!(1, 0, 0, 0);
        scalar_contains(a.dot(&b), 1.0);
    }

    #[test]
    fn dot_orthogonal() {
        let a: FlintArray<f64, 4> = flint64_arr!(1, 0, 0, 0);
        let b: FlintArray<f64, 4> = flint64_arr!(0, 1, 0, 0);
        scalar_contains(a.dot(&b), 0.0);
    }

    #[test]
    fn dot_known() {
        // [1,2,3,4] · [1,2,3,4] = 1 + 4 + 9 + 16 = 30
        let a: FlintArray<f64, 4> = flint64_arr!(1, 2, 3, 4);
        let b: FlintArray<f64, 4> = flint64_arr!(1, 2, 3, 4);
        scalar_contains(a.dot(&b), 30.0);
    }

    #[test]
    #[should_panic]
    fn dot_panic_wrong_len() {
        let a: FlintArray<f64, 4> = flint64_arr!(1, 0, 0, 0);
        let b = flint64_vec!(1, 2, 3); // 3 elements — not 4
        let _ = a.dot(&b);
    }

    // --- dot_batch ---

    #[test]
    fn dot_batch_known() {
        // [1,0,0,0] · rows [[1,2,3,4], [5,6,7,8]] = [1, 5]
        let a: FlintArray<f64, 4> = flint64_arr!(1, 0, 0, 0);
        let cols = flint64_vec!(1, 2, 3, 4, 5, 6, 7, 8);
        let result = a.dot_batch(&cols);
        vec_contains(&result, &[1.0, 5.0]);
    }

    #[test]
    fn dot_batch_empty() {
        let a: FlintArray<f64, 4> = flint64_arr!(1, 0, 0, 0);
        let cols = FlintVec::<f64> { lb: vec![], ub: vec![] };
        let result = a.dot_batch(&cols);
        assert_eq!(result.lb.len(), 0);
    }

    #[test]
    #[should_panic]
    fn dot_batch_panic_bad_len() {
        let a: FlintArray<f64, 4> = flint64_arr!(1, 0, 0, 0);
        let cols = flint64_vec!(1, 2, 3, 4, 5); // 5 elements — not divisible by 4
        let _ = a.dot_batch(&cols);
    }

    // --- interval containment ---

    #[test]
    fn apply_interval_containment() {
        // scale(2,3,5,1) applied to a vector where v[0] is the wide interval [1, 2].
        // result[0] = 2 * v[0] must contain both 2.0 and 4.0 (the endpoint images).
        let m: FlintArray<f64, 16> =
            flint64_arr!(2, 0, 0, 0, 0, 3, 0, 0, 0, 0, 5, 0, 0, 0, 0, 1);
        let v = FlintArray::<f64, 4> {
            lb: [1.0, 2.0, 3.0, 1.0],
            ub: [2.0, 2.0, 3.0, 1.0],
        };
        let result = m.apply(&v);
        assert!(
            result.lb[0] <= 2.0 && 4.0 <= result.ub[0],
            "result[0] interval [{}, {}] must contain both 2.0 and 4.0",
            result.lb[0], result.ub[0]
        );
        assert!(result.lb[1] <= 6.0 && 6.0 <= result.ub[1]);
        assert!(result.lb[2] <= 15.0 && 15.0 <= result.ub[2]);
        assert!(result.lb[3] <= 1.0 && 1.0 <= result.ub[3]);
    }
}
