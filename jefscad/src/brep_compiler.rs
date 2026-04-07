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
    CircularArc2, CircularArc3, Curve2Kind, Curve3Kind, CylindricalSurface,
    Line2, Line3, Plane, Point2, Point3, SurfaceKind,
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    use super::*;
    use crate::brep_kernel::{Orientation, SolidId};

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
}
