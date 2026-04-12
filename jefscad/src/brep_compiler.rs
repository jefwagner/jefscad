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
    CircularArc2, CircularArc3, ConicalSurface, Curve2, Curve2Kind, Curve3Kind, CylindricalSurface,
    Line2, Line3, LinearExtrusionSurface, Path2D, Plane, Point2, Point3, Polyline3,
    RevolutionSurface, SphericalSurface, SurfaceKind,
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

// ── build_extrusion ───────────────────────────────────────────────────────────

/// Reasons `build_extrusion` can fail.
#[derive(Debug, PartialEq)]
pub enum ExtrusionError {
    /// `path.closed` is `false`.
    PathNotClosed,
    /// `path.segments` is empty.
    PathEmpty,
    /// `height` is zero or negative.
    NonPositiveHeight,
    /// `path.closed` is `true` but `path.current_pos()` is not within
    /// `ctx.tolerance.pos_tol` of `path.start`.
    GeometricallyOpen,
}

/// End-point (at t_max) of a `Curve2Kind`.
fn curve2_end(c: &Curve2Kind) -> Point2 {
    match c {
        Curve2Kind::Line2(l)        => l.p1,
        Curve2Kind::CircularArc2(a) => a.eval(a.t1),
        Curve2Kind::Polyline2(pl)   => *pl.points.last().expect("Polyline2 has points"),
        Curve2Kind::Nurbs(_)        => todo!("curve2_end for NurbsCurve2"),
    }
}

/// Parameter range `[t_min, t_max]` of a `Curve2Kind`.
fn curve2_t_range(c: &Curve2Kind) -> (f64, f64) {
    match c {
        Curve2Kind::Line2(l)        => (l.t_min, l.t_max),
        Curve2Kind::CircularArc2(a) => (a.t0, a.t1),
        Curve2Kind::Polyline2(pl)   => (0.0, pl.n_segments() as f64),
        Curve2Kind::Nurbs(_)        => todo!("curve2_t_range for NurbsCurve2"),
    }
}

/// Lift a 2-D path segment to a 3-D curve at height `z`.
///
/// For line segments the result is a `Line3` in the z=`z` plane.
/// For circular arcs the result is a `CircularArc3` with `normal = +Z`, `ref_dir = +X`,
/// preserving the same angle parameterization so edge t-values stay compatible.
fn lift_curve2(c: &Curve2Kind, z: f64) -> Curve3Kind {
    let p3 = |u: f64, v: f64| Point3::new(u, v, z);
    match c {
        Curve2Kind::Line2(l) => Curve3Kind::Line3(
            Line3::new(p3(l.p0.u, l.p0.v), p3(l.p1.u, l.p1.v)),
        ),
        Curve2Kind::CircularArc2(a) => Curve3Kind::CircularArc3(
            CircularArc3::new(
                p3(a.center.u, a.center.v),
                Point3::new(0.0, 0.0, 1.0), // normal = +Z
                Point3::new(1.0, 0.0, 0.0), // ref_dir = +X (angle measured from +X)
                a.radius,
                a.t0,
                a.t1,
            ),
        ),
        Curve2Kind::Polyline2(pl) => Curve3Kind::Polyline3(
            Polyline3::new(pl.points.iter().map(|pt| Point3::new(pt.u, pt.v, z)).collect()),
        ),
        Curve2Kind::Nurbs(_) => todo!("lift NurbsCurve2 to Curve3Kind"),
    }
}

/// Lift a 2-D profile segment to a 3-D curve in the x-z half-plane (y = 0).
///
/// Used for revolution seam edges.  `Path2D` coordinates `(u, v)` map to 3-D `(u, 0, v)`.
fn lift_xz_curve2(c: &Curve2Kind) -> Curve3Kind {
    let p3 = |u: f64, v: f64| Point3::new(u, 0.0, v);
    match c {
        Curve2Kind::Line2(l) => Curve3Kind::Line3(
            Line3::new(p3(l.p0.u, l.p0.v), p3(l.p1.u, l.p1.v)),
        ),
        Curve2Kind::CircularArc2(a) => Curve3Kind::CircularArc3(
            CircularArc3::new(
                p3(a.center.u, a.center.v),
                Point3::new(0.0, 1.0, 0.0), // normal = +Y  (the x-z plane)
                Point3::new(1.0, 0.0, 0.0), // ref_dir = +X
                a.radius,
                a.t0,
                a.t1,
            ),
        ),
        Curve2Kind::Polyline2(pl) => Curve3Kind::Polyline3(
            Polyline3::new(pl.points.iter().map(|pt| Point3::new(pt.u, 0.0, pt.v)).collect()),
        ),
        Curve2Kind::Nurbs(_) => todo!("lift_xz_curve2 for NurbsCurve2"),
    }
}

/// Build the B-rep solid for a linear extrusion of a closed [`Path2D`] by `height`
/// along +Z.
///
/// The path is assumed to lie in the X-Y plane (z = 0).  Each segment becomes one
/// lateral face backed by a [`LinearExtrusionSurface`].  Bottom and top caps are
/// flat [`Plane`] faces.
///
/// Topology for an N-segment path: 2N vertices, 3N edges, N+2 faces, 6N coedges.
///
/// # Errors
/// Returns [`ExtrusionError`] if the path is not closed, empty, the height is
/// non-positive, or the path is geometrically open (endpoints don't meet within
/// `ctx.tolerance.pos_tol`).
pub fn build_extrusion(
    ctx: &mut SolidModelingContext,
    path: &Path2D,
    height: f64,
    prov_id: u64,
    geom_id: u64,
) -> Result<SolidId, ExtrusionError> {
    // ── Validation ────────────────────────────────────────────────────────────
    if !path.closed               { return Err(ExtrusionError::PathNotClosed);    }
    if path.segments.is_empty()   { return Err(ExtrusionError::PathEmpty);        }
    if height <= 0.0              { return Err(ExtrusionError::NonPositiveHeight); }
    {
        let cp = path.current_pos();
        let dx = cp.u - path.start.u;
        let dy = cp.v - path.start.v;
        if (dx * dx + dy * dy).sqrt() > ctx.tolerance.pos_tol {
            return Err(ExtrusionError::GeometricallyOpen);
        }
    }

    let n   = path.segments.len();
    let h   = height;
    let tol = ctx.tolerance.pos_tol;
    let p3  = |x: f64, y: f64, z: f64| Point3::new(x, y, z);
    let p2  = |u: f64, v: f64| Point2::new(u, v);
    let up  = p3(0.0, 0.0, 1.0);

    // ── Knot points ───────────────────────────────────────────────────────────
    // knots[i] = 2-D start of segment i (= end of segment i-1 for a closed path).
    let mut knots: Vec<Point2> = Vec::with_capacity(n);
    knots.push(path.start);
    for seg in &path.segments[..n - 1] {
        knots.push(curve2_end(seg));
    }

    // ── Vertices: N bottom (z=0) + N top (z=h) ───────────────────────────────
    let verts_bot: Vec<_> = knots.iter()
        .map(|k| ctx.push_vertex(Vertex::new(p3(k.u, k.v, 0.0), tol)))
        .collect();
    let verts_top: Vec<_> = knots.iter()
        .map(|k| ctx.push_vertex(Vertex::new(p3(k.u, k.v, h), tol)))
        .collect();

    // ── Curves3: N bottom + N top (lifted from path) + N vertical seams ──────
    let c3_bot: Vec<_>  = path.segments.iter()
        .map(|s| ctx.push_curve3(lift_curve2(s, 0.0)))
        .collect();
    let c3_top: Vec<_>  = path.segments.iter()
        .map(|s| ctx.push_curve3(lift_curve2(s, h)))
        .collect();
    let c3_seam: Vec<_> = knots.iter()
        .map(|k| ctx.push_curve3(Curve3Kind::Line3(
            Line3::new(p3(k.u, k.v, 0.0), p3(k.u, k.v, h)),
        )))
        .collect();

    // ── Edges: N bottom + N top + N seams ────────────────────────────────────
    let e_bot: Vec<_> = (0..n).map(|i| {
        let (t0, t1) = curve2_t_range(&path.segments[i]);
        ctx.push_edge(Edge::new(c3_bot[i], verts_bot[i], verts_bot[(i+1)%n], t0, t1))
    }).collect();
    let e_top: Vec<_> = (0..n).map(|i| {
        let (t0, t1) = curve2_t_range(&path.segments[i]);
        ctx.push_edge(Edge::new(c3_top[i], verts_top[i], verts_top[(i+1)%n], t0, t1))
    }).collect();
    let e_seam: Vec<_> = (0..n).map(|i|
        ctx.push_edge(Edge::new(c3_seam[i], verts_bot[i], verts_top[i], 0.0, 1.0))
    ).collect();

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
            let ce = ctx.push_coedge(CoEdge::new($edge, $orient, $face, $pcurve));
            ctx.get_mut_edge($edge).coedges.push(ce);
            ce
        }};
    }

    // ── Lateral faces (one per segment) ──────────────────────────────────────
    // Face i loop (CCW from outside): bot[i] Fwd | seam[i+1] Fwd | top[i] Rev | seam[i] Rev
    //
    // PCurve trick: Line2((0,v_const),(1,v_const)) gives eval(t) = (t, v_const) for any t,
    // directly mapping the edge parameter to the surface u-coordinate.
    for i in 0..n {
        let j = (i + 1) % n;
        let (t_min, t_max) = curve2_t_range(&path.segments[i]);
        let profile = lift_curve2(&path.segments[i], 0.0);
        let les = LinearExtrusionSurface::new(profile, up);
        let (face_id, loop_id) = make_face!(SurfaceKind::Extrusion(les), FaceSense::Aligned);

        let pc_bot    = ctx.push_curve2(Curve2Kind::Line2(Line2::new(p2(0.0,   0.0), p2(1.0,   0.0))));
        let pc_seam_r = ctx.push_curve2(Curve2Kind::Line2(Line2::new(p2(t_max, 0.0), p2(t_max, h  ))));
        let pc_top    = ctx.push_curve2(Curve2Kind::Line2(Line2::new(p2(0.0,   h  ), p2(1.0,   h  ))));
        let pc_seam_l = ctx.push_curve2(Curve2Kind::Line2(Line2::new(p2(t_min, 0.0), p2(t_min, h  ))));

        let ce_bot    = add_coedge!(e_bot[i],   Orientation::Forward, face_id, pc_bot);
        let ce_seam_r = add_coedge!(e_seam[j],  Orientation::Forward, face_id, pc_seam_r);
        let ce_top    = add_coedge!(e_top[i],   Orientation::Reverse, face_id, pc_top);
        let ce_seam_l = add_coedge!(e_seam[i],  Orientation::Reverse, face_id, pc_seam_l);
        ctx.get_mut_loop(loop_id).coedges.extend([ce_bot, ce_seam_r, ce_top, ce_seam_l]);
    }

    // ── Bottom cap (z=0, outward normal = -Z) ─────────────────────────────────
    // Plane: u_dir=+X, v_dir=+Y → natural normal = +Z → AntiAligned gives outward = -Z.
    // Loop: traverse segments in reverse order, each Reverse → CW in XY from above.
    // Consecutive chain: seg[N-1] Rev ends at knot[N-1], seg[N-2] Rev starts there ✓
    // PCurve: cap UV = (x, y), so pcurve is the 2-D path segment directly.
    {
        let plane = Plane::new(p3(0.0, 0.0, 0.0), p3(1.0, 0.0, 0.0), p3(0.0, 1.0, 0.0));
        let (face_id, loop_id) = make_face!(SurfaceKind::Plane(plane), FaceSense::AntiAligned);
        let ces: Vec<_> = (0..n).rev().map(|i| {
            let pc = ctx.push_curve2(path.segments[i].clone());
            add_coedge!(e_bot[i], Orientation::Reverse, face_id, pc)
        }).collect();
        ctx.get_mut_loop(loop_id).coedges.extend(ces);
    }

    // ── Top cap (z=h, outward normal = +Z) ───────────────────────────────────
    // Plane: u_dir=+X, v_dir=+Y → natural normal = +Z → Aligned.
    // Loop: traverse segments in forward order, each Forward → CCW in XY from above.
    // PCurve: cap UV = (x, y), same 2-D shape.
    {
        let plane = Plane::new(p3(0.0, 0.0, h), p3(1.0, 0.0, 0.0), p3(0.0, 1.0, 0.0));
        let (face_id, loop_id) = make_face!(SurfaceKind::Plane(plane), FaceSense::Aligned);
        let ces: Vec<_> = (0..n).map(|i| {
            let pc = ctx.push_curve2(path.segments[i].clone());
            add_coedge!(e_top[i], Orientation::Forward, face_id, pc)
        }).collect();
        ctx.get_mut_loop(loop_id).coedges.extend(ces);
    }

    Ok(solid_id)
}

// ── build_revolution ──────────────────────────────────────────────────────────

/// Reasons `build_revolution` can fail.
#[derive(Debug, PartialEq)]
pub enum RevolutionError {
    /// `path.segments` is empty.
    PathEmpty,
    /// A knot point has x < -`ctx.tolerance.pos_tol` (profile enters the negative half-plane).
    ProfileBelowAxis,
    /// The path is open and neither endpoint lies on the Z-axis (x ≤ `ctx.tolerance.pos_tol`).
    OpenProfileNoAxisEndpoint,
}

/// Build the B-rep solid by revolving a [`Path2D`] profile 360° around the Z-axis.
///
/// The profile is in the x-z half-plane: `Path2D` coordinate `u` is the radial distance
/// from the Z-axis and `v` is the height.  Three cases are handled automatically:
///
/// * **Both endpoints on axis** (`x ≤ tol`): no caps; degenerate pole edges at both ends.
/// * **One endpoint on axis**: one disk cap at the off-axis endpoint.
/// * **Closed path** (`path.closed == true`): torus-like; no caps required.
///
/// # Topology
///
/// For an N-segment profile:
/// * Case 1 (both on axis): N+1 vertices, 2N+1 edges, N faces, 4N coedges.
/// * Case 2 (one on axis): N+1 vertices, 2N+1 edges, N+1 faces, 4N+1 coedges.
/// * Case 3 (closed):        N vertices,    2N edges, N faces, 4N coedges.
///
/// Each lateral face is a [`RevolutionSurface`] with `u` = angle ∈ [0, 2π] and
/// `v` = profile parameter.  The seam is the x-z half-plane.
///
/// # Errors
/// Returns [`RevolutionError`] if the path is empty, a knot has x < −tol, or the
/// path is open with neither endpoint on the axis.
pub fn build_revolution(
    ctx: &mut SolidModelingContext,
    path: &Path2D,
    prov_id: u64,
    geom_id: u64,
) -> Result<SolidId, RevolutionError> {
    use std::f64::consts::TAU;

    if path.segments.is_empty() {
        return Err(RevolutionError::PathEmpty);
    }

    let tol = ctx.tolerance.pos_tol;
    let n   = path.segments.len();

    // ── Collect distinct knot points ──────────────────────────────────────────
    // Open path:   N+1 knots (knots[0..=N])
    // Closed path: N   knots (knots[0..N-1]; end of last seg == start)
    let n_knots = if path.closed { n } else { n + 1 };
    let mut knots: Vec<Point2> = Vec::with_capacity(n_knots);
    knots.push(path.start);
    for seg in &path.segments[..n - 1] {
        knots.push(curve2_end(seg));
    }
    if !path.closed {
        knots.push(curve2_end(path.segments.last().unwrap()));
    }

    // ── Validation ────────────────────────────────────────────────────────────
    for k in &knots {
        if k.u < -tol {
            return Err(RevolutionError::ProfileBelowAxis);
        }
    }

    let first_on_axis = knots[0].u         <= tol;
    let last_on_axis  = knots[n_knots - 1].u <= tol;

    if !path.closed && !first_on_axis && !last_on_axis {
        return Err(RevolutionError::OpenProfileNoAxisEndpoint);
    }

    let p3  = |x: f64, y: f64, z: f64| Point3::new(x, y, z);
    let p2  = |u: f64, v: f64|         Point2::new(u, v);

    // ── Vertices: one per knot, on the seam (y = 0) ──────────────────────────
    let verts: Vec<_> = knots.iter()
        .map(|k| ctx.push_vertex(Vertex::new(p3(k.u, 0.0, k.v), tol)))
        .collect();

    // ── Circle edges: one per knot ────────────────────────────────────────────
    // x ≤ tol → degenerate (Line3 p0==p1, t ∈ [0, 1]).
    // x > tol → full CCW circle (CircularArc3, t ∈ [0, 2π]).
    let circles: Vec<_> = knots.iter().zip(verts.iter()).map(|(k, &v)| {
        if k.u <= tol {
            let c = ctx.push_curve3(Curve3Kind::Line3(
                Line3::new(p3(0.0, 0.0, k.v), p3(0.0, 0.0, k.v)),
            ));
            ctx.push_edge(Edge::new(c, v, v, 0.0, 1.0))
        } else {
            let c = ctx.push_curve3(Curve3Kind::CircularArc3(
                CircularArc3::new(
                    p3(0.0, 0.0, k.v), p3(0.0, 0.0, 1.0), p3(1.0, 0.0, 0.0), k.u, 0.0, TAU,
                ),
            ));
            ctx.push_edge(Edge::new(c, v, v, 0.0, TAU))
        }
    }).collect();

    // ── Seam edges: one per segment (profile lifted into x-z plane) ──────────
    let seams: Vec<_> = (0..n).map(|i| {
        let j      = if path.closed { (i + 1) % n } else { i + 1 };
        let (t0, t1) = curve2_t_range(&path.segments[i]);
        let c = ctx.push_curve3(lift_xz_curve2(&path.segments[i]));
        ctx.push_edge(Edge::new(c, verts[i], verts[j], t0, t1))
    }).collect();

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
            let ce = ctx.push_coedge(CoEdge::new($edge, $orient, $face, $pcurve));
            ctx.get_mut_edge($edge).coedges.push(ce);
            ce
        }};
    }

    // ── Lateral faces ─────────────────────────────────────────────────────────
    //
    // UV rectangle for segment i: u ∈ [0, 2π], v ∈ [t0, t1].
    // Loop CCW in UV (Aligned outward normal):
    //   circle[i] Fwd | seam[i] Fwd | circle[j] Rev | seam[i] Rev
    //
    // PCurve conventions:
    //   Non-degenerate circle (t ∈ [0,2π]): Line2((0,v),(1,v)) → eval(t)=(t, v)   [slope=1]
    //   Degenerate circle     (t ∈ [0,1]):  Line2((0,v),(τ,v)) → eval(t)=(τ·t, v) [full span]
    //   Seam right (u=2π): Line2((τ,0),(τ,1)) → eval(t)=(τ, t)
    //   Seam left  (u=0):  Line2((0, 0),(0,1)) → eval(t)=(0, t)
    for i in 0..n {
        let j = if path.closed { (i + 1) % n } else { i + 1 };
        let (t0_seg, t1_seg) = curve2_t_range(&path.segments[i]);

        let rev_surf = RevolutionSurface::new(
            lift_xz_curve2(&path.segments[i]),
            p3(0.0, 0.0, 0.0),
            p3(0.0, 0.0, 1.0),
        );
        let (face_id, loop_id) = make_face!(SurfaceKind::Revolution(rev_surf), FaceSense::Aligned);

        // PCurve for circle[i] (bottom of face, at v = t0_seg)
        let pc_circ_bot = if knots[i].u <= tol {
            ctx.push_curve2(Curve2Kind::Line2(Line2::new(p2(0.0, t0_seg), p2(TAU, t0_seg))))
        } else {
            ctx.push_curve2(Curve2Kind::Line2(Line2::new(p2(0.0, t0_seg), p2(1.0, t0_seg))))
        };
        // PCurve for seam[i] Forward (right side, u = 2π)
        let pc_seam_rgt = ctx.push_curve2(Curve2Kind::Line2(Line2::new(p2(TAU, 0.0), p2(TAU, 1.0))));
        // PCurve for circle[j] (top of face, at v = t1_seg)
        let pc_circ_top = if knots[j].u <= tol {
            ctx.push_curve2(Curve2Kind::Line2(Line2::new(p2(0.0, t1_seg), p2(TAU, t1_seg))))
        } else {
            ctx.push_curve2(Curve2Kind::Line2(Line2::new(p2(0.0, t1_seg), p2(1.0, t1_seg))))
        };
        // PCurve for seam[i] Reverse (left side, u = 0)
        let pc_seam_lft = ctx.push_curve2(Curve2Kind::Line2(Line2::new(p2(0.0, 0.0), p2(0.0, 1.0))));

        let ce_bot = add_coedge!(circles[i],  Orientation::Forward, face_id, pc_circ_bot);
        let ce_sr  = add_coedge!(seams[i],    Orientation::Forward, face_id, pc_seam_rgt);
        let ce_top = add_coedge!(circles[j],  Orientation::Reverse, face_id, pc_circ_top);
        let ce_sl  = add_coedge!(seams[i],    Orientation::Reverse, face_id, pc_seam_lft);
        ctx.get_mut_loop(loop_id).coedges.extend([ce_bot, ce_sr, ce_top, ce_sl]);
    }

    // ── Caps (open path, case 2) ──────────────────────────────────────────────
    // Start cap (non-degenerate start): lateral uses circle[0] Fwd → cap uses Rev + AntiAligned.
    // End cap   (non-degenerate end):   lateral uses circle[n] Rev → cap uses Fwd + Aligned.
    if !path.closed {
        if !first_on_axis {
            let k = knots[0];
            let plane = Plane::new(p3(0.0, 0.0, k.v), p3(1.0, 0.0, 0.0), p3(0.0, 1.0, 0.0));
            let (face_id, loop_id) = make_face!(SurfaceKind::Plane(plane), FaceSense::AntiAligned);
            let pc = ctx.push_curve2(Curve2Kind::CircularArc2(
                CircularArc2::new(p2(0.0, 0.0), k.u, 0.0, TAU),
            ));
            let ce = add_coedge!(circles[0], Orientation::Reverse, face_id, pc);
            ctx.get_mut_loop(loop_id).coedges.push(ce);
        }
        if !last_on_axis {
            let k = knots[n];
            let plane = Plane::new(p3(0.0, 0.0, k.v), p3(1.0, 0.0, 0.0), p3(0.0, 1.0, 0.0));
            let (face_id, loop_id) = make_face!(SurfaceKind::Plane(plane), FaceSense::Aligned);
            let pc = ctx.push_curve2(Curve2Kind::CircularArc2(
                CircularArc2::new(p2(0.0, 0.0), k.u, 0.0, TAU),
            ));
            let ce = add_coedge!(circles[n], Orientation::Forward, face_id, pc);
            ctx.get_mut_loop(loop_id).coedges.push(ce);
        }
    }

    Ok(solid_id)
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
            Curve3Kind::Polyline3(_) => {
                todo!("transform absorption for Polyline3 not yet implemented")
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
            SurfaceKind::Extrusion(_) | SurfaceKind::Revolution(_) => {
                todo!("transform absorption for Extrusion/Revolution not yet implemented")
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
            Curve2Kind::Polyline2(_) => {
                todo!("pcurve v-scaling for Polyline2 not yet implemented")
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

    // ── build_extrusion ───────────────────────────────────────────────────────

    use crate::geom::{Curve2, Path2D, Point2};

    fn triangle_path() -> Path2D {
        let mut p = Path2D::new(Point2::new(0.0, 0.0));
        p.line_to(Point2::new(1.0, 0.0))
         .line_to(Point2::new(0.5, 1.0))
         .line_to_close();
        p
    }

    fn std_extrude_triangle() -> (SolidModelingContext, SolidId) {
        let mut ctx = SolidModelingContext::new();
        let sid = build_extrusion(&mut ctx, &triangle_path(), 2.0, 7, 13).unwrap();
        (ctx, sid)
    }

    // Validation

    #[test]
    fn extrude_err_not_closed() {
        let mut ctx = SolidModelingContext::new();
        let mut path = Path2D::new(Point2::new(0.0, 0.0));
        path.line_to(Point2::new(1.0, 0.0)).line_to(Point2::new(0.5, 1.0));
        // closed = false (default)
        assert_eq!(
            build_extrusion(&mut ctx, &path, 2.0, 0, 0),
            Err(ExtrusionError::PathNotClosed)
        );
    }

    #[test]
    fn extrude_err_empty() {
        let mut ctx = SolidModelingContext::new();
        let mut path = Path2D::new(Point2::new(0.0, 0.0));
        path.close();
        assert_eq!(
            build_extrusion(&mut ctx, &path, 2.0, 0, 0),
            Err(ExtrusionError::PathEmpty)
        );
    }

    #[test]
    fn extrude_err_zero_height() {
        let mut ctx = SolidModelingContext::new();
        assert_eq!(
            build_extrusion(&mut ctx, &triangle_path(), 0.0, 0, 0),
            Err(ExtrusionError::NonPositiveHeight)
        );
    }

    #[test]
    fn extrude_err_negative_height() {
        let mut ctx = SolidModelingContext::new();
        assert_eq!(
            build_extrusion(&mut ctx, &triangle_path(), -1.0, 0, 0),
            Err(ExtrusionError::NonPositiveHeight)
        );
    }

    #[test]
    fn extrude_err_geometrically_open() {
        let mut ctx = SolidModelingContext::new();
        let mut path = Path2D::new(Point2::new(0.0, 0.0));
        path.line_to(Point2::new(1.0, 0.0))
            .line_to(Point2::new(0.5, 1.0));
        path.close(); // sets flag without adding closing segment — current_pos ≠ start
        assert_eq!(
            build_extrusion(&mut ctx, &path, 2.0, 0, 0),
            Err(ExtrusionError::GeometricallyOpen)
        );
    }

    // Entity counts — triangle (N=3 → 6V, 9E, 5F, 18CE)

    #[test]
    fn extrude_triangle_vertex_count() {
        let (ctx, _) = std_extrude_triangle();
        assert_eq!(ctx.vertices.len(), 6);
    }

    #[test]
    fn extrude_triangle_edge_count() {
        let (ctx, _) = std_extrude_triangle();
        assert_eq!(ctx.edges.len(), 9);
    }

    #[test]
    fn extrude_triangle_coedge_count() {
        let (ctx, _) = std_extrude_triangle();
        assert_eq!(ctx.coedges.len(), 18);
    }

    #[test]
    fn extrude_triangle_face_count() {
        let (ctx, _) = std_extrude_triangle();
        assert_eq!(ctx.faces.len(), 5);
    }

    #[test]
    fn extrude_triangle_shell_solid_count() {
        let (ctx, _) = std_extrude_triangle();
        assert_eq!(ctx.shells.len(), 1);
        assert_eq!(ctx.solids.len(), 1);
    }

    // Vertex positions

    #[test]
    fn extrude_triangle_bottom_vertices_at_z0() {
        let (ctx, _) = std_extrude_triangle();
        let bot: Vec<_> = ctx.vertices.iter().filter(|v| v.point.z == 0.0).collect();
        assert_eq!(bot.len(), 3);
        let pts: Vec<_> = bot.iter().map(|v| (v.point.x, v.point.y)).collect();
        assert!(pts.contains(&(0.0, 0.0)));
        assert!(pts.contains(&(1.0, 0.0)));
        assert!(pts.contains(&(0.5, 1.0)));
    }

    #[test]
    fn extrude_triangle_top_vertices_at_z_height() {
        let (ctx, _) = std_extrude_triangle();
        let top: Vec<_> = ctx.vertices.iter().filter(|v| v.point.z == 2.0).collect();
        assert_eq!(top.len(), 3);
        let pts: Vec<_> = top.iter().map(|v| (v.point.x, v.point.y)).collect();
        assert!(pts.contains(&(0.0, 0.0)));
        assert!(pts.contains(&(1.0, 0.0)));
        assert!(pts.contains(&(0.5, 1.0)));
    }

    // Surface types

    #[test]
    fn extrude_triangle_lateral_surfaces_are_extrusion() {
        let (ctx, _) = std_extrude_triangle();
        let lateral_count = ctx.faces.iter()
            .filter(|f| matches!(ctx.surfaces[f.surface.0], SurfaceKind::Extrusion(_)))
            .count();
        assert_eq!(lateral_count, 3);
    }

    #[test]
    fn extrude_triangle_cap_surfaces_are_planes() {
        let (ctx, _) = std_extrude_triangle();
        let plane_count = ctx.faces.iter()
            .filter(|f| matches!(ctx.surfaces[f.surface.0], SurfaceKind::Plane(_)))
            .count();
        assert_eq!(plane_count, 2);
    }

    #[test]
    fn extrude_lateral_direction_is_z() {
        let (ctx, _) = std_extrude_triangle();
        for surf in &ctx.surfaces {
            if let SurfaceKind::Extrusion(les) = surf {
                assert_eq!(les.direction, Point3::new(0.0, 0.0, 1.0));
            }
        }
    }

    // Face sense

    #[test]
    fn extrude_lateral_faces_are_aligned() {
        let (ctx, _) = std_extrude_triangle();
        for face in &ctx.faces {
            if matches!(ctx.surfaces[face.surface.0], SurfaceKind::Extrusion(_)) {
                assert_eq!(face.sense, FaceSense::Aligned);
            }
        }
    }

    #[test]
    fn extrude_bottom_cap_is_antialigned() {
        let (ctx, _) = std_extrude_triangle();
        // Bottom cap: plane at z=0
        let bottom = ctx.faces.iter().find(|f| {
            if let SurfaceKind::Plane(pl) = &ctx.surfaces[f.surface.0] {
                pl.p0.z == 0.0
            } else { false }
        }).expect("bottom cap face");
        assert_eq!(bottom.sense, FaceSense::AntiAligned);
    }

    #[test]
    fn extrude_top_cap_is_aligned() {
        let (ctx, _) = std_extrude_triangle();
        let top = ctx.faces.iter().find(|f| {
            if let SurfaceKind::Plane(pl) = &ctx.surfaces[f.surface.0] {
                pl.p0.z == 2.0
            } else { false }
        }).expect("top cap face");
        assert_eq!(top.sense, FaceSense::Aligned);
    }

    // Topology consistency

    #[test]
    fn extrude_triangle_each_edge_has_two_coedges() {
        let (ctx, _) = std_extrude_triangle();
        for (i, edge) in ctx.edges.iter().enumerate() {
            assert_eq!(edge.coedges.len(), 2, "edge {i} must have exactly 2 coedges");
        }
    }

    #[test]
    fn extrude_triangle_each_edge_one_fwd_one_rev() {
        let (ctx, _) = std_extrude_triangle();
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

    #[test]
    fn extrude_triangle_lateral_loops_have_four_coedges() {
        let (ctx, _) = std_extrude_triangle();
        for face in &ctx.faces {
            if matches!(ctx.surfaces[face.surface.0], SurfaceKind::Extrusion(_)) {
                let lp = ctx.get_loop(face.outer);
                assert_eq!(lp.coedges.len(), 4);
            }
        }
    }

    #[test]
    fn extrude_triangle_cap_loops_have_three_coedges() {
        let (ctx, _) = std_extrude_triangle();
        for face in &ctx.faces {
            if matches!(ctx.surfaces[face.surface.0], SurfaceKind::Plane(_)) {
                let lp = ctx.get_loop(face.outer);
                assert_eq!(lp.coedges.len(), 3);
            }
        }
    }

    #[test]
    fn extrude_triangle_loop_coedges_form_closed_chain() {
        let (ctx, _) = std_extrude_triangle();
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
                    "loop coedge {i} end must equal coedge {} start", (i+1)%n);
            }
        }
    }

    // PCurve sanity

    #[test]
    fn extrude_triangle_bottom_pcurves_at_v0() {
        // For each lateral face, coedges[0] is the bottom edge.
        // Its pcurve is Line2((0,0),(1,0)): eval(t).v must be 0.0 everywhere.
        let (ctx, _) = std_extrude_triangle();
        for face in &ctx.faces {
            if matches!(ctx.surfaces[face.surface.0], SurfaceKind::Extrusion(_)) {
                let lp   = ctx.get_loop(face.outer);
                let ce   = ctx.get_coedge(lp.coedges[0]);
                let edge = ctx.get_edge(ce.edge);
                let pc   = ctx.get_curve2(ce.pcurve);
                assert_eq!(pc.eval(edge.t0).v, 0.0, "bottom pcurve v at t0 must be 0");
                assert_eq!(pc.eval(edge.t1).v, 0.0, "bottom pcurve v at t1 must be 0");
            }
        }
    }

    #[test]
    fn extrude_triangle_top_pcurves_at_v_height() {
        // For each lateral face, coedges[2] is the top edge (Reverse).
        // Its pcurve is Line2((0,h),(1,h)): eval(t).v must be height everywhere.
        let (ctx, _) = std_extrude_triangle();
        let height = 2.0_f64;
        for face in &ctx.faces {
            if matches!(ctx.surfaces[face.surface.0], SurfaceKind::Extrusion(_)) {
                let lp   = ctx.get_loop(face.outer);
                let ce   = ctx.get_coedge(lp.coedges[2]);
                let edge = ctx.get_edge(ce.edge);
                let pc   = ctx.get_curve2(ce.pcurve);
                assert_eq!(pc.eval(edge.t0).v, height, "top pcurve v at t0 must be height");
                assert_eq!(pc.eval(edge.t1).v, height, "top pcurve v at t1 must be height");
            }
        }
    }

    // Provenance

    #[test]
    fn extrude_triangle_face_provenance() {
        let (ctx, _) = std_extrude_triangle();
        for face in &ctx.faces {
            assert_eq!(face.prov.sources.len(), 1);
            assert_eq!(face.prov.sources[0].prov_id, 7);
            assert_eq!(face.prov.sources[0].geom_id, 13);
            assert_eq!(face.prov.last_op, None);
        }
    }

    // Square cross-validation against build_cuboid

    #[test]
    fn extrude_square_matches_cuboid_entity_counts() {
        // Square (0,0)→(2,0)→(2,3)→(0,3), extruded to h=4 should match build_cuboid(2,3,4)
        let mut ctx = SolidModelingContext::new();
        let mut path = Path2D::new(Point2::new(0.0, 0.0));
        path.line_to(Point2::new(2.0, 0.0))
            .line_to(Point2::new(2.0, 3.0))
            .line_to(Point2::new(0.0, 3.0))
            .line_to_close();
        build_extrusion(&mut ctx, &path, 4.0, 0, 0).unwrap();
        assert_eq!(ctx.vertices.len(), 8);
        assert_eq!(ctx.edges.len(),    12);
        assert_eq!(ctx.faces.len(),    6);
        assert_eq!(ctx.coedges.len(),  24);
    }

    // ── build_revolution ──────────────────────────────────────────────────────

    use std::f64::consts::FRAC_PI_2;

    /// Case 2, N=1: line from (0,0) to (r,h) — degenerate at start, disk cap at end.
    fn cone_path() -> Path2D {
        let mut p = Path2D::new(Point2::new(0.0, 0.0));
        p.line_to(Point2::new(1.0, 2.0));
        p
    }

    fn std_revolve_cone() -> (SolidModelingContext, SolidId) {
        let mut ctx = SolidModelingContext::new();
        let sid = build_revolution(&mut ctx, &cone_path(), 5, 9).unwrap();
        (ctx, sid)
    }

    /// Case 1, N=2: two lines (0,0)→(1,1)→(0,2) — both endpoints on axis, no caps.
    fn bipoint_path() -> Path2D {
        let mut p = Path2D::new(Point2::new(0.0, 0.0));
        p.line_to(Point2::new(1.0, 1.0)).line_to(Point2::new(0.0, 2.0));
        p
    }

    fn std_revolve_bipoint() -> (SolidModelingContext, SolidId) {
        let mut ctx = SolidModelingContext::new();
        let sid = build_revolution(&mut ctx, &bipoint_path(), 5, 9).unwrap();
        (ctx, sid)
    }

    /// Case 3, N=4: closed square (1,0)→(2,0)→(2,1)→(1,1)→close — torus-like, no caps.
    fn ring_path() -> Path2D {
        let mut p = Path2D::new(Point2::new(1.0, 0.0));
        p.line_to(Point2::new(2.0, 0.0))
         .line_to(Point2::new(2.0, 1.0))
         .line_to(Point2::new(1.0, 1.0))
         .line_to_close();
        p
    }

    fn std_revolve_ring() -> (SolidModelingContext, SolidId) {
        let mut ctx = SolidModelingContext::new();
        let sid = build_revolution(&mut ctx, &ring_path(), 5, 9).unwrap();
        (ctx, sid)
    }

    // Validation — PathEmpty

    #[test]
    fn revolve_err_empty() {
        let mut ctx = SolidModelingContext::new();
        let mut path = Path2D::new(Point2::new(0.0, 0.0));
        path.close();
        assert_eq!(build_revolution(&mut ctx, &path, 0, 0), Err(RevolutionError::PathEmpty));
    }

    // Validation — ProfileBelowAxis

    #[test]
    fn revolve_err_start_below_axis() {
        let mut ctx = SolidModelingContext::new();
        let mut path = Path2D::new(Point2::new(-0.5, 0.0));
        path.line_to(Point2::new(0.0, 1.0));
        assert_eq!(
            build_revolution(&mut ctx, &path, 0, 0),
            Err(RevolutionError::ProfileBelowAxis)
        );
    }

    #[test]
    fn revolve_err_interior_knot_below_axis() {
        let mut ctx = SolidModelingContext::new();
        let mut path = Path2D::new(Point2::new(1.0, 0.0));
        path.line_to(Point2::new(-0.5, 1.0)).line_to(Point2::new(0.0, 2.0));
        assert_eq!(
            build_revolution(&mut ctx, &path, 0, 0),
            Err(RevolutionError::ProfileBelowAxis)
        );
    }

    #[test]
    fn revolve_err_closed_below_axis() {
        let mut ctx = SolidModelingContext::new();
        let mut path = Path2D::new(Point2::new(1.0, 0.0));
        path.line_to(Point2::new(-0.5, 0.5))
            .line_to(Point2::new(1.0, 1.0))
            .line_to_close();
        assert_eq!(
            build_revolution(&mut ctx, &path, 0, 0),
            Err(RevolutionError::ProfileBelowAxis)
        );
    }

    // Validation — OpenProfileNoAxisEndpoint

    #[test]
    fn revolve_err_open_neither_endpoint_on_axis() {
        let mut ctx = SolidModelingContext::new();
        let mut path = Path2D::new(Point2::new(1.0, 0.0));
        path.line_to(Point2::new(2.0, 1.0));
        assert_eq!(
            build_revolution(&mut ctx, &path, 0, 0),
            Err(RevolutionError::OpenProfileNoAxisEndpoint)
        );
    }

    // Entity counts

    #[test]
    fn revolve_cone_entity_counts() {
        // Case 2, N=1: 2V, 3E, 2F (1 lateral + 1 cap), 5CE
        let (ctx, _) = std_revolve_cone();
        assert_eq!(ctx.vertices.len(),  2);
        assert_eq!(ctx.edges.len(),     3);
        assert_eq!(ctx.faces.len(),     2);
        assert_eq!(ctx.coedges.len(),   5);
    }

    #[test]
    fn revolve_bipoint_entity_counts() {
        // Case 1, N=2: 3V, 5E, 2F, 8CE
        let (ctx, _) = std_revolve_bipoint();
        assert_eq!(ctx.vertices.len(),  3);
        assert_eq!(ctx.edges.len(),     5);
        assert_eq!(ctx.faces.len(),     2);
        assert_eq!(ctx.coedges.len(),   8);
    }

    #[test]
    fn revolve_ring_entity_counts() {
        // Case 3, N=4: 4V, 8E, 4F, 16CE
        let (ctx, _) = std_revolve_ring();
        assert_eq!(ctx.vertices.len(),  4);
        assert_eq!(ctx.edges.len(),     8);
        assert_eq!(ctx.faces.len(),     4);
        assert_eq!(ctx.coedges.len(),  16);
    }

    // Surface types

    #[test]
    fn revolve_cone_lateral_is_revolution() {
        let (ctx, _) = std_revolve_cone();
        let rev_count = ctx.faces.iter()
            .filter(|f| matches!(ctx.surfaces[f.surface.0], SurfaceKind::Revolution(_)))
            .count();
        assert_eq!(rev_count, 1);
    }

    #[test]
    fn revolve_cone_cap_is_plane() {
        let (ctx, _) = std_revolve_cone();
        let plane_count = ctx.faces.iter()
            .filter(|f| matches!(ctx.surfaces[f.surface.0], SurfaceKind::Plane(_)))
            .count();
        assert_eq!(plane_count, 1);
    }

    // Degenerate edge

    #[test]
    fn revolve_cone_degenerate_edge_has_one_coedge() {
        let (ctx, _) = std_revolve_cone();
        // The degenerate apex edge (v0==v1, at z=0) is only adjacent to one face.
        let deg_edges: Vec<_> = ctx.edges.iter()
            .filter(|e| e.v0 == e.v1 && e.t0 == 0.0 && e.t1 == 1.0)
            .collect();
        assert_eq!(deg_edges.len(), 1, "expect exactly one degenerate edge");
        assert_eq!(deg_edges[0].coedges.len(), 1);
    }

    // Face sense

    #[test]
    fn revolve_cone_lateral_face_is_aligned() {
        let (ctx, _) = std_revolve_cone();
        for face in &ctx.faces {
            if matches!(ctx.surfaces[face.surface.0], SurfaceKind::Revolution(_)) {
                assert_eq!(face.sense, FaceSense::Aligned);
            }
        }
    }

    #[test]
    fn revolve_cone_cap_face_is_aligned() {
        // End cap at top (outward = +Z) → Aligned.
        let (ctx, _) = std_revolve_cone();
        for face in &ctx.faces {
            if matches!(ctx.surfaces[face.surface.0], SurfaceKind::Plane(_)) {
                assert_eq!(face.sense, FaceSense::Aligned);
            }
        }
    }

    // Topology consistency

    #[test]
    fn revolve_cone_non_degenerate_edges_have_two_coedges() {
        let (ctx, _) = std_revolve_cone();
        for edge in &ctx.edges {
            if !(edge.v0 == edge.v1 && edge.t0 == 0.0 && edge.t1 == 1.0) {
                // non-degenerate
                assert_eq!(edge.coedges.len(), 2, "non-degenerate edge must have 2 coedges");
            }
        }
    }

    #[test]
    fn revolve_cone_loop_coedges_form_closed_chain() {
        let (ctx, _) = std_revolve_cone();
        for lp in &ctx.loops {
            let nc = lp.coedges.len();
            for i in 0..nc {
                let ce_cur  = ctx.get_coedge(lp.coedges[i]);
                let ce_next = ctx.get_coedge(lp.coedges[(i + 1) % nc]);
                let end_cur = match ce_cur.orientation {
                    Orientation::Forward => ctx.get_edge(ce_cur.edge).v1,
                    Orientation::Reverse => ctx.get_edge(ce_cur.edge).v0,
                };
                let start_next = match ce_next.orientation {
                    Orientation::Forward => ctx.get_edge(ce_next.edge).v0,
                    Orientation::Reverse => ctx.get_edge(ce_next.edge).v1,
                };
                assert_eq!(end_cur, start_next,
                    "loop coedge {i} end must equal coedge {} start", (i + 1) % nc);
            }
        }
    }

    // PCurve sanity

    #[test]
    fn revolve_cone_seam_pcurve_maps_to_u0_and_utau() {
        use std::f64::consts::TAU;
        // Lateral face: seam used twice — Fwd pcurve maps t→(TAU,t), Rev pcurve maps t→(0,t).
        let (ctx, _) = std_revolve_cone();
        let lat_face = ctx.faces.iter()
            .find(|f| matches!(ctx.surfaces[f.surface.0], SurfaceKind::Revolution(_)))
            .expect("lateral face");
        let lp = ctx.get_loop(lat_face.outer);
        // coedges[1] = seam Fwd (right, u=TAU), coedges[3] = seam Rev (left, u=0)
        let ce_seam_fwd = ctx.get_coedge(lp.coedges[1]);
        let ce_seam_rev = ctx.get_coedge(lp.coedges[3]);
        let pc_fwd = ctx.get_curve2(ce_seam_fwd.pcurve);
        let pc_rev = ctx.get_curve2(ce_seam_rev.pcurve);
        let t_mid  = 0.5_f64;
        assert!((pc_fwd.eval(t_mid).u - TAU).abs() < 1e-12, "seam Fwd pcurve u must be TAU");
        assert!( pc_rev.eval(t_mid).u.abs()          < 1e-12, "seam Rev pcurve u must be 0");
    }

    #[test]
    fn revolve_cone_circle_pcurve_maps_angle_to_u() {
        use std::f64::consts::TAU;
        // The disk cap reuses the top circle (non-degenerate, t ∈ [0,2π]).
        // Its PCurve is a CircularArc2 (cap) — the lateral face's top circle PCurve is Line2 with slope 1.
        let (ctx, _) = std_revolve_cone();
        let lat_face = ctx.faces.iter()
            .find(|f| matches!(ctx.surfaces[f.surface.0], SurfaceKind::Revolution(_)))
            .expect("lateral face");
        let lp = ctx.get_loop(lat_face.outer);
        // coedges[2] = circle[top] Reverse
        let ce_top = ctx.get_coedge(lp.coedges[2]);
        let pc_top = ctx.get_curve2(ce_top.pcurve);
        // Line2((0, t1),(1, t1)) → eval(t) = (t, t1). At t=TAU/4: u = TAU/4.
        let t_test = TAU / 4.0;
        assert!((pc_top.eval(t_test).u - t_test).abs() < 1e-12, "circle top pcurve u must equal t");
    }

    // Cross-validation with build_sphere

    #[test]
    fn revolve_semicircle_matches_sphere_entity_counts() {
        use std::f64::consts::FRAC_PI_2;
        // Revolve a semicircle: start=(0,-1), CircularArc2 center=(0,0) r=1, t0=-π/2, t1=+π/2.
        // Both endpoints on axis → Case 1, N=1 → 2V, 3E, 1F, 4CE  (same as build_sphere).
        let mut ctx_rev  = SolidModelingContext::new();
        let arc = Curve2Kind::CircularArc2(CircularArc2::new(
            Point2::new(0.0, 0.0), 1.0, -FRAC_PI_2, FRAC_PI_2,
        ));
        let mut path = Path2D::new(arc.eval(-FRAC_PI_2));
        path.segments.push(arc);  // bypass builder to set the arc directly

        // Hmm, actually Path2D doesn't expose direct segment push publicly.
        // Use arc_to instead:
        let mut path2 = Path2D::new(Point2::new(0.0, -1.0));
        path2.arc_to(Point2::new(0.0, 0.0), std::f64::consts::PI);
        build_revolution(&mut ctx_rev, &path2, 0, 0).unwrap();

        let mut ctx_sph = SolidModelingContext::new();
        build_sphere(&mut ctx_sph, 1.0, 0, 0);

        assert_eq!(ctx_rev.vertices.len(), ctx_sph.vertices.len());
        assert_eq!(ctx_rev.edges.len(),    ctx_sph.edges.len());
        assert_eq!(ctx_rev.faces.len(),    ctx_sph.faces.len());
        assert_eq!(ctx_rev.coedges.len(),  ctx_sph.coedges.len());
    }

    // Provenance

    #[test]
    fn revolve_cone_face_provenance() {
        let (ctx, _) = std_revolve_cone();
        for face in &ctx.faces {
            assert_eq!(face.prov.sources.len(), 1);
            assert_eq!(face.prov.sources[0].prov_id, 5);
            assert_eq!(face.prov.sources[0].geom_id, 9);
        }
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
