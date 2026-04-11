//! B-rep compiler: converts CSG primitives into trimmed-surface B-rep solids.
//!
//! Each `build_*` function takes a [`SolidModelingContext`] and primitive parameters,
//! populates the arena with all geometry and topology, and returns the [`SolidId`] of
//! the resulting solid.

use crate::brep_kernel::{
    CoEdge, Edge, Face, FaceSense, Loop, LoopId, Orientation, ProvenanceData,
    Shell, Solid, SolidId, SolidModelingContext, Vertex,
};
use crate::geom::{
    CircularArc2, CircularArc3, ConicalSurface, Curve2Kind, Curve3Kind, CylindricalSurface,
    Line2, Line3, Plane, Point2, Point3, SphericalSurface, SurfaceKind,
};

// ── build_cuboid ──────────────────────────────────────────────────────────────

/// Build the B-rep for a cuboid with one corner at the origin and the opposite
/// corner at `(dx, dy, dz)`.
///
/// Topology: 8 vertices, 12 Line3 edges, 24 coedges, 6 faces, 1 shell, 1 solid.
/// All six faces use `FaceSense::Aligned` (the plane's natural normal points outward).
///
/// `prov_id` and `geom_id` are stored in each face's [`ProvenanceData`].
pub fn build_cuboid(
    ctx: &mut SolidModelingContext,
    dx: f64,
    dy: f64,
    dz: f64,
    prov_id: u64,
    geom_id: u64,
) -> SolidId {
    let tol = ctx.tolerance.pos_tol;

    // ── Vertices ──────────────────────────────────────────────────────────────
    // V0..V3 at z=0 (CCW from origin), V4..V7 at z=dz directly above V0..V3.
    let p = |x, y, z| Point3::new(x, y, z);
    let v0 = ctx.push_vertex(Vertex::new(p(0.0, 0.0,  0.0), tol));
    let v1 = ctx.push_vertex(Vertex::new(p( dx, 0.0,  0.0), tol));
    let v2 = ctx.push_vertex(Vertex::new(p( dx,  dy,  0.0), tol));
    let v3 = ctx.push_vertex(Vertex::new(p(0.0,  dy,  0.0), tol));
    let v4 = ctx.push_vertex(Vertex::new(p(0.0, 0.0,  dz), tol));
    let v5 = ctx.push_vertex(Vertex::new(p( dx, 0.0,  dz), tol));
    let v6 = ctx.push_vertex(Vertex::new(p( dx,  dy,  dz), tol));
    let v7 = ctx.push_vertex(Vertex::new(p(0.0,  dy,  dz), tol));

    // ── Edges (Line3, t ∈ [0,1]) ──────────────────────────────────────────────
    // Bottom ring: E0..E3  Top ring: E4..E7  Verticals: E8..E11
    // Each edge: push curve first (immutable access to vertex positions), then push edge.
    let line3 = |a: Point3, b: Point3| Curve3Kind::Line3(Line3::new(a, b));

    macro_rules! push_edge {
        ($ctx:expr, $pa:expr, $pb:expr, $va:expr, $vb:expr) => {{
            let crv = line3($pa, $pb);
            let c   = $ctx.push_curve3(crv);
            $ctx.push_edge(Edge::new(c, $va, $vb, 0.0, 1.0))
        }};
    }

    // Snapshot vertex positions before any mutable borrows of ctx.
    let pts = [
        ctx.get_vertex(v0).point, // 0
        ctx.get_vertex(v1).point, // 1
        ctx.get_vertex(v2).point, // 2
        ctx.get_vertex(v3).point, // 3
        ctx.get_vertex(v4).point, // 4
        ctx.get_vertex(v5).point, // 5
        ctx.get_vertex(v6).point, // 6
        ctx.get_vertex(v7).point, // 7
    ];

    let e0  = push_edge!(ctx, pts[0], pts[1], v0, v1);
    let e1  = push_edge!(ctx, pts[1], pts[2], v1, v2);
    let e2  = push_edge!(ctx, pts[2], pts[3], v2, v3);
    let e3  = push_edge!(ctx, pts[3], pts[0], v3, v0);
    let e4  = push_edge!(ctx, pts[4], pts[5], v4, v5);
    let e5  = push_edge!(ctx, pts[5], pts[6], v5, v6);
    let e6  = push_edge!(ctx, pts[6], pts[7], v6, v7);
    let e7  = push_edge!(ctx, pts[7], pts[4], v7, v4);
    let e8  = push_edge!(ctx, pts[0], pts[4], v0, v4);
    let e9  = push_edge!(ctx, pts[1], pts[5], v1, v5);
    let e10 = push_edge!(ctx, pts[2], pts[6], v2, v6);
    let e11 = push_edge!(ctx, pts[3], pts[7], v3, v7);

    // ── Topology skeleton ─────────────────────────────────────────────────────
    let solid_id = ctx.push_solid(Solid::new({
        // shell ID not yet known; we'll use a placeholder and patch below
        crate::brep_kernel::ShellId(usize::MAX)
    }));
    let shell_id = ctx.push_shell(Shell::new(solid_id, true));
    // patch solid.outer
    ctx.get_mut_solid(solid_id).outer = shell_id;

    let prov = || ProvenanceData::primitive(prov_id, geom_id);

    // Helper: create a Face with a placeholder LoopId, push to context, register
    // in shell, then return (face_id, loop_id).
    macro_rules! make_face {
        ($surf:expr, $sense:expr) => {{
            let surf_id = ctx.push_surface($surf);
            let face_id = ctx.push_face(Face::new(
                shell_id, surf_id,
                LoopId(usize::MAX), // patched below
                $sense, prov(),
            ));
            let loop_id = ctx.push_loop(Loop::new(face_id, true));
            ctx.get_mut_face(face_id).outer = loop_id;
            ctx.get_mut_shell(shell_id).faces.push(face_id);
            (face_id, loop_id)
        }};
    }

    // UV helper: project a Point3 onto a plane's UV coordinates.
    // Works because u_dir and v_dir are unit vectors.
    let uv = |pt: Point3, p0: Point3, u_dir: Point3, v_dir: Point3| -> Point2 {
        let d = pt - p0;
        Point2::new(d.dot(u_dir), d.dot(v_dir))
    };

    // Helper: push a Line2 pcurve and return its Curve2Id.
    let line2 = |ctx: &mut SolidModelingContext, a: Point2, b: Point2| {
        ctx.push_curve2(Curve2Kind::Line2(Line2::new(a, b)))
    };

    // Helper: push a coedge and register it back on the edge.
    macro_rules! add_coedge {
        ($ctx:expr, $edge:expr, $orient:expr, $face:expr, $pcurve:expr) => {{
            let ce_id = $ctx.push_coedge(CoEdge::new($edge, $orient, $face, $pcurve));
            $ctx.get_mut_edge($edge).coedges.push(ce_id);
            ce_id
        }};
    }

    // ── Face 0: Bottom (z = 0, outward normal = (0,0,-1)) ────────────────────
    // Plane: p0=(0,0,0), u=(1,0,0), v=(0,-1,0)  →  natural normal = (0,0,-1) Aligned
    {
        let p0    = p(0.0, 0.0, 0.0);
        let u_dir = p(1.0, 0.0, 0.0);
        let v_dir = p(0.0,-1.0, 0.0);
        let surf  = SurfaceKind::Plane(Plane::new(p0, u_dir, v_dir));
        let (face_id, loop_id) = make_face!(surf, FaceSense::Aligned);

        // Loop: E0 Rev, E3 Rev, E2 Rev, E1 Rev  (V1→V0→V3→V2→V1)
        // PCurves: Line2 from UV(edge.v0) to UV(edge.v1), regardless of orientation.
        let pc_e0 = line2(ctx, uv(pts[0], p0, u_dir, v_dir), uv(pts[1], p0, u_dir, v_dir));
        let pc_e3 = line2(ctx, uv(pts[3], p0, u_dir, v_dir), uv(pts[0], p0, u_dir, v_dir));
        let pc_e2 = line2(ctx, uv(pts[2], p0, u_dir, v_dir), uv(pts[3], p0, u_dir, v_dir));
        let pc_e1 = line2(ctx, uv(pts[1], p0, u_dir, v_dir), uv(pts[2], p0, u_dir, v_dir));

        let ce0 = add_coedge!(ctx, e0, Orientation::Reverse, face_id, pc_e0);
        let ce3 = add_coedge!(ctx, e3, Orientation::Reverse, face_id, pc_e3);
        let ce2 = add_coedge!(ctx, e2, Orientation::Reverse, face_id, pc_e2);
        let ce1 = add_coedge!(ctx, e1, Orientation::Reverse, face_id, pc_e1);
        ctx.get_mut_loop(loop_id).coedges.extend([ce0, ce3, ce2, ce1]);
    }

    // ── Face 1: Top (z = dz, outward normal = (0,0,1)) ───────────────────────
    // Plane: p0=(0,0,dz), u=(1,0,0), v=(0,1,0)  →  natural normal = (0,0,1) Aligned
    {
        let p0    = p(0.0, 0.0, dz);
        let u_dir = p(1.0, 0.0, 0.0);
        let v_dir = p(0.0, 1.0, 0.0);
        let surf  = SurfaceKind::Plane(Plane::new(p0, u_dir, v_dir));
        let (face_id, loop_id) = make_face!(surf, FaceSense::Aligned);

        // Loop: E4 Fwd, E5 Fwd, E6 Fwd, E7 Fwd  (V4→V5→V6→V7→V4)
        let pc_e4 = line2(ctx, uv(pts[4], p0, u_dir, v_dir), uv(pts[5], p0, u_dir, v_dir));
        let pc_e5 = line2(ctx, uv(pts[5], p0, u_dir, v_dir), uv(pts[6], p0, u_dir, v_dir));
        let pc_e6 = line2(ctx, uv(pts[6], p0, u_dir, v_dir), uv(pts[7], p0, u_dir, v_dir));
        let pc_e7 = line2(ctx, uv(pts[7], p0, u_dir, v_dir), uv(pts[4], p0, u_dir, v_dir));

        let ce4 = add_coedge!(ctx, e4, Orientation::Forward, face_id, pc_e4);
        let ce5 = add_coedge!(ctx, e5, Orientation::Forward, face_id, pc_e5);
        let ce6 = add_coedge!(ctx, e6, Orientation::Forward, face_id, pc_e6);
        let ce7 = add_coedge!(ctx, e7, Orientation::Forward, face_id, pc_e7);
        ctx.get_mut_loop(loop_id).coedges.extend([ce4, ce5, ce6, ce7]);
    }

    // ── Face 2: Front (y = 0, outward normal = (0,-1,0)) ─────────────────────
    // Plane: p0=(0,0,0), u=(1,0,0), v=(0,0,1)  →  natural normal = (0,-1,0) Aligned
    {
        let p0    = p(0.0, 0.0, 0.0);
        let u_dir = p(1.0, 0.0, 0.0);
        let v_dir = p(0.0, 0.0, 1.0);
        let surf  = SurfaceKind::Plane(Plane::new(p0, u_dir, v_dir));
        let (face_id, loop_id) = make_face!(surf, FaceSense::Aligned);

        // Loop: E0 Fwd, E9 Fwd, E4 Rev, E8 Rev  (V0→V1→V5→V4→V0)
        let pc_e0 = line2(ctx, uv(pts[0], p0, u_dir, v_dir), uv(pts[1], p0, u_dir, v_dir));
        let pc_e9 = line2(ctx, uv(pts[1], p0, u_dir, v_dir), uv(pts[5], p0, u_dir, v_dir));
        let pc_e4 = line2(ctx, uv(pts[4], p0, u_dir, v_dir), uv(pts[5], p0, u_dir, v_dir));
        let pc_e8 = line2(ctx, uv(pts[0], p0, u_dir, v_dir), uv(pts[4], p0, u_dir, v_dir));

        let ce0 = add_coedge!(ctx, e0, Orientation::Forward,  face_id, pc_e0);
        let ce9 = add_coedge!(ctx, e9, Orientation::Forward,  face_id, pc_e9);
        let ce4 = add_coedge!(ctx, e4, Orientation::Reverse,  face_id, pc_e4);
        let ce8 = add_coedge!(ctx, e8, Orientation::Reverse,  face_id, pc_e8);
        ctx.get_mut_loop(loop_id).coedges.extend([ce0, ce9, ce4, ce8]);
    }

    // ── Face 3: Back (y = dy, outward normal = (0,1,0)) ──────────────────────
    // Plane: p0=(dx,dy,0), u=(-1,0,0), v=(0,0,1)  →  natural normal = (0,1,0) Aligned
    {
        let p0    = p(dx, dy, 0.0);
        let u_dir = p(-1.0, 0.0, 0.0);
        let v_dir = p(0.0,  0.0, 1.0);
        let surf  = SurfaceKind::Plane(Plane::new(p0, u_dir, v_dir));
        let (face_id, loop_id) = make_face!(surf, FaceSense::Aligned);

        // Loop: E2 Fwd, E11 Fwd, E6 Rev, E10 Rev  (V2→V3→V7→V6→V2)
        let pc_e2  = line2(ctx, uv(pts[2], p0, u_dir, v_dir), uv(pts[3],  p0, u_dir, v_dir));
        let pc_e11 = line2(ctx, uv(pts[3], p0, u_dir, v_dir), uv(pts[7],  p0, u_dir, v_dir));
        let pc_e6  = line2(ctx, uv(pts[6], p0, u_dir, v_dir), uv(pts[7],  p0, u_dir, v_dir));
        let pc_e10 = line2(ctx, uv(pts[2], p0, u_dir, v_dir), uv(pts[6],  p0, u_dir, v_dir));

        let ce2  = add_coedge!(ctx, e2,  Orientation::Forward, face_id, pc_e2);
        let ce11 = add_coedge!(ctx, e11, Orientation::Forward, face_id, pc_e11);
        let ce6  = add_coedge!(ctx, e6,  Orientation::Reverse, face_id, pc_e6);
        let ce10 = add_coedge!(ctx, e10, Orientation::Reverse, face_id, pc_e10);
        ctx.get_mut_loop(loop_id).coedges.extend([ce2, ce11, ce6, ce10]);
    }

    // ── Face 4: Left (x = 0, outward normal = (-1,0,0)) ──────────────────────
    // Plane: p0=(0,dy,0), u=(0,-1,0), v=(0,0,1)  →  natural normal = (-1,0,0) Aligned
    {
        let p0    = p(0.0, dy, 0.0);
        let u_dir = p(0.0, -1.0, 0.0);
        let v_dir = p(0.0,  0.0, 1.0);
        let surf  = SurfaceKind::Plane(Plane::new(p0, u_dir, v_dir));
        let (face_id, loop_id) = make_face!(surf, FaceSense::Aligned);

        // Loop: E3 Fwd, E8 Fwd, E7 Rev, E11 Rev  (V3→V0→V4→V7→V3)
        let pc_e3  = line2(ctx, uv(pts[3], p0, u_dir, v_dir), uv(pts[0], p0, u_dir, v_dir));
        let pc_e8  = line2(ctx, uv(pts[0], p0, u_dir, v_dir), uv(pts[4], p0, u_dir, v_dir));
        let pc_e7  = line2(ctx, uv(pts[7], p0, u_dir, v_dir), uv(pts[4], p0, u_dir, v_dir));
        let pc_e11 = line2(ctx, uv(pts[3], p0, u_dir, v_dir), uv(pts[7], p0, u_dir, v_dir));

        let ce3  = add_coedge!(ctx, e3,  Orientation::Forward, face_id, pc_e3);
        let ce8  = add_coedge!(ctx, e8,  Orientation::Forward, face_id, pc_e8);
        let ce7  = add_coedge!(ctx, e7,  Orientation::Reverse, face_id, pc_e7);
        let ce11 = add_coedge!(ctx, e11, Orientation::Reverse, face_id, pc_e11);
        ctx.get_mut_loop(loop_id).coedges.extend([ce3, ce8, ce7, ce11]);
    }

    // ── Face 5: Right (x = dx, outward normal = (1,0,0)) ─────────────────────
    // Plane: p0=(dx,0,0), u=(0,1,0), v=(0,0,1)  →  natural normal = (1,0,0) Aligned
    {
        let p0    = p(dx,  0.0, 0.0);
        let u_dir = p(0.0, 1.0, 0.0);
        let v_dir = p(0.0, 0.0, 1.0);
        let surf  = SurfaceKind::Plane(Plane::new(p0, u_dir, v_dir));
        let (face_id, loop_id) = make_face!(surf, FaceSense::Aligned);

        // Loop: E1 Fwd, E10 Fwd, E5 Rev, E9 Rev  (V1→V2→V6→V5→V1)
        let pc_e1  = line2(ctx, uv(pts[1], p0, u_dir, v_dir), uv(pts[2], p0, u_dir, v_dir));
        let pc_e10 = line2(ctx, uv(pts[2], p0, u_dir, v_dir), uv(pts[6], p0, u_dir, v_dir));
        let pc_e5  = line2(ctx, uv(pts[5], p0, u_dir, v_dir), uv(pts[6], p0, u_dir, v_dir));
        let pc_e9  = line2(ctx, uv(pts[1], p0, u_dir, v_dir), uv(pts[5], p0, u_dir, v_dir));

        let ce1  = add_coedge!(ctx, e1,  Orientation::Forward, face_id, pc_e1);
        let ce10 = add_coedge!(ctx, e10, Orientation::Forward, face_id, pc_e10);
        let ce5  = add_coedge!(ctx, e5,  Orientation::Reverse, face_id, pc_e5);
        let ce9  = add_coedge!(ctx, e9,  Orientation::Reverse, face_id, pc_e9);
        ctx.get_mut_loop(loop_id).coedges.extend([ce1, ce10, ce5, ce9]);
    }

    solid_id
}

// ── build_cylinder ────────────────────────────────────────────────────────────

/// Build the B-rep for a cylinder of radius `r` and height `h`.
///
/// The base circle is centred at the origin in the z=0 plane; the axis runs along +z.
/// The seam is at the intersection with the x-z half-plane (u = 0 / 2π on the surface).
///
/// Topology: 2 vertices, 3 edges, 6 coedges, 3 loops, 3 faces, 1 shell, 1 solid.
///
/// Faces (in push order):
/// 0. Lateral  — `CylindricalSurface`, `FaceSense::Aligned`
/// 1. Base cap — `Plane` at z=0, `FaceSense::AntiAligned` (natural normal = +z; outward = -z)
/// 2. Top cap  — `Plane` at z=h, `FaceSense::Aligned`    (natural normal = +z; outward = +z)
pub fn build_cylinder(
    ctx: &mut SolidModelingContext,
    r: f64,
    h: f64,
    prov_id: u64,
    geom_id: u64,
) -> SolidId {
    use std::f64::consts::TAU; // 2π
    let tol = ctx.tolerance.pos_tol;
    let p3  = |x, y, z| Point3::new(x, y, z);
    let p2  = |u, v| Point2::new(u, v);

    // ── Vertices ──────────────────────────────────────────────────────────────
    // Both vertices lie on the seam (x=r, y=0).
    let v_bot = ctx.push_vertex(Vertex::new(p3(r, 0.0, 0.0), tol));
    let v_top = ctx.push_vertex(Vertex::new(p3(r, 0.0, h),   tol));

    // ── Curves3 ───────────────────────────────────────────────────────────────
    // E_base and E_top are full circles (closed: v0 == v1). t ∈ [0, 2π].
    // E_seam is the vertical seam line. t ∈ [0, 1].
    let normal_up  = p3(0.0, 0.0, 1.0);
    let ref_x      = p3(1.0, 0.0, 0.0);

    let c_base = ctx.push_curve3(Curve3Kind::CircularArc3(
        CircularArc3::new(p3(0.0, 0.0, 0.0), normal_up, ref_x, r, 0.0, TAU),
    ));
    let c_top  = ctx.push_curve3(Curve3Kind::CircularArc3(
        CircularArc3::new(p3(0.0, 0.0, h),   normal_up, ref_x, r, 0.0, TAU),
    ));
    let c_seam = ctx.push_curve3(Curve3Kind::Line3(
        Line3::new(p3(r, 0.0, 0.0), p3(r, 0.0, h)),
    ));

    // ── Edges ─────────────────────────────────────────────────────────────────
    // Closed circle edges: v0 == v1, t ∈ [0, 2π].
    let e_base = ctx.push_edge(Edge::new(c_base, v_bot, v_bot, 0.0, TAU));
    let e_top  = ctx.push_edge(Edge::new(c_top,  v_top, v_top, 0.0, TAU));
    let e_seam = ctx.push_edge(Edge::new(c_seam, v_bot, v_top, 0.0, 1.0));

    // ── Topology skeleton ─────────────────────────────────────────────────────
    let solid_id = ctx.push_solid(Solid::new(crate::brep_kernel::ShellId(usize::MAX)));
    let shell_id = ctx.push_shell(Shell::new(solid_id, true));
    ctx.get_mut_solid(solid_id).outer = shell_id;

    let prov = || ProvenanceData::primitive(prov_id, geom_id);

    macro_rules! make_face {
        ($surf:expr, $sense:expr) => {{
            let surf_id = ctx.push_surface($surf);
            let face_id = ctx.push_face(Face::new(
                shell_id, surf_id, LoopId(usize::MAX), $sense, prov(),
            ));
            let loop_id = ctx.push_loop(Loop::new(face_id, true));
            ctx.get_mut_face(face_id).outer = loop_id;
            ctx.get_mut_shell(shell_id).faces.push(face_id);
            (face_id, loop_id)
        }};
    }

    macro_rules! add_coedge {
        ($edge:expr, $orient:expr, $face:expr, $pcurve:expr) => {{
            let ce_id = ctx.push_coedge(CoEdge::new($edge, $orient, $face, $pcurve));
            ctx.get_mut_edge($edge).coedges.push(ce_id);
            ce_id
        }};
    }

    // ── Face 0: Lateral (CylindricalSurface, FaceSense::Aligned) ─────────────
    // UV rectangle: u ∈ [0, 2π], v ∈ [0, h].
    // Loop CCW in UV: (0,0)→(2π,0)→(2π,h)→(0,h)→(0,0)
    // = E_base Fwd | E_seam Fwd (right) | E_top Rev | E_seam Rev (left)
    //
    // PCurve parameterization note: Line2.eval(t) = p0 + (p1-p0)*t.
    // For E_base / E_top (t ∈ [0,2π]): p0=(0,0),p1=(1,0) → eval(t)=(t,0)   maps angle→u ✓
    // For E_top              reverse  : p0=(0,h),p1=(1,h) → eval(t)=(t,h)   ✓
    // For E_seam right (t ∈ [0,1]):   p0=(2π,0),p1=(2π,h) → eval(t)=(2π, h*t) ✓
    // For E_seam left  (t ∈ [0,1]):   p0=(0,0), p1=(0,h)  → eval(t)=(0,  h*t) ✓
    {
        let cyl = CylindricalSurface::new(
            p3(0.0, 0.0, 0.0), p3(0.0, 0.0, 1.0), p3(1.0, 0.0, 0.0), r,
        );
        let (face_id, loop_id) = make_face!(SurfaceKind::Cylinder(cyl), FaceSense::Aligned);

        let pc_base_lat  = ctx.push_curve2(Curve2Kind::Line2(Line2::new(p2(0.0, 0.0), p2(1.0, 0.0))));
        let pc_seam_rgt  = ctx.push_curve2(Curve2Kind::Line2(Line2::new(p2(TAU, 0.0), p2(TAU, h  ))));
        let pc_top_lat   = ctx.push_curve2(Curve2Kind::Line2(Line2::new(p2(0.0, h  ), p2(1.0, h  ))));
        let pc_seam_lft  = ctx.push_curve2(Curve2Kind::Line2(Line2::new(p2(0.0, 0.0), p2(0.0, h  ))));

        let ce_base = add_coedge!(e_base, Orientation::Forward,  face_id, pc_base_lat);
        let ce_sr   = add_coedge!(e_seam, Orientation::Forward,  face_id, pc_seam_rgt);
        let ce_top  = add_coedge!(e_top,  Orientation::Reverse,  face_id, pc_top_lat);
        let ce_sl   = add_coedge!(e_seam, Orientation::Reverse,  face_id, pc_seam_lft);
        ctx.get_mut_loop(loop_id).coedges.extend([ce_base, ce_sr, ce_top, ce_sl]);
    }

    // ── Face 1: Base cap (Plane z=0, FaceSense::AntiAligned) ─────────────────
    // Plane u=(1,0,0), v=(0,1,0) → natural normal = (0,0,1); outward = (0,0,-1) via AntiAligned.
    // PCurve: CircularArc2(center=(0,0), r, 0, 2π) — matches E_base parameterization directly.
    // Loop CW in UV (Reverse) to satisfy the AntiAligned convention.
    {
        let plane = Plane::new(p3(0.0, 0.0, 0.0), p3(1.0, 0.0, 0.0), p3(0.0, 1.0, 0.0));
        let (face_id, loop_id) = make_face!(SurfaceKind::Plane(plane), FaceSense::AntiAligned);

        let pc = ctx.push_curve2(Curve2Kind::CircularArc2(
            CircularArc2::new(p2(0.0, 0.0), r, 0.0, TAU),
        ));
        let ce = add_coedge!(e_base, Orientation::Reverse, face_id, pc);
        ctx.get_mut_loop(loop_id).coedges.push(ce);
    }

    // ── Face 2: Top cap (Plane z=h, FaceSense::Aligned) ──────────────────────
    // Plane u=(1,0,0), v=(0,1,0) → natural normal = (0,0,1) = outward.
    // Loop CCW in UV (Forward) to satisfy the Aligned convention.
    {
        let plane = Plane::new(p3(0.0, 0.0, h), p3(1.0, 0.0, 0.0), p3(0.0, 1.0, 0.0));
        let (face_id, loop_id) = make_face!(SurfaceKind::Plane(plane), FaceSense::Aligned);

        let pc = ctx.push_curve2(Curve2Kind::CircularArc2(
            CircularArc2::new(p2(0.0, 0.0), r, 0.0, TAU),
        ));
        let ce = add_coedge!(e_top, Orientation::Forward, face_id, pc);
        ctx.get_mut_loop(loop_id).coedges.push(ce);
    }

    solid_id
}

// ── build_cone ────────────────────────────────────────────────────────────────

/// Build the B-rep for a cone with base radius `r`, height `h`.
///
/// The base circle is centred at the origin in the z=0 plane; the apex is at (0,0,h).
/// The seam is at the intersection with the x-z half-plane.
///
/// Topology: 2 vertices, 3 edges, 5 coedges, 2 loops, 2 faces, 1 shell, 1 solid.
///
/// The apex has no face; it is represented by a **degenerate edge** (`E_apex_deg`,
/// `v0 == v1 == V_apex`, Line3 p0==p1) that closes the top of the lateral UV rectangle.
/// Because no second face borders that edge, `E_apex_deg` carries only 1 coedge.
///
/// Faces (in push order):
/// 0. Lateral — `ConicalSurface`, `FaceSense::Aligned`
/// 1. Base cap — `Plane` at z=0, `FaceSense::AntiAligned`
pub fn build_cone(
    ctx: &mut SolidModelingContext,
    r: f64,
    h: f64,
    prov_id: u64,
    geom_id: u64,
) -> SolidId {
    use std::f64::consts::TAU;
    let tol    = ctx.tolerance.pos_tol;
    let p3     = |x, y, z| Point3::new(x, y, z);
    let p2     = |u, v| Point2::new(u, v);
    let v_max  = (r * r + h * h).sqrt(); // slant distance from apex to base circle

    // ── Vertices ──────────────────────────────────────────────────────────────
    let v_apex = ctx.push_vertex(Vertex::new(p3(0.0, 0.0, h  ), tol));
    let v_base = ctx.push_vertex(Vertex::new(p3(r,   0.0, 0.0), tol));

    // ── Curves3 ───────────────────────────────────────────────────────────────
    // E_base: full circle at z=0, CCW from above, t ∈ [0, 2π].  Closed: v0==v1==v_base.
    // E_apex_deg: degenerate point at apex. p0==p1, v0==v1==v_apex, t ∈ [0, 1].
    // E_seam: straight line from apex (t=0) to base-seam point (t=1).
    let c_base     = ctx.push_curve3(Curve3Kind::CircularArc3(
        CircularArc3::new(p3(0.0, 0.0, 0.0), p3(0.0, 0.0, 1.0), p3(1.0, 0.0, 0.0), r, 0.0, TAU),
    ));
    let c_apex_deg = ctx.push_curve3(Curve3Kind::Line3(
        Line3::new(p3(0.0, 0.0, h), p3(0.0, 0.0, h)),
    ));
    let c_seam     = ctx.push_curve3(Curve3Kind::Line3(
        Line3::new(p3(0.0, 0.0, h), p3(r, 0.0, 0.0)),
    ));

    // ── Edges ─────────────────────────────────────────────────────────────────
    let e_base     = ctx.push_edge(Edge::new(c_base,     v_base, v_base, 0.0, TAU));
    let e_apex_deg = ctx.push_edge(Edge::new(c_apex_deg, v_apex, v_apex, 0.0, 1.0));
    let e_seam     = ctx.push_edge(Edge::new(c_seam,     v_apex, v_base, 0.0, 1.0));

    // ── Topology skeleton ─────────────────────────────────────────────────────
    let solid_id = ctx.push_solid(Solid::new(crate::brep_kernel::ShellId(usize::MAX)));
    let shell_id = ctx.push_shell(Shell::new(solid_id, true));
    ctx.get_mut_solid(solid_id).outer = shell_id;

    let prov = || ProvenanceData::primitive(prov_id, geom_id);

    macro_rules! make_face {
        ($surf:expr, $sense:expr) => {{
            let surf_id = ctx.push_surface($surf);
            let face_id = ctx.push_face(Face::new(
                shell_id, surf_id, LoopId(usize::MAX), $sense, prov(),
            ));
            let loop_id = ctx.push_loop(Loop::new(face_id, true));
            ctx.get_mut_face(face_id).outer = loop_id;
            ctx.get_mut_shell(shell_id).faces.push(face_id);
            (face_id, loop_id)
        }};
    }

    macro_rules! add_coedge {
        ($edge:expr, $orient:expr, $face:expr, $pcurve:expr) => {{
            let ce_id = ctx.push_coedge(CoEdge::new($edge, $orient, $face, $pcurve));
            ctx.get_mut_edge($edge).coedges.push(ce_id);
            ce_id
        }};
    }

    // ── Face 0: Lateral (ConicalSurface, FaceSense::Aligned) ──────────────────
    //
    // ConicalSurface: apex=(0,0,h), axis=(0,0,-1), ref_dir=(1,0,0), ha=atan(r/h)
    // eval(u,v) = (v·sin(ha)·cos(u),  −v·sin(ha)·sin(u),  h − v·cos(ha))
    //   → u=0: seam direction (+x); u increases → CW from above
    //
    // UV rectangle: u ∈ [0, 2π], v ∈ [0, v_max].
    // Loop CCW in UV: (0,0)→(2π,0)→(2π,v_max)→(0,v_max)→(0,0)
    //   = E_apex_deg Fwd | E_seam Fwd | E_base Fwd | E_seam Rev
    //
    // PCurve notes (all Line2.eval(t) = p0 + (p1−p0)·t):
    //   E_apex_deg (t∈[0,1]): p0=(0,0),    p1=(2π,0)     → (2π·t, 0)
    //   E_seam Fwd (t∈[0,1]): p0=(2π,0),   p1=(2π,v_max) → (2π, v_max·t)
    //   E_base Fwd (t∈[0,2π]): p0=(2π,vm), p1=(2π−1,vm)  → (2π−t, v_max) ← u decreases as t↑
    //   E_seam Rev (t∈[0,1]): p0=(0,0),    p1=(0,v_max)  → (0, v_max·t)  [traversed t:1→0]
    {
        let ha   = (r / h).atan();
        let cone = ConicalSurface::new(
            p3(0.0, 0.0, h), p3(0.0, 0.0, -1.0), p3(1.0, 0.0, 0.0), ha,
        );
        let (face_id, loop_id) = make_face!(SurfaceKind::Cone(cone), FaceSense::Aligned);

        let pc_apex = ctx.push_curve2(Curve2Kind::Line2(Line2::new(
            p2(0.0, 0.0), p2(TAU, 0.0),
        )));
        let pc_seam_rgt = ctx.push_curve2(Curve2Kind::Line2(Line2::new(
            p2(TAU, 0.0), p2(TAU, v_max),
        )));
        // E_base on ConicalSurface: CCW E_base (t↑) maps to decreasing u, so
        // p1 = p0 + (−1, 0) so that eval(t) = (TAU−t, v_max).
        let pc_base_lat = ctx.push_curve2(Curve2Kind::Line2(Line2::new(
            p2(TAU, v_max), p2(TAU - 1.0, v_max),
        )));
        let pc_seam_lft = ctx.push_curve2(Curve2Kind::Line2(Line2::new(
            p2(0.0, 0.0), p2(0.0, v_max),
        )));

        let ce_apex = add_coedge!(e_apex_deg, Orientation::Forward, face_id, pc_apex);
        let ce_sr   = add_coedge!(e_seam,     Orientation::Forward, face_id, pc_seam_rgt);
        let ce_base = add_coedge!(e_base,      Orientation::Forward, face_id, pc_base_lat);
        let ce_sl   = add_coedge!(e_seam,      Orientation::Reverse, face_id, pc_seam_lft);
        ctx.get_mut_loop(loop_id).coedges.extend([ce_apex, ce_sr, ce_base, ce_sl]);
    }

    // ── Face 1: Base cap (Plane z=0, FaceSense::AntiAligned) ──────────────────
    // Same convention as cylinder base: natural normal = +z, outward = −z → AntiAligned.
    // Loop CW in UV (Reverse E_base) for outward −z.
    {
        let plane = Plane::new(p3(0.0, 0.0, 0.0), p3(1.0, 0.0, 0.0), p3(0.0, 1.0, 0.0));
        let (face_id, loop_id) = make_face!(SurfaceKind::Plane(plane), FaceSense::AntiAligned);

        let pc = ctx.push_curve2(Curve2Kind::CircularArc2(
            CircularArc2::new(p2(0.0, 0.0), r, 0.0, TAU),
        ));
        let ce = add_coedge!(e_base, Orientation::Reverse, face_id, pc);
        ctx.get_mut_loop(loop_id).coedges.push(ce);
    }

    solid_id
}

// ── build_sphere ──────────────────────────────────────────────────────────────

/// Build the B-rep for a sphere of radius `r` centred at the origin.
///
/// The axis is +z (north pole at (0,0,r)); the seam is the prime meridian
/// (x-z half-plane, u = 0 / 2π).
///
/// Topology: 2 vertices, 3 edges, 4 coedges, 1 loop, 1 face, 1 shell, 1 solid.
/// Both pole edges are degenerate (v0==v1, constant 3D curve); each carries only
/// 1 coedge because the poles have no second adjacent face.
///
/// # UV map (Mercator-style)
///
// The UV rectangle of the SphericalSurface (u = longitude, v = latitude):
//
//   u=0                                    u=2π
//   │                                        │
//   v=+π/2 ╔══════════════════════════════════╗
//  (N pole) ║ E_north_deg  ←←←←←←←←←←←←←←  ║
//           ║                                  ║
// E_seam    ║                                  ║ E_seam
// (Rev) ↓   ║       SphericalSurface           ║  ↑ (Fwd)
//           ║                                  ║
//  (S pole) ║ E_south_deg  →→→→→→→→→→→→→→→  ║
//   v=-π/2  ╚══════════════════════════════════╝
//
// Loop (CCW in UV = outward-normal via right-hand rule, FaceSense::Aligned):
//   E_south_deg Fwd  (→, bottom)
//   E_seam      Fwd  (↑, right seam u=2π)
//   E_north_deg Fwd  (←, top)
//   E_seam      Rev  (↓, left seam u=0)
pub fn build_sphere(
    ctx: &mut SolidModelingContext,
    r: f64,
    prov_id: u64,
    geom_id: u64,
) -> SolidId {
    use std::f64::consts::{FRAC_PI_2, TAU};
    let tol = ctx.tolerance.pos_tol;
    let p3  = |x, y, z| Point3::new(x, y, z);
    let p2  = |u, v| Point2::new(u, v);

    // ── Vertices ──────────────────────────────────────────────────────────────
    let v_s = ctx.push_vertex(Vertex::new(p3(0.0, 0.0, -r), tol)); // south pole
    let v_n = ctx.push_vertex(Vertex::new(p3(0.0, 0.0,  r), tol)); // north pole

    // ── Curves3 ───────────────────────────────────────────────────────────────
    // E_seam: semicircle along the prime meridian (x-z half-plane).
    //   normal = -(axis × ref_dir) = -(0,0,1)×(1,0,0) = -(0,1,0) = (0,-1,0)
    //   e2 = normal × ref_dir = (0,-1,0)×(1,0,0) = (0,0,1) = axis
    //   eval(t) = (r·cos(t), 0, r·sin(t))  →  V_S at t=−π/2, V_N at t=+π/2
    let c_seam      = ctx.push_curve3(Curve3Kind::CircularArc3(
        CircularArc3::new(p3(0.0,0.0,0.0), p3(0.0,-1.0,0.0), p3(1.0,0.0,0.0), r, -FRAC_PI_2, FRAC_PI_2),
    ));
    // E_south_deg / E_north_deg: degenerate points at the poles, t ∈ [0, 1].
    let c_south_deg = ctx.push_curve3(Curve3Kind::Line3(
        Line3::new(p3(0.0, 0.0, -r), p3(0.0, 0.0, -r)),
    ));
    let c_north_deg = ctx.push_curve3(Curve3Kind::Line3(
        Line3::new(p3(0.0, 0.0,  r), p3(0.0, 0.0,  r)),
    ));

    // ── Edges ─────────────────────────────────────────────────────────────────
    let e_seam      = ctx.push_edge(Edge::new(c_seam,      v_s, v_n, -FRAC_PI_2, FRAC_PI_2));
    let e_south_deg = ctx.push_edge(Edge::new(c_south_deg, v_s, v_s, 0.0, 1.0));
    let e_north_deg = ctx.push_edge(Edge::new(c_north_deg, v_n, v_n, 0.0, 1.0));

    // ── Topology skeleton ─────────────────────────────────────────────────────
    let solid_id = ctx.push_solid(Solid::new(crate::brep_kernel::ShellId(usize::MAX)));
    let shell_id = ctx.push_shell(Shell::new(solid_id, true));
    ctx.get_mut_solid(solid_id).outer = shell_id;

    let prov = || ProvenanceData::primitive(prov_id, geom_id);

    macro_rules! add_coedge {
        ($edge:expr, $orient:expr, $face:expr, $pcurve:expr) => {{
            let ce_id = ctx.push_coedge(CoEdge::new($edge, $orient, $face, $pcurve));
            ctx.get_mut_edge($edge).coedges.push(ce_id);
            ce_id
        }};
    }

    // ── Face 0: full sphere (SphericalSurface, FaceSense::Aligned) ────────────
    //
    // SphericalSurface: center=origin, r, ref_dir=(1,0,0), axis=(0,0,1)
    //   e2 = axis × ref_dir = (0,1,0)
    //   eval(u,v) = r·(cos(v)·cos(u),  cos(v)·sin(u),  sin(v))
    //
    // PCurve convention — Line2.eval(t) = p0 + (p1−p0)·t:
    //   E_south_deg (t∈[0,1]): p0=(0,−π/2),  p1=(2π,−π/2) → (2π·t, −π/2)
    //   E_seam  Fwd (t∈[−π/2,+π/2]): p0=(2π,0), p1=(2π,1)  → (2π,  t)  ← right seam
    //   E_north_deg (t∈[0,1]): p0=(2π,+π/2), p1=(0, +π/2)  → (2π·(1−t), +π/2)
    //   E_seam  Rev (t∈[−π/2,+π/2]): p0=(0, 0), p1=(0, 1)  → (0,   t)  ← left seam
    {
        let sphere = SphericalSurface::new(
            p3(0.0, 0.0, 0.0), r, p3(1.0, 0.0, 0.0), p3(0.0, 0.0, 1.0),
        );
        let surf_id = ctx.push_surface(SurfaceKind::Sphere(sphere));
        let face_id = ctx.push_face(Face::new(
            shell_id, surf_id, LoopId(usize::MAX), FaceSense::Aligned, prov(),
        ));
        let loop_id = ctx.push_loop(Loop::new(face_id, true));
        ctx.get_mut_face(face_id).outer = loop_id;
        ctx.get_mut_shell(shell_id).faces.push(face_id);

        let pc_south = ctx.push_curve2(Curve2Kind::Line2(Line2::new(
            p2(0.0, -FRAC_PI_2), p2(TAU, -FRAC_PI_2),
        )));
        let pc_seam_rgt = ctx.push_curve2(Curve2Kind::Line2(Line2::new(
            p2(TAU, 0.0), p2(TAU, 1.0),
        )));
        let pc_north = ctx.push_curve2(Curve2Kind::Line2(Line2::new(
            p2(TAU, FRAC_PI_2), p2(0.0, FRAC_PI_2),
        )));
        let pc_seam_lft = ctx.push_curve2(Curve2Kind::Line2(Line2::new(
            p2(0.0, 0.0), p2(0.0, 1.0),
        )));

        let ce_s  = add_coedge!(e_south_deg, Orientation::Forward, face_id, pc_south);
        let ce_sr = add_coedge!(e_seam,      Orientation::Forward, face_id, pc_seam_rgt);
        let ce_n  = add_coedge!(e_north_deg, Orientation::Forward, face_id, pc_north);
        let ce_sl = add_coedge!(e_seam,      Orientation::Reverse, face_id, pc_seam_lft);
        ctx.get_mut_loop(loop_id).coedges.extend([ce_s, ce_sr, ce_n, ce_sl]);
    }

    solid_id
}

// ── compile_primitive ─────────────────────────────────────────────────────────

/// Dispatch a [`CsgPrimitive`] to the appropriate `build_*` function and absorb
/// the affine `transform` into the resulting B-rep geometry.
///
/// `transform` is a row-major column-vector 4×4 affine matrix (the same layout
/// as `CsgNode::flat_transform`).  The identity matrix leaves the B-rep unchanged.
///
/// **Absorption rules:**
/// - Vertices: always transformed as points (homogeneous w = 1).
/// - `Line3` edges: p0 and p1 transformed as points; parameter domain unchanged.
/// - `CircularArc3` edges: requires the transform to be isotropic (uniform scale ×
///   rotation). center transformed as a point; ref_dir / normal transformed as vectors
///   and re-normalised; radius scaled by `s`.
/// - `Plane` surfaces: p0 as point, u_dir / v_dir as vectors (scaling absorbed into
///   the direction vectors — pcurves on plane faces need no update).
/// - `CylindricalSurface` / `ConicalSurface`: isotropic required. origin / apex as
///   points; axis / ref_dir as vectors (re-normalised); radius scaled by `s`. Pcurves
///   on the lateral face have their v-coordinates (world-space axial or slant distance)
///   scaled by `s` via topology traversal.
/// - `SphericalSurface`: isotropic required. center as point; ref_dir / axis as
///   vectors (re-normalised); radius scaled by `s`. Pcurves unchanged (angles).
///
/// Non-isotropic transforms on curved surfaces will panic with `todo!()` until the
/// NURBS fallback is implemented.
///
/// `prov_id` and `geom_id` are forwarded unchanged to the builder and stored in every
/// face's [`ProvenanceData`].
pub fn compile_primitive(
    ctx: &mut SolidModelingContext,
    prim: &crate::csg_lang::CsgPrimitive,
    transform: &[f64; 16],
    prov_id: u64,
    geom_id: u64,
) -> SolidId {
    use crate::csg_lang::CsgPrimitive;

    // Snapshot arena lengths so we know which entries belong to this build.
    let v_start  = ctx.vertices.len();
    let c3_start = ctx.curves3.len();
    let c2_start = ctx.curves2.len();
    let s_start  = ctx.surfaces.len();

    let solid_id = match prim {
        CsgPrimitive::Cuboid { dx, dy, dz } => build_cuboid(ctx, *dx, *dy, *dz, prov_id, geom_id),
        CsgPrimitive::Cylinder { r, h }     => build_cylinder(ctx, *r, *h, prov_id, geom_id),
        CsgPrimitive::Cone { r, h }         => build_cone(ctx, *r, *h, prov_id, geom_id),
        CsgPrimitive::Sphere { r }          => build_sphere(ctx, *r, prov_id, geom_id),
    };

    // Skip the walk when the transform is the identity.
    if is_identity(transform) {
        return solid_id;
    }

    // ── Extract linear part and translation ───────────────────────────────────
    // Row-major layout: row i, col j → index i*4 + j.
    // Linear part M (3×3) is the top-left block; translation d is column 3, rows 0-2.
    let m = |r: usize, c: usize| transform[r * 4 + c];
    let d = Point3::new(m(0, 3), m(1, 3), m(2, 3));

    // Apply M to a vector (w=0): only the linear part.
    let apply_vec = |v: Point3| Point3::new(
        m(0,0)*v.x + m(0,1)*v.y + m(0,2)*v.z,
        m(1,0)*v.x + m(1,1)*v.y + m(1,2)*v.z,
        m(2,0)*v.x + m(2,1)*v.y + m(2,2)*v.z,
    );
    // Apply the full 4×4 to a point (w=1): linear part + translation.
    let apply_pt = |p: Point3| apply_vec(p) + d;

    // Isotropic check: Mᵀ·M = s²·I.  Returns scale factor s, or panics.
    let isotropic_scale = || -> f64 {
        // Compute the three diagonal entries of Mᵀ·M and the three off-diagonal entries.
        let btb = |r: usize, c: usize| -> f64 {
            (0..3).map(|k| m(k, r) * m(k, c)).sum()
        };
        let s2 = btb(0, 0);
        let eps = 1e-9 * s2.abs().max(1.0);
        assert!(
            (btb(1, 1) - s2).abs() < eps && (btb(2, 2) - s2).abs() < eps
            && btb(0, 1).abs() < eps && btb(0, 2).abs() < eps && btb(1, 2).abs() < eps,
            "non-isotropic transform on curved primitive: NURBS fallback not yet implemented"
        );
        s2.sqrt()
    };

    // ── Transform vertices ────────────────────────────────────────────────────
    for v in &mut ctx.vertices[v_start..] {
        v.point = apply_pt(v.point);
    }

    // ── Transform Curve3 entities ─────────────────────────────────────────────
    for c3 in &mut ctx.curves3[c3_start..] {
        match c3 {
            Curve3Kind::Line3(l) => {
                l.p0 = apply_pt(l.p0);
                l.p1 = apply_pt(l.p1);
            }
            Curve3Kind::CircularArc3(a) => {
                let s = isotropic_scale();
                a.center  = apply_pt(a.center);
                a.ref_dir = apply_vec(a.ref_dir).normalize();
                a.normal  = apply_vec(a.normal).normalize();
                a.radius *= s;
            }
            Curve3Kind::Nurbs(_) | Curve3Kind::Ssi(_) => {
                todo!("transform absorption for NurbsCurve3 / SsiCurve3 not yet implemented")
            }
        }
    }

    // ── Transform Surface entities ────────────────────────────────────────────
    // For Cylindrical and Conical surfaces, we also need to scale the v-coordinates
    // of pcurves on that face (v is a world-space distance in those parameterisations).
    // We do the pcurve update after updating the surface so we can retrieve s once.
    for surf_idx in s_start..ctx.surfaces.len() {
        match ctx.surfaces[surf_idx] {
            SurfaceKind::Plane(ref mut pl) => {
                pl.p0    = apply_pt(pl.p0);
                pl.u_dir = apply_vec(pl.u_dir);
                pl.v_dir = apply_vec(pl.v_dir);
            }
            SurfaceKind::Cylinder(ref mut cy) => {
                let s = isotropic_scale();
                cy.origin  = apply_pt(cy.origin);
                cy.axis    = apply_vec(cy.axis).normalize();
                cy.ref_dir = apply_vec(cy.ref_dir).normalize();
                cy.radius *= s;
                // Scale v-coordinates of lateral-face pcurves.
                scale_lateral_pcurves(ctx, surf_idx, s, c2_start);
            }
            SurfaceKind::Cone(ref mut co) => {
                let s = isotropic_scale();
                co.apex    = apply_pt(co.apex);
                co.axis    = apply_vec(co.axis).normalize();
                co.ref_dir = apply_vec(co.ref_dir).normalize();
                // half_angle is scale-invariant (r/h ratio unchanged).
                scale_lateral_pcurves(ctx, surf_idx, s, c2_start);
            }
            SurfaceKind::Sphere(ref mut sp) => {
                let s = isotropic_scale();
                sp.center  = apply_pt(sp.center);
                sp.ref_dir = apply_vec(sp.ref_dir).normalize();
                sp.axis    = apply_vec(sp.axis).normalize();
                sp.radius *= s;
                // SphericalSurface u,v are angles — pcurves unchanged.
            }
            SurfaceKind::Nurbs(_) => {
                todo!("transform absorption for NurbsSurf not yet implemented")
            }
        }
    }

    solid_id
}

/// Scale the v-coordinate of every pcurve on the face backed by `surf_idx` by `s`.
///
/// Used for `CylindricalSurface` and `ConicalSurface` where `v` is a world-space
/// distance (axial or slant) and must scale with the isotropic factor.
/// Pcurves outside the freshly-built range `[c2_start, …)` are not touched.
fn scale_lateral_pcurves(
    ctx: &mut SolidModelingContext,
    surf_idx: usize,
    s: f64,
    c2_start: usize,
) {
    use crate::brep_kernel::{SurfaceId, CoEdgeId};
    // Find the face that owns this surface.
    let surf_id = SurfaceId(surf_idx);
    let face_id = match ctx.faces.iter().position(|f| f.surface == surf_id) {
        Some(i) => crate::brep_kernel::FaceId(i),
        None    => return,
    };
    // Collect all CoEdge IDs in the face's outer loop.
    let loop_id = ctx.get_face(face_id).outer;
    let coedge_ids: Vec<CoEdgeId> = ctx.get_loop(loop_id).coedges.clone();
    // Scale v-coords of each pcurve that lives in the freshly-built range.
    for ce_id in coedge_ids {
        let pc_id = ctx.get_coedge(ce_id).pcurve;
        if pc_id.0 < c2_start {
            continue;
        }
        match &mut ctx.curves2[pc_id.0] {
            Curve2Kind::Line2(l) => {
                l.p0.v *= s;
                l.p1.v *= s;
            }
            Curve2Kind::CircularArc2(a) => {
                a.center.v *= s;
            }
            Curve2Kind::Nurbs(_) => {
                todo!("pcurve v-scaling for NurbsCurve2 not yet implemented")
            }
        }
    }
}

/// Returns `true` if `transform` is the 4×4 identity matrix (within 1e-12).
fn is_identity(transform: &[f64; 16]) -> bool {
    #[rustfmt::skip]
    const ID: [f64; 16] = [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ];
    transform.iter().zip(ID.iter()).all(|(a, b)| (a - b).abs() < 1e-12)
}

// ── compile_csg_node ──────────────────────────────────────────────────────────

/// Compile a [`CsgNode`] into a B-rep solid, absorbing the node's `flat_transform`
/// into the resulting geometry.
///
/// For primitive leaf nodes the call is forwarded to [`compile_primitive`] with the
/// node's `flat_transform`, `prov_id`, and `geom_id`.  Boolean operation nodes
/// (`Op`) are not yet supported and will panic with `todo!()`.
pub fn compile_csg_node(
    ctx: &mut SolidModelingContext,
    node: &crate::csg_lang::CsgNode,
) -> SolidId {
    use crate::csg_lang::CsgBaseNode;
    match &node.base {
        CsgBaseNode::Prim(prim) => {
            compile_primitive(ctx, prim, &node.flat_transform, node.prov_id, node.geom_id)
        }
        CsgBaseNode::Op(_) => {
            todo!("boolean CSG operations not yet supported in compile_csg_node")
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    use super::*;
    use crate::brep_kernel::{Orientation, SolidId};
    use crate::geom::Curve3;

    fn std_cuboid() -> (SolidModelingContext, SolidId) {
        let mut ctx = SolidModelingContext::new();
        let sid = build_cuboid(&mut ctx, 2.0, 3.0, 4.0, 11, 22);
        (ctx, sid)
    }

    // ── Entity counts ─────────────────────────────────────────────────────────

    #[test]
    fn cuboid_vertex_count() {
        let (ctx, _) = std_cuboid();
        assert_eq!(ctx.vertices.len(), 8);
    }

    #[test]
    fn cuboid_edge_count() {
        let (ctx, _) = std_cuboid();
        assert_eq!(ctx.edges.len(), 12);
    }

    #[test]
    fn cuboid_coedge_count() {
        let (ctx, _) = std_cuboid();
        assert_eq!(ctx.coedges.len(), 24);
    }

    #[test]
    fn cuboid_loop_count() {
        let (ctx, _) = std_cuboid();
        assert_eq!(ctx.loops.len(), 6);
    }

    #[test]
    fn cuboid_face_count() {
        let (ctx, _) = std_cuboid();
        assert_eq!(ctx.faces.len(), 6);
    }

    #[test]
    fn cuboid_shell_and_solid_count() {
        let (ctx, _) = std_cuboid();
        assert_eq!(ctx.shells.len(), 1);
        assert_eq!(ctx.solids.len(), 1);
    }

    // ── Vertex positions ──────────────────────────────────────────────────────

    #[test]
    fn cuboid_vertex_positions() {
        let (ctx, _) = std_cuboid();
        let pts: Vec<Point3> = ctx.vertices.iter().map(|v| v.point).collect();
        // Corner at origin; opposite at (2,3,4)
        assert!(pts.contains(&Point3::new(0.0, 0.0, 0.0)));
        assert!(pts.contains(&Point3::new(2.0, 0.0, 0.0)));
        assert!(pts.contains(&Point3::new(2.0, 3.0, 0.0)));
        assert!(pts.contains(&Point3::new(0.0, 3.0, 0.0)));
        assert!(pts.contains(&Point3::new(0.0, 0.0, 4.0)));
        assert!(pts.contains(&Point3::new(2.0, 0.0, 4.0)));
        assert!(pts.contains(&Point3::new(2.0, 3.0, 4.0)));
        assert!(pts.contains(&Point3::new(0.0, 3.0, 4.0)));
    }

    // ── Topology consistency ──────────────────────────────────────────────────

    #[test]
    fn cuboid_each_edge_has_exactly_two_coedges() {
        let (ctx, _) = std_cuboid();
        for edge in &ctx.edges {
            assert_eq!(edge.coedges.len(), 2,
                "every manifold edge must have exactly 2 coedges");
        }
    }

    #[test]
    fn cuboid_each_edge_one_forward_one_reverse() {
        let (ctx, _) = std_cuboid();
        for edge in &ctx.edges {
            let fwd = edge.coedges.iter()
                .filter(|&&ce| ctx.get_coedge(ce).orientation == Orientation::Forward)
                .count();
            let rev = edge.coedges.iter()
                .filter(|&&ce| ctx.get_coedge(ce).orientation == Orientation::Reverse)
                .count();
            assert_eq!(fwd, 1, "each edge must have exactly one Forward coedge");
            assert_eq!(rev, 1, "each edge must have exactly one Reverse coedge");
        }
    }

    #[test]
    fn cuboid_each_face_has_outer_loop_with_four_coedges() {
        let (ctx, _) = std_cuboid();
        for face in &ctx.faces {
            let lp = ctx.get_loop(face.outer);
            assert!(lp.is_outer);
            assert_eq!(lp.coedges.len(), 4,
                "each cuboid face loop must have exactly 4 coedges");
        }
    }

    #[test]
    fn cuboid_all_faces_aligned() {
        let (ctx, _) = std_cuboid();
        for face in &ctx.faces {
            assert_eq!(face.sense, FaceSense::Aligned,
                "all cuboid faces should use FaceSense::Aligned");
        }
    }

    #[test]
    fn cuboid_shell_owns_six_faces() {
        let (ctx, sid) = std_cuboid();
        let shell = ctx.get_shell(ctx.get_solid(sid).outer);
        assert_eq!(shell.faces.len(), 6);
    }

    #[test]
    fn cuboid_solid_outer_shell_is_outer() {
        let (ctx, sid) = std_cuboid();
        let shell = ctx.get_shell(ctx.get_solid(sid).outer);
        assert!(shell.is_outer);
    }

    // ── Provenance ────────────────────────────────────────────────────────────

    #[test]
    fn cuboid_face_provenance() {
        let (ctx, _) = std_cuboid();
        for face in &ctx.faces {
            assert_eq!(face.prov.sources.len(), 1);
            assert_eq!(face.prov.sources[0].prov_id, 11);
            assert_eq!(face.prov.sources[0].geom_id, 22);
            assert_eq!(face.prov.last_op, None);
        }
    }

    // ── Loop connectivity: consecutive coedges share a vertex ─────────────────

    #[test]
    fn cuboid_loop_coedges_form_closed_chain() {
        let (ctx, _) = std_cuboid();
        for lp in &ctx.loops {
            let n = lp.coedges.len();
            for i in 0..n {
                let ce_cur  = ctx.get_coedge(lp.coedges[i]);
                let ce_next = ctx.get_coedge(lp.coedges[(i + 1) % n]);
                // end vertex of current coedge
                let end_cur = match ce_cur.orientation {
                    Orientation::Forward => ctx.get_edge(ce_cur.edge).v1,
                    Orientation::Reverse => ctx.get_edge(ce_cur.edge).v0,
                };
                // start vertex of next coedge
                let start_next = match ce_next.orientation {
                    Orientation::Forward => ctx.get_edge(ce_next.edge).v0,
                    Orientation::Reverse => ctx.get_edge(ce_next.edge).v1,
                };
                assert_eq!(end_cur, start_next,
                    "coedge {} end must equal coedge {} start in loop", i, (i+1)%n);
            }
        }
    }

    // ── build_cylinder ────────────────────────────────────────────────────────

    fn std_cylinder() -> (SolidModelingContext, SolidId) {
        let mut ctx = SolidModelingContext::new();
        let sid = build_cylinder(&mut ctx, 3.0, 5.0, 7, 13);
        (ctx, sid)
    }

    // Entity counts

    #[test]
    fn cylinder_vertex_count() {
        let (ctx, _) = std_cylinder();
        assert_eq!(ctx.vertices.len(), 2);
    }

    #[test]
    fn cylinder_edge_count() {
        let (ctx, _) = std_cylinder();
        assert_eq!(ctx.edges.len(), 3);
    }

    #[test]
    fn cylinder_coedge_count() {
        let (ctx, _) = std_cylinder();
        assert_eq!(ctx.coedges.len(), 6);
    }

    #[test]
    fn cylinder_loop_count() {
        let (ctx, _) = std_cylinder();
        assert_eq!(ctx.loops.len(), 3);
    }

    #[test]
    fn cylinder_face_count() {
        let (ctx, _) = std_cylinder();
        assert_eq!(ctx.faces.len(), 3);
    }

    #[test]
    fn cylinder_shell_and_solid_count() {
        let (ctx, _) = std_cylinder();
        assert_eq!(ctx.shells.len(), 1);
        assert_eq!(ctx.solids.len(), 1);
    }

    // Vertex positions

    #[test]
    fn cylinder_vertex_positions() {
        let (ctx, _) = std_cylinder();
        let pts: Vec<Point3> = ctx.vertices.iter().map(|v| v.point).collect();
        assert!(pts.contains(&Point3::new(3.0, 0.0, 0.0)), "V_bot missing");
        assert!(pts.contains(&Point3::new(3.0, 0.0, 5.0)), "V_top missing");
    }

    // Closed vs open edges

    #[test]
    fn cylinder_base_and_top_edges_are_closed() {
        let (ctx, _) = std_cylinder();
        // E_base (index 0) and E_top (index 1) are closed circles: v0 == v1
        let e_base = &ctx.edges[0];
        let e_top  = &ctx.edges[1];
        assert_eq!(e_base.v0, e_base.v1, "E_base should be a closed circle");
        assert_eq!(e_top.v0,  e_top.v1,  "E_top should be a closed circle");
    }

    #[test]
    fn cylinder_seam_edge_is_open() {
        let (ctx, _) = std_cylinder();
        // E_seam (index 2) connects V_bot to V_top
        let e_seam = &ctx.edges[2];
        assert_ne!(e_seam.v0, e_seam.v1, "E_seam should connect two distinct vertices");
    }

    // Edge coedge invariants

    #[test]
    fn cylinder_each_edge_has_exactly_two_coedges() {
        let (ctx, _) = std_cylinder();
        for edge in &ctx.edges {
            assert_eq!(edge.coedges.len(), 2);
        }
    }

    #[test]
    fn cylinder_each_edge_one_forward_one_reverse() {
        let (ctx, _) = std_cylinder();
        for edge in &ctx.edges {
            let fwd = edge.coedges.iter()
                .filter(|&&ce| ctx.get_coedge(ce).orientation == Orientation::Forward)
                .count();
            let rev = edge.coedges.iter()
                .filter(|&&ce| ctx.get_coedge(ce).orientation == Orientation::Reverse)
                .count();
            assert_eq!(fwd, 1);
            assert_eq!(rev, 1);
        }
    }

    // Face senses

    #[test]
    fn cylinder_face_senses() {
        let (ctx, _) = std_cylinder();
        // push order: lateral(0), base(1), top(2)
        assert_eq!(ctx.faces[0].sense, FaceSense::Aligned,      "lateral must be Aligned");
        assert_eq!(ctx.faces[1].sense, FaceSense::AntiAligned,  "base must be AntiAligned");
        assert_eq!(ctx.faces[2].sense, FaceSense::Aligned,      "top must be Aligned");
    }

    // Loop coedge counts

    #[test]
    fn cylinder_lateral_loop_has_four_coedges() {
        let (ctx, _) = std_cylinder();
        let lateral_face = &ctx.faces[0];
        let lp = ctx.get_loop(lateral_face.outer);
        assert_eq!(lp.coedges.len(), 4);
    }

    #[test]
    fn cylinder_cap_loops_have_one_coedge_each() {
        let (ctx, _) = std_cylinder();
        for face in &ctx.faces[1..] {
            let lp = ctx.get_loop(face.outer);
            assert_eq!(lp.coedges.len(), 1, "each cap loop must have exactly 1 coedge");
        }
    }

    // Loop chain closure (reuse the same logic as cuboid test)

    #[test]
    fn cylinder_loop_coedges_form_closed_chain() {
        let (ctx, _) = std_cylinder();
        for lp in &ctx.loops {
            let n = lp.coedges.len();
            for i in 0..n {
                let ce_cur  = ctx.get_coedge(lp.coedges[i]);
                let ce_next = ctx.get_coedge(lp.coedges[(i + 1) % n]);
                let end_cur = match ce_cur.orientation {
                    Orientation::Forward => ctx.get_edge(ce_cur.edge).v1,
                    Orientation::Reverse => ctx.get_edge(ce_cur.edge).v0,
                };
                let start_next = match ce_next.orientation {
                    Orientation::Forward => ctx.get_edge(ce_next.edge).v0,
                    Orientation::Reverse => ctx.get_edge(ce_next.edge).v1,
                };
                assert_eq!(end_cur, start_next,
                    "coedge chain broken at position {} in loop", i);
            }
        }
    }

    // Provenance

    #[test]
    fn cylinder_face_provenance() {
        let (ctx, _) = std_cylinder();
        for face in &ctx.faces {
            assert_eq!(face.prov.sources.len(), 1);
            assert_eq!(face.prov.sources[0].prov_id, 7);
            assert_eq!(face.prov.sources[0].geom_id, 13);
            assert_eq!(face.prov.last_op, None);
        }
    }

    // ── build_cone ────────────────────────────────────────────────────────────

    fn std_cone() -> (SolidModelingContext, SolidId) {
        let mut ctx = SolidModelingContext::new();
        let sid = build_cone(&mut ctx, 3.0, 4.0, 5, 6);
        (ctx, sid)
    }

    // Entity counts

    #[test]
    fn cone_vertex_count() {
        let (ctx, _) = std_cone();
        assert_eq!(ctx.vertices.len(), 2);
    }

    #[test]
    fn cone_edge_count() {
        let (ctx, _) = std_cone();
        assert_eq!(ctx.edges.len(), 3);
    }

    #[test]
    fn cone_coedge_count() {
        // 4 in lateral loop + 1 in base cap = 5
        let (ctx, _) = std_cone();
        assert_eq!(ctx.coedges.len(), 5);
    }

    #[test]
    fn cone_loop_count() {
        let (ctx, _) = std_cone();
        assert_eq!(ctx.loops.len(), 2);
    }

    #[test]
    fn cone_face_count() {
        let (ctx, _) = std_cone();
        assert_eq!(ctx.faces.len(), 2);
    }

    #[test]
    fn cone_shell_and_solid_count() {
        let (ctx, _) = std_cone();
        assert_eq!(ctx.shells.len(), 1);
        assert_eq!(ctx.solids.len(), 1);
    }

    // Vertex positions

    #[test]
    fn cone_vertex_positions() {
        let (ctx, _) = std_cone();
        let pts: Vec<Point3> = ctx.vertices.iter().map(|v| v.point).collect();
        assert!(pts.contains(&Point3::new(0.0, 0.0, 4.0)), "V_apex missing");
        assert!(pts.contains(&Point3::new(3.0, 0.0, 0.0)), "V_base missing");
    }

    // Degenerate edge

    #[test]
    fn cone_apex_edge_is_degenerate() {
        let (ctx, _) = std_cone();
        // E_apex_deg is index 1
        let e = &ctx.edges[1];
        assert_eq!(e.v0, e.v1, "apex edge must be closed (v0==v1)");
        let curve = ctx.get_curve3(e.curve3);
        assert!(curve.is_degenerate(), "apex curve must be degenerate");
    }

    #[test]
    fn cone_base_edge_is_closed_nondegenerate() {
        let (ctx, _) = std_cone();
        let e = &ctx.edges[0]; // E_base
        assert_eq!(e.v0, e.v1, "base edge must be closed");
        let curve = ctx.get_curve3(e.curve3);
        assert!(!curve.is_degenerate(), "base circle must not be degenerate");
    }

    #[test]
    fn cone_seam_edge_is_open() {
        let (ctx, _) = std_cone();
        let e = &ctx.edges[2]; // E_seam
        assert_ne!(e.v0, e.v1, "seam must connect two distinct vertices");
    }

    // Degenerate edge has only 1 coedge (no second face at apex)

    #[test]
    fn cone_apex_edge_has_one_coedge() {
        let (ctx, _) = std_cone();
        assert_eq!(ctx.edges[1].coedges.len(), 1,
            "degenerate apex edge has no second face, so only 1 coedge");
    }

    // Non-degenerate edges each have 2 coedges (one Fwd, one Rev)

    #[test]
    fn cone_non_degenerate_edges_have_two_coedges() {
        let (ctx, _) = std_cone();
        for (i, edge) in ctx.edges.iter().enumerate() {
            if edge.v0 == edge.v1 && ctx.get_curve3(edge.curve3).is_degenerate() {
                continue; // skip apex degenerate
            }
            assert_eq!(edge.coedges.len(), 2,
                "edge {} should have exactly 2 coedges", i);
        }
    }

    #[test]
    fn cone_non_degenerate_edges_one_forward_one_reverse() {
        let (ctx, _) = std_cone();
        for edge in &ctx.edges {
            if edge.v0 == edge.v1 && ctx.get_curve3(edge.curve3).is_degenerate() {
                continue;
            }
            let fwd = edge.coedges.iter()
                .filter(|&&ce| ctx.get_coedge(ce).orientation == Orientation::Forward)
                .count();
            let rev = edge.coedges.iter()
                .filter(|&&ce| ctx.get_coedge(ce).orientation == Orientation::Reverse)
                .count();
            assert_eq!(fwd, 1);
            assert_eq!(rev, 1);
        }
    }

    // Face senses

    #[test]
    fn cone_face_senses() {
        let (ctx, _) = std_cone();
        assert_eq!(ctx.faces[0].sense, FaceSense::Aligned,     "lateral must be Aligned");
        assert_eq!(ctx.faces[1].sense, FaceSense::AntiAligned, "base must be AntiAligned");
    }

    // Loop coedge counts

    #[test]
    fn cone_lateral_loop_has_four_coedges() {
        let (ctx, _) = std_cone();
        let lp = ctx.get_loop(ctx.faces[0].outer);
        assert_eq!(lp.coedges.len(), 4);
    }

    #[test]
    fn cone_base_loop_has_one_coedge() {
        let (ctx, _) = std_cone();
        let lp = ctx.get_loop(ctx.faces[1].outer);
        assert_eq!(lp.coedges.len(), 1);
    }

    // Loop chain closure

    #[test]
    fn cone_loop_coedges_form_closed_chain() {
        let (ctx, _) = std_cone();
        for lp in &ctx.loops {
            let n = lp.coedges.len();
            for i in 0..n {
                let ce_cur  = ctx.get_coedge(lp.coedges[i]);
                let ce_next = ctx.get_coedge(lp.coedges[(i + 1) % n]);
                let end_cur = match ce_cur.orientation {
                    Orientation::Forward => ctx.get_edge(ce_cur.edge).v1,
                    Orientation::Reverse => ctx.get_edge(ce_cur.edge).v0,
                };
                let start_next = match ce_next.orientation {
                    Orientation::Forward => ctx.get_edge(ce_next.edge).v0,
                    Orientation::Reverse => ctx.get_edge(ce_next.edge).v1,
                };
                assert_eq!(end_cur, start_next,
                    "coedge chain broken at position {} in loop", i);
            }
        }
    }

    // Provenance

    #[test]
    fn cone_face_provenance() {
        let (ctx, _) = std_cone();
        for face in &ctx.faces {
            assert_eq!(face.prov.sources.len(), 1);
            assert_eq!(face.prov.sources[0].prov_id, 5);
            assert_eq!(face.prov.sources[0].geom_id, 6);
            assert_eq!(face.prov.last_op, None);
        }
    }

    // ── build_sphere ──────────────────────────────────────────────────────────

    fn std_sphere() -> (SolidModelingContext, SolidId) {
        let mut ctx = SolidModelingContext::new();
        let sid = build_sphere(&mut ctx, 5.0, 3, 9);
        (ctx, sid)
    }

    // Entity counts

    #[test]
    fn sphere_vertex_count() {
        let (ctx, _) = std_sphere();
        assert_eq!(ctx.vertices.len(), 2);
    }

    #[test]
    fn sphere_edge_count() {
        let (ctx, _) = std_sphere();
        assert_eq!(ctx.edges.len(), 3);
    }

    #[test]
    fn sphere_coedge_count() {
        // 4 coedges in the single loop: south_deg, seam_fwd, north_deg, seam_rev
        let (ctx, _) = std_sphere();
        assert_eq!(ctx.coedges.len(), 4);
    }

    #[test]
    fn sphere_loop_count() {
        let (ctx, _) = std_sphere();
        assert_eq!(ctx.loops.len(), 1);
    }

    #[test]
    fn sphere_face_count() {
        let (ctx, _) = std_sphere();
        assert_eq!(ctx.faces.len(), 1);
    }

    #[test]
    fn sphere_shell_and_solid_count() {
        let (ctx, _) = std_sphere();
        assert_eq!(ctx.shells.len(), 1);
        assert_eq!(ctx.solids.len(), 1);
    }

    // Vertex positions

    #[test]
    fn sphere_vertex_positions() {
        let (ctx, _) = std_sphere();
        let pts: Vec<Point3> = ctx.vertices.iter().map(|v| v.point).collect();
        assert!(pts.contains(&Point3::new(0.0, 0.0, -5.0)), "V_S missing");
        assert!(pts.contains(&Point3::new(0.0, 0.0,  5.0)), "V_N missing");
    }

    // Seam edge connects poles; pole edges are degenerate

    #[test]
    fn sphere_seam_connects_poles() {
        let (ctx, _) = std_sphere();
        let e_seam = &ctx.edges[0];
        let pts: Vec<Point3> = ctx.vertices.iter().map(|v| v.point).collect();
        let p0 = ctx.get_vertex(e_seam.v0).point;
        let p1 = ctx.get_vertex(e_seam.v1).point;
        assert_ne!(e_seam.v0, e_seam.v1, "seam must connect distinct poles");
        assert!(
            (p0 == Point3::new(0.0,0.0,-5.0) && p1 == Point3::new(0.0,0.0, 5.0)) ||
            (p0 == Point3::new(0.0,0.0, 5.0) && p1 == Point3::new(0.0,0.0,-5.0)),
            "seam endpoints must be the two poles; got {:?} and {:?}", p0, p1
        );
        let _ = pts; // suppress unused warning
    }

    #[test]
    fn sphere_pole_edges_are_degenerate() {
        let (ctx, _) = std_sphere();
        for edge in &ctx.edges[1..] { // E_south_deg and E_north_deg
            assert_eq!(edge.v0, edge.v1, "pole edge must be closed");
            assert!(ctx.get_curve3(edge.curve3).is_degenerate(),
                "pole curve must be degenerate");
        }
    }

    // E_seam has 2 coedges; pole edges each have 1 (no second face at poles)

    #[test]
    fn sphere_seam_has_two_coedges() {
        let (ctx, _) = std_sphere();
        assert_eq!(ctx.edges[0].coedges.len(), 2);
    }

    #[test]
    fn sphere_seam_one_forward_one_reverse() {
        let (ctx, _) = std_sphere();
        let edge = &ctx.edges[0];
        let fwd = edge.coedges.iter()
            .filter(|&&ce| ctx.get_coedge(ce).orientation == Orientation::Forward)
            .count();
        let rev = edge.coedges.iter()
            .filter(|&&ce| ctx.get_coedge(ce).orientation == Orientation::Reverse)
            .count();
        assert_eq!(fwd, 1);
        assert_eq!(rev, 1);
    }

    #[test]
    fn sphere_pole_edges_have_one_coedge_each() {
        let (ctx, _) = std_sphere();
        assert_eq!(ctx.edges[1].coedges.len(), 1, "south_deg must have 1 coedge");
        assert_eq!(ctx.edges[2].coedges.len(), 1, "north_deg must have 1 coedge");
    }

    // Face

    #[test]
    fn sphere_face_is_aligned() {
        let (ctx, _) = std_sphere();
        assert_eq!(ctx.faces[0].sense, FaceSense::Aligned);
    }

    #[test]
    fn sphere_loop_has_four_coedges() {
        let (ctx, _) = std_sphere();
        let lp = ctx.get_loop(ctx.faces[0].outer);
        assert_eq!(lp.coedges.len(), 4);
    }

    // Loop chain closure

    #[test]
    fn sphere_loop_coedges_form_closed_chain() {
        let (ctx, _) = std_sphere();
        for lp in &ctx.loops {
            let n = lp.coedges.len();
            for i in 0..n {
                let ce_cur  = ctx.get_coedge(lp.coedges[i]);
                let ce_next = ctx.get_coedge(lp.coedges[(i + 1) % n]);
                let end_cur = match ce_cur.orientation {
                    Orientation::Forward => ctx.get_edge(ce_cur.edge).v1,
                    Orientation::Reverse => ctx.get_edge(ce_cur.edge).v0,
                };
                let start_next = match ce_next.orientation {
                    Orientation::Forward => ctx.get_edge(ce_next.edge).v0,
                    Orientation::Reverse => ctx.get_edge(ce_next.edge).v1,
                };
                assert_eq!(end_cur, start_next,
                    "coedge chain broken at position {} in loop", i);
            }
        }
    }

    // Provenance

    #[test]
    fn sphere_face_provenance() {
        let (ctx, _) = std_sphere();
        let face = &ctx.faces[0];
        assert_eq!(face.prov.sources.len(), 1);
        assert_eq!(face.prov.sources[0].prov_id, 3);
        assert_eq!(face.prov.sources[0].geom_id, 9);
        assert_eq!(face.prov.last_op, None);
    }

    // ── compile_primitive ─────────────────────────────────────────────────────

    use crate::csg_lang::CsgPrimitive;
    use crate::geom::SurfaceKind;

    #[rustfmt::skip]
    const ID: [f64; 16] = [
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    ];

    fn approx(a: f64, b: f64) -> bool { (a - b).abs() < 1e-10 }
    fn pt_approx(p: Point3, x: f64, y: f64, z: f64) -> bool {
        approx(p.x, x) && approx(p.y, y) && approx(p.z, z)
    }

    /// Compile with identity transform, forwarding prov/geom ids.
    fn compile(prim: CsgPrimitive) -> (SolidModelingContext, SolidId) {
        let mut ctx = SolidModelingContext::new();
        let sid = compile_primitive(&mut ctx, &prim, &ID, 7, 42);
        (ctx, sid)
    }

    /// Compile with an explicit transform; prov/geom ids are zeroed.
    fn compile_with(prim: CsgPrimitive, transform: [f64; 16]) -> (SolidModelingContext, SolidId) {
        let mut ctx = SolidModelingContext::new();
        let sid = compile_primitive(&mut ctx, &prim, &transform, 0, 0);
        (ctx, sid)
    }

    // ── Dispatch / entity counts (regression) ─────────────────────────────────

    #[test]
    fn compile_cuboid_entity_counts() {
        let (ctx, _) = compile(CsgPrimitive::Cuboid { dx: 2.0, dy: 3.0, dz: 4.0 });
        assert_eq!(ctx.vertices.len(), 8);
        assert_eq!(ctx.edges.len(), 12);
        assert_eq!(ctx.coedges.len(), 24);
        assert_eq!(ctx.faces.len(), 6);
    }

    #[test]
    fn compile_cylinder_entity_counts() {
        let (ctx, _) = compile(CsgPrimitive::Cylinder { r: 1.0, h: 2.0 });
        assert_eq!(ctx.vertices.len(), 2);
        assert_eq!(ctx.edges.len(), 3);
        assert_eq!(ctx.coedges.len(), 6);
        assert_eq!(ctx.faces.len(), 3);
    }

    #[test]
    fn compile_cone_entity_counts() {
        let (ctx, _) = compile(CsgPrimitive::Cone { r: 1.0, h: 2.0 });
        assert_eq!(ctx.vertices.len(), 2);
        assert_eq!(ctx.edges.len(), 3);
        assert_eq!(ctx.coedges.len(), 5);
        assert_eq!(ctx.faces.len(), 2);
    }

    #[test]
    fn compile_sphere_entity_counts() {
        let (ctx, _) = compile(CsgPrimitive::Sphere { r: 1.0 });
        assert_eq!(ctx.vertices.len(), 2);
        assert_eq!(ctx.edges.len(), 3);
        assert_eq!(ctx.coedges.len(), 4);
        assert_eq!(ctx.faces.len(), 1);
    }

    #[test]
    fn compile_primitive_provenance_passthrough() {
        let (ctx, _) = compile(CsgPrimitive::Cuboid { dx: 1.0, dy: 1.0, dz: 1.0 });
        for face in &ctx.faces {
            assert_eq!(face.prov.sources[0].prov_id, 7);
            assert_eq!(face.prov.sources[0].geom_id, 42);
        }
    }

    // ── Translation ───────────────────────────────────────────────────────────

    #[test]
    fn compile_cuboid_translation() {
        #[rustfmt::skip]
        let t = [
            1.0, 0.0, 0.0, 1.0,
            0.0, 1.0, 0.0, 2.0,
            0.0, 0.0, 1.0, 3.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        let (ctx, _) = compile_with(CsgPrimitive::Cuboid { dx: 2.0, dy: 3.0, dz: 4.0 }, t);
        let pts: Vec<Point3> = ctx.vertices.iter().map(|v| v.point).collect();
        // (0,0,0) → (1,2,3);  (2,3,4) → (3,5,7)
        assert!(pts.iter().any(|p| pt_approx(*p, 1.0, 2.0, 3.0)));
        assert!(pts.iter().any(|p| pt_approx(*p, 3.0, 5.0, 7.0)));
    }

    #[test]
    fn compile_sphere_translation() {
        #[rustfmt::skip]
        let t = [
            1.0, 0.0, 0.0, 5.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        let (ctx, _) = compile_with(CsgPrimitive::Sphere { r: 1.0 }, t);
        let SurfaceKind::Sphere(s) = ctx.surfaces[0] else { panic!("expected Sphere") };
        assert!(pt_approx(s.center, 5.0, 0.0, 0.0), "center should be (5,0,0)");
        assert!(approx(s.radius, 1.0), "radius should be unchanged");
    }

    #[test]
    fn compile_cylinder_translation() {
        #[rustfmt::skip]
        let t = [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 5.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        let (ctx, _) = compile_with(CsgPrimitive::Cylinder { r: 1.0, h: 2.0 }, t);
        let pts: Vec<Point3> = ctx.vertices.iter().map(|v| v.point).collect();
        // (1,0,0) → (1,0,5);  (1,0,2) → (1,0,7)
        assert!(pts.iter().any(|p| pt_approx(*p, 1.0, 0.0, 5.0)));
        assert!(pts.iter().any(|p| pt_approx(*p, 1.0, 0.0, 7.0)));
        // Cylinder origin shifted
        let SurfaceKind::Cylinder(c) = ctx.surfaces[0] else { panic!("expected Cylinder") };
        assert!(pt_approx(c.origin, 0.0, 0.0, 5.0), "origin should be (0,0,5)");
    }

    // ── Uniform scale ─────────────────────────────────────────────────────────

    #[test]
    fn compile_cuboid_uniform_scale() {
        #[rustfmt::skip]
        let t = [
            2.0, 0.0, 0.0, 0.0,
            0.0, 2.0, 0.0, 0.0,
            0.0, 0.0, 2.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        let (ctx, _) = compile_with(CsgPrimitive::Cuboid { dx: 2.0, dy: 3.0, dz: 4.0 }, t);
        let pts: Vec<Point3> = ctx.vertices.iter().map(|v| v.point).collect();
        // (0,0,0) unchanged; (2,3,4) → (4,6,8)
        assert!(pts.iter().any(|p| pt_approx(*p, 0.0, 0.0, 0.0)));
        assert!(pts.iter().any(|p| pt_approx(*p, 4.0, 6.0, 8.0)));
    }

    #[test]
    fn compile_sphere_uniform_scale() {
        #[rustfmt::skip]
        let t = [
            2.0, 0.0, 0.0, 0.0,
            0.0, 2.0, 0.0, 0.0,
            0.0, 0.0, 2.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        let (ctx, _) = compile_with(CsgPrimitive::Sphere { r: 1.5 }, t);
        let SurfaceKind::Sphere(s) = ctx.surfaces[0] else { panic!("expected Sphere") };
        assert!(pt_approx(s.center, 0.0, 0.0, 0.0), "center should stay at origin");
        assert!(approx(s.radius, 3.0), "radius should be 2 * 1.5 = 3.0");
    }

    #[test]
    fn compile_cylinder_uniform_scale_geometry() {
        #[rustfmt::skip]
        let t = [
            2.0, 0.0, 0.0, 0.0,
            0.0, 2.0, 0.0, 0.0,
            0.0, 0.0, 2.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        let (ctx, _) = compile_with(CsgPrimitive::Cylinder { r: 1.5, h: 3.0 }, t);
        let pts: Vec<Point3> = ctx.vertices.iter().map(|v| v.point).collect();
        // (1.5,0,0) → (3,0,0);  (1.5,0,3) → (3,0,6)
        assert!(pts.iter().any(|p| pt_approx(*p, 3.0, 0.0, 0.0)));
        assert!(pts.iter().any(|p| pt_approx(*p, 3.0, 0.0, 6.0)));
        let SurfaceKind::Cylinder(c) = ctx.surfaces[0] else { panic!("expected Cylinder") };
        assert!(approx(c.radius, 3.0), "radius should be 2 * 1.5 = 3.0");
        assert!(pt_approx(c.axis, 0.0, 0.0, 1.0), "axis should remain (0,0,1)");
    }

    #[test]
    fn compile_cylinder_uniform_scale_pcurves() {
        // Lateral-face pcurves that carry a v-coordinate (axial distance) must scale.
        // Plane-face pcurves (CircularArc2 on caps) must NOT change.
        #[rustfmt::skip]
        let t = [
            2.0, 0.0, 0.0, 0.0,
            0.0, 2.0, 0.0, 0.0,
            0.0, 0.0, 2.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        let (ctx, _) = compile_with(CsgPrimitive::Cylinder { r: 1.5, h: 3.0 }, t);
        // Curve2 push order in build_cylinder (lateral face first):
        //   [0] pc_base_lat  Line2(p0=(0,0), p1=(1,0))     v=0 — unchanged
        //   [1] pc_seam_rgt  Line2(p0=(TAU,0), p1=(TAU,h)) v endpoint → 2h
        //   [2] pc_top_lat   Line2(p0=(0,h), p1=(1,h))     v → 2h
        //   [3] pc_seam_lft  Line2(p0=(0,0), p1=(0,h))     v endpoint → 2h
        //   [4] pc_base_cap  CircularArc2 r=1.5 on Plane    — unchanged
        //   [5] pc_top_cap   CircularArc2 r=1.5 on Plane    — unchanged
        let scaled_h = 6.0_f64; // 2 * h
        let Curve2Kind::Line2(seam_rgt) = ctx.curves2[1] else { panic!() };
        assert!(approx(seam_rgt.p1.v, scaled_h), "seam_rgt p1.v expected {scaled_h}");
        let Curve2Kind::Line2(top_lat) = ctx.curves2[2] else { panic!() };
        assert!(approx(top_lat.p0.v, scaled_h), "top_lat p0.v expected {scaled_h}");
        assert!(approx(top_lat.p1.v, scaled_h), "top_lat p1.v expected {scaled_h}");
        let Curve2Kind::Line2(seam_lft) = ctx.curves2[3] else { panic!() };
        assert!(approx(seam_lft.p1.v, scaled_h), "seam_lft p1.v expected {scaled_h}");
        // Plane-face pcurves: CircularArc2 radius should NOT change
        let Curve2Kind::CircularArc2(base_cap) = ctx.curves2[4] else { panic!() };
        assert!(approx(base_cap.radius, 1.5), "cap CircularArc2 radius should be unchanged");
    }

    // ── Rotation ──────────────────────────────────────────────────────────────

    #[test]
    fn compile_cylinder_rotation_z() {
        // 90° rotation around Z: (1,0,0) → (0,1,0)
        use std::f64::consts::FRAC_PI_2;
        let (co, si) = (FRAC_PI_2.cos(), FRAC_PI_2.sin());
        #[rustfmt::skip]
        let t = [
            co, -si, 0.0, 0.0,
            si,  co, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        let (ctx, _) = compile_with(CsgPrimitive::Cylinder { r: 1.0, h: 2.0 }, t);
        let pts: Vec<Point3> = ctx.vertices.iter().map(|v| v.point).collect();
        // (1,0,0) → (0,1,0);  (1,0,2) → (0,1,2)
        assert!(pts.iter().any(|p| pt_approx(*p, 0.0, 1.0, 0.0)));
        assert!(pts.iter().any(|p| pt_approx(*p, 0.0, 1.0, 2.0)));
        let SurfaceKind::Cylinder(c) = ctx.surfaces[0] else { panic!("expected Cylinder") };
        assert!(pt_approx(c.axis,    0.0, 0.0, 1.0), "axis should remain (0,0,1)");
        assert!(pt_approx(c.ref_dir, 0.0, 1.0, 0.0), "ref_dir should rotate to (0,1,0)");
        assert!(approx(c.radius, 1.0), "radius should be unchanged");
    }

    // ── Cone ──────────────────────────────────────────────────────────────────

    #[test]
    fn compile_cone_uniform_scale_geometry() {
        #[rustfmt::skip]
        let t = [
            3.0, 0.0, 0.0, 0.0,
            0.0, 3.0, 0.0, 0.0,
            0.0, 0.0, 3.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        let (ctx, _) = compile_with(CsgPrimitive::Cone { r: 1.0, h: 2.0 }, t);
        let pts: Vec<Point3> = ctx.vertices.iter().map(|v| v.point).collect();
        // base-seam vertex (1,0,0) → (3,0,0); apex (0,0,2) → (0,0,6)
        assert!(pts.iter().any(|p| pt_approx(*p, 3.0, 0.0, 0.0)));
        assert!(pts.iter().any(|p| pt_approx(*p, 0.0, 0.0, 6.0)));
        let SurfaceKind::Cone(cone) = ctx.surfaces[0] else { panic!("expected Cone") };
        // half_angle = atan(r/h) is scale-invariant (ratio stays the same)
        let expected_ha = (1.0_f64 / 2.0_f64).atan();
        assert!(approx(cone.half_angle, expected_ha), "half_angle should be unchanged");
    }

    #[test]
    fn compile_cone_uniform_scale_pcurves() {
        // Lateral pcurves whose v-coordinate is a slant distance must scale.
        #[rustfmt::skip]
        let t = [
            3.0, 0.0, 0.0, 0.0,
            0.0, 3.0, 0.0, 0.0,
            0.0, 0.0, 3.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];
        let (ctx, _) = compile_with(CsgPrimitive::Cone { r: 1.0, h: 2.0 }, t);
        // Curve2 push order in build_cone (lateral face first):
        //   [0] pc_apex      Line2 v=0 everywhere — unchanged
        //   [1] pc_seam_rgt  Line2(p0=(TAU,0), p1=(TAU,v_max))   → v_max * 3
        //   [2] pc_base_lat  Line2(p0=(TAU,v_max), p1=(TAU-1,v_max)) → v_max * 3
        //   [3] pc_seam_lft  Line2(p0=(0,0), p1=(0,v_max))       → v_max * 3
        //   [4] pc_base_cap  CircularArc2 on Plane                — unchanged
        let v_max_orig = (1.0_f64 * 1.0 + 2.0 * 2.0_f64).sqrt(); // sqrt(r² + h²)
        let v_max_scaled = 3.0 * v_max_orig;
        let Curve2Kind::Line2(seam_rgt) = ctx.curves2[1] else { panic!() };
        assert!(approx(seam_rgt.p1.v, v_max_scaled), "seam_rgt p1.v expected {v_max_scaled}");
        let Curve2Kind::Line2(base_lat) = ctx.curves2[2] else { panic!() };
        assert!(approx(base_lat.p0.v, v_max_scaled), "base_lat p0.v expected {v_max_scaled}");
        assert!(approx(base_lat.p1.v, v_max_scaled), "base_lat p1.v expected {v_max_scaled}");
        let Curve2Kind::Line2(seam_lft) = ctx.curves2[3] else { panic!() };
        assert!(approx(seam_lft.p1.v, v_max_scaled), "seam_lft p1.v expected {v_max_scaled}");
        // Plane-face pcurve unchanged
        let Curve2Kind::CircularArc2(base_cap) = ctx.curves2[4] else { panic!() };
        assert!(approx(base_cap.radius, 1.0), "cap CircularArc2 radius should be unchanged");
    }

    // ── compile_csg_node ──────────────────────────────────────────────────────

    use crate::csg_lang::CsgNode;

    fn compile_node(node: &CsgNode) -> (SolidModelingContext, SolidId) {
        let mut ctx = SolidModelingContext::new();
        let sid = compile_csg_node(&mut ctx, node);
        (ctx, sid)
    }

    // ─── Dispatch: correct primitive is built ─────────────────────────────────

    #[test]
    fn csg_node_cuboid_entity_counts() {
        let (ctx, _) = compile_node(&CsgNode::cuboid(2.0, 3.0, 4.0));
        assert_eq!(ctx.vertices.len(), 8);
        assert_eq!(ctx.faces.len(), 6);
    }

    #[test]
    fn csg_node_cylinder_entity_counts() {
        let (ctx, _) = compile_node(&CsgNode::cylinder(1.0, 2.0));
        assert_eq!(ctx.vertices.len(), 2);
        assert_eq!(ctx.faces.len(), 3);
    }

    #[test]
    fn csg_node_cone_entity_counts() {
        let (ctx, _) = compile_node(&CsgNode::cone(1.0, 2.0));
        assert_eq!(ctx.vertices.len(), 2);
        assert_eq!(ctx.faces.len(), 2);
    }

    #[test]
    fn csg_node_sphere_entity_counts() {
        let (ctx, _) = compile_node(&CsgNode::sphere(1.0));
        assert_eq!(ctx.vertices.len(), 2);
        assert_eq!(ctx.faces.len(), 1);
    }

    // ─── flat_transform is forwarded ──────────────────────────────────────────

    #[test]
    fn csg_node_translation_reaches_geometry() {
        let node = CsgNode::cuboid(1.0, 1.0, 1.0).translate(3.0, 0.0, 0.0);
        let (ctx, _) = compile_node(&node);
        let pts: Vec<Point3> = ctx.vertices.iter().map(|v| v.point).collect();
        // Corner (0,0,0) shifted to (3,0,0); opposite (1,1,1) shifted to (4,1,1)
        assert!(pts.iter().any(|p| pt_approx(*p, 3.0, 0.0, 0.0)));
        assert!(pts.iter().any(|p| pt_approx(*p, 4.0, 1.0, 1.0)));
    }

    #[test]
    fn csg_node_scale_reaches_sphere_radius() {
        let node = CsgNode::sphere(1.0).scale(2.0, 2.0, 2.0);
        let (ctx, _) = compile_node(&node);
        let SurfaceKind::Sphere(s) = ctx.surfaces[0] else { panic!("expected Sphere") };
        assert!(approx(s.radius, 2.0), "radius should be scaled to 2.0");
    }

    // ─── prov_id and geom_id are forwarded ────────────────────────────────────

    #[test]
    fn csg_node_provenance_forwarded() {
        let node = CsgNode::sphere(1.0);
        let expected_prov = node.prov_id;
        let expected_geom = node.geom_id;
        let (ctx, _) = compile_node(&node);
        for face in &ctx.faces {
            assert_eq!(face.prov.sources[0].prov_id, expected_prov);
            assert_eq!(face.prov.sources[0].geom_id, expected_geom);
        }
    }
}
