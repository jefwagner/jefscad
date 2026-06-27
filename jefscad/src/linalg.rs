//! Linear-algebra primitives: 4×4 affine matrices.
//!
//! [`Mat4`] is a row-major, column-vector (right-multiply) 4×4 matrix newtype matching
//! the layout used by `CsgNode::flat_transform` and the b-rep compiler's transform
//! absorption. It replaces the former `flint::FlintArray<f64, 16>` wrapper once the
//! `flint` dependency is removed (Phase 0-a).
//!
//! # SIMD
//!
//! `Mat4` is a plain `[f64; 16]` scalar implementation. The method-based interface
//! (inner array private) means a future SIMD-backed `Mat4` can swap representations
//! with zero call-site changes. See the Phase 0-a design notes for the deferral
//! rationale (matrix math is not a hot path; f64 SIMD on x86 is awkward and would
//! re-import a nightly feature gate).

use crate::geom::Point3;

/// Quantize scale for geometry-id hashing. Deliberately coarse (1e-6 lattice) so that
/// near-identical engine-built matrices canonicalize to the same id. Used by
/// `csg_lang::quantize_matrix`. Distinct from [`IDENTITY_QUANT_SCALE`]: the hash
/// purpose wants coarse snapping; the identity test must not swallow real transforms.
pub const QUANTIZE_SCALE: f64 = 1e6;

/// Quantize scale for the [`Mat4::is_identity`] test. Tight (1e-12 lattice, so two
/// matrices are "same identity" if every entry differs by < 5e-13). Tight enough that a
/// real sub-micron translation (e.g. a user's sliver-avoidance offset — motivation #3)
/// is *not* swallowed as "no transform." This is a different scale from
/// [`QUANTIZE_SCALE`] on purpose: the 1e-6 hash scale would be a correctness bug here.
pub const IDENTITY_QUANT_SCALE: f64 = 1e12;

/// A row-major 4×4 matrix, column-vector / right-multiply convention.
///
/// Stored as 16 `f64`s in row-major order: `m[row*4 + col]`. An affine transform of a
/// point `p` is `Mat4 · p` where `p` is a column vector with homogeneous `w = 1`.
///
/// `#[repr(transparent)]` guarantees the layout is exactly `[f64; 16]`, so the type is
/// FFI-safe and zero-cost to reinterpret as a raw 4×4 array (useful for future STEP
/// export that wants contiguous doubles).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4([f64; 16]);

impl Mat4 {
    /// The 4×4 identity matrix.
    pub const IDENTITY: Mat4 = Mat4([
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]);

    /// Construct from a row-major `[f64; 16]`.
    pub const fn from_array(arr: [f64; 16]) -> Mat4 {
        Mat4(arr)
    }

    /// Matrix composition: returns `self · rhs`.
    ///
    /// Used to compose transforms as a CSG tree is built (`with_transform`): the
    /// child's flat_transform is `parent · child_local`.
    pub fn mat_mul(&self, rhs: &Mat4) -> Mat4 {
        let a = &self.0;
        let b = &rhs.0;
        let mut out = [0.0; 16];
        for row in 0..4 {
            for col in 0..4 {
                // out[row*4 + col] = sum over k of a[row, k] * b[k, col]
                let mut sum = 0.0;
                for k in 0..4 {
                    sum += a[row * 4 + k] * b[k * 4 + col];
                }
                out[row * 4 + col] = sum;
            }
        }
        Mat4(out)
    }

    /// Apply to a point (homogeneous `w = 1`): translation applies.
    pub fn apply_pt(&self, p: Point3) -> Point3 {
        let m = &self.0;
        // p as column vector (x, y, z, 1); result row i = row i · (x,y,z,1).
        Point3 {
            x: m[0] * p.x + m[1] * p.y + m[2] * p.z + m[3],
            y: m[4] * p.x + m[5] * p.y + m[6] * p.z + m[7],
            z: m[8] * p.x + m[9] * p.y + m[10] * p.z + m[11],
        }
    }

    /// Apply to a vector (homogeneous `w = 0`): translation ignored, only the linear
    /// part applies. Use for directions/normals.
    pub fn apply_vec(&self, v: Point3) -> Point3 {
        let m = &self.0;
        // v as column vector (x, y, z, 0); translation column (m[3],m[7],m[11]) dropped.
        Point3 {
            x: m[0] * v.x + m[1] * v.y + m[2] * v.z,
            y: m[4] * v.x + m[5] * v.y + m[6] * v.z,
            z: m[8] * v.x + m[9] * v.y + m[10] * v.z,
        }
    }

    /// Inverse. Panics on singular (det == 0).
    ///
    /// Engine-built affine transforms are always invertible; a singular matrix is a
    /// programmer error, not a runtime condition.
    ///
    /// # Future: STEP import
    ///
    /// When STEP import of arbitrary external matrices lands, this should be revisited
    /// — external matrices can legitimately be singular. Likely split into an
    /// infallible `inverse_affine` (this, for engine use) and a fallible variant
    /// returning `Result`/`Option` for import.
    pub fn inverse(&self) -> Mat4 {
        let m = &self.0;
        // Upper-left 3×3 linear part A and translation column t.
        // (row-major: m[row*4 + col]; translation is column 3 → m[3], m[7], m[11].)
        let a = [m[0], m[1], m[2], m[4], m[5], m[6], m[8], m[9], m[10]];
        let t = [m[3], m[7], m[11]];

        let det = a[0] * (a[4] * a[8] - a[5] * a[7]) - a[1] * (a[3] * a[8] - a[5] * a[6])
            + a[2] * (a[3] * a[7] - a[4] * a[6]);
        assert!(det != 0.0, "Mat4::inverse: singular transform (det == 0)");
        let id = 1.0 / det;

        // Inverse of A (adjugate / det).
        #[rustfmt::skip]
        let ia = [
             (a[4]*a[8] - a[5]*a[7]) * id,
            -(a[1]*a[8] - a[2]*a[7]) * id,
             (a[1]*a[5] - a[2]*a[4]) * id,
            -(a[3]*a[8] - a[5]*a[6]) * id,
             (a[0]*a[8] - a[2]*a[6]) * id,
            -(a[0]*a[5] - a[2]*a[3]) * id,
             (a[3]*a[7] - a[4]*a[6]) * id,
            -(a[0]*a[7] - a[1]*a[6]) * id,
             (a[0]*a[4] - a[1]*a[3]) * id,
        ];

        // Inverse translation: −A⁻¹ · t.
        let it = [
            -(ia[0] * t[0] + ia[1] * t[1] + ia[2] * t[2]),
            -(ia[3] * t[0] + ia[4] * t[1] + ia[5] * t[2]),
            -(ia[6] * t[0] + ia[7] * t[1] + ia[8] * t[2]),
        ];

        Mat4([
            ia[0], ia[1], ia[2], it[0], ia[3], ia[4], ia[5], it[1], ia[6], ia[7], ia[8], it[2],
            0.0, 0.0, 0.0, 1.0,
        ])
    }

    /// True if this is the identity matrix.
    ///
    /// Uses the quantize-and-compare-to-identity pattern at [`IDENTITY_QUANT_SCALE`]
    /// (1e-12 lattice). Two matrices are "same identity" if every entry rounds to the
    /// same lattice cell as the identity. Tight enough that a real sub-micron
    /// translation is *not* swallowed (motivation #3: tolerance-aware booleans).
    pub fn is_identity(&self) -> bool {
        quantize_identity(&self.0) == quantize_identity(&Mat4::IDENTITY.0)
    }

    /// Borrow the inner row-major array. Use for inspection / tests / export; do **not**
    /// call this in a hot loop — reach for [`apply_pt`](Self::apply_pt) /
    /// [`apply_vec`](Self::apply_vec) instead so a future SIMD-backed `Mat4` can swap
    /// representations without call-site changes.
    pub fn as_array(&self) -> &[f64; 16] {
        &self.0
    }
}

/// Quantize a matrix's entries to an integer lattice at the identity-test scale
/// (1e-12). Helper for [`Mat4::is_identity`].
fn quantize_identity(m: &[f64; 16]) -> [i64; 16] {
    let mut out = [0i64; 16];
    for i in 0..16 {
        out[i] = (m[i] * IDENTITY_QUANT_SCALE).round() as i64;
    }
    out
}

// ---------------------------------------------------------------------------
// Constructors
// ---------------------------------------------------------------------------

/// Build a translation matrix.
pub fn mat_translation(dx: f64, dy: f64, dz: f64) -> Mat4 {
    Mat4([
        1.0, 0.0, 0.0, dx, 0.0, 1.0, 0.0, dy, 0.0, 0.0, 1.0, dz, 0.0, 0.0, 0.0, 1.0,
    ])
}

/// Build a (non-uniform) scale matrix.
#[rustfmt::skip]
pub fn mat_scale(sx: f64, sy: f64, sz: f64) -> Mat4 {
    Mat4([
        sx,  0.0, 0.0, 0.0,
        0.0, sy,  0.0, 0.0,
        0.0, 0.0, sz,  0.0,
        0.0, 0.0, 0.0, 1.0,
    ])
}

/// Build a rotation matrix about an arbitrary axis through the origin (Rodrigues'
/// formula). `axis` is normalized internally; `angle` is in radians, right-hand
/// rule (positive angle rotates counter-clockwise looking back along the axis).
pub fn mat_rot_aa(axis: [f64; 3], angle: f64) -> Mat4 {
    let len = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    let [ux, uy, uz] = [axis[0] / len, axis[1] / len, axis[2] / len];

    let c = angle.cos();
    let s = angle.sin();
    let t = 1.0 - c; // (1 - cos θ)

    Mat4([
        c + ux * ux * t,
        ux * uy * t - uz * s,
        ux * uz * t + uy * s,
        0.0,
        uy * ux * t + uz * s,
        c + uy * uy * t,
        uy * uz * t - ux * s,
        0.0,
        uz * ux * t - uy * s,
        uz * uy * t + ux * s,
        c + uz * uz * t,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ])
}

#[cfg(test)]
mod test {
    use super::*;

    // -----------------------------------------------------------------------
    // Construction & identity
    // -----------------------------------------------------------------------

    #[test]
    fn identity_is_identity() {
        assert!(Mat4::IDENTITY.is_identity());
    }

    #[test]
    fn from_array_round_trips_via_as_array() {
        let arr = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ];
        let m = Mat4::from_array(arr);
        assert_eq!(m.as_array(), &arr);
    }

    #[test]
    fn identity_array_is_canonical() {
        // Sanity: the IDENTITY constant is the expected row-major identity.
        assert_eq!(
            Mat4::IDENTITY.as_array(),
            &[
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0
            ]
        );
    }

    // -----------------------------------------------------------------------
    // is_identity — boundary behaviour (motivation #3: don't swallow real transforms)
    // -----------------------------------------------------------------------

    #[test]
    fn tiny_translation_below_lattice_is_treated_as_identity() {
        // 1e-13 is below half a 1e-12 lattice cell — rounds to 0. Treated as identity.
        // (Acceptable: this is below FP noise floor for engine-built matrices.)
        let m = Mat4::from_array([
            1.0, 0.0, 0.0, 1e-13, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0, //
        ]);
        assert!(m.is_identity());
    }

    #[test]
    fn sub_micron_translation_is_not_swallowed() {
        // A real sub-micron translation (e.g. a user's sliver-avoidance offset) must
        // NOT be treated as "no transform" — motivation #3. 1e-7 is well above the
        // 1e-12 lattice cell.
        let m = Mat4::from_array([
            1.0, 0.0, 0.0, 1e-7, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0, //
        ]);
        assert!(!m.is_identity());
    }

    #[test]
    fn non_identity_linear_part_is_not_identity() {
        // A scale of 2.0 in x — definitely not identity.
        let m = Mat4::from_array([
            2.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0, //
        ]);
        assert!(!m.is_identity());
    }

    // -----------------------------------------------------------------------
    // mat_mul
    // -----------------------------------------------------------------------

    #[test]
    fn mat_mul_with_identity_is_noop() {
        let m = Mat4::from_array([
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ]);
        let product = m.mat_mul(&Mat4::IDENTITY);
        assert_eq!(product.as_array(), m.as_array());

        // And the other order (identity · m == m).
        let product2 = Mat4::IDENTITY.mat_mul(&m);
        assert_eq!(product2.as_array(), m.as_array());
    }

    #[test]
    fn mat_mul_composes_translation_then_translation() {
        // T1 translates by (1,0,0); T2 translates by (0,2,0).
        // T1 · T2 applied to origin: T2 first moves origin to (0,2,0), then T1 moves
        // to (1,2,0). So (T1·T2)·origin == (1,2,0).
        let t1 = Mat4::from_array([
            1.0, 0.0, 0.0, 1.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0, //
        ]);
        let t2 = Mat4::from_array([
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 2.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0, //
        ]);
        let composed = t1.mat_mul(&t2);
        let origin = Point3::new(0.0, 0.0, 0.0);
        let result = composed.apply_pt(origin);
        assert_eq!((result.x, result.y, result.z), (1.0, 2.0, 0.0));
    }

    #[test]
    fn mat_mul_scale_then_rotate_matches_individual_applies() {
        // S scales by (2,2,2); R is 90° about z (x→y, y→-x).
        // Apply S then R to (1,0,0): S→(2,0,0), R→(0,2,0).
        // (R·S)·p should give the same as R applied to (S·p).
        let s = mat_scale(2.0, 2.0, 2.0);
        let r = mat_rot_aa([0.0, 0.0, 1.0], std::f64::consts::FRAC_PI_2);
        let composed = r.mat_mul(&s);

        let p = Point3::new(1.0, 0.0, 0.0);
        let direct = r.apply_pt(s.apply_pt(p));
        let via_composed = composed.apply_pt(p);

        assert!((direct.x - via_composed.x).abs() < 1e-12);
        assert!((direct.y - via_composed.y).abs() < 1e-12);
        assert!((direct.z - via_composed.z).abs() < 1e-12);
    }

    // -----------------------------------------------------------------------
    // apply_pt vs apply_vec (the w=1 vs w=0 distinction)
    // -----------------------------------------------------------------------

    #[test]
    fn apply_pt_applies_translation() {
        let t = Mat4::from_array([
            1.0, 0.0, 0.0, 5.0, //
            0.0, 1.0, 0.0, 6.0, //
            0.0, 0.0, 1.0, 7.0, //
            0.0, 0.0, 0.0, 1.0, //
        ]);
        let p = Point3::new(1.0, 2.0, 3.0);
        let r = t.apply_pt(p);
        assert_eq!((r.x, r.y, r.z), (6.0, 8.0, 10.0));
    }

    #[test]
    fn apply_vec_ignores_translation() {
        // Same translation matrix as above, but applied as a vector: translation must
        // not apply (w=0). Only the linear part (identity here) acts → unchanged.
        let t = Mat4::from_array([
            1.0, 0.0, 0.0, 5.0, //
            0.0, 1.0, 0.0, 6.0, //
            0.0, 0.0, 1.0, 7.0, //
            0.0, 0.0, 0.0, 1.0, //
        ]);
        let v = Point3::new(1.0, 2.0, 3.0);
        let r = t.apply_vec(v);
        assert_eq!((r.x, r.y, r.z), (1.0, 2.0, 3.0));
    }

    #[test]
    fn apply_pt_and_apply_vec_match_for_pure_linear_transform() {
        // With no translation (last column = 0,0,0,1 for w), pt and vec agree.
        let s = mat_scale(2.0, 3.0, 4.0);
        let p = Point3::new(1.0, 1.0, 1.0);
        let as_pt = s.apply_pt(p);
        let as_vec = s.apply_vec(p);
        assert_eq!((as_pt.x, as_pt.y, as_pt.z), (2.0, 3.0, 4.0));
        assert_eq!((as_vec.x, as_vec.y, as_vec.z), (2.0, 3.0, 4.0));
    }

    // -----------------------------------------------------------------------
    // inverse
    // -----------------------------------------------------------------------

    #[test]
    fn inverse_of_identity_is_identity() {
        let inv = Mat4::IDENTITY.inverse();
        assert!(inv.is_identity());
    }

    #[test]
    fn inverse_round_trips_translation() {
        let t = Mat4::from_array([
            1.0, 0.0, 0.0, 5.0, //
            0.0, 1.0, 0.0, 6.0, //
            0.0, 0.0, 1.0, 7.0, //
            0.0, 0.0, 0.0, 1.0, //
        ]);
        let inv = t.inverse();
        // t · inv == identity
        let round = t.mat_mul(&inv);
        assert!(round.is_identity());
        // And applying t then inv returns the original point.
        let p = Point3::new(1.0, 2.0, 3.0);
        let r = inv.apply_pt(t.apply_pt(p));
        assert!((r.x - p.x).abs() < 1e-12);
        assert!((r.y - p.y).abs() < 1e-12);
        assert!((r.z - p.z).abs() < 1e-12);
    }

    #[test]
    fn inverse_round_trips_rotation() {
        let r = mat_rot_aa([0.0, 0.0, 1.0], std::f64::consts::FRAC_PI_4); // 45°
        let inv = r.inverse();
        let round = r.mat_mul(&inv);
        assert!(round.is_identity());
    }

    // -----------------------------------------------------------------------
    // constructors (mat_translation / mat_scale / mat_rot_aa)
    // -----------------------------------------------------------------------

    #[test]
    fn mat_translation_moves_origin() {
        let t = mat_translation(1.0, 2.0, 3.0);
        let r = t.apply_pt(Point3::new(0.0, 0.0, 0.0));
        assert_eq!((r.x, r.y, r.z), (1.0, 2.0, 3.0));
        // And is identity for vectors.
        let v = t.apply_vec(Point3::new(1.0, 1.0, 1.0));
        assert_eq!((v.x, v.y, v.z), (1.0, 1.0, 1.0));
    }

    #[test]
    fn mat_scale_scales_components() {
        let s = mat_scale(2.0, 3.0, 4.0);
        let r = s.apply_pt(Point3::new(1.0, 1.0, 1.0));
        assert_eq!((r.x, r.y, r.z), (2.0, 3.0, 4.0));
    }

    #[test]
    fn mat_rot_aa_90deg_about_z_maps_x_to_y() {
        let r = mat_rot_aa([0.0, 0.0, 1.0], std::f64::consts::FRAC_PI_2);
        let result = r.apply_pt(Point3::new(1.0, 0.0, 0.0));
        assert!((result.x - 0.0).abs() < 1e-12);
        assert!((result.y - 1.0).abs() < 1e-12);
        assert!((result.z - 0.0).abs() < 1e-12);
    }

    #[test]
    fn mat_rot_aa_preserves_length() {
        let r = mat_rot_aa([1.0, 1.0, 1.0], 0.7); // arbitrary axis/angle
        let p = Point3::new(1.0, 2.0, 3.0);
        let rp = r.apply_pt(p);
        let len2_before = p.x * p.x + p.y * p.y + p.z * p.z;
        let len2_after = rp.x * rp.x + rp.y * rp.y + rp.z * rp.z;
        assert!((len2_before - len2_after).abs() < 1e-9);
    }
}
