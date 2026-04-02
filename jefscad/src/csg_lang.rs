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
// Matrix quantization and identity check
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

fn is_identity_transform(mat: &[f64; 16]) -> bool {
    quantize_matrix(mat) == quantize_matrix(&IDENTITY_4X4)
}

// ---------------------------------------------------------------------------
// Canonical view and structural hashing
// ---------------------------------------------------------------------------

/// The canonical representation of a node's base, used for geometry hashing.
/// For commutative ops (Union, Intersection) children are flattened and sorted.
/// For Difference, subtract is sorted; the base is ordered.
pub(crate) enum CanonicalBase {
    Prim(CsgPrimitive),
    Union { children: Vec<u64> },
    Intersection { children: Vec<u64> },
    Difference { base: u64, subtract: Vec<u64> },
    Select { input: u64, policy: SelectPolicy },
}

/// A view of a CsgNode in its canonical form, used to compute geom_id.
pub(crate) struct CanonicalCsgNodeView {
    pub(crate) canonical_base: CanonicalBase,
    pub(crate) quantized_transform: [i64; 16],
}

impl CanonicalCsgNodeView {
    pub(crate) fn from_node(node: &CsgNode) -> Self {
        let quantized_transform = quantize_matrix(&node.flat_transform);
        let canonical_base = match &node.base {
            CsgBaseNode::Prim(prim) => CanonicalBase::Prim(prim.clone()),
            CsgBaseNode::Op(CsgOp::Union { children }) => {
                let mut ids = collect_flattened_union(children);
                ids.sort_unstable();
                CanonicalBase::Union { children: ids }
            }
            CsgBaseNode::Op(CsgOp::Intersection { children }) => {
                let mut ids = collect_flattened_intersection(children);
                ids.sort_unstable();
                CanonicalBase::Intersection { children: ids }
            }
            CsgBaseNode::Op(CsgOp::Difference { base, subtract }) => {
                let mut sub_ids: Vec<u64> = subtract.iter().map(|n| n.geom_id).collect();
                sub_ids.sort_unstable();
                CanonicalBase::Difference { base: base.geom_id, subtract: sub_ids }
            }
            CsgBaseNode::Op(CsgOp::Select { input, policy }) => {
                CanonicalBase::Select { input: input.geom_id, policy: policy.clone() }
            }
        };
        CanonicalCsgNodeView { canonical_base, quantized_transform }
    }

    pub(crate) fn geom_id(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;

        let mut h = DefaultHasher::new();
        for &v in &self.quantized_transform {
            h.write_i64(v);
        }
        match &self.canonical_base {
            CanonicalBase::Prim(prim) => {
                h.write_u8(0);
                hash_primitive(&mut h, prim);
            }
            CanonicalBase::Union { children } => {
                h.write_u8(1);
                h.write_usize(children.len());
                for &id in children {
                    h.write_u64(id);
                }
            }
            CanonicalBase::Intersection { children } => {
                h.write_u8(2);
                h.write_usize(children.len());
                for &id in children {
                    h.write_u64(id);
                }
            }
            CanonicalBase::Difference { base, subtract } => {
                h.write_u8(3);
                h.write_u64(*base);
                h.write_usize(subtract.len());
                for &id in subtract {
                    h.write_u64(id);
                }
            }
            CanonicalBase::Select { input, policy } => {
                h.write_u8(4);
                h.write_u64(*input);
                hash_select_policy(&mut h, policy);
            }
        }
        h.finish()
    }
}

/// Collect geom_ids for a Union's children, recursively inlining any child
/// Union that has an identity flat_transform.
fn collect_flattened_union(children: &[NodeRef]) -> Vec<u64> {
    let mut ids = Vec::new();
    for child in children {
        if let CsgBaseNode::Op(CsgOp::Union { children: grandchildren }) = &child.base {
            if is_identity_transform(&child.flat_transform) {
                ids.extend(collect_flattened_union(grandchildren));
                continue;
            }
        }
        ids.push(child.geom_id);
    }
    ids
}

/// Same as collect_flattened_union but for Intersection.
fn collect_flattened_intersection(children: &[NodeRef]) -> Vec<u64> {
    let mut ids = Vec::new();
    for child in children {
        if let CsgBaseNode::Op(CsgOp::Intersection { children: grandchildren }) = &child.base {
            if is_identity_transform(&child.flat_transform) {
                ids.extend(collect_flattened_intersection(grandchildren));
                continue;
            }
        }
        ids.push(child.geom_id);
    }
    ids
}

fn hash_primitive(h: &mut impl std::hash::Hasher, prim: &CsgPrimitive) {
    use std::hash::Hasher;
    match prim {
        CsgPrimitive::Cuboid { dx, dy, dz } => {
            h.write_u8(0);
            h.write_u64(dx.to_bits());
            h.write_u64(dy.to_bits());
            h.write_u64(dz.to_bits());
        }
        CsgPrimitive::Cylinder { r, h: height } => {
            h.write_u8(1);
            h.write_u64(r.to_bits());
            h.write_u64(height.to_bits());
        }
        CsgPrimitive::Sphere { r } => {
            h.write_u8(2);
            h.write_u64(r.to_bits());
        }
        CsgPrimitive::Cone { r, h: height } => {
            h.write_u8(3);
            h.write_u64(r.to_bits());
            h.write_u64(height.to_bits());
        }
    }
}

fn hash_select_policy(h: &mut impl std::hash::Hasher, policy: &SelectPolicy) {
    use std::hash::Hasher;
    match policy {
        SelectPolicy::ContainsPoint { point } => {
            h.write_u8(0);
            for &v in point {
                h.write_u64(v.to_bits());
            }
        }
        SelectPolicy::ClosestToPoint { point } => {
            h.write_u8(1);
            for &v in point {
                h.write_u64(v.to_bits());
            }
        }
        SelectPolicy::LargestByVolume => {
            h.write_u8(2);
        }
    }
}

// ---------------------------------------------------------------------------
// CsgNode constructors and transform methods
// ---------------------------------------------------------------------------

impl CsgNode {
    // --- internal builders --------------------------------------------------

    fn new_primitive(prim: CsgPrimitive) -> NodeRef {
        let mut node = CsgNode {
            geom_id: 0,
            prov_id: next_prov_id(),
            base: CsgBaseNode::Prim(prim),
            transforms: Vec::new(),
            flat_transform: IDENTITY_4X4,
            meta: None,
        };
        node.geom_id = CanonicalCsgNodeView::from_node(&node).geom_id();
        Arc::new(node)
    }

    /// Return a new node that is `self` with `t` appended to the transform stack.
    /// `mat` is the 4×4 matrix for `t`; the new flat_transform = self.flat_transform · mat.
    fn with_transform(&self, t: AffineTransform, mat: [f64; 16]) -> NodeRef {
        let mut transforms = self.transforms.clone();
        transforms.push(t);
        let mut node = CsgNode {
            geom_id: 0,
            prov_id: next_prov_id(),
            base: self.base.clone(),
            transforms,
            flat_transform: mat_mul(&self.flat_transform, &mat),
            meta: self.meta.clone(),
        };
        node.geom_id = CanonicalCsgNodeView::from_node(&node).geom_id();
        Arc::new(node)
    }

    // --- internal op builder ------------------------------------------------

    fn new_op(op: CsgOp) -> NodeRef {
        let mut node = CsgNode {
            geom_id: 0,
            prov_id: next_prov_id(),
            base: CsgBaseNode::Op(op),
            transforms: Vec::new(),
            flat_transform: IDENTITY_4X4,
            meta: None,
        };
        node.geom_id = CanonicalCsgNodeView::from_node(&node).geom_id();
        Arc::new(node)
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

    // --- operator constructors ----------------------------------------------

    pub fn union(children: Vec<NodeRef>) -> NodeRef {
        assert!(!children.is_empty(), "union requires at least one child");
        Self::new_op(CsgOp::Union { children })
    }

    pub fn intersection(children: Vec<NodeRef>) -> NodeRef {
        assert!(!children.is_empty(), "intersection requires at least one child");
        Self::new_op(CsgOp::Intersection { children })
    }

    pub fn difference(base: NodeRef, subtract: Vec<NodeRef>) -> NodeRef {
        assert!(!subtract.is_empty(), "difference requires at least one node to subtract");
        Self::new_op(CsgOp::Difference { base, subtract })
    }

    pub fn select(input: NodeRef, policy: SelectPolicy) -> NodeRef {
        Self::new_op(CsgOp::Select { input, policy })
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

    // -----------------------------------------------------------------------
    // Group E: Operator constructors
    // -----------------------------------------------------------------------

    #[test]
    fn union_base_is_union_op() {
        let u = CsgNode::union(vec![CsgNode::sphere(1.0), CsgNode::cuboid(1.0, 1.0, 1.0)]);
        assert!(matches!(&u.base, CsgBaseNode::Op(CsgOp::Union { .. })));
    }

    #[test]
    fn union_stores_correct_child_count() {
        let u = CsgNode::union(vec![
            CsgNode::sphere(1.0),
            CsgNode::cuboid(1.0, 1.0, 1.0),
            CsgNode::cylinder(1.0, 2.0),
        ]);
        match &u.base {
            CsgBaseNode::Op(CsgOp::Union { children }) => assert_eq!(children.len(), 3),
            _ => panic!("expected Union"),
        }
    }

    #[test]
    fn union_preserves_child_arc_identity() {
        let a = CsgNode::sphere(1.0);
        let b = CsgNode::cuboid(1.0, 1.0, 1.0);
        let u = CsgNode::union(vec![Arc::clone(&a), Arc::clone(&b)]);
        match &u.base {
            CsgBaseNode::Op(CsgOp::Union { children }) => {
                assert!(Arc::ptr_eq(&children[0], &a));
                assert!(Arc::ptr_eq(&children[1], &b));
            }
            _ => panic!("expected Union"),
        }
    }

    #[test]
    fn union_has_identity_flat_transform() {
        let u = CsgNode::union(vec![CsgNode::sphere(1.0), CsgNode::sphere(2.0)]);
        assert!(mat_approx_eq(&u.flat_transform, &IDENTITY));
    }

    #[test]
    fn union_has_empty_transform_stack() {
        let u = CsgNode::union(vec![CsgNode::sphere(1.0), CsgNode::sphere(2.0)]);
        assert!(u.transforms.is_empty());
    }

    #[test]
    fn union_prov_id_differs_from_children() {
        let a = CsgNode::sphere(1.0);
        let b = CsgNode::sphere(2.0);
        let u = CsgNode::union(vec![Arc::clone(&a), Arc::clone(&b)]);
        assert_ne!(u.prov_id, a.prov_id);
        assert_ne!(u.prov_id, b.prov_id);
    }

    #[test]
    #[should_panic]
    fn union_panics_on_empty_children() {
        CsgNode::union(vec![]);
    }

    #[test]
    fn intersection_base_is_intersection_op() {
        let i = CsgNode::intersection(vec![
            CsgNode::sphere(1.0),
            CsgNode::cuboid(2.0, 2.0, 2.0),
        ]);
        assert!(matches!(&i.base, CsgBaseNode::Op(CsgOp::Intersection { .. })));
    }

    #[test]
    fn intersection_preserves_child_arc_identity() {
        let a = CsgNode::sphere(1.0);
        let b = CsgNode::cuboid(2.0, 2.0, 2.0);
        let i = CsgNode::intersection(vec![Arc::clone(&a), Arc::clone(&b)]);
        match &i.base {
            CsgBaseNode::Op(CsgOp::Intersection { children }) => {
                assert_eq!(children.len(), 2);
                assert!(Arc::ptr_eq(&children[0], &a));
                assert!(Arc::ptr_eq(&children[1], &b));
            }
            _ => panic!("expected Intersection"),
        }
    }

    #[test]
    #[should_panic]
    fn intersection_panics_on_empty_children() {
        CsgNode::intersection(vec![]);
    }

    #[test]
    fn difference_base_is_difference_op() {
        let d = CsgNode::difference(CsgNode::cuboid(2.0, 2.0, 2.0), vec![CsgNode::sphere(0.5)]);
        assert!(matches!(&d.base, CsgBaseNode::Op(CsgOp::Difference { .. })));
    }

    #[test]
    fn difference_preserves_base_and_subtract_arc_identity() {
        let base = CsgNode::cuboid(2.0, 2.0, 2.0);
        let hole = CsgNode::sphere(0.5);
        let d = CsgNode::difference(Arc::clone(&base), vec![Arc::clone(&hole)]);
        match &d.base {
            CsgBaseNode::Op(CsgOp::Difference { base: b, subtract }) => {
                assert!(Arc::ptr_eq(b, &base));
                assert_eq!(subtract.len(), 1);
                assert!(Arc::ptr_eq(&subtract[0], &hole));
            }
            _ => panic!("expected Difference"),
        }
    }

    #[test]
    fn difference_has_identity_flat_transform() {
        let d = CsgNode::difference(CsgNode::cuboid(2.0, 2.0, 2.0), vec![CsgNode::sphere(0.5)]);
        assert!(mat_approx_eq(&d.flat_transform, &IDENTITY));
    }

    #[test]
    #[should_panic]
    fn difference_panics_on_empty_subtract() {
        CsgNode::difference(CsgNode::sphere(1.0), vec![]);
    }

    #[test]
    fn select_base_is_select_op() {
        let s = CsgNode::select(CsgNode::sphere(1.0), SelectPolicy::LargestByVolume);
        assert!(matches!(&s.base, CsgBaseNode::Op(CsgOp::Select { .. })));
    }

    #[test]
    fn select_preserves_input_arc_identity() {
        let input = CsgNode::sphere(1.0);
        let s = CsgNode::select(Arc::clone(&input), SelectPolicy::LargestByVolume);
        match &s.base {
            CsgBaseNode::Op(CsgOp::Select { input: i, policy }) => {
                assert!(Arc::ptr_eq(i, &input));
                assert!(matches!(policy, SelectPolicy::LargestByVolume));
            }
            _ => panic!("expected Select"),
        }
    }

    #[test]
    fn select_contains_point_stores_point() {
        let pt = [1.0, 2.0, 3.0];
        let s = CsgNode::select(CsgNode::sphere(1.0), SelectPolicy::ContainsPoint { point: pt });
        match &s.base {
            CsgBaseNode::Op(CsgOp::Select {
                policy: SelectPolicy::ContainsPoint { point },
                ..
            }) => assert_eq!(*point, pt),
            _ => panic!("expected Select with ContainsPoint"),
        }
    }

    // -----------------------------------------------------------------------
    // Group F: Canonical view and structural hashing
    // -----------------------------------------------------------------------

    #[test]
    fn canonical_view_prim_stores_primitive() {
        let n = CsgNode::sphere(2.5);
        let cv = CanonicalCsgNodeView::from_node(&n);
        assert!(matches!(cv.canonical_base, CanonicalBase::Prim(CsgPrimitive::Sphere { r }) if r == 2.5));
    }

    #[test]
    fn canonical_view_fresh_node_has_identity_quantized_transform() {
        let n = CsgNode::sphere(1.0);
        let cv = CanonicalCsgNodeView::from_node(&n);
        let s = QUANTIZE_SCALE as i64;
        #[rustfmt::skip]
        let expected: [i64; 16] = [
            s, 0, 0, 0,
            0, s, 0, 0,
            0, 0, s, 0,
            0, 0, 0, s,
        ];
        assert_eq!(cv.quantized_transform, expected);
    }

    #[test]
    fn geom_id_same_primitive_params_same_id() {
        let a = CsgNode::sphere(1.0);
        let b = CsgNode::sphere(1.0);
        assert_eq!(a.geom_id, b.geom_id);
    }

    #[test]
    fn geom_id_different_primitive_params_different_id() {
        let a = CsgNode::sphere(1.0);
        let b = CsgNode::sphere(2.0);
        assert_ne!(a.geom_id, b.geom_id);
    }

    #[test]
    fn geom_id_different_primitive_types_different_id() {
        // sphere(1.0) vs cuboid(1.0,1.0,1.0): same numeric values but different type
        let s = CsgNode::sphere(1.0);
        let c = CsgNode::cuboid(1.0, 1.0, 1.0);
        assert_ne!(s.geom_id, c.geom_id);
    }

    #[test]
    fn geom_id_same_transform_gives_same_id() {
        let a = CsgNode::sphere(1.0).translate(3.0, 0.0, 0.0);
        let b = CsgNode::sphere(1.0).translate(3.0, 0.0, 0.0);
        assert_eq!(a.geom_id, b.geom_id);
    }

    #[test]
    fn geom_id_different_transform_gives_different_id() {
        let a = CsgNode::sphere(1.0).translate(1.0, 0.0, 0.0);
        let b = CsgNode::sphere(1.0).translate(2.0, 0.0, 0.0);
        assert_ne!(a.geom_id, b.geom_id);
    }

    #[test]
    fn geom_id_union_is_order_independent() {
        let a = CsgNode::sphere(1.0);
        let b = CsgNode::cuboid(2.0, 2.0, 2.0);
        let u1 = CsgNode::union(vec![Arc::clone(&a), Arc::clone(&b)]);
        let u2 = CsgNode::union(vec![Arc::clone(&b), Arc::clone(&a)]);
        assert_eq!(u1.geom_id, u2.geom_id);
    }

    #[test]
    fn geom_id_intersection_is_order_independent() {
        let a = CsgNode::sphere(1.0);
        let b = CsgNode::cuboid(2.0, 2.0, 2.0);
        let i1 = CsgNode::intersection(vec![Arc::clone(&a), Arc::clone(&b)]);
        let i2 = CsgNode::intersection(vec![Arc::clone(&b), Arc::clone(&a)]);
        assert_eq!(i1.geom_id, i2.geom_id);
    }

    #[test]
    fn geom_id_union_vs_intersection_different() {
        let a = CsgNode::sphere(1.0);
        let b = CsgNode::cuboid(2.0, 2.0, 2.0);
        let u = CsgNode::union(vec![Arc::clone(&a), Arc::clone(&b)]);
        let i = CsgNode::intersection(vec![Arc::clone(&a), Arc::clone(&b)]);
        assert_ne!(u.geom_id, i.geom_id);
    }

    #[test]
    fn geom_id_difference_base_order_matters() {
        let a = CsgNode::sphere(1.0);
        let b = CsgNode::cuboid(2.0, 2.0, 2.0);
        let d1 = CsgNode::difference(Arc::clone(&a), vec![Arc::clone(&b)]);
        let d2 = CsgNode::difference(Arc::clone(&b), vec![Arc::clone(&a)]);
        assert_ne!(d1.geom_id, d2.geom_id);
    }

    #[test]
    fn geom_id_difference_subtract_order_independent() {
        let base = CsgNode::cuboid(4.0, 4.0, 4.0);
        let b = CsgNode::sphere(1.0);
        let c = CsgNode::cylinder(0.5, 2.0);
        let d1 = CsgNode::difference(Arc::clone(&base), vec![Arc::clone(&b), Arc::clone(&c)]);
        let d2 = CsgNode::difference(Arc::clone(&base), vec![Arc::clone(&c), Arc::clone(&b)]);
        assert_eq!(d1.geom_id, d2.geom_id);
    }

    #[test]
    fn geom_id_union_flattens_nested_union_without_transform() {
        let a = CsgNode::sphere(1.0);
        let b = CsgNode::cuboid(1.0, 1.0, 1.0);
        let c = CsgNode::cylinder(1.0, 2.0);
        let inner = CsgNode::union(vec![Arc::clone(&a), Arc::clone(&b)]);
        let nested = CsgNode::union(vec![inner, Arc::clone(&c)]);
        let flat = CsgNode::union(vec![Arc::clone(&a), Arc::clone(&b), Arc::clone(&c)]);
        assert_eq!(nested.geom_id, flat.geom_id);
    }

    #[test]
    fn geom_id_union_does_not_flatten_when_inner_has_transform() {
        let a = CsgNode::sphere(1.0);
        let b = CsgNode::cuboid(1.0, 1.0, 1.0);
        let c = CsgNode::cylinder(1.0, 2.0);
        let inner = CsgNode::union(vec![Arc::clone(&a), Arc::clone(&b)]).translate(1.0, 0.0, 0.0);
        let nested = CsgNode::union(vec![inner, Arc::clone(&c)]);
        let flat = CsgNode::union(vec![Arc::clone(&a), Arc::clone(&b), Arc::clone(&c)]);
        assert_ne!(nested.geom_id, flat.geom_id);
    }

    #[test]
    fn geom_id_intersection_flattens_nested_intersection_without_transform() {
        let a = CsgNode::sphere(1.0);
        let b = CsgNode::cuboid(2.0, 2.0, 2.0);
        let c = CsgNode::cylinder(0.5, 3.0);
        let inner = CsgNode::intersection(vec![Arc::clone(&a), Arc::clone(&b)]);
        let nested = CsgNode::intersection(vec![inner, Arc::clone(&c)]);
        let flat = CsgNode::intersection(vec![Arc::clone(&a), Arc::clone(&b), Arc::clone(&c)]);
        assert_eq!(nested.geom_id, flat.geom_id);
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
