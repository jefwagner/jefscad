//! Point-in-solid classification predicates using Flint interval arithmetic.
//!
//! The top-level entry point is [`classify_node`], which transforms a world-space
//! query point into a primitive's local frame (via the inverse of `CsgNode::flat_transform`)
//! and evaluates the appropriate implicit function with outward interval rounding.
//!
//! # Result semantics
//! - [`Classification::Inside`]  — the point is provably strictly inside the solid.
//! - [`Classification::Outside`] — the point is provably strictly outside the solid.
//! - [`Classification::Indeterminate`] — the interval straddles the boundary; the caller
//!   must decide (refine, subdivide, or treat as on-surface).

// ---------------------------------------------------------------------------
// Classification
// ---------------------------------------------------------------------------

/// The result of a point-in-solid predicate query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    /// The query point is provably strictly inside the solid.
    Inside,
    /// The query point is provably strictly outside the solid.
    Outside,
    /// The interval arithmetic result straddles the boundary; the exact
    /// classification cannot be determined without refinement.
    Indeterminate,
}

// ---------------------------------------------------------------------------
// Sphere predicate
// ---------------------------------------------------------------------------

/// Classify a point `p` against an axis-aligned sphere of radius `r` centered at
/// the origin, using Flint interval arithmetic.
///
/// - `Inside`        — `||p||² < r²` is provably true.
/// - `Outside`       — `||p||² > r²` is provably true.
/// - `Indeterminate` — the interval for `||p||²` straddles `r²`; exact
///                     classification requires refinement.
pub(crate) fn classify_sphere_local(p: [f64; 3], r: f64) -> Classification {
    use flint::Flint;
    use std::cmp::Ordering;

    let [x, y, z] = p;
    let fx: Flint<f64> = Flint::from(x);
    let fy: Flint<f64> = Flint::from(y);
    let fz: Flint<f64> = Flint::from(z);
    let fr: Flint<f64> = Flint::from(r);

    let dist2 = fx * fx + fy * fy + fz * fz;
    let r2    = fr * fr;

    match dist2.partial_cmp(&r2) {
        Some(Ordering::Less)    => Classification::Inside,
        Some(Ordering::Greater) => Classification::Outside,
        _                       => Classification::Indeterminate,
    }
}

// ---------------------------------------------------------------------------
// Cuboid predicate
// ---------------------------------------------------------------------------

/// Classify a point `p` against the axis-aligned cuboid `[0,dx]×[0,dy]×[0,dz]`
/// using Flint interval arithmetic.
///
/// Each axis is tested independently against its `[0, dim]` range:
/// - The overall result is `Outside` if any axis is provably outside its range.
/// - The overall result is `Inside` if every axis is provably strictly inside.
/// - Otherwise `Indeterminate`.
pub(crate) fn classify_cuboid_local(p: [f64; 3], dx: f64, dy: f64, dz: f64) -> Classification {
    use flint::Flint;
    use std::cmp::Ordering;

    let classify_axis = |val: f64, hi: f64| -> Classification {
        let fval:  Flint<f64> = Flint::from(val);
        let fzero: Flint<f64> = Flint::from(0.0_f64);
        let fhi:   Flint<f64> = Flint::from(hi);

        match (fval.partial_cmp(&fzero), fval.partial_cmp(&fhi)) {
            // Entirely below zero, or entirely above hi → outside this axis range
            (Some(Ordering::Less), _) | (_, Some(Ordering::Greater)) => Classification::Outside,
            // Strictly above zero AND strictly below hi → inside this axis range
            (Some(Ordering::Greater), Some(Ordering::Less)) => Classification::Inside,
            _ => Classification::Indeterminate,
        }
    };

    match (
        classify_axis(p[0], dx),
        classify_axis(p[1], dy),
        classify_axis(p[2], dz),
    ) {
        (Classification::Outside, _, _)
        | (_, Classification::Outside, _)
        | (_, _, Classification::Outside) => Classification::Outside,
        (Classification::Inside, Classification::Inside, Classification::Inside) => {
            Classification::Inside
        }
        _ => Classification::Indeterminate,
    }
}

// ---------------------------------------------------------------------------
// Cylinder predicate
// ---------------------------------------------------------------------------

/// Classify a point `p` against the cylinder with radius `r` and height `h`,
/// whose axis runs along +Z with the base circle at z = 0 (origin).
///
/// Two independent sub-tests are combined:
/// - **Lateral**: `x² + y² vs r²` (same logic as sphere in 2-D)
/// - **Axial**:   `z vs [0, h]`    (same logic as a cuboid single axis)
///
/// The combined result is `Outside` if either sub-test is `Outside`, `Inside`
/// if both are `Inside`, and `Indeterminate` otherwise.
pub(crate) fn classify_cylinder_local(p: [f64; 3], r: f64, h: f64) -> Classification {
    use flint::Flint;
    use std::cmp::Ordering;

    // --- lateral sub-test: x² + y² vs r² ---
    let fx: Flint<f64> = Flint::from(p[0]);
    let fy: Flint<f64> = Flint::from(p[1]);
    let fr: Flint<f64> = Flint::from(r);
    let rho2 = fx * fx + fy * fy;
    let r2   = fr * fr;

    let lateral = match rho2.partial_cmp(&r2) {
        Some(Ordering::Less)    => Classification::Inside,
        Some(Ordering::Greater) => Classification::Outside,
        _                       => Classification::Indeterminate,
    };

    // --- axial sub-test: z vs [0, h] ---
    let fz:    Flint<f64> = Flint::from(p[2]);
    let fzero: Flint<f64> = Flint::from(0.0_f64);
    let fh:    Flint<f64> = Flint::from(h);

    let axial = match (fz.partial_cmp(&fzero), fz.partial_cmp(&fh)) {
        (Some(Ordering::Less), _) | (_, Some(Ordering::Greater)) => Classification::Outside,
        (Some(Ordering::Greater), Some(Ordering::Less))           => Classification::Inside,
        _                                                          => Classification::Indeterminate,
    };

    // --- combine ---
    match (lateral, axial) {
        (Classification::Outside, _) | (_, Classification::Outside) => Classification::Outside,
        (Classification::Inside, Classification::Inside)             => Classification::Inside,
        _                                                             => Classification::Indeterminate,
    }
}

// ---------------------------------------------------------------------------
// Cone predicate
// ---------------------------------------------------------------------------

/// Classify a point `p` against the cone with base radius `r` and height `h`.
/// The base circle lies at z = 0; the apex is at z = h.  At height z the
/// maximum allowable radius is `r·(1 − z/h)`.
///
/// Two independent sub-tests are combined:
/// - **Axial**:   `z vs [0, h]` — same as the cylinder axial check.
/// - **Lateral**: `x² + y² vs (r·(1 − z/h))²` — radius shrinks linearly to zero.
///
/// Combined: `Outside` if either sub-test is `Outside`, `Inside` if both are
/// `Inside`, `Indeterminate` otherwise.
pub(crate) fn classify_cone_local(p: [f64; 3], r: f64, h: f64) -> Classification {
    use flint::Flint;
    use std::cmp::Ordering;

    // --- axial sub-test: z vs [0, h] ---
    let fz:    Flint<f64> = Flint::from(p[2]);
    let fzero: Flint<f64> = Flint::from(0.0_f64);
    let fh:    Flint<f64> = Flint::from(h);

    let axial = match (fz.partial_cmp(&fzero), fz.partial_cmp(&fh)) {
        (Some(Ordering::Less), _) | (_, Some(Ordering::Greater)) => Classification::Outside,
        (Some(Ordering::Greater), Some(Ordering::Less))           => Classification::Inside,
        _                                                          => Classification::Indeterminate,
    };

    if axial == Classification::Outside {
        return Classification::Outside;
    }

    // --- lateral sub-test: x² + y² vs (r·(1 − z/h))² ---
    let fx: Flint<f64> = Flint::from(p[0]);
    let fy: Flint<f64> = Flint::from(p[1]);
    let fr: Flint<f64> = Flint::from(r);
    let fh2: Flint<f64> = fh; // alias for clarity

    let rho2       = fx * fx + fy * fy;
    let scale      = fr * (fh2 - fz) / fh2; // r·(1 − z/h) = r·(h − z)/h
    let max_rho2   = scale * scale;

    let lateral = match rho2.partial_cmp(&max_rho2) {
        Some(Ordering::Less)    => Classification::Inside,
        Some(Ordering::Greater) => Classification::Outside,
        _                       => Classification::Indeterminate,
    };

    match (lateral, axial) {
        (Classification::Outside, _) | (_, Classification::Outside) => Classification::Outside,
        (Classification::Inside, Classification::Inside)             => Classification::Inside,
        _                                                             => Classification::Indeterminate,
    }
}

// ---------------------------------------------------------------------------
// Primitive dispatcher
// ---------------------------------------------------------------------------

/// Classify a point `p` against `prim` in the primitive's local frame (no
/// transform applied).  `Extrude` and `Revolve` are not yet supported.
pub(crate) fn classify_primitive_local(
    p: [f64; 3],
    prim: &crate::csg_lang::CsgPrimitive,
) -> Classification {
    use crate::csg_lang::CsgPrimitive;
    match prim {
        CsgPrimitive::Sphere   { r }        => classify_sphere_local(p, *r),
        CsgPrimitive::Cuboid   { dx, dy, dz } => classify_cuboid_local(p, *dx, *dy, *dz),
        CsgPrimitive::Cylinder { r, h }     => classify_cylinder_local(p, *r, *h),
        CsgPrimitive::Cone     { r, h }     => classify_cone_local(p, *r, *h),
        CsgPrimitive::Extrude  { .. }       => todo!("point-in-extrusion predicate"),
        CsgPrimitive::Revolve  { .. }       => todo!("point-in-revolution predicate"),
    }
}

// ---------------------------------------------------------------------------
// Node classifier
// ---------------------------------------------------------------------------

/// Invert an affine 4×4 matrix (row-major, column-vector convention).
///
/// Exploits the affine structure: bottom row is always [0,0,0,1].
/// Returns the 4×4 inverse as a plain f64 array.
///
/// # Panics
/// Panics if the upper-left 3×3 block is singular (det == 0).
fn mat4_inv_f64(m: [f64; 16]) -> [f64; 16] {
    // Upper-left 3×3 linear part A and translation column t
    let a = [m[0], m[1], m[2], m[4], m[5], m[6], m[8], m[9], m[10]];
    let t = [m[3], m[7], m[11]];

    let det = a[0] * (a[4] * a[8] - a[5] * a[7])
            - a[1] * (a[3] * a[8] - a[5] * a[6])
            + a[2] * (a[3] * a[7] - a[4] * a[6]);
    assert!(det != 0.0, "mat4_inv_f64: singular transform (det == 0)");
    let id = 1.0 / det;

    // Inverse of A (adjugate / det)
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

    // Inverse translation: −A⁻¹ · t
    let it = [
        -(ia[0]*t[0] + ia[1]*t[1] + ia[2]*t[2]),
        -(ia[3]*t[0] + ia[4]*t[1] + ia[5]*t[2]),
        -(ia[6]*t[0] + ia[7]*t[1] + ia[8]*t[2]),
    ];

    [ia[0], ia[1], ia[2], it[0], ia[3], ia[4], ia[5], it[1], ia[6], ia[7], ia[8], it[2], 0.0, 0.0, 0.0, 1.0]
}

/// Classify a world-space point `p_world` against the solid described by `node`.
///
/// The node must have a primitive base; CSG ops are not yet supported.
///
/// # Strategy
/// 1. Compute the inverse of `flat_transform`'s midpoint (plain f64).
/// 2. Wrap it as a `FlintArray` (adding 1 ULP per entry) and apply to the
///    homogeneous query point to obtain `p_local` as a Flint interval vector.
/// 3. Extract the midpoint coordinates of `p_local` and call
///    `classify_primitive_local`.
///
/// # Panics
/// Panics if `node.base` is a CSG op (not yet implemented).
pub fn classify_node(p_world: [f64; 3], node: &crate::csg_lang::CsgNode) -> Classification {
    use crate::csg_lang::CsgBaseNode;
    use flint::FlintArray;

    let prim = match &node.base {
        CsgBaseNode::Prim(p) => p,
        CsgBaseNode::Op(_)   => todo!("classify_node: CSG op nodes not yet supported"),
    };

    // Invert the transform and wrap as Flint (1 ULP per entry)
    let inv_mid = mat4_inv_f64(node.flat_transform.midpoint());
    let inv_mat = FlintArray::from_f64(inv_mid);

    // Query point as homogeneous Flint column vector
    let [x, y, z] = p_world;
    let p_h = FlintArray::from_f64([x, y, z, 1.0]);

    // Transform to local frame; extract xyz midpoint
    let p_local = inv_mat.apply(&p_h);
    let mid = p_local.midpoint();

    classify_primitive_local([mid[0], mid[1], mid[2]], prim)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn variants_are_distinct() {
        assert_ne!(Classification::Inside, Classification::Outside);
        assert_ne!(Classification::Inside, Classification::Indeterminate);
        assert_ne!(Classification::Outside, Classification::Indeterminate);
    }

    #[test]
    fn inside_matches() {
        let c = Classification::Inside;
        assert!(matches!(c, Classification::Inside));
        assert!(!matches!(c, Classification::Outside));
        assert!(!matches!(c, Classification::Indeterminate));
    }

    #[test]
    fn outside_matches() {
        let c = Classification::Outside;
        assert!(!matches!(c, Classification::Inside));
        assert!(matches!(c, Classification::Outside));
        assert!(!matches!(c, Classification::Indeterminate));
    }

    #[test]
    fn indeterminate_matches() {
        let c = Classification::Indeterminate;
        assert!(!matches!(c, Classification::Inside));
        assert!(!matches!(c, Classification::Outside));
        assert!(matches!(c, Classification::Indeterminate));
    }

    #[test]
    fn classification_is_copy() {
        let a = Classification::Inside;
        let b = a; // Copy — both remain valid
        assert_eq!(a, b);
    }

    #[test]
    fn classification_debug_is_readable() {
        assert_eq!(format!("{:?}", Classification::Inside),        "Inside");
        assert_eq!(format!("{:?}", Classification::Outside),       "Outside");
        assert_eq!(format!("{:?}", Classification::Indeterminate), "Indeterminate");
    }

    // -----------------------------------------------------------------------
    // classify_sphere_local
    // -----------------------------------------------------------------------

    #[test]
    fn sphere_local_center_is_inside() {
        // ||origin||² = 0, r² = 1 — provably strictly inside
        assert_eq!(classify_sphere_local([0.0, 0.0, 0.0], 1.0), Classification::Inside);
    }

    #[test]
    fn sphere_local_interior_point_is_inside() {
        // ||(0.5,0,0)||² = 0.25 << r² = 1 — interval entirely below
        assert_eq!(classify_sphere_local([0.5, 0.0, 0.0], 1.0), Classification::Inside);
    }

    #[test]
    fn sphere_local_3d_interior_is_inside() {
        // ||(0.3,0.4,0.5)||² = 0.5 < r² = 1
        assert_eq!(classify_sphere_local([0.3, 0.4, 0.5], 1.0), Classification::Inside);
    }

    #[test]
    fn sphere_local_far_point_is_outside() {
        // ||(10,0,0)||² = 100 >> r² = 1 — interval entirely above
        assert_eq!(classify_sphere_local([10.0, 0.0, 0.0], 1.0), Classification::Outside);
    }

    #[test]
    fn sphere_local_negative_coords_outside() {
        // ||(-2,0,0)||² = 4 > r² = 1
        assert_eq!(classify_sphere_local([-2.0, 0.0, 0.0], 1.0), Classification::Outside);
    }

    #[test]
    fn sphere_local_large_r_wraps_point() {
        // ||(3,4,0)||² = 25 < r² = 100 — inside a radius-10 sphere
        assert_eq!(classify_sphere_local([3.0, 4.0, 0.0], 10.0), Classification::Inside);
    }

    #[test]
    fn sphere_local_exact_boundary_is_indeterminate() {
        // ||(1,0,0)||² = r² = 1 exactly — Flint intervals for dist² and r² overlap
        assert_eq!(classify_sphere_local([1.0, 0.0, 0.0], 1.0), Classification::Indeterminate);
    }

    #[test]
    fn sphere_local_one_ulp_below_boundary_is_indeterminate() {
        // 0.9999999999999999 is one ULP below 1.0; the widened interval for dist²
        // still straddles r² = 1.
        let one_ulp_below = f64::from_bits(1_f64.to_bits() - 1);
        assert_eq!(
            classify_sphere_local([one_ulp_below, 0.0, 0.0], 1.0),
            Classification::Indeterminate,
        );
    }

    #[test]
    fn sphere_local_one_ulp_above_boundary_is_indeterminate() {
        // 1.0000000000000002 is one ULP above 1.0; widened dist² still straddles r².
        let one_ulp_above = f64::from_bits(1_f64.to_bits() + 1);
        assert_eq!(
            classify_sphere_local([one_ulp_above, 0.0, 0.0], 1.0),
            Classification::Indeterminate,
        );
    }

    // -----------------------------------------------------------------------
    // classify_cuboid_local
    // -----------------------------------------------------------------------

    #[test]
    fn cuboid_local_interior_is_inside() {
        // (0.5,0.5,0.5) is strictly inside [0,1]³
        assert_eq!(
            classify_cuboid_local([0.5, 0.5, 0.5], 1.0, 1.0, 1.0),
            Classification::Inside,
        );
    }

    #[test]
    fn cuboid_local_non_unit_interior_is_inside() {
        // (1,2,3) is strictly inside [0,2]×[0,4]×[0,6]
        assert_eq!(
            classify_cuboid_local([1.0, 2.0, 3.0], 2.0, 4.0, 6.0),
            Classification::Inside,
        );
    }

    #[test]
    fn cuboid_local_outside_positive_x_is_outside() {
        assert_eq!(
            classify_cuboid_local([2.0, 0.5, 0.5], 1.0, 1.0, 1.0),
            Classification::Outside,
        );
    }

    #[test]
    fn cuboid_local_outside_negative_x_is_outside() {
        assert_eq!(
            classify_cuboid_local([-1.0, 0.5, 0.5], 1.0, 1.0, 1.0),
            Classification::Outside,
        );
    }

    #[test]
    fn cuboid_local_outside_y_is_outside() {
        assert_eq!(
            classify_cuboid_local([0.5, 5.0, 0.5], 1.0, 1.0, 1.0),
            Classification::Outside,
        );
    }

    #[test]
    fn cuboid_local_outside_z_is_outside() {
        assert_eq!(
            classify_cuboid_local([0.5, 0.5, -0.1], 1.0, 1.0, 1.0),
            Classification::Outside,
        );
    }

    #[test]
    fn cuboid_local_origin_corner_is_indeterminate() {
        // (0,0,0) is on the lower-bound corner of every axis
        assert_eq!(
            classify_cuboid_local([0.0, 0.0, 0.0], 1.0, 1.0, 1.0),
            Classification::Indeterminate,
        );
    }

    #[test]
    fn cuboid_local_far_corner_is_indeterminate() {
        // (1,1,1) is on the upper-bound corner
        assert_eq!(
            classify_cuboid_local([1.0, 1.0, 1.0], 1.0, 1.0, 1.0),
            Classification::Indeterminate,
        );
    }

    #[test]
    fn cuboid_local_face_center_is_indeterminate() {
        // (1, 0.5, 0.5) lies on the dx face
        assert_eq!(
            classify_cuboid_local([1.0, 0.5, 0.5], 1.0, 1.0, 1.0),
            Classification::Indeterminate,
        );
    }

    #[test]
    fn cuboid_local_outside_beats_indeterminate() {
        // x is clearly outside; z is on the boundary — whole point is still Outside
        assert_eq!(
            classify_cuboid_local([2.0, 0.5, 1.0], 1.0, 1.0, 1.0),
            Classification::Outside,
        );
    }

    // -----------------------------------------------------------------------
    // classify_cylinder_local
    // -----------------------------------------------------------------------

    #[test]
    fn cylinder_local_axis_center_is_inside() {
        // On-axis at mid-height: x²+y²=0 << r², z=h/2 strictly inside [0,h]
        assert_eq!(
            classify_cylinder_local([0.0, 0.0, 1.0], 1.0, 2.0),
            Classification::Inside,
        );
    }

    #[test]
    fn cylinder_local_off_axis_interior_is_inside() {
        // (0.3, 0.4, 1.0): ρ²=0.25 < r²=1, z=1 inside [0,2]
        assert_eq!(
            classify_cylinder_local([0.3, 0.4, 1.0], 1.0, 2.0),
            Classification::Inside,
        );
    }

    #[test]
    fn cylinder_local_outside_radially_is_outside() {
        // ρ=3 >> r=1; z is fine — lateral sub-test wins
        assert_eq!(
            classify_cylinder_local([3.0, 0.0, 1.0], 1.0, 2.0),
            Classification::Outside,
        );
    }

    #[test]
    fn cylinder_local_above_cap_is_outside() {
        // z > h: axial sub-test → outside
        assert_eq!(
            classify_cylinder_local([0.0, 0.0, 5.0], 1.0, 2.0),
            Classification::Outside,
        );
    }

    #[test]
    fn cylinder_local_below_base_is_outside() {
        // z < 0: axial sub-test → outside
        assert_eq!(
            classify_cylinder_local([0.0, 0.0, -1.0], 1.0, 2.0),
            Classification::Outside,
        );
    }

    #[test]
    fn cylinder_local_on_side_wall_is_indeterminate() {
        // ρ = r exactly; z is interior → lateral straddles → Indeterminate
        assert_eq!(
            classify_cylinder_local([1.0, 0.0, 1.0], 1.0, 2.0),
            Classification::Indeterminate,
        );
    }

    #[test]
    fn cylinder_local_on_top_cap_is_indeterminate() {
        // z = h; ρ interior → axial straddles → Indeterminate
        assert_eq!(
            classify_cylinder_local([0.0, 0.0, 2.0], 1.0, 2.0),
            Classification::Indeterminate,
        );
    }

    #[test]
    fn cylinder_local_on_base_cap_is_indeterminate() {
        // z = 0; ρ interior → axial straddles → Indeterminate
        assert_eq!(
            classify_cylinder_local([0.0, 0.0, 0.0], 1.0, 2.0),
            Classification::Indeterminate,
        );
    }

    #[test]
    fn cylinder_local_outside_radially_beats_indeterminate_axial() {
        // Radially outside AND on z=0 boundary — Outside wins
        assert_eq!(
            classify_cylinder_local([5.0, 0.0, 0.0], 1.0, 2.0),
            Classification::Outside,
        );
    }

    // -----------------------------------------------------------------------
    // classify_cone_local
    // -----------------------------------------------------------------------

    #[test]
    fn cone_local_axis_mid_height_is_inside() {
        // On-axis at z=h/2: ρ=0 < r·(1-0.5)=0.5; z strictly inside [0,h]
        assert_eq!(
            classify_cone_local([0.0, 0.0, 1.0], 1.0, 2.0),
            Classification::Inside,
        );
    }

    #[test]
    fn cone_local_interior_point_is_inside() {
        // z=1, max_r = r·(1-1/2) = 0.5; ρ=0.1 << 0.5
        assert_eq!(
            classify_cone_local([0.1, 0.0, 1.0], 1.0, 2.0),
            Classification::Inside,
        );
    }

    #[test]
    fn cone_local_below_base_is_outside() {
        // z < 0 → axial sub-test → Outside
        assert_eq!(
            classify_cone_local([0.0, 0.0, -0.5], 1.0, 2.0),
            Classification::Outside,
        );
    }

    #[test]
    fn cone_local_above_apex_is_outside() {
        // z > h → axial sub-test → Outside
        assert_eq!(
            classify_cone_local([0.0, 0.0, 3.0], 1.0, 2.0),
            Classification::Outside,
        );
    }

    #[test]
    fn cone_local_outside_lateral_surface_is_outside() {
        // z=1, max_r=0.5; ρ=0.9 >> 0.5 → lateral Outside
        assert_eq!(
            classify_cone_local([0.9, 0.0, 1.0], 1.0, 2.0),
            Classification::Outside,
        );
    }

    #[test]
    fn cone_local_base_center_is_indeterminate() {
        // z=0 is on the base boundary → axial Indeterminate
        assert_eq!(
            classify_cone_local([0.0, 0.0, 0.0], 1.0, 2.0),
            Classification::Indeterminate,
        );
    }

    #[test]
    fn cone_local_apex_is_indeterminate() {
        // z=h=2: axial straddles h; lateral: max_r = r·(h-h)/h = 0, ρ=0 → also straddles
        assert_eq!(
            classify_cone_local([0.0, 0.0, 2.0], 1.0, 2.0),
            Classification::Indeterminate,
        );
    }

    #[test]
    fn cone_local_on_lateral_surface_is_indeterminate() {
        // z=1, max_r=0.5; ρ=0.5 exactly → lateral straddles
        assert_eq!(
            classify_cone_local([0.5, 0.0, 1.0], 1.0, 2.0),
            Classification::Indeterminate,
        );
    }

    #[test]
    fn cone_local_outside_axial_beats_indeterminate_lateral() {
        // z=-1 (outside below); lateral would be indeterminate at ρ≈max_r → Outside wins
        assert_eq!(
            classify_cone_local([1.0, 0.0, -1.0], 1.0, 2.0),
            Classification::Outside,
        );
    }

    // -----------------------------------------------------------------------
    // classify_primitive_local — dispatch tests (one per variant)
    // -----------------------------------------------------------------------

    #[test]
    fn dispatch_sphere_routes_correctly() {
        use crate::csg_lang::CsgPrimitive;
        let prim = CsgPrimitive::Sphere { r: 2.0 };
        assert_eq!(classify_primitive_local([0.0, 0.0, 0.0], &prim), Classification::Inside);
        assert_eq!(classify_primitive_local([3.0, 0.0, 0.0], &prim), Classification::Outside);
        assert_eq!(classify_primitive_local([2.0, 0.0, 0.0], &prim), Classification::Indeterminate);
    }

    #[test]
    fn dispatch_cuboid_routes_correctly() {
        use crate::csg_lang::CsgPrimitive;
        let prim = CsgPrimitive::Cuboid { dx: 2.0, dy: 2.0, dz: 2.0 };
        assert_eq!(classify_primitive_local([1.0, 1.0, 1.0], &prim), Classification::Inside);
        assert_eq!(classify_primitive_local([3.0, 1.0, 1.0], &prim), Classification::Outside);
        assert_eq!(classify_primitive_local([2.0, 1.0, 1.0], &prim), Classification::Indeterminate);
    }

    #[test]
    fn dispatch_cylinder_routes_correctly() {
        use crate::csg_lang::CsgPrimitive;
        let prim = CsgPrimitive::Cylinder { r: 1.0, h: 4.0 };
        assert_eq!(classify_primitive_local([0.0, 0.0, 2.0], &prim), Classification::Inside);
        assert_eq!(classify_primitive_local([5.0, 0.0, 2.0], &prim), Classification::Outside);
        assert_eq!(classify_primitive_local([1.0, 0.0, 2.0], &prim), Classification::Indeterminate);
    }

    #[test]
    fn dispatch_cone_routes_correctly() {
        use crate::csg_lang::CsgPrimitive;
        let prim = CsgPrimitive::Cone { r: 1.0, h: 2.0 };
        assert_eq!(classify_primitive_local([0.0, 0.0, 1.0], &prim), Classification::Inside);
        assert_eq!(classify_primitive_local([0.9, 0.0, 1.0], &prim), Classification::Outside);
        assert_eq!(classify_primitive_local([0.5, 0.0, 1.0], &prim), Classification::Indeterminate);
    }

    // -----------------------------------------------------------------------
    // classify_node — integration tests
    // -----------------------------------------------------------------------

    #[test]
    fn node_sphere_identity_center_inside() {
        // Untransformed sphere r=1: origin is inside.
        let node = crate::csg_lang::CsgNode::sphere(1.0);
        assert_eq!(classify_node([0.0, 0.0, 0.0], &node), Classification::Inside);
    }

    #[test]
    fn node_sphere_identity_far_outside() {
        // Untransformed sphere r=1: (5,0,0) is outside.
        let node = crate::csg_lang::CsgNode::sphere(1.0);
        assert_eq!(classify_node([5.0, 0.0, 0.0], &node), Classification::Outside);
    }

    #[test]
    fn node_sphere_translated_center_inside() {
        // Sphere r=1 translated to (3,0,0): point (3,0,0) → local origin → Inside.
        let node = crate::csg_lang::CsgNode::sphere(1.0).translate(3.0, 0.0, 0.0);
        assert_eq!(classify_node([3.0, 0.0, 0.0], &node), Classification::Inside);
    }

    #[test]
    fn node_sphere_translated_old_center_outside() {
        // Sphere r=1 translated to (3,0,0): world origin (0,0,0) → local (-3,0,0) → Outside.
        let node = crate::csg_lang::CsgNode::sphere(1.0).translate(3.0, 0.0, 0.0);
        assert_eq!(classify_node([0.0, 0.0, 0.0], &node), Classification::Outside);
    }

    #[test]
    fn node_sphere_scaled_interior_inside() {
        // Sphere r=1 scaled by 3: becomes radius 3 in world space.
        // Point (2,0,0) is inside r=3 sphere.
        let node = crate::csg_lang::CsgNode::sphere(1.0).scale(3.0, 3.0, 3.0);
        assert_eq!(classify_node([2.0, 0.0, 0.0], &node), Classification::Inside);
    }

    #[test]
    fn node_sphere_scaled_exterior_outside() {
        // Sphere r=1 scaled by 3: point (5,0,0) is outside r=3 sphere.
        let node = crate::csg_lang::CsgNode::sphere(1.0).scale(3.0, 3.0, 3.0);
        assert_eq!(classify_node([5.0, 0.0, 0.0], &node), Classification::Outside);
    }

    #[test]
    fn node_cuboid_translated_interior_inside() {
        // Cuboid (dx=2,dy=2,dz=2) translated by (5,0,0): local frame [0..2]³
        // World point (6,1,1) → local (1,1,1) → Inside.
        let node = crate::csg_lang::CsgNode::cuboid(2.0, 2.0, 2.0).translate(5.0, 0.0, 0.0);
        assert_eq!(classify_node([6.0, 1.0, 1.0], &node), Classification::Inside);
    }

    #[test]
    fn node_cuboid_translated_exterior_outside() {
        // World origin (0,0,0) → local (-5,0,0) → x < 0 → Outside.
        let node = crate::csg_lang::CsgNode::cuboid(2.0, 2.0, 2.0).translate(5.0, 0.0, 0.0);
        assert_eq!(classify_node([0.0, 0.0, 0.0], &node), Classification::Outside);
    }
}
