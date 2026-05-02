use std::collections::HashMap;

use crate::brep_kernel::{Curve2Id, Curve3Id, FaceId, SolidModelingContext, VertexId};
use crate::geom::{Curve3Kind, Plane, Point3, SurfaceKind};

#[derive(Debug, Clone)]
pub struct FaceFaceIntersection {
    pub v_start:  VertexId,
    pub v_end:    VertexId,
    pub curve3:   Curve3Id,
    pub pcurve_a: Curve2Id,
    pub pcurve_b: Curve2Id,
}

pub struct SsiTable(HashMap<(FaceId, FaceId), Option<FaceFaceIntersection>>);

fn canonical(fa: FaceId, fb: FaceId) -> (FaceId, FaceId) {
    if fa.0 <= fb.0 { (fa, fb) } else { (fb, fa) }
}

impl SsiTable {
    pub fn new() -> Self {
        SsiTable(HashMap::new())
    }

    pub fn insert(&mut self, fa: FaceId, fb: FaceId, result: Option<FaceFaceIntersection>) {
        self.0.insert(canonical(fa, fb), result);
    }

    pub fn get(&self, fa: FaceId, fb: FaceId) -> Option<&Option<FaceFaceIntersection>> {
        self.0.get(&canonical(fa, fb))
    }
}

// ── SSI dispatcher ───────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum SurfTag { Plane, Cylinder, Cone, Sphere, Extrusion, Revolution, Nurbs }

fn surf_tag(kind: &SurfaceKind) -> SurfTag {
    match kind {
        SurfaceKind::Plane(_)      => SurfTag::Plane,
        SurfaceKind::Cylinder(_)   => SurfTag::Cylinder,
        SurfaceKind::Cone(_)       => SurfTag::Cone,
        SurfaceKind::Sphere(_)     => SurfTag::Sphere,
        SurfaceKind::Extrusion(_)  => SurfTag::Extrusion,
        SurfaceKind::Revolution(_) => SurfTag::Revolution,
        SurfaceKind::Nurbs(_)      => SurfTag::Nurbs,
    }
}

pub fn intersect_faces(
    ctx: &mut SolidModelingContext,
    face_a: FaceId,
    face_b: FaceId,
) -> Option<FaceFaceIntersection> {
    let sid_a = ctx.get_face(face_a).surface;
    let sid_b = ctx.get_face(face_b).surface;
    let tag_a = surf_tag(ctx.get_surface(sid_a));
    let tag_b = surf_tag(ctx.get_surface(sid_b));
    match (tag_a, tag_b) {
        (SurfTag::Plane, SurfTag::Plane) => intersect_plane_plane(ctx, face_a, face_b),
        _ => None,
    }
}

// ── Plane geometry helpers ────────────────────────────────────────────────────

/// Unwraps the `Plane` from a planar face. Only call after a `SurfTag::Plane` check.
fn plane_of_face<'a>(ctx: &'a SolidModelingContext, face_id: FaceId) -> &'a Plane {
    let surf_id = ctx.get_face(face_id).surface;
    match ctx.get_surface(surf_id) {
        SurfaceKind::Plane(p) => p,
        _ => panic!("plane_of_face called on non-planar face"),
    }
}

/// Intersection line of two planes given in equation form `n · p = d`.
///
/// Returns `(origin, unit_dir)` where `origin` is the minimum-norm point on the
/// line (perpendicular to `dir`) and `dir = (n_a × n_b).normalize()`.
/// Returns `None` if the planes are parallel (`|n_a × n_b| < 1e-10`).
fn plane_plane_line(
    n_a: Point3, d_a: f64,
    n_b: Point3, d_b: f64,
) -> Option<(Point3, Point3)> {
    let cross = n_a.cross(n_b);
    let len   = cross.length();
    if len < 1e-10 {
        return None;
    }
    let dir = cross * (1.0 / len);

    // Minimum-norm origin: write P = λn_a + μn_b, substitute into both plane
    // equations.  With unit normals the Gram matrix is [[1, c],[c, 1]] where
    // c = n_a·n_b, and its determinant is 1 - c² = sin²θ = len².
    let c    = n_a.dot(n_b);
    let sin2 = len * len;
    let lam  = (d_a - c * d_b) / sin2;
    let mu   = (d_b - c * d_a) / sin2;
    let origin = n_a * lam + n_b * mu;

    Some((origin, dir))
}

fn intersect_plane_plane(
    _ctx: &mut SolidModelingContext,
    _face_a: FaceId,
    _face_b: FaceId,
) -> Option<FaceFaceIntersection> {
    None // stub — implemented in next step
}

// ── AABB ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl Aabb {
    pub fn new(min: [f64; 3], max: [f64; 3]) -> Self {
        Self { min, max }
    }

    /// Conservative fallback: a box that overlaps everything.
    pub fn unbounded() -> Self {
        Self { min: [f64::NEG_INFINITY; 3], max: [f64::INFINITY; 3] }
    }

    fn empty() -> Self {
        Self { min: [f64::INFINITY; 3], max: [f64::NEG_INFINITY; 3] }
    }

    fn expand(&mut self, p: [f64; 3]) {
        for i in 0..3 {
            if p[i] < self.min[i] { self.min[i] = p[i]; }
            if p[i] > self.max[i] { self.max[i] = p[i]; }
        }
    }
}

/// Returns `true` if the two boxes share any point (touching boundaries count).
pub fn aabb_overlap(a: &Aabb, b: &Aabb) -> bool {
    (0..3).all(|i| a.min[i] <= b.max[i] && b.min[i] <= a.max[i])
}

pub fn face_aabb(ctx: &SolidModelingContext, face_id: FaceId) -> Aabb {
    let sid = ctx.get_face(face_id).surface;
    match surf_tag(ctx.get_surface(sid)) {
        SurfTag::Plane => plane_face_aabb(ctx, face_id),
        _ => Aabb::unbounded(),
    }
}

fn plane_face_aabb(ctx: &SolidModelingContext, face_id: FaceId) -> Aabb {
    use std::f64::consts::{PI, TAU};

    let outer_id = ctx.get_face(face_id).outer;
    let coedge_ids = ctx.get_loop(outer_id).coedges.clone();
    let mut aabb = Aabb::empty();

    for ceid in coedge_ids {
        let edge_id = ctx.get_coedge(ceid).edge;
        let (t0, t1, c3id) = {
            let e = ctx.get_edge(edge_id);
            (e.t0, e.t1, e.curve3)
        };

        match ctx.get_curve3(c3id) {
            Curve3Kind::Line3(l) => {
                let p0 = l.p0 + (l.p1 - l.p0) * t0;
                let p1 = l.p0 + (l.p1 - l.p0) * t1;
                aabb.expand([p0.x, p0.y, p0.z]);
                aabb.expand([p1.x, p1.y, p1.z]);
            }
            Curve3Kind::CircularArc3(arc) => {
                let arc = *arc;
                let e2 = arc.normal.cross(arc.ref_dir);
                let u = [arc.ref_dir.x, arc.ref_dir.y, arc.ref_dir.z];
                let v = [e2.x, e2.y, e2.z];

                // Arc endpoints
                let eval = |t: f64| {
                    let p = arc.center + (arc.ref_dir * t.cos() + e2 * t.sin()) * arc.radius;
                    [p.x, p.y, p.z]
                };
                aabb.expand(eval(t0));
                aabb.expand(eval(t1));

                // Per-axis extrema: d/dt x_i = 0 at t = atan2(v_i, u_i) and t + π
                let span = t1 - t0;
                for i in 0..3 {
                    let t_peak = f64::atan2(v[i], u[i]);
                    for &t_cand in &[t_peak, t_peak + PI] {
                        let t_rel = (t_cand - t0).rem_euclid(TAU);
                        if t_rel <= span {
                            aabb.expand(eval(t0 + t_rel));
                        }
                    }
                }
            }
            _ => return Aabb::unbounded(),
        }
    }
    aabb
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::brep_kernel::SolidModelingContext;
    use crate::brep_compiler::{build_cuboid, build_cylinder};

    // ── SsiTable ──────────────────────────────────────────────────────────────

    #[test]
    fn ssi_table_canonical_key_symmetry() {
        let fa = FaceId(0);
        let fb = FaceId(1);
        let ffi = FaceFaceIntersection {
            v_start:  VertexId(0),
            v_end:    VertexId(1),
            curve3:   Curve3Id(0),
            pcurve_a: Curve2Id(0),
            pcurve_b: Curve2Id(1),
        };
        let mut table = SsiTable::new();
        table.insert(fa, fb, Some(ffi));
        assert!(matches!(table.get(fb, fa), Some(Some(_))));
    }

    #[test]
    fn ssi_table_three_state_distinguishable() {
        let fa = FaceId(0);
        let fb = FaceId(1);
        let fc = FaceId(2);
        let mut table = SsiTable::new();
        table.insert(fa, fb, None);
        assert!(matches!(table.get(fa, fb), Some(None))); // tested, no intersection
        assert!(matches!(table.get(fa, fc), None));        // not yet computed
    }

    // ── plane geometry helpers ────────────────────────────────────────────────

    #[test]
    fn plane_plane_line_parallel_is_none() {
        let n = Point3::new(0.0, 0.0, 1.0);
        assert!(plane_plane_line(n, 0.0, n, 1.0).is_none());
    }

    #[test]
    fn plane_plane_line_antiparallel_is_none() {
        let n_a = Point3::new(0.0, 0.0,  1.0);
        let n_b = Point3::new(0.0, 0.0, -1.0);
        assert!(plane_plane_line(n_a, 0.0, n_b, 0.0).is_none());
    }

    #[test]
    fn plane_plane_line_perpendicular_at_origin() {
        let n_a = Point3::new(0.0, 0.0, 1.0);
        let n_b = Point3::new(0.0, 1.0, 0.0);
        let (origin, dir) = plane_plane_line(n_a, 0.0, n_b, 0.0).unwrap();
        let eps = 1e-10;
        assert!(origin.length() < eps, "origin should be (0,0,0), got {:?}", origin);
        assert!((dir.length() - 1.0).abs() < eps, "dir should be unit, len={}", dir.length());
    }

    #[test]
    fn plane_plane_line_offset_perpendicular() {
        // x=1 plane crossed with y=2 plane: line along z, origin at (1,2,0)
        let n_a = Point3::new(1.0, 0.0, 0.0);
        let n_b = Point3::new(0.0, 1.0, 0.0);
        let (origin, dir) = plane_plane_line(n_a, 1.0, n_b, 2.0).unwrap();
        let eps = 1e-10;
        assert!((n_a.dot(origin) - 1.0).abs() < eps, "origin not on plane A: {}", n_a.dot(origin));
        assert!((n_b.dot(origin) - 2.0).abs() < eps, "origin not on plane B: {}", n_b.dot(origin));
        assert!((dir.length() - 1.0).abs() < eps, "dir not unit: {}", dir.length());
    }

    #[test]
    fn plane_of_face_returns_valid_plane() {
        let mut ctx = SolidModelingContext::new();
        let sid = build_cuboid(&mut ctx, 1.0, 1.0, 1.0, 0, 0);
        let shell_id = ctx.get_solid(sid).outer;
        let faces = ctx.get_shell(shell_id).faces.clone();
        for &fid in &faces {
            let p = plane_of_face(&ctx, fid);
            let normal = p.u_dir.cross(p.v_dir);
            let eps = 1e-10;
            // Normal is unit
            assert!((normal.length() - 1.0).abs() < eps, "normal not unit: {}", normal.length());
            // u_dir and v_dir are perpendicular
            assert!(p.u_dir.dot(p.v_dir).abs() < eps, "u_dir · v_dir = {}", p.u_dir.dot(p.v_dir));
            // Plane offset d = n · p0 lies on the cuboid surface, so within [0, 1]
            let d = normal.dot(p.p0);
            assert!(d >= -eps && d <= 1.0 + eps, "face offset d={} outside [0,1]", d);
        }
    }

    // ── intersect_faces dispatcher ────────────────────────────────────────────

    #[test]
    fn intersect_faces_non_planar_pair_is_none() {
        let mut ctx = SolidModelingContext::new();
        let sid = build_cylinder(&mut ctx, 1.0, 2.0, 0, 0);
        let shell_id = ctx.get_solid(sid).outer;
        let faces = ctx.get_shell(shell_id).faces.clone();
        let cyl_face = *faces.iter().find(|&&fid| {
            let surf_id = ctx.get_face(fid).surface;
            matches!(ctx.get_surface(surf_id), SurfaceKind::Cylinder(_))
        }).unwrap();
        let plane_face = *faces.iter().find(|&&fid| {
            let surf_id = ctx.get_face(fid).surface;
            matches!(ctx.get_surface(surf_id), SurfaceKind::Plane(_))
        }).unwrap();
        assert!(intersect_faces(&mut ctx, cyl_face, plane_face).is_none());
    }

    // ── AABB ──────────────────────────────────────────────────────────────────

    #[test]
    fn aabb_overlap_interior() {
        let a = Aabb::new([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
        let b = Aabb::new([1.0, 1.0, 1.0], [3.0, 3.0, 3.0]);
        assert!(aabb_overlap(&a, &b));
    }

    #[test]
    fn aabb_overlap_separated() {
        let a = Aabb::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let b = Aabb::new([2.0, 0.0, 0.0], [3.0, 1.0, 1.0]);
        assert!(!aabb_overlap(&a, &b));
    }

    #[test]
    fn aabb_overlap_touching() {
        // Boxes sharing exactly one face — conservative: touching counts as overlap.
        let a = Aabb::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let b = Aabb::new([1.0, 0.0, 0.0], [2.0, 1.0, 1.0]);
        assert!(aabb_overlap(&a, &b));
    }

    #[test]
    fn face_aabb_planar_cuboid_faces() {
        let mut ctx = SolidModelingContext::new();
        let sid = build_cuboid(&mut ctx, 1.0, 1.0, 1.0, 0, 0);
        let shell_id = ctx.get_solid(sid).outer;
        let faces = ctx.get_shell(shell_id).faces.clone();
        for &fid in &faces {
            let aabb = face_aabb(&ctx, fid);
            let eps = 1e-10;
            // AABB is contained within the unit cube
            for i in 0..3 {
                assert!(aabb.min[i] >= -eps);
                assert!(aabb.max[i] <= 1.0 + eps);
            }
            // Planar face: exactly one axis is flat (min == max)
            let flat = (0..3).filter(|&i| (aabb.max[i] - aabb.min[i]).abs() < eps).count();
            assert_eq!(flat, 1, "face {:?} should be flat on exactly one axis", fid);
        }
    }

    #[test]
    fn face_aabb_cylinder_cap_full_diameter() {
        // Vertex-only logic would return a single point (the seam vertex).
        // Correct logic uses the arc extrema and should span the full diameter.
        let mut ctx = SolidModelingContext::new();
        let sid = build_cylinder(&mut ctx, 1.0, 2.0, 0, 0);
        let shell_id = ctx.get_solid(sid).outer;
        let faces = ctx.get_shell(shell_id).faces.clone();
        let cap_face = *faces.iter().find(|&&fid| {
            let surf_id = ctx.get_face(fid).surface;
            matches!(ctx.get_surface(surf_id), SurfaceKind::Plane(_))
        }).unwrap();
        let aabb = face_aabb(&ctx, cap_face);
        let eps = 1e-10;
        assert!((aabb.min[0] + 1.0).abs() < eps, "min x = {}", aabb.min[0]);
        assert!((aabb.max[0] - 1.0).abs() < eps, "max x = {}", aabb.max[0]);
        assert!((aabb.min[1] + 1.0).abs() < eps, "min y = {}", aabb.min[1]);
        assert!((aabb.max[1] - 1.0).abs() < eps, "max y = {}", aabb.max[1]);
        assert!((aabb.max[2] - aabb.min[2]).abs() < eps, "cap must be flat in z");
    }

    // ── intersect_faces dispatcher ────────────────────────────────────────────

    #[test]
    fn intersect_faces_planar_pair_dispatches_plane_plane() {
        // Both faces are Plane — exercises the (Plane, Plane) arm.
        // Returns None from stub; will gain content once intersect_plane_plane is implemented.
        let mut ctx = SolidModelingContext::new();
        let sid = build_cuboid(&mut ctx, 1.0, 1.0, 1.0, 0, 0);
        let shell_id = ctx.get_solid(sid).outer;
        let faces = ctx.get_shell(shell_id).faces.clone();
        assert!(intersect_faces(&mut ctx, faces[0], faces[1]).is_none());
    }
}
