// Phase-0 staging: the boolean surface-surface-intersection scaffolding in this
// module is not yet wired into a public boolean API. Phase 0-c migrates it onto the
// defining-only b-rep structs; remove this allow once the boolean is wired up.
#![allow(dead_code)]

use std::collections::HashMap;

use crate::brep_kernel::{Curve2Id, Curve3Id, FaceId, SolidModelingContext, Vertex, VertexId};
use crate::geom::{Curve2Kind, Curve3Kind, Line2, Line3, Plane, Point2, Point3, SurfaceKind};

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
    ctx: &mut SolidModelingContext,
    face_a: FaceId,
    face_b: FaceId,
) -> Option<FaceFaceIntersection> {
    let (n_a, d_a) = {
        let p = plane_of_face(ctx, face_a);
        let n = p.u_dir.cross(p.v_dir);
        (n, n.dot(p.p0))
    };
    let (n_b, d_b) = {
        let p = plane_of_face(ctx, face_b);
        let n = p.u_dir.cross(p.v_dir);
        (n, n.dot(p.p0))
    };

    let (origin, dir) = plane_plane_line(n_a, d_a, n_b, d_b)?;

    let [ta0, ta1] = clip_line_to_face(ctx, face_a, origin, dir)?;
    let [tb0, tb1] = clip_line_to_face(ctx, face_b, origin, dir)?;

    let t_start = f64::max(ta0, tb0);
    let t_end   = f64::min(ta1, tb1);
    if t_start > t_end + 1e-10 {
        return None;
    }

    let p_start = origin + dir * t_start;
    let p_end   = origin + dir * t_end;

    let tol      = ctx.tolerance.pos_tol;
    let v_start  = ctx.push_vertex(Vertex::new(p_start, tol));
    let v_end    = ctx.push_vertex(Vertex::new(p_end,   tol));
    let curve3   = ctx.push_curve3(Curve3Kind::Line3(Line3::new(p_start, p_end)));
    let pcurve_a = push_intersection_pcurve(ctx, face_a, p_start, p_end);
    let pcurve_b = push_intersection_pcurve(ctx, face_b, p_start, p_end);

    Some(FaceFaceIntersection { v_start, v_end, curve3, pcurve_a, pcurve_b })
}

/// Project `p_start`/`p_end` onto the UV plane of `face_id` and push a Line2 pcurve.
fn push_intersection_pcurve(
    ctx: &mut SolidModelingContext,
    face_id: FaceId,
    p_start: Point3,
    p_end: Point3,
) -> Curve2Id {
    let (uv_start, uv_end) = {
        let pl = plane_of_face(ctx, face_id);
        let p0 = pl.p0;
        let u  = pl.u_dir;
        let v  = pl.v_dir;
        (
            Point2::new((p_start - p0).dot(u), (p_start - p0).dot(v)),
            Point2::new((p_end   - p0).dot(u), (p_end   - p0).dot(v)),
        )
    };
    ctx.push_curve2(Curve2Kind::Line2(Line2::new(uv_start, uv_end)))
}

// ── clip_line_to_face ─────────────────────────────────────────────────────────

/// Drop the dominant axis of `normal` and return the two surviving axis indices.
fn axis_pair(normal: Point3) -> (usize, usize) {
    let ax = [normal.x.abs(), normal.y.abs(), normal.z.abs()];
    let drop = ax.iter().enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(2);
    match drop {
        0 => (1, 2), // drop X → project to YZ
        1 => (0, 2), // drop Y → project to XZ
        _ => (0, 1), // drop Z → project to XY
    }
}

/// Clip the infinite line `origin + t*dir` to the boundary of `face_id`.
///
/// The line is assumed to lie in the face's plane. Returns `[t_min, t_max]` at
/// the two boundary crossings, or `None` if the line misses the face or is
/// tangent at a single point.
fn clip_line_to_face(
    ctx: &SolidModelingContext,
    face_id: FaceId,
    origin: Point3,
    dir: Point3,
) -> Option<[f64; 2]> {
    let plane = plane_of_face(ctx, face_id);
    let normal = plane.u_dir.cross(plane.v_dir);
    let (a0, a1) = axis_pair(normal);

    let o = [origin.x, origin.y, origin.z];
    let d = [dir.x,    dir.y,    dir.z   ];

    let outer_id = ctx.get_face(face_id).outer;
    let coedge_ids = ctx.get_loop(outer_id).coedges.clone();

    let mut hits: Vec<f64> = Vec::new();

    for ceid in coedge_ids {
        let edge_id = ctx.get_coedge(ceid).edge;
        let edge = ctx.get_edge(edge_id);
        let (t0_edge, t1_edge, c3id) = (edge.t0, edge.t1, edge.curve3);

        let crate::geom::Curve3Kind::Line3(line) = ctx.get_curve3(c3id) else { continue };

        // Edge: e(u) = line.p0 + (line.p1 - line.p0) * u,  u ∈ [t0_edge, t1_edge]
        let ep = [line.p0.x, line.p0.y, line.p0.z];
        let eq = [line.p1.x, line.p1.y, line.p1.z];
        let ed = [eq[a0] - ep[a0], eq[a1] - ep[a1]];

        // Solve:  d[a0]*t - ed[0]*u = ep[a0] - o[a0]
        //         d[a1]*t - ed[1]*u = ep[a1] - o[a1]
        let det = ed[0] * d[a1] - ed[1] * d[a0];
        if det.abs() < 1e-12 { continue; }

        let r0 = ep[a0] - o[a0];
        let r1 = ep[a1] - o[a1];
        let t_hit = (ed[0] * r1 - ed[1] * r0) / det;
        let u_hit = (d[a0] * r1 - d[a1] * r0) / det;

        if u_hit >= t0_edge - 1e-10 && u_hit <= t1_edge + 1e-10 {
            hits.push(t_hit);
        }
    }

    // Deduplicate hits at shared vertices (two edges meeting at a corner both fire).
    hits.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut dedup: Vec<f64> = Vec::new();
    for &t in &hits {
        if dedup.last().map_or(true, |&last| (t - last).abs() > 1e-10) {
            dedup.push(t);
        }
    }

    if dedup.len() < 2 {
        None
    } else {
        Some([dedup[0], *dedup.last().unwrap()])
    }
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

    // ── clip_line_to_face ─────────────────────────────────────────────────────

    fn unit_cuboid_faces(ctx: &mut SolidModelingContext) -> Vec<FaceId> {
        let sid = build_cuboid(ctx, 1.0, 1.0, 1.0, 0, 0);
        let shell_id = ctx.get_solid(sid).outer;
        ctx.get_shell(shell_id).faces.clone()
    }

    fn face_by_normal(ctx: &SolidModelingContext, faces: &[FaceId], nx: f64, ny: f64, nz: f64) -> FaceId {
        let target = Point3::new(nx, ny, nz);
        *faces.iter().find(|&&fid| {
            let surf_id = ctx.get_face(fid).surface;
            if let SurfaceKind::Plane(p) = ctx.get_surface(surf_id) {
                (p.u_dir.cross(p.v_dir) - target).length() < 1e-6
            } else {
                false
            }
        }).expect("face with given normal not found")
    }

    #[test]
    fn clip_line_miss_outside_parallel() {
        // Line at y=2.0 (parallel to x-axis) never enters y∈[0,1] of top face.
        let mut ctx = SolidModelingContext::new();
        let faces = unit_cuboid_faces(&mut ctx);
        let top = face_by_normal(&ctx, &faces, 0.0, 0.0, 1.0);
        let origin = Point3::new(0.5, 2.0, 1.0);
        let dir    = Point3::new(1.0, 0.0, 0.0);
        assert!(clip_line_to_face(&ctx, top, origin, dir).is_none());
    }

    #[test]
    fn clip_line_axis_aligned_through_opposite_edges() {
        // Line x=0.5 dir=+y: enters bottom edge (y=0) at t=1.0, exits top edge (y=1) at t=2.0.
        let mut ctx = SolidModelingContext::new();
        let faces = unit_cuboid_faces(&mut ctx);
        let top = face_by_normal(&ctx, &faces, 0.0, 0.0, 1.0);
        let origin = Point3::new(0.5, -1.0, 1.0);
        let dir    = Point3::new(0.0,  1.0, 0.0);
        let [t0, t1] = clip_line_to_face(&ctx, top, origin, dir).unwrap();
        let eps = 1e-10;
        assert!((t0 - 1.0).abs() < eps, "t0={t0}");
        assert!((t1 - 2.0).abs() < eps, "t1={t1}");
    }

    #[test]
    fn clip_line_axis_aligned_through_adjacent_edges() {
        // Line y=0.5 dir=+x: enters left edge (x=0) at t=1.0, exits right edge (x=1) at t=2.0.
        let mut ctx = SolidModelingContext::new();
        let faces = unit_cuboid_faces(&mut ctx);
        let top = face_by_normal(&ctx, &faces, 0.0, 0.0, 1.0);
        let origin = Point3::new(-1.0, 0.5, 1.0);
        let dir    = Point3::new( 1.0, 0.0, 0.0);
        let [t0, t1] = clip_line_to_face(&ctx, top, origin, dir).unwrap();
        let eps = 1e-10;
        assert!((t0 - 1.0).abs() < eps, "t0={t0}");
        assert!((t1 - 2.0).abs() < eps, "t1={t1}");
    }

    #[test]
    fn clip_line_diagonal_direction() {
        // Line origin=(0,0.5,1) dir=(1,1,0): exits left edge at t=0.0, top edge at t=0.5.
        let mut ctx = SolidModelingContext::new();
        let faces = unit_cuboid_faces(&mut ctx);
        let top = face_by_normal(&ctx, &faces, 0.0, 0.0, 1.0);
        let origin = Point3::new(0.0, 0.5, 1.0);
        let dir    = Point3::new(1.0, 1.0, 0.0);
        let [t0, t1] = clip_line_to_face(&ctx, top, origin, dir).unwrap();
        let eps = 1e-10;
        assert!((t0 - 0.0).abs() < eps, "t0={t0}");
        assert!((t1 - 0.5).abs() < eps, "t1={t1}");
    }

    #[test]
    fn clip_line_corner_to_corner_dedup_both_ends() {
        // Line passes through corners (0,0,1) at t=1 and (1,1,1) at t=2; each shared by
        // two edges — deduplication must yield exactly two hits.
        let mut ctx = SolidModelingContext::new();
        let faces = unit_cuboid_faces(&mut ctx);
        let top = face_by_normal(&ctx, &faces, 0.0, 0.0, 1.0);
        let origin = Point3::new(-1.0, -1.0, 1.0);
        let dir    = Point3::new( 1.0,  1.0, 0.0);
        let [t0, t1] = clip_line_to_face(&ctx, top, origin, dir).unwrap();
        let eps = 1e-10;
        assert!((t0 - 1.0).abs() < eps, "t0={t0}");
        assert!((t1 - 2.0).abs() < eps, "t1={t1}");
    }

    #[test]
    fn clip_line_tangent_at_corner_is_none() {
        // Line touches corner (0,0,1) at t=0 then moves away — only one unique hit.
        let mut ctx = SolidModelingContext::new();
        let faces = unit_cuboid_faces(&mut ctx);
        let top = face_by_normal(&ctx, &faces, 0.0, 0.0, 1.0);
        let origin = Point3::new(0.0,  0.0, 1.0);
        let dir    = Point3::new(1.0, -1.0, 0.0);
        assert!(clip_line_to_face(&ctx, top, origin, dir).is_none());
    }

    #[test]
    fn clip_line_enter_edge_exit_corner_dedup_one_end() {
        // Line enters left edge at t=0, exits corner (1,1,1) at t=1
        // (right and top edges both fire at t=1 → deduplicated to one hit).
        let mut ctx = SolidModelingContext::new();
        let faces = unit_cuboid_faces(&mut ctx);
        let top = face_by_normal(&ctx, &faces, 0.0, 0.0, 1.0);
        let origin = Point3::new(0.0, 0.5, 1.0);
        let dir    = Point3::new(1.0, 0.5, 0.0);
        let [t0, t1] = clip_line_to_face(&ctx, top, origin, dir).unwrap();
        let eps = 1e-10;
        assert!((t0 - 0.0).abs() < eps, "t0={t0}");
        assert!((t1 - 1.0).abs() < eps, "t1={t1}");
    }

    #[test]
    fn clip_line_side_face_different_normal() {
        // Left face (x=0, normal=(-1,0,0)): line at y=0.5 dir=+z enters z=0 at t=1, exits z=1 at t=2.
        let mut ctx = SolidModelingContext::new();
        let faces = unit_cuboid_faces(&mut ctx);
        let left = face_by_normal(&ctx, &faces, -1.0, 0.0, 0.0);
        let origin = Point3::new(0.0, 0.5, -1.0);
        let dir    = Point3::new(0.0, 0.0,  1.0);
        let [t0, t1] = clip_line_to_face(&ctx, left, origin, dir).unwrap();
        let eps = 1e-10;
        assert!((t0 - 1.0).abs() < eps, "t0={t0}");
        assert!((t1 - 2.0).abs() < eps, "t1={t1}");
    }

    // ── intersect_plane_plane ────────────────────────────────────────────────

    fn solid_faces(ctx: &SolidModelingContext, sid: crate::brep_kernel::SolidId) -> Vec<FaceId> {
        let shell_id = ctx.get_solid(sid).outer;
        ctx.get_shell(shell_id).faces.clone()
    }

    #[test]
    fn intersect_pp_parallel_planes_is_none() {
        // A top face (z=1) and B top face (z=2): parallel normals → None.
        let mut ctx = SolidModelingContext::new();
        let sid_a = build_cuboid(&mut ctx, 1.0, 1.0, 1.0, 0, 0);
        let sid_b = build_cuboid(&mut ctx, 1.0, 1.0, 2.0, 0, 0);
        let top_a = face_by_normal(&ctx, &solid_faces(&ctx, sid_a), 0.0, 0.0,  1.0);
        let top_b = face_by_normal(&ctx, &solid_faces(&ctx, sid_b), 0.0, 0.0,  1.0);
        assert!(intersect_plane_plane(&mut ctx, top_a, top_b).is_none());
    }

    #[test]
    fn intersect_pp_nonparallel_line_misses_face_is_none() {
        // A top face (z=1, x∈[0,1]) and B right face (x=2): intersection line at x=2
        // misses A's top face → None.
        let mut ctx = SolidModelingContext::new();
        let sid_a = build_cuboid(&mut ctx, 1.0, 1.0, 1.0, 0, 0);
        let sid_b = build_cuboid(&mut ctx, 2.0, 1.0, 1.0, 0, 0);
        let top_a   = face_by_normal(&ctx, &solid_faces(&ctx, sid_a), 0.0, 0.0, 1.0);
        let right_b = face_by_normal(&ctx, &solid_faces(&ctx, sid_b), 1.0, 0.0, 0.0);
        assert!(intersect_plane_plane(&mut ctx, top_a, right_b).is_none());
    }

    #[test]
    fn intersect_pp_nonparallel_disjoint_t_is_none() {
        // A top face (z=1, y∈[0,1]) and B left face (x=0.5, y∈[1.5,2.5]):
        // both clip to finite t-ranges that don't overlap → None.
        // B = unit cube translated (0.5, 1.5, 0.5).
        use crate::csg_lang::CsgNode;
        use crate::brep_compiler::compile_csg_node;
        let mut ctx = SolidModelingContext::new();
        let sid_a = build_cuboid(&mut ctx, 1.0, 1.0, 1.0, 0, 0);
        let b_node = CsgNode::cuboid(1.0, 1.0, 1.0).translate(0.5, 1.5, 0.5);
        let sid_b  = compile_csg_node(&mut ctx, &b_node);
        let top_a  = face_by_normal(&ctx, &solid_faces(&ctx, sid_a), 0.0,  0.0, 1.0);
        let left_b = face_by_normal(&ctx, &solid_faces(&ctx, sid_b), -1.0, 0.0, 0.0);
        assert!(intersect_plane_plane(&mut ctx, top_a, left_b).is_none());
    }

    #[test]
    fn intersect_pp_intersects_vertex_positions() {
        // A top face (z=1, x∈[0,1], y∈[0,1]) and B left face (x=0.5, y∈[0.1,0.6]).
        // B = (1×0.5×1) cuboid translated (0.5, 0.1, 0.5).
        // Both endpoints lie in the interior of A (test-case-4 geometry).
        use crate::csg_lang::CsgNode;
        use crate::brep_compiler::compile_csg_node;
        let mut ctx = SolidModelingContext::new();
        let sid_a  = build_cuboid(&mut ctx, 1.0, 1.0, 1.0, 0, 0);
        let b_node = CsgNode::cuboid(1.0, 0.5, 1.0).translate(0.5, 0.1, 0.5);
        let sid_b  = compile_csg_node(&mut ctx, &b_node);
        let top_a  = face_by_normal(&ctx, &solid_faces(&ctx, sid_a), 0.0,  0.0, 1.0);
        let left_b = face_by_normal(&ctx, &solid_faces(&ctx, sid_b), -1.0, 0.0, 0.0);
        let ffi = intersect_plane_plane(&mut ctx, top_a, left_b).unwrap();

        let ps = ctx.get_vertex(ffi.v_start).point;
        let pe = ctx.get_vertex(ffi.v_end).point;
        let eps = 1e-9;

        // Both endpoints at x=0.5, z=1 (on the intersection line).
        assert!((ps.x - 0.5).abs() < eps && (ps.z - 1.0).abs() < eps, "p_start={ps:?}");
        assert!((pe.x - 0.5).abs() < eps && (pe.z - 1.0).abs() < eps, "p_end={pe:?}");

        // y-values should be 0.1 and 0.6 (B's y-boundary, interior to A's y∈[0,1]).
        let ys = [ps.y, pe.y];
        assert!(ys.iter().any(|&y| (y - 0.6).abs() < eps), "y=0.6 missing: {ys:?}");
        assert!(ys.iter().any(|&y| (y - 0.1).abs() < eps), "y=0.1 missing: {ys:?}");
    }

    #[test]
    fn intersect_pp_intersects_curve3_and_pcurves() {
        // A top (z=1, x∈[0,2], y∈[0,2]) ∩ B right (x=1, y∈[0,2], z∈[0,2]).
        // Intersection line: x=1, z=1, dir=+y → segment (1,0,1)–(1,2,1).
        // Verifies curve3 connects the vertices and pcurves lie on their planes.
        let mut ctx = SolidModelingContext::new();
        let sid_a = build_cuboid(&mut ctx, 2.0, 2.0, 1.0, 0, 0);
        let sid_b = build_cuboid(&mut ctx, 1.0, 2.0, 2.0, 0, 0);
        let top_a   = face_by_normal(&ctx, &solid_faces(&ctx, sid_a), 0.0, 0.0, 1.0);
        let right_b = face_by_normal(&ctx, &solid_faces(&ctx, sid_b), 1.0, 0.0, 0.0);
        let ffi = intersect_plane_plane(&mut ctx, top_a, right_b).unwrap();

        let ps = ctx.get_vertex(ffi.v_start).point;
        let pe = ctx.get_vertex(ffi.v_end).point;
        let eps = 1e-9;

        // Both endpoints on x=1, z=1; y-values are 0.0 and 2.0.
        assert!((ps.x - 1.0).abs() < eps && (ps.z - 1.0).abs() < eps, "p_start={ps:?}");
        assert!((pe.x - 1.0).abs() < eps && (pe.z - 1.0).abs() < eps, "p_end={pe:?}");
        let ys = [ps.y, pe.y];
        assert!(ys.iter().any(|&y| y.abs() < eps),        "y=0 missing: {ys:?}");
        assert!(ys.iter().any(|&y| (y - 2.0).abs() < eps), "y=2 missing: {ys:?}");

        // curve3 is a Line3 whose endpoints match the vertices.
        let crate::geom::Curve3Kind::Line3(line) = ctx.get_curve3(ffi.curve3) else {
            panic!("curve3 is not a Line3");
        };
        let line = *line;
        assert!((line.p0 - ps).length().min((line.p0 - pe).length()) < eps, "curve3 p0 mismatch");
        assert!((line.p1 - ps).length().min((line.p1 - pe).length()) < eps, "curve3 p1 mismatch");

        // pcurve_a lies on A's top face (z=1 plane): both UV points should have the
        // same u-coord (x=1 in A's UV) and v-coords 0 and 2.
        let crate::geom::Curve2Kind::Line2(pc_a) = ctx.get_curve2(ffi.pcurve_a) else {
            panic!("pcurve_a is not a Line2");
        };
        let pc_a = *pc_a;
        assert!((pc_a.p0.u - 1.0).abs() < eps, "pcurve_a p0.u={}", pc_a.p0.u);
        assert!((pc_a.p1.u - 1.0).abs() < eps, "pcurve_a p1.u={}", pc_a.p1.u);
        let vs_a = [pc_a.p0.v, pc_a.p1.v];
        assert!(vs_a.iter().any(|&v| v.abs() < eps),        "pcurve_a v=0 missing");
        assert!(vs_a.iter().any(|&v| (v - 2.0).abs() < eps), "pcurve_a v=2 missing");

        // pcurve_b lies on B's right face (x=1 plane): both UV points should have the
        // same v-coord (z=1 in B's UV) and u-coords 0 and 2.
        let crate::geom::Curve2Kind::Line2(pc_b) = ctx.get_curve2(ffi.pcurve_b) else {
            panic!("pcurve_b is not a Line2");
        };
        let pc_b = *pc_b;
        assert!((pc_b.p0.v - 1.0).abs() < eps, "pcurve_b p0.v={}", pc_b.p0.v);
        assert!((pc_b.p1.v - 1.0).abs() < eps, "pcurve_b p1.v={}", pc_b.p1.v);
        let us_b = [pc_b.p0.u, pc_b.p1.u];
        assert!(us_b.iter().any(|&u| u.abs() < eps),        "pcurve_b u=0 missing");
        assert!(us_b.iter().any(|&u| (u - 2.0).abs() < eps), "pcurve_b u=2 missing");
    }
}
