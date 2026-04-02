//! Defining the rust types for a Constructive Solid Geometry solid modeling language

use std::fmt;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

// ---------------------------------------------------------------------------
// Primitive parameter types
// ---------------------------------------------------------------------------

type Num = f64;

// ---------------------------------------------------------------------------
// Provenance ID generation
// ---------------------------------------------------------------------------

static NEXT_PROV_ID: AtomicU64 = AtomicU64::new(1);

fn next_prov_id() -> u64 {
    NEXT_PROV_ID.fetch_add(1, Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Core node types
// ---------------------------------------------------------------------------

/// An owned shared reference to a CsgNode
pub type NodeRef = Arc<CsgNode>;

/// A CSG Node
pub struct CsgNode {
    /// A hash built from the base and flat_transform (stubbed to 0 until hashing is implemented)
    pub(crate) geom_id: u64,
    /// A unique value to trace provenance through the system
    pub(crate) prov_id: u64,
    /// The bare constructive solid geometry without transforms
    pub(crate) base: CsgBaseNode,
    /// The ordered stack of transforms applied to the base
    pub(crate) transforms: Vec<AffineTransform>,
    /// The combined/flattened matrix rep of the transforms.
    ///
    /// Convention: row-major 4×4, column homogeneous vectors.
    /// Application: p' = M · p  (matrix left-multiplies column vector).
    /// Chaining: right-multiply each new transform — M_new = M_old · T.
    /// The bottom row is always [0, 0, 0, 1]; translations live in column 3.
    pub(crate) flat_transform: [f64; 16],
    /// Optional metadata to attach to the CSG node
    pub(crate) meta: Option<Arc<CsgMetadata>>,
}

/// A 'bare' CSG node without transform or metadata
#[derive(Clone, Debug)]
pub(crate) enum CsgBaseNode {
    Prim(CsgPrimitive),
    Op(CsgOp),
}

/// Solid primitives that make up more complex solids
#[derive(Clone, Debug)]
pub(crate) enum CsgPrimitive {
    Cuboid { dx: Num, dy: Num, dz: Num },
    Cylinder { r: Num, h: Num },
    Sphere { r: Num },
    Cone { r: Num, h: Num },
    // Much later: Extrusion, SolidOfRot
}

/// Operations to combine or select CsgNodes
#[derive(Clone, Debug)]
pub(crate) enum CsgOp {
    Union { children: Vec<NodeRef> },
    Intersection { children: Vec<NodeRef> },
    Difference { base: NodeRef, subtract: Vec<NodeRef> },
    Select { input: NodeRef, policy: SelectPolicy },
}

/// A selection policy for the Select operation
#[derive(Clone, Debug)]
pub(crate) enum SelectPolicy {
    ContainsPoint { point: [Num; 3] },
    ClosestToPoint { point: [Num; 3] },
    LargestByVolume,
}

/// A geometric transformation of a CSG solid
#[derive(Clone, Debug)]
pub(crate) enum AffineTransform {
    Translation { delta: [Num; 3] },
    RotationAA { axis: [Num; 3], angle: Num },
    Scale { sx: Num, sy: Num, sz: Num },
}

/// Optional metadata attached to a node
#[derive(Debug)]
pub(crate) struct CsgMetadata {
    // TODO: color, material id, label, texture info
}

impl fmt::Debug for CsgNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("base", &self.base)
            .field("transforms", &self.transforms)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Identity matrix constant
// ---------------------------------------------------------------------------

const IDENTITY_4X4: [f64; 16] = [
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    0.0, 0.0, 0.0, 1.0,
];

// ---------------------------------------------------------------------------
// Matrix helpers (row-major, right-multiply convention)
// ---------------------------------------------------------------------------

/// Right-multiply: result = lhs · rhs  (4×4 row-major)
fn mat_mul(lhs: &[f64; 16], rhs: &[f64; 16]) -> [f64; 16] {
    let mut out = [0.0f64; 16];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                out[i * 4 + j] += lhs[i * 4 + k] * rhs[k * 4 + j];
            }
        }
    }
    out
}

/// Build the 4×4 matrix for a Translation transform.
/// Row-major, column-vector convention: deltas go in column 3.
#[rustfmt::skip]
fn mat_translation(dx: f64, dy: f64, dz: f64) -> [f64; 16] {
    [
        1.0, 0.0, 0.0, dx,
        0.0, 1.0, 0.0, dy,
        0.0, 0.0, 1.0, dz,
        0.0, 0.0, 0.0, 1.0,
    ]
}

/// Build the 4×4 matrix for a non-uniform Scale transform.
#[rustfmt::skip]
fn mat_scale(sx: f64, sy: f64, sz: f64) -> [f64; 16] {
    [
        sx,  0.0, 0.0, 0.0,
        0.0, sy,  0.0, 0.0,
        0.0, 0.0, sz,  0.0,
        0.0, 0.0, 0.0, 1.0,
    ]
}

/// Build the 4×4 rotation matrix for an arbitrary axis/angle (Rodrigues' formula).
/// `axis` is normalised internally; `angle` is in radians (right-hand rule).
fn mat_rot_aa(axis: [f64; 3], angle: f64) -> [f64; 16] {
    let len = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
    let [ux, uy, uz] = [axis[0] / len, axis[1] / len, axis[2] / len];

    let c = angle.cos();
    let s = angle.sin();
    let t = 1.0 - c; // (1 - cos θ)

    #[rustfmt::skip]
    let m = [
        c + ux*ux*t,       ux*uy*t - uz*s,   ux*uz*t + uy*s,   0.0,
        uy*ux*t + uz*s,    c + uy*uy*t,       uy*uz*t - ux*s,   0.0,
        uz*ux*t - uy*s,    uz*uy*t + ux*s,    c + uz*uz*t,       0.0,
        0.0,               0.0,               0.0,               1.0,
    ];
    m
}

// ---------------------------------------------------------------------------
// Matrix quantization
// ---------------------------------------------------------------------------

/// Scale factor for converting f64 matrix entries to i64 buckets.
/// Noise smaller than `1 / QUANTIZE_SCALE` (i.e. < 1e-6) is absorbed by rounding,
/// while values differing by ≥ 1e-6 produce distinct integers.
const QUANTIZE_SCALE: f64 = 1e6;

/// Quantize a 4×4 f64 matrix to i64 by scaling and rounding each entry.
/// Used to build geometry hashes that are stable under floating-point noise.
fn quantize_matrix(mat: &[f64; 16]) -> [i64; 16] {
    mat.map(|v| (v * QUANTIZE_SCALE).round() as i64)
}

// ---------------------------------------------------------------------------
// CsgNode constructors and transform methods
// ---------------------------------------------------------------------------

impl CsgNode {
    // --- internal builders --------------------------------------------------

    fn new_primitive(prim: CsgPrimitive) -> NodeRef {
        Arc::new(CsgNode {
            geom_id: 0, // stubbed until hashing is implemented
            prov_id: next_prov_id(),
            base: CsgBaseNode::Prim(prim),
            transforms: Vec::new(),
            flat_transform: IDENTITY_4X4,
            meta: None,
        })
    }

    /// Return a new node that is `self` with `t` appended to the transform stack.
    /// `mat` is the 4×4 matrix for `t`; the new flat_transform = self.flat_transform · mat.
    fn with_transform(&self, t: AffineTransform, mat: [f64; 16]) -> NodeRef {
        let mut transforms = self.transforms.clone();
        transforms.push(t);
        Arc::new(CsgNode {
            geom_id: 0,
            prov_id: next_prov_id(),
            base: self.base.clone(),
            transforms,
            flat_transform: mat_mul(&self.flat_transform, &mat),
            meta: self.meta.clone(),
        })
    }

    // --- primitive constructors ---------------------------------------------

    pub fn sphere(r: f64) -> NodeRef {
        Self::new_primitive(CsgPrimitive::Sphere { r })
    }

    pub fn cuboid(dx: f64, dy: f64, dz: f64) -> NodeRef {
        Self::new_primitive(CsgPrimitive::Cuboid { dx, dy, dz })
    }

    pub fn cylinder(r: f64, h: f64) -> NodeRef {
        Self::new_primitive(CsgPrimitive::Cylinder { r, h })
    }

    pub fn cone(r: f64, h: f64) -> NodeRef {
        Self::new_primitive(CsgPrimitive::Cone { r, h })
    }

    // --- transform methods (each returns a new NodeRef) ---------------------

    pub fn translate(&self, dx: f64, dy: f64, dz: f64) -> NodeRef {
        self.with_transform(
            AffineTransform::Translation { delta: [dx, dy, dz] },
            mat_translation(dx, dy, dz),
        )
    }

    pub fn scale(&self, sx: f64, sy: f64, sz: f64) -> NodeRef {
        self.with_transform(
            AffineTransform::Scale { sx, sy, sz },
            mat_scale(sx, sy, sz),
        )
    }

    pub fn rot_x(&self, angle_rad: f64) -> NodeRef {
        self.with_transform(
            AffineTransform::RotationAA { axis: [1.0, 0.0, 0.0], angle: angle_rad },
            mat_rot_aa([1.0, 0.0, 0.0], angle_rad),
        )
    }

    pub fn rot_y(&self, angle_rad: f64) -> NodeRef {
        self.with_transform(
            AffineTransform::RotationAA { axis: [0.0, 1.0, 0.0], angle: angle_rad },
            mat_rot_aa([0.0, 1.0, 0.0], angle_rad),
        )
    }

    pub fn rot_z(&self, angle_rad: f64) -> NodeRef {
        self.with_transform(
            AffineTransform::RotationAA { axis: [0.0, 0.0, 1.0], angle: angle_rad },
            mat_rot_aa([0.0, 0.0, 1.0], angle_rad),
        )
    }

    /// Rotation around an arbitrary axis by `angle_rad` (right-hand rule).
    /// `axis` need not be a unit vector; it will be normalised internally.
    pub fn rot_aa(&self, axis: [f64; 3], angle_rad: f64) -> NodeRef {
        self.with_transform(
            AffineTransform::RotationAA { axis, angle: angle_rad },
            mat_rot_aa(axis, angle_rad),
        )
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use std::f64::consts::PI;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    const EPS: f64 = 1e-10;

    #[rustfmt::skip]
    const IDENTITY: [f64; 16] = [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ];

    fn mat_approx_eq(a: &[f64; 16], b: &[f64; 16]) -> bool {
        a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < EPS)
    }

    // -----------------------------------------------------------------------
    // Group A: Primitive constructors
    // -----------------------------------------------------------------------

    #[test]
    fn sphere_stores_radius() {
        let n = CsgNode::sphere(2.5);
        match &n.base {
            CsgBaseNode::Prim(CsgPrimitive::Sphere { r }) => assert_eq!(*r, 2.5),
            _ => panic!("expected Sphere primitive"),
        }
    }

    #[test]
    fn cuboid_stores_dimensions() {
        let n = CsgNode::cuboid(1.0, 2.0, 3.0);
        match &n.base {
            CsgBaseNode::Prim(CsgPrimitive::Cuboid { dx, dy, dz }) => {
                assert_eq!((*dx, *dy, *dz), (1.0, 2.0, 3.0));
            }
            _ => panic!("expected Cuboid primitive"),
        }
    }

    #[test]
    fn cylinder_stores_r_and_h() {
        let n = CsgNode::cylinder(1.5, 4.0);
        match &n.base {
            CsgBaseNode::Prim(CsgPrimitive::Cylinder { r, h }) => {
                assert_eq!((*r, *h), (1.5, 4.0));
            }
            _ => panic!("expected Cylinder primitive"),
        }
    }

    #[test]
    fn cone_stores_r_and_h() {
        let n = CsgNode::cone(1.0, 3.0);
        match &n.base {
            CsgBaseNode::Prim(CsgPrimitive::Cone { r, h }) => {
                assert_eq!((*r, *h), (1.0, 3.0));
            }
            _ => panic!("expected Cone primitive"),
        }
    }

    #[test]
    fn fresh_node_has_empty_transform_stack() {
        let n = CsgNode::sphere(1.0);
        assert!(n.transforms.is_empty());
    }

    #[test]
    fn fresh_node_has_identity_flat_transform() {
        let n = CsgNode::sphere(1.0);
        assert!(mat_approx_eq(&n.flat_transform, &IDENTITY));
    }

    #[test]
    fn prov_ids_are_unique() {
        let a = CsgNode::sphere(1.0);
        let b = CsgNode::sphere(1.0);
        assert_ne!(a.prov_id, b.prov_id);
    }

    // -----------------------------------------------------------------------
    // Group B: Single transforms
    // -----------------------------------------------------------------------

    #[test]
    fn translate_adds_one_entry_to_stack() {
        let n = CsgNode::sphere(1.0).translate(1.0, 2.0, 3.0);
        assert_eq!(n.transforms.len(), 1);
    }

    #[test]
    fn translate_flat_transform_correct() {
        let n = CsgNode::sphere(1.0).translate(2.0, 3.0, 4.0);
        // Row-major, column-vector convention: dx/dy/dz in last column
        #[rustfmt::skip]
        let expected = [
            1.0, 0.0, 0.0, 2.0,
            0.0, 1.0, 0.0, 3.0,
            0.0, 0.0, 1.0, 4.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        assert!(mat_approx_eq(&n.flat_transform, &expected));
    }

    #[test]
    fn scale_flat_transform_correct() {
        let n = CsgNode::sphere(1.0).scale(2.0, 3.0, 4.0);
        #[rustfmt::skip]
        let expected = [
            2.0, 0.0, 0.0, 0.0,
            0.0, 3.0, 0.0, 0.0,
            0.0, 0.0, 4.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        assert!(mat_approx_eq(&n.flat_transform, &expected));
    }

    #[test]
    fn rot_x_90deg_flat_transform_correct() {
        // Right-hand rule around X: Y→Z for positive angle.
        // [1   0      0    0]
        // [0   cos θ  -sin θ  0]
        // [0   sin θ   cos θ  0]
        // [0   0      0    1]
        let n = CsgNode::sphere(1.0).rot_x(PI / 2.0);
        #[rustfmt::skip]
        let expected = [
            1.0,  0.0,  0.0,  0.0,
            0.0,  0.0, -1.0,  0.0,
            0.0,  1.0,  0.0,  0.0,
            0.0,  0.0,  0.0,  1.0,
        ];
        assert!(mat_approx_eq(&n.flat_transform, &expected));
    }

    #[test]
    fn rot_y_90deg_flat_transform_correct() {
        // Right-hand rule around Y: Z→X for positive angle.
        // [cos θ   0  sin θ  0]
        // [0       1  0      0]
        // [-sin θ  0  cos θ  0]
        // [0       0  0      1]
        let n = CsgNode::sphere(1.0).rot_y(PI / 2.0);
        #[rustfmt::skip]
        let expected = [
             0.0,  0.0,  1.0,  0.0,
             0.0,  1.0,  0.0,  0.0,
            -1.0,  0.0,  0.0,  0.0,
             0.0,  0.0,  0.0,  1.0,
        ];
        assert!(mat_approx_eq(&n.flat_transform, &expected));
    }

    #[test]
    fn rot_z_90deg_flat_transform_correct() {
        // Right-hand rule around Z: X→Y for positive angle.
        // [cos θ  -sin θ  0  0]
        // [sin θ   cos θ  0  0]
        // [0       0      1  0]
        // [0       0      0  1]
        let n = CsgNode::sphere(1.0).rot_z(PI / 2.0);
        #[rustfmt::skip]
        let expected = [
            0.0, -1.0,  0.0,  0.0,
            1.0,  0.0,  0.0,  0.0,
            0.0,  0.0,  1.0,  0.0,
            0.0,  0.0,  0.0,  1.0,
        ];
        assert!(mat_approx_eq(&n.flat_transform, &expected));
    }

    #[test]
    fn rot_aa_z_axis_matches_rot_z() {
        // rot_aa around the Z axis should give the same matrix as rot_z
        let aa = CsgNode::sphere(1.0).rot_aa([0.0, 0.0, 1.0], PI / 2.0);
        let rz = CsgNode::sphere(1.0).rot_z(PI / 2.0);
        assert!(mat_approx_eq(&aa.flat_transform, &rz.flat_transform));
    }

    #[test]
    fn rot_aa_unnormalised_axis_same_as_normalised() {
        // Passing a non-unit axis vector should produce the same result
        let unit = CsgNode::sphere(1.0).rot_aa([0.0, 0.0, 1.0], PI / 3.0);
        let scaled = CsgNode::sphere(1.0).rot_aa([0.0, 0.0, 5.0], PI / 3.0);
        assert!(mat_approx_eq(&unit.flat_transform, &scaled.flat_transform));
    }

    // --- immutability -------------------------------------------------------

    #[test]
    fn transform_returns_new_noderef() {
        let original = CsgNode::sphere(1.0);
        let translated = original.translate(1.0, 0.0, 0.0);
        assert!(!Arc::ptr_eq(&original, &translated));
    }

    #[test]
    fn transform_does_not_mutate_original() {
        let original = CsgNode::sphere(1.0);
        let _translated = original.translate(1.0, 0.0, 0.0);
        assert!(mat_approx_eq(&original.flat_transform, &IDENTITY));
        assert!(original.transforms.is_empty());
    }

    #[test]
    fn transform_preserves_base_geometry() {
        let n = CsgNode::sphere(2.5).translate(1.0, 0.0, 0.0);
        match &n.base {
            CsgBaseNode::Prim(CsgPrimitive::Sphere { r }) => assert_eq!(*r, 2.5),
            _ => panic!("base should be preserved through transform"),
        }
    }

    // -----------------------------------------------------------------------
    // Group C: Chaining
    // -----------------------------------------------------------------------

    #[test]
    fn chain_two_transforms_stack_length_is_two() {
        let n = CsgNode::sphere(1.0).translate(1.0, 0.0, 0.0).rot_x(PI / 2.0);
        assert_eq!(n.transforms.len(), 2);
    }

    #[test]
    fn chain_translate_then_scale_flat_transform_correct() {
        // Right-multiply: M = T · S
        //
        // T = translate(1,2,3), S = scale(2,2,2)
        //
        // T · S:
        //   [1 0 0 1]   [2 0 0 0]   [2 0 0 1]
        //   [0 1 0 2] × [0 2 0 0] = [0 2 0 2]
        //   [0 0 1 3]   [0 0 2 0]   [0 0 2 3]
        //   [0 0 0 1]   [0 0 0 1]   [0 0 0 1]
        //
        // Geometric effect: scale is applied to the point first, then translate.
        let n = CsgNode::sphere(1.0)
            .translate(1.0, 2.0, 3.0)
            .scale(2.0, 2.0, 2.0);
        #[rustfmt::skip]
        let expected = [
            2.0, 0.0, 0.0, 1.0,
            0.0, 2.0, 0.0, 2.0,
            0.0, 0.0, 2.0, 3.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        assert!(mat_approx_eq(&n.flat_transform, &expected));
    }

    // -----------------------------------------------------------------------
    // Group D: Matrix quantization
    // -----------------------------------------------------------------------

    #[test]
    fn quantize_identity_gives_expected_integers() {
        // 1.0 → QUANTIZE_SCALE (as i64), 0.0 → 0
        let q = quantize_matrix(&IDENTITY_4X4);
        let s = QUANTIZE_SCALE as i64;
        #[rustfmt::skip]
        let expected: [i64; 16] = [
            s, 0, 0, 0,
            0, s, 0, 0,
            0, 0, s, 0,
            0, 0, 0, s,
        ];
        assert_eq!(q, expected);
    }

    #[test]
    fn quantize_translation_correct() {
        // Translation (2.5, -1.0, 0.0): dx/dy/dz live in column 3 (indices 3, 7, 11)
        let mat = mat_translation(2.5, -1.0, 0.0);
        let q = quantize_matrix(&mat);
        let s = QUANTIZE_SCALE as i64;
        #[rustfmt::skip]
        let expected: [i64; 16] = [
            s, 0, 0,  2_500_000,
            0, s, 0, -1_000_000,
            0, 0, s,  0,
            0, 0, 0,  s,
        ];
        assert_eq!(q, expected);
    }

    #[test]
    fn quantize_rot90_snaps_near_zero_entries() {
        // rot_z(π/2): cos(π/2) ≈ 6.1e-17 should snap to 0; sin(π/2) = 1.0 → SCALE
        let mat = mat_rot_aa([0.0, 0.0, 1.0], PI / 2.0);
        let q = quantize_matrix(&mat);
        let s = QUANTIZE_SCALE as i64;
        #[rustfmt::skip]
        let expected: [i64; 16] = [
             0, -s, 0, 0,
             s,  0, 0, 0,
             0,  0, s, 0,
             0,  0, 0, s,
        ];
        assert_eq!(q, expected);
    }

    #[test]
    fn quantize_noise_below_precision_is_stable() {
        // Noise smaller than 1/QUANTIZE_SCALE (= 1e-6) must not affect the result
        let clean = mat_translation(1.0, 1.0, 1.0);
        let noisy: [f64; 16] = clean.map(|v| v + 1e-10);
        assert_eq!(quantize_matrix(&clean), quantize_matrix(&noisy));
    }

    #[test]
    fn quantize_distinct_inputs_give_distinct_outputs() {
        let a = mat_translation(1.0, 0.0, 0.0);
        let b = mat_translation(2.0, 0.0, 0.0);
        assert_ne!(quantize_matrix(&a), quantize_matrix(&b));
    }

    #[test]
    fn chain_does_not_mutate_intermediate_nodes() {
        let base = CsgNode::sphere(1.0);
        let step1 = base.translate(1.0, 0.0, 0.0);
        let _step2 = step1.rot_x(PI / 4.0);

        // base: identity, empty stack
        assert!(mat_approx_eq(&base.flat_transform, &IDENTITY));
        assert!(base.transforms.is_empty());

        // step1: only translation, stack length 1
        #[rustfmt::skip]
        let t_expected = [
            1.0, 0.0, 0.0, 1.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        assert!(mat_approx_eq(&step1.flat_transform, &t_expected));
        assert_eq!(step1.transforms.len(), 1);
    }
}
