//! Boolean operation scaffolding for B-rep solids.
//!
//! Phase 5 starts with planar polyhedra (all faces are `Plane`, all edges are `Line3`,
//! all pcurves are `Line2`).  The two core topology mutations are:
//!
//! - [`split_edge`] — insert a vertex at parameter `t` along an edge, replacing it with
//!   two sub-edges and updating every loop that references the original.
//! - [`split_face`] — split a planar face into two faces along a line that connects a
//!   point on one boundary edge to a point on another.

use crate::brep_kernel::{
    CoEdge, CoEdgeId, Curve2Id, Edge, EdgeId, FaceId, Loop, LoopId,
    Orientation, ShellId, SolidModelingContext, Vertex, VertexId,
};
use crate::geom::{Curve2, Curve2Kind, Curve3, Curve3Kind, Line2, Line3, Point2, Point3, SurfaceKind};

// ── Private helpers ────────────────────────────────────────────────────────────

/// Return the loop inside `face_id` that contains `ce_id`.  Panics if not found.
fn find_loop_for_coedge(ctx: &SolidModelingContext, face_id: FaceId, ce_id: CoEdgeId) -> LoopId {
    let face = ctx.get_face(face_id);
    let outer = face.outer;
    let inners: Vec<LoopId> = face.inners.clone();

    if ctx.get_loop(outer).coedges.contains(&ce_id) {
        return outer;
    }
    for lid in inners {
        if ctx.get_loop(lid).coedges.contains(&ce_id) {
            return lid;
        }
    }
    panic!("coedge {ce_id:?} not found in any loop of face {face_id:?}");
}

/// Return the end vertex of a coedge (respects orientation).
fn coedge_end(ctx: &SolidModelingContext, ce_id: CoEdgeId) -> VertexId {
    let ce = ctx.get_coedge(ce_id);
    let edge = ctx.get_edge(ce.edge);
    match ce.orientation {
        Orientation::Forward => edge.v1,
        Orientation::Reverse => edge.v0,
    }
}

/// Split a `Line2` pcurve at `t_split`, returning two new `Curve2Id`s.
///
/// Both halves share `p0`/`p1`; only `t_min`/`t_max` differ.  Because
/// `Line2::eval(t) = p0 + (p1 - p0) * t` uses the raw parameter, each sub-curve
/// evaluates correctly over its sub-domain without remapping.
fn split_line2(ctx: &mut SolidModelingContext, c2_id: Curve2Id, t_split: f64) -> (Curve2Id, Curve2Id) {
    let orig = match ctx.get_curve2(c2_id) {
        Curve2Kind::Line2(l) => *l,
        _ => todo!("split_line2: only Line2 pcurves supported in Phase 5"),
    };
    let ca = Curve2Kind::Line2(Line2 { p0: orig.p0, p1: orig.p1, t_min: orig.t_min, t_max: t_split  });
    let cb = Curve2Kind::Line2(Line2 { p0: orig.p0, p1: orig.p1, t_min: t_split,  t_max: orig.t_max });
    (ctx.push_curve2(ca), ctx.push_curve2(cb))
}

// ── Public topology mutations ──────────────────────────────────────────────────

/// Split an edge at parameter `t_split`, inserting a new vertex.
///
/// Returns `(new_vertex, edge_a, edge_b)` where `edge_a` covers `[t0, t_split]` and
/// `edge_b` covers `[t_split, t1]`.  Every loop that contained a coedge of the original
/// edge is updated in place: the old coedge is replaced by two new coedges in the
/// correct traversal order.
///
/// # Panics
/// Panics if `t_split` is not strictly inside `[t0, t1]`, or if the curve is not a
/// `Line3` (only `Line3` is supported in Phase 5).
pub fn split_edge(
    ctx: &mut SolidModelingContext,
    edge_id: EdgeId,
    t_split: f64,
) -> (VertexId, EdgeId, EdgeId) {
    let edge = ctx.get_edge(edge_id).clone();
    assert!(
        t_split > edge.t0 && t_split < edge.t1,
        "t_split {t_split} must be strictly inside [{}, {}]",
        edge.t0, edge.t1,
    );

    let split_pt = match ctx.get_curve3(edge.curve3) {
        Curve3Kind::Line3(l) => l.eval(t_split),
        _ => todo!("split_edge: only Line3 supported in Phase 5"),
    };
    let tol = ctx.tolerance.pos_tol;
    let new_v = ctx.push_vertex(Vertex::new(split_pt, tol));

    // Sub-edges share the original Line3 curve; only the t-range differs.
    let edge_a = ctx.push_edge(Edge::new(edge.curve3, edge.v0, new_v,  edge.t0,  t_split));
    let edge_b = ctx.push_edge(Edge::new(edge.curve3, new_v,  edge.v1, t_split,  edge.t1));

    let coedge_ids: Vec<CoEdgeId> = edge.coedges.iter().copied().collect();
    for ce_id in coedge_ids {
        let ce = ctx.get_coedge(ce_id).clone();

        let (pca, pcb) = split_line2(ctx, ce.pcurve, t_split);

        // Forward: first sub-coedge on edge_a (v0→M), second on edge_b (M→v1).
        // Reverse: first sub-coedge on edge_b reversed (v1→M), second on edge_a (M→v0).
        let (first_edge, first_pc, second_edge, second_pc) = match ce.orientation {
            Orientation::Forward => (edge_a, pca, edge_b, pcb),
            Orientation::Reverse => (edge_b, pcb, edge_a, pca),
        };

        let ce_first  = ctx.push_coedge(CoEdge::new(first_edge,  ce.orientation, ce.face, first_pc));
        let ce_second = ctx.push_coedge(CoEdge::new(second_edge, ce.orientation, ce.face, second_pc));

        ctx.get_mut_edge(first_edge).coedges.push(ce_first);
        ctx.get_mut_edge(second_edge).coedges.push(ce_second);

        // Replace old coedge with the two new ones in its loop.
        let loop_id = find_loop_for_coedge(ctx, ce.face, ce_id);
        let pos = ctx.get_loop(loop_id).coedges.iter().position(|&x| x == ce_id)
            .expect("coedge must be in its loop");
        ctx.get_mut_loop(loop_id).coedges.splice(pos..pos + 1, [ce_first, ce_second]);
    }

    (new_v, edge_a, edge_b)
}

/// Split a planar face along the straight line connecting a point on `entry_edge`
/// (at parameter `t_entry`) to a point on `exit_edge` (at parameter `t_exit`).
///
/// Returns `(face_a, face_b)`.  Face A contains the arc from the exit split point
/// back around to the entry split point (wrapping); face B contains the arc from the
/// entry split point forward to the exit split point.
///
/// The original face is removed from the shell; both new faces are added.  The
/// original face entity and its now-orphaned outer loop remain in the arena (they
/// cannot be deleted) but are no longer reachable from the shell.
///
/// # Panics
/// Panics if `entry_edge == exit_edge`, if the face has inner loops, or if curves
/// are not `Line2`/`Line3` (Phase 5 restriction).
pub fn split_face(
    ctx: &mut SolidModelingContext,
    face_id: FaceId,
    entry_edge_id: EdgeId,
    t_entry: f64,
    exit_edge_id: EdgeId,
    t_exit: f64,
) -> (FaceId, FaceId) {
    assert_ne!(entry_edge_id, exit_edge_id, "split_face: entry and exit must be different edges");
    assert!(
        ctx.get_face(face_id).inners.is_empty(),
        "split_face: faces with inner loops not yet supported",
    );

    // Snapshot face metadata before any mutation.
    let outer_loop_id = ctx.get_face(face_id).outer;
    let shell_id      = ctx.get_face(face_id).shell;
    let sense         = ctx.get_face(face_id).sense;
    let surface_id    = ctx.get_face(face_id).surface;
    let prov          = ctx.get_face(face_id).prov.clone();

    // Get UV coordinates of the split points from the pcurves before splitting.
    let loop_ces: Vec<CoEdgeId> = ctx.get_loop(outer_loop_id).coedges.clone();

    let entry_ce = *loop_ces.iter()
        .find(|&&ce| ctx.get_coedge(ce).edge == entry_edge_id)
        .expect("entry_edge must be on the face's outer loop");
    let exit_ce = *loop_ces.iter()
        .find(|&&ce| ctx.get_coedge(ce).edge == exit_edge_id)
        .expect("exit_edge must be on the face's outer loop");

    let uv_entry: Point2 = match ctx.get_curve2(ctx.get_coedge(entry_ce).pcurve) {
        Curve2Kind::Line2(l) => l.eval(t_entry),
        _ => todo!("split_face: only Line2 pcurves"),
    };
    let uv_exit: Point2 = match ctx.get_curve2(ctx.get_coedge(exit_ce).pcurve) {
        Curve2Kind::Line2(l) => l.eval(t_exit),
        _ => todo!("split_face: only Line2 pcurves"),
    };

    // Split the two edges; outer_loop_id is updated in place by each call.
    let (v_entry, _, _) = split_edge(ctx, entry_edge_id, t_entry);
    let (v_exit,  _, _) = split_edge(ctx, exit_edge_id,  t_exit);

    // Read the updated outer loop (now has 2 extra coedges).
    let updated: Vec<CoEdgeId> = ctx.get_loop(outer_loop_id).coedges.clone();
    let n = updated.len();

    // i_ef: index of the coedge whose end vertex is v_entry.
    // i_xf: index of the coedge whose end vertex is v_exit.
    let i_ef = updated.iter().position(|&ce| coedge_end(ctx, ce) == v_entry)
        .expect("entry split vertex must appear as a coedge end in the updated loop");
    let i_xf = updated.iter().position(|&ce| coedge_end(ctx, ce) == v_exit)
        .expect("exit split vertex must appear as a coedge end in the updated loop");

    // Build arc slices.
    // arc_a: from (i_xf+1) wrapping around to i_ef inclusive  → goes into face A
    // arc_b: from (i_ef+1) up to i_xf inclusive               → goes into face B
    let arc_a: Vec<CoEdgeId> = {
        let mut v = Vec::new();
        let mut i = (i_xf + 1) % n;
        loop {
            v.push(updated[i]);
            if i == i_ef { break; }
            i = (i + 1) % n;
        }
        v
    };
    let arc_b: Vec<CoEdgeId> = {
        let mut v = Vec::new();
        let mut i = (i_ef + 1) % n;
        loop {
            v.push(updated[i]);
            if i == i_xf { break; }
            i = (i + 1) % n;
        }
        v
    };

    // Create the split edge (3-D Line3 + per-face pcurves).
    let p_entry = ctx.get_vertex(v_entry).point;
    let p_exit  = ctx.get_vertex(v_exit).point;
    let split_c3  = ctx.push_curve3(Curve3Kind::Line3(Line3::new(p_entry, p_exit)));
    let split_eid = ctx.push_edge(Edge::new(split_c3, v_entry, v_exit, 0.0, 1.0));

    let pc_fwd = ctx.push_curve2(Curve2Kind::Line2(Line2::new(uv_entry, uv_exit)));
    let pc_rev = ctx.push_curve2(Curve2Kind::Line2(Line2::new(uv_exit,  uv_entry)));

    // ── Face A ────────────────────────────────────────────────────────────────
    // Loop: [ce_split_fwd, arc_a...]   path: M_entry → M_exit → … → M_entry
    let face_a = ctx.push_face(crate::brep_kernel::Face::new(
        shell_id, surface_id, LoopId(usize::MAX), sense, prov.clone(),
    ));
    let loop_a = ctx.push_loop(Loop::new(face_a, true));
    ctx.get_mut_face(face_a).outer = loop_a;

    let ce_fwd = ctx.push_coedge(CoEdge::new(split_eid, Orientation::Forward, face_a, pc_fwd));
    ctx.get_mut_edge(split_eid).coedges.push(ce_fwd);
    ctx.get_mut_loop(loop_a).coedges.push(ce_fwd);
    for &ce in &arc_a {
        ctx.get_mut_coedge(ce).face = face_a;
        ctx.get_mut_loop(loop_a).coedges.push(ce);
    }

    // ── Face B ────────────────────────────────────────────────────────────────
    // Loop: [ce_split_rev, arc_b...]   path: M_exit → M_entry → … → M_exit
    let face_b = ctx.push_face(crate::brep_kernel::Face::new(
        shell_id, surface_id, LoopId(usize::MAX), sense, prov.clone(),
    ));
    let loop_b = ctx.push_loop(Loop::new(face_b, true));
    ctx.get_mut_face(face_b).outer = loop_b;

    let ce_rev = ctx.push_coedge(CoEdge::new(split_eid, Orientation::Reverse, face_b, pc_rev));
    ctx.get_mut_edge(split_eid).coedges.push(ce_rev);
    ctx.get_mut_loop(loop_b).coedges.push(ce_rev);
    for &ce in &arc_b {
        ctx.get_mut_coedge(ce).face = face_b;
        ctx.get_mut_loop(loop_b).coedges.push(ce);
    }

    // ── Shell update ──────────────────────────────────────────────────────────
    // Replace original face with two new ones in the shell's face list.
    let shell_faces = &mut ctx.get_mut_shell(shell_id).faces;
    if let Some(pos) = shell_faces.iter().position(|&f| f == face_id) {
        shell_faces.splice(pos..pos + 1, [face_a, face_b]);
    }

    (face_a, face_b)
}

// ── Face classification ────────────────────────────────────────────────────────

/// Compute the centroid of a face's outer loop (average of the start-vertex of each
/// coedge).  For convex faces this always lies in the interior.
///
/// Used as the default sample point for [`classify_face_wrt_node`].
pub(crate) fn face_centroid(ctx: &SolidModelingContext, face_id: FaceId) -> crate::geom::Point3 {
    use crate::geom::Point3;
    let loop_id = ctx.get_face(face_id).outer;
    let ces: Vec<CoEdgeId> = ctx.get_loop(loop_id).coedges.clone();
    assert!(!ces.is_empty(), "face_centroid: outer loop has no coedges");

    let mut sum = Point3::new(0.0, 0.0, 0.0);
    for &ce_id in &ces {
        let ce   = ctx.get_coedge(ce_id);
        let edge = ctx.get_edge(ce.edge);
        let vid  = match ce.orientation {
            Orientation::Forward => edge.v0,
            Orientation::Reverse => edge.v1,
        };
        sum = sum + ctx.get_vertex(vid).point;
    }
    let n = ces.len() as f64;
    Point3::new(sum.x / n, sum.y / n, sum.z / n)
}

/// Classify a B-rep face as [`Classification::Inside`], [`Classification::Outside`],
/// or [`Classification::Indeterminate`] relative to the solid described by `node`.
///
/// Samples the centroid of the face's outer-loop boundary vertices and delegates to
/// [`crate::predicates::classify_node`].  Only primitive `CsgNode`s are supported
/// (boolean-op nodes via `todo!`).
///
/// `Indeterminate` is returned when the sample falls on `node`'s surface within Flint
/// interval width.  For coincident faces (the face lies entirely on a face of `node`'s
/// solid), every sample will be `Indeterminate`.
pub fn classify_face_wrt_node(
    ctx: &SolidModelingContext,
    face_id: FaceId,
    node: &crate::csg_lang::CsgNode,
) -> crate::predicates::Classification {
    let c = face_centroid(ctx, face_id);
    crate::predicates::classify_node([c.x, c.y, c.z], node)
}

// ── Plane-plane SSI ────────────────────────────────────────────────────────────

/// One intersection between the plane-plane line and a face boundary edge.
#[derive(Debug, Clone)]
pub struct BoundaryHit {
    pub edge_id: EdgeId,
    /// Parameter in `[0, 1]` on the B-rep edge (maps linearly to `[edge.t0, edge.t1]`).
    pub t_edge: f64,
    /// Parameter on the infinite intersection line: `origin + t_line * dir`.
    pub t_line: f64,
    pub point: Point3,
}

/// Result of intersecting two planar faces.
///
/// Each face contributes exactly two boundary hits — the entry and exit of the
/// intersection line through that face's polygon.  The two faces may differ in which
/// portion of the line they cover; the overlap is the segment that actually needs to be
/// introduced into both B-reps.
#[derive(Debug, Clone)]
pub struct FaceFaceIntersection {
    pub hits_a: [BoundaryHit; 2],
    pub hits_b: [BoundaryHit; 2],
}

/// Extract the outward unit normal and a point on a planar face's surface.
///
/// Returns `None` for non-planar surfaces (Phase 5 restriction).
fn plane_of_face(ctx: &SolidModelingContext, face_id: FaceId) -> Option<(Point3, Point3)> {
    match ctx.get_surface(ctx.get_face(face_id).surface) {
        SurfaceKind::Plane(p) => Some((p.u_dir.cross(p.v_dir).normalize(), p.p0)),
        _ => None,
    }
}

/// Compute the plane-plane intersection line as `(origin, direction)`.
///
/// `direction = n_a × n_b`.  `origin` is the minimum-norm point on the line, found by
/// solving the 2×2 system `[n_a·n_a  n_a·n_b; n_a·n_b  n_b·n_b] [α; β] = [d_a; d_b]`
/// and returning `α·n_a + β·n_b`.
///
/// Returns `None` when the planes are parallel (`|n_a × n_b| < 1e-10`).
fn plane_plane_line(
    n_a: Point3, p_a: Point3,
    n_b: Point3, p_b: Point3,
) -> Option<(Point3, Point3)> {
    let dir = n_a.cross(n_b);
    if dir.length() < 1e-10 { return None; }

    let d_a = n_a.dot(p_a);
    let d_b = n_b.dot(p_b);
    let a = n_a.dot(n_a);
    let b = n_a.dot(n_b);
    let c = n_b.dot(n_b);
    let det = a * c - b * b;
    let alpha = (d_a * c - d_b * b) / det;
    let beta  = (d_b * a - d_a * b) / det;
    let origin = n_a * alpha + n_b * beta;
    Some((origin, dir))
}

/// Intersect an infinite ray `origin + t * dir` with the line segment `[v0, v1]`.
///
/// Returns `(t_ray, t_seg)` where `t_seg ∈ [0, 1]`, or `None` if lines are parallel or
/// the intersection lies outside the segment.  Uses the 2D Cramer's-rule projection onto
/// the coordinate plane with the largest normal component (`dir × edge`), for numerical
/// stability.
fn intersect_ray_segment(
    origin: Point3,
    dir: Point3,
    v0: Point3,
    v1: Point3,
) -> Option<(f64, f64)> {
    let edge = v1 - v0;
    let n = dir.cross(edge);
    let nx = n.x.abs();
    let ny = n.y.abs();
    let nz = n.z.abs();
    let rhs = v0 - origin;

    // Solve  [dir.i  -edge.i] [t]   [rhs.i]
    //        [dir.j  -edge.j] [s] = [rhs.j]
    // via Cramer's rule, picking (i,j) as the rows with largest cross-product component.
    let (det, t, s) = if nz >= nx && nz >= ny {
        let d = dir.x * (-edge.y) - (-edge.x) * dir.y;
        let t = (rhs.x * (-edge.y) - (-edge.x) * rhs.y) / d;
        let s = (dir.x * rhs.y - dir.y * rhs.x) / d;
        (d, t, s)
    } else if ny >= nx {
        let d = dir.x * (-edge.z) - (-edge.x) * dir.z;
        let t = (rhs.x * (-edge.z) - (-edge.x) * rhs.z) / d;
        let s = (dir.x * rhs.z - dir.z * rhs.x) / d;
        (d, t, s)
    } else {
        let d = dir.y * (-edge.z) - (-edge.y) * dir.z;
        let t = (rhs.y * (-edge.z) - (-edge.y) * rhs.z) / d;
        let s = (dir.y * rhs.z - dir.z * rhs.y) / d;
        (d, t, s)
    };

    if det.abs() < 1e-12 { return None; }
    if s < -1e-9 || s > 1.0 + 1e-9 { return None; }
    Some((t, s.clamp(0.0, 1.0)))
}

/// Clip the infinite intersection line to the polygon boundary of `face_id`.
///
/// Returns exactly two [`BoundaryHit`]s (entry, exit) ordered by `t_line`, or `None` if
/// the line does not cross exactly two boundary edges.  For Phase 5 convex planar
/// polyhedra this is always 0 or 2.
///
/// Vertex hits (line passes through a corner) are deduped: if two adjacent edges both
/// report an intersection at the shared vertex (`t_seg ≈ 0` or `1`), only one is kept.
fn clip_line_to_face(
    ctx: &SolidModelingContext,
    face_id: FaceId,
    origin: Point3,
    dir: Point3,
) -> Option<[BoundaryHit; 2]> {
    let ces: Vec<CoEdgeId> = ctx.get_loop(ctx.get_face(face_id).outer).coedges.clone();
    let mut hits: Vec<BoundaryHit> = Vec::new();

    for &ce_id in &ces {
        let ce   = ctx.get_coedge(ce_id);
        let edge = ctx.get_edge(ce.edge);
        let v0   = ctx.get_vertex(edge.v0).point;
        let v1   = ctx.get_vertex(edge.v1).point;

        if let Some((t_line, t_seg)) = intersect_ray_segment(origin, dir, v0, v1) {
            let t_edge = edge.t0 + (edge.t1 - edge.t0) * t_seg;
            let point  = v0 + (v1 - v0) * t_seg;
            // Deduplicate vertex hits: skip if this point is ≈ the last hit.
            if let Some(prev) = hits.last() {
                let dx = point.x - prev.point.x;
                let dy = point.y - prev.point.y;
                let dz = point.z - prev.point.z;
                if (dx*dx + dy*dy + dz*dz).sqrt() < 1e-10 { continue; }
            }
            hits.push(BoundaryHit { edge_id: ce.edge, t_edge, t_line, point });
        }
    }

    // Also deduplicate first vs last (loop wrap-around vertex hit).
    if hits.len() >= 2 {
        let last = hits.last().unwrap().point;
        let first = hits[0].point;
        let dx = last.x - first.x;
        let dy = last.y - first.y;
        let dz = last.z - first.z;
        if (dx*dx + dy*dy + dz*dz).sqrt() < 1e-10 {
            hits.pop();
        }
    }

    if hits.len() != 2 { return None; }

    if hits[0].t_line > hits[1].t_line { hits.swap(0, 1); }
    Some([hits.remove(0), hits.remove(0)])
}

/// Find the intersection of two planar faces.
///
/// Returns `None` when:
/// - either face is non-planar,
/// - the planes are parallel (including coincident),
/// - the intersection line does not cross both face polygons, or
/// - the clipped segments do not overlap (faces are adjacent but not intersecting).
///
/// Otherwise returns two pairs of [`BoundaryHit`]s — one pair per face — representing
/// where the intersection line enters and exits each face's boundary polygon.
pub fn intersect_planar_faces(
    ctx: &SolidModelingContext,
    face_a: FaceId,
    face_b: FaceId,
) -> Option<FaceFaceIntersection> {
    let (n_a, p_a) = plane_of_face(ctx, face_a)?;
    let (n_b, p_b) = plane_of_face(ctx, face_b)?;
    let (origin, dir) = plane_plane_line(n_a, p_a, n_b, p_b)?;
    let hits_a = clip_line_to_face(ctx, face_a, origin, dir)?;
    let hits_b = clip_line_to_face(ctx, face_b, origin, dir)?;

    // Verify the clipped segments overlap on the shared line parameter.
    let t_start = hits_a[0].t_line.max(hits_b[0].t_line);
    let t_end   = hits_a[1].t_line.min(hits_b[1].t_line);
    if t_end < t_start + 1e-10 { return None; }

    Some(FaceFaceIntersection { hits_a, hits_b })
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    use super::*;
    use crate::brep_kernel::{
        FaceSense, ProvenanceData, Shell, Solid, SolidModelingContext,
    };
    use crate::geom::{Line2, Line3, Plane, Point2, Point3, SurfaceKind};

    fn pt3(x: f64, y: f64, z: f64) -> Point3 { Point3::new(x, y, z) }
    fn pt2(u: f64, v: f64) -> Point2 { Point2::new(u, v) }

    /// Build a unit square face in the XY-plane as a minimal B-rep.
    ///
    /// ```
    ///   V3(0,1,0)──E23──V2(1,1,0)
    ///       |                |
    ///      E30             E12
    ///       |                |
    ///   V0(0,0,0)──E01──V1(1,0,0)
    /// ```
    ///
    /// Outer loop (CCW from +Z): ce01(E01,Fwd), ce12(E12,Fwd), ce23(E23,Fwd), ce30(E30,Fwd)
    ///
    /// Returns (solid_id, shell_id, face_id, [v0,v1,v2,v3], [e01,e12,e23,e30]).
    fn make_square(ctx: &mut SolidModelingContext)
        -> (crate::brep_kernel::SolidId, ShellId, FaceId, [VertexId; 4], [EdgeId; 4])
    {
        let tol = ctx.tolerance.pos_tol;

        // Vertices
        let v0 = ctx.push_vertex(Vertex::new(pt3(0.0, 0.0, 0.0), tol));
        let v1 = ctx.push_vertex(Vertex::new(pt3(1.0, 0.0, 0.0), tol));
        let v2 = ctx.push_vertex(Vertex::new(pt3(1.0, 1.0, 0.0), tol));
        let v3 = ctx.push_vertex(Vertex::new(pt3(0.0, 1.0, 0.0), tol));

        // Curves + edges
        let mk_edge = |ctx: &mut SolidModelingContext, a: Point3, b: Point3, va: VertexId, vb: VertexId| {
            let c = ctx.push_curve3(Curve3Kind::Line3(Line3::new(a, b)));
            ctx.push_edge(Edge::new(c, va, vb, 0.0, 1.0))
        };
        let pts = [pt3(0.,0.,0.), pt3(1.,0.,0.), pt3(1.,1.,0.), pt3(0.,1.,0.)];
        let e01 = mk_edge(ctx, pts[0], pts[1], v0, v1);
        let e12 = mk_edge(ctx, pts[1], pts[2], v1, v2);
        let e23 = mk_edge(ctx, pts[2], pts[3], v2, v3);
        let e30 = mk_edge(ctx, pts[3], pts[0], v3, v0);

        // Topology skeleton (solid → shell → face placeholder → loop → coedges)
        let solid_id = ctx.push_solid(Solid::new(crate::brep_kernel::ShellId(usize::MAX)));
        let shell_id = ctx.push_shell(Shell::new(solid_id, true));
        ctx.get_mut_solid(solid_id).outer = shell_id;

        let surf_id = ctx.push_surface(SurfaceKind::Plane(Plane::new(
            pt3(0.,0.,0.), pt3(1.,0.,0.), pt3(0.,1.,0.),
        )));
        let prov = ProvenanceData::primitive(1, 1);
        let face_id = ctx.push_face(crate::brep_kernel::Face::new(
            shell_id, surf_id, LoopId(usize::MAX), FaceSense::Aligned, prov,
        ));

        let loop_id = ctx.push_loop(Loop::new(face_id, true));
        ctx.get_mut_face(face_id).outer = loop_id;

        // PCurves and coedges
        let mk_ce = |ctx: &mut SolidModelingContext, eid: EdgeId, uv0: Point2, uv1: Point2| {
            let pc = ctx.push_curve2(Curve2Kind::Line2(Line2::new(uv0, uv1)));
            let ce = ctx.push_coedge(CoEdge::new(eid, Orientation::Forward, face_id, pc));
            ctx.get_mut_edge(eid).coedges.push(ce);
            ctx.get_mut_loop(loop_id).coedges.push(ce);
        };
        mk_ce(ctx, e01, pt2(0.,0.), pt2(1.,0.));
        mk_ce(ctx, e12, pt2(1.,0.), pt2(1.,1.));
        mk_ce(ctx, e23, pt2(1.,1.), pt2(0.,1.));
        mk_ce(ctx, e30, pt2(0.,1.), pt2(0.,0.));

        ctx.get_mut_shell(shell_id).faces.push(face_id);

        (solid_id, shell_id, face_id, [v0, v1, v2, v3], [e01, e12, e23, e30])
    }

    /// Verify that a loop forms a closed chain: end of each coedge == start of next.
    fn assert_loop_closed(ctx: &SolidModelingContext, loop_id: LoopId) {
        let ces: Vec<CoEdgeId> = ctx.get_loop(loop_id).coedges.clone();
        let n = ces.len();
        assert!(n >= 3, "loop must have at least 3 coedges, got {n}");
        for i in 0..n {
            let end  = coedge_end(ctx, ces[i]);
            let start_ce = ces[(i + 1) % n];
            let start = {
                let ce = ctx.get_coedge(start_ce);
                let edge = ctx.get_edge(ce.edge);
                match ce.orientation {
                    Orientation::Forward => edge.v0,
                    Orientation::Reverse => edge.v1,
                }
            };
            assert_eq!(end, start,
                "loop {loop_id:?}: coedge {i} ends at {end:?} but coedge {} starts at {start:?}",
                (i + 1) % n);
        }
    }

    // ── split_edge tests ──────────────────────────────────────────────────────

    #[test]
    fn split_edge_produces_midpoint_vertex() {
        let mut ctx = SolidModelingContext::new();
        let (_, _, _, [v0, v1, _, _], [e01, ..]) = make_square(&mut ctx);

        let (new_v, ea, eb) = split_edge(&mut ctx, e01, 0.5);

        let mid = ctx.get_vertex(new_v).point;
        assert!((mid.x - 0.5).abs() < 1e-12);
        assert_eq!(mid.y, 0.0);
        assert_eq!(mid.z, 0.0);

        // Sub-edge connectivity
        assert_eq!(ctx.get_edge(ea).v0, v0);
        assert_eq!(ctx.get_edge(ea).v1, new_v);
        assert_eq!(ctx.get_edge(eb).v0, new_v);
        assert_eq!(ctx.get_edge(eb).v1, v1);
    }

    #[test]
    fn split_edge_updates_t_ranges() {
        let mut ctx = SolidModelingContext::new();
        let (_, _, _, _, [e01, ..]) = make_square(&mut ctx);

        let (_, ea, eb) = split_edge(&mut ctx, e01, 0.25);

        assert_eq!(ctx.get_edge(ea).t0, 0.0);
        assert!((ctx.get_edge(ea).t1 - 0.25).abs() < 1e-12);
        assert!((ctx.get_edge(eb).t0 - 0.25).abs() < 1e-12);
        assert_eq!(ctx.get_edge(eb).t1, 1.0);
    }

    #[test]
    fn split_edge_loop_grows_by_one() {
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_id, _, [e01, ..]) = make_square(&mut ctx);
        let loop_id = ctx.get_face(face_id).outer;

        assert_eq!(ctx.get_loop(loop_id).coedges.len(), 4);
        split_edge(&mut ctx, e01, 0.5);
        assert_eq!(ctx.get_loop(loop_id).coedges.len(), 5);
    }

    #[test]
    fn split_edge_loop_remains_closed() {
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_id, _, [e01, ..]) = make_square(&mut ctx);

        split_edge(&mut ctx, e01, 0.5);

        let loop_id = ctx.get_face(face_id).outer;
        assert_loop_closed(&ctx, loop_id);
    }

    #[test]
    fn split_edge_each_sub_edge_has_one_coedge() {
        let mut ctx = SolidModelingContext::new();
        let (_, _, _, _, [e01, ..]) = make_square(&mut ctx);

        let (_, ea, eb) = split_edge(&mut ctx, e01, 0.5);

        assert_eq!(ctx.get_edge(ea).coedges.len(), 1);
        assert_eq!(ctx.get_edge(eb).coedges.len(), 1);
    }

    #[test]
    fn split_edge_arbitrary_t() {
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_id, _, [e01, ..]) = make_square(&mut ctx);

        split_edge(&mut ctx, e01, 0.75);

        let loop_id = ctx.get_face(face_id).outer;
        assert_loop_closed(&ctx, loop_id);
        assert_eq!(ctx.get_loop(loop_id).coedges.len(), 5);
    }

    // ── split_face tests ──────────────────────────────────────────────────────

    #[test]
    fn split_face_shell_has_two_faces() {
        let mut ctx = SolidModelingContext::new();
        let (_, shell_id, face_id, _, [e01, _, e23, _]) = make_square(&mut ctx);

        let (fa, fb) = split_face(&mut ctx, face_id, e01, 0.5, e23, 0.5);

        let shell_faces = &ctx.get_shell(shell_id).faces;
        assert_eq!(shell_faces.len(), 2);
        assert!(shell_faces.contains(&fa));
        assert!(shell_faces.contains(&fb));
        assert!(!shell_faces.contains(&face_id));
    }

    #[test]
    fn split_face_both_loops_closed() {
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_id, _, [e01, _, e23, _]) = make_square(&mut ctx);

        let (fa, fb) = split_face(&mut ctx, face_id, e01, 0.5, e23, 0.5);

        assert_loop_closed(&ctx, ctx.get_face(fa).outer);
        assert_loop_closed(&ctx, ctx.get_face(fb).outer);
    }

    #[test]
    fn split_face_both_loops_have_four_coedges() {
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_id, _, [e01, _, e23, _]) = make_square(&mut ctx);

        let (fa, fb) = split_face(&mut ctx, face_id, e01, 0.5, e23, 0.5);

        assert_eq!(ctx.get_loop(ctx.get_face(fa).outer).coedges.len(), 4);
        assert_eq!(ctx.get_loop(ctx.get_face(fb).outer).coedges.len(), 4);
    }

    #[test]
    fn split_face_split_edge_has_two_coedges_opposite_orientations() {
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_id, _, [e01, _, e23, _]) = make_square(&mut ctx);

        split_face(&mut ctx, face_id, e01, 0.5, e23, 0.5);

        // The last edge pushed is the split edge (most recently created).
        let split_eid = EdgeId(ctx.edges.len() - 1);
        let ces = &ctx.get_edge(split_eid).coedges;
        assert_eq!(ces.len(), 2);
        let ori0 = ctx.get_coedge(ces[0]).orientation;
        let ori1 = ctx.get_coedge(ces[1]).orientation;
        assert_ne!(ori0, ori1, "split edge coedges must have opposite orientations");
    }

    #[test]
    fn split_face_off_center() {
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_id, _, [e01, _, e23, _]) = make_square(&mut ctx);

        let (fa, fb) = split_face(&mut ctx, face_id, e01, 0.25, e23, 0.75);

        assert_loop_closed(&ctx, ctx.get_face(fa).outer);
        assert_loop_closed(&ctx, ctx.get_face(fb).outer);
    }

    #[test]
    fn split_face_using_adjacent_edges() {
        // Split along e01 and e12 — adjacent edges; produces a triangle and a pentagon.
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_id, _, [e01, e12, _, _]) = make_square(&mut ctx);

        let (fa, fb) = split_face(&mut ctx, face_id, e01, 0.5, e12, 0.5);

        assert_loop_closed(&ctx, ctx.get_face(fa).outer);
        assert_loop_closed(&ctx, ctx.get_face(fb).outer);

        // One face gets 3 coedges (triangle), the other gets 5 (pentagon).
        let na = ctx.get_loop(ctx.get_face(fa).outer).coedges.len();
        let nb = ctx.get_loop(ctx.get_face(fb).outer).coedges.len();
        let (small, large) = if na < nb { (na, nb) } else { (nb, na) };
        assert_eq!(small, 3);
        assert_eq!(large, 5);
    }

    #[test]
    fn split_face_coedges_reference_correct_faces() {
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_id, _, [e01, _, e23, _]) = make_square(&mut ctx);

        let (fa, fb) = split_face(&mut ctx, face_id, e01, 0.5, e23, 0.5);

        for &ce in &ctx.get_loop(ctx.get_face(fa).outer).coedges.clone() {
            assert_eq!(ctx.get_coedge(ce).face, fa,
                "coedge in face_a loop should reference face_a");
        }
        for &ce in &ctx.get_loop(ctx.get_face(fb).outer).coedges.clone() {
            assert_eq!(ctx.get_coedge(ce).face, fb,
                "coedge in face_b loop should reference face_b");
        }
    }

    #[test]
    fn split_face_split_vertex_at_correct_position() {
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_id, _, [e01, _, e23, _]) = make_square(&mut ctx);

        split_face(&mut ctx, face_id, e01, 0.5, e23, 0.5);

        // Entry split vertex: midpoint of E01 = (0.5, 0.0, 0.0)
        // Exit split vertex:  midpoint of E23 = midpoint of (1,1,0)→(0,1,0) = (0.5, 1.0, 0.0)
        // New vertices are the last two pushed (before the split edge vertices).
        // Simpler: check by finding vertices with expected positions.
        let mut found_entry = false;
        let mut found_exit  = false;
        for v in &ctx.vertices {
            if (v.point.x - 0.5).abs() < 1e-12 && v.point.y.abs() < 1e-12 { found_entry = true; }
            if (v.point.x - 0.5).abs() < 1e-12 && (v.point.y - 1.0).abs() < 1e-12 { found_exit = true; }
        }
        assert!(found_entry, "entry split vertex at (0.5, 0, 0) not found");
        assert!(found_exit,  "exit split vertex at (0.5, 1, 0) not found");
    }

    // ── classify_face_wrt_node tests ──────────────────────────────────────────
    //
    // make_square builds a unit square face in the XY-plane at z = 0.
    // Its boundary vertices are V0=(0,0,0), V1=(1,0,0), V2=(1,1,0), V3=(0,1,0).
    // Centroid = (0.5, 0.5, 0.0).

    #[test]
    fn face_centroid_is_vertex_average() {
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_id, _, _) = make_square(&mut ctx);

        let c = face_centroid(&ctx, face_id);
        assert!((c.x - 0.5).abs() < 1e-12);
        assert!((c.y - 0.5).abs() < 1e-12);
        assert!(c.z.abs() < 1e-12);
    }

    #[test]
    fn face_centroid_after_split_face() {
        // After splitting the square midway through E01 and E23, face A has vertices
        // (0,0,0), (0.5,0,0), (0.5,1,0), (0,1,0) → centroid (0.25, 0.5, 0).
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_id, _, [e01, _, e23, _]) = make_square(&mut ctx);
        let (fa, _fb) = split_face(&mut ctx, face_id, e01, 0.5, e23, 0.5);

        let c = face_centroid(&ctx, fa);
        assert!((c.x - 0.25).abs() < 1e-12, "cx = {}", c.x);
        assert!((c.y - 0.5 ).abs() < 1e-12, "cy = {}", c.y);
        assert!(c.z.abs()          < 1e-12, "cz = {}", c.z);
    }

    #[test]
    fn classify_face_outside_primitive() {
        // Centroid (0.5, 0.5, 0.0) vs a sphere of radius 0.1 centred at origin:
        // distance² = 0.5 >> r² = 0.01 → Outside.
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_id, _, _) = make_square(&mut ctx);
        let node = crate::csg_lang::CsgNode::sphere(0.1);

        assert_eq!(
            classify_face_wrt_node(&ctx, face_id, &node),
            crate::predicates::Classification::Outside,
        );
    }

    #[test]
    fn classify_face_inside_primitive() {
        // Centroid (0.5, 0.5, 0.0) vs a sphere of radius 10: well inside.
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_id, _, _) = make_square(&mut ctx);
        let node = crate::csg_lang::CsgNode::sphere(10.0);

        assert_eq!(
            classify_face_wrt_node(&ctx, face_id, &node),
            crate::predicates::Classification::Inside,
        );
    }

    #[test]
    fn classify_face_outside_cuboid_shifted_away() {
        // Unit cuboid shifted up to z∈[2,3]: centroid (0.5,0.5,0) is below → Outside.
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_id, _, _) = make_square(&mut ctx);
        let node = crate::csg_lang::CsgNode::cuboid(1.0, 1.0, 1.0).translate(0.0, 0.0, 2.0);

        assert_eq!(
            classify_face_wrt_node(&ctx, face_id, &node),
            crate::predicates::Classification::Outside,
        );
    }

    #[test]
    fn classify_face_inside_large_cuboid() {
        // Large cuboid [-5,5]³: centroid (0.5,0.5,0) is strictly interior → Inside.
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_id, _, _) = make_square(&mut ctx);
        let node = crate::csg_lang::CsgNode::cuboid(10.0, 10.0, 10.0).translate(-5.0, -5.0, -5.0);

        assert_eq!(
            classify_face_wrt_node(&ctx, face_id, &node),
            crate::predicates::Classification::Inside,
        );
    }

    #[test]
    fn classify_face_on_surface_is_indeterminate() {
        // Unit cuboid [0,1]³: centroid (0.5,0.5,0) lies on the z=0 face → Indeterminate.
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_id, _, _) = make_square(&mut ctx);
        let node = crate::csg_lang::CsgNode::cuboid(1.0, 1.0, 1.0);

        assert_eq!(
            classify_face_wrt_node(&ctx, face_id, &node),
            crate::predicates::Classification::Indeterminate,
        );
    }

    #[test]
    fn classify_face_fragment_inside() {
        // Split the square midway; face B has centroid (0.75, 0.5, 0).
        // Large cuboid [-5,5]³ → Inside.
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_id, _, [e01, _, e23, _]) = make_square(&mut ctx);
        let (_fa, fb) = split_face(&mut ctx, face_id, e01, 0.5, e23, 0.5);
        let node = crate::csg_lang::CsgNode::cuboid(10.0, 10.0, 10.0).translate(-5.0, -5.0, -5.0);

        assert_eq!(
            classify_face_wrt_node(&ctx, fb, &node),
            crate::predicates::Classification::Inside,
        );
    }

    // ── intersect_planar_faces tests ──────────────────────────────────────────
    //
    // Face A: unit square [0,1]×[0,1] at z=0   (make_square)
    // Face B: unit square [0,1]×[-0.5,0.5] at y=0.5, standing vertically
    //         plane: origin=(0,0.5,0), u_dir=(1,0,0), v_dir=(0,0,1)
    //         normal = u_dir × v_dir = (0,-1,0)
    //         The plane-plane intersection line is y=0.5, z=0, running in +x.
    //
    // Helper: build the vertical face in an existing ctx.
    fn make_vertical_square(
        ctx: &mut SolidModelingContext,
    ) -> (crate::brep_kernel::SolidId, ShellId, FaceId, [EdgeId; 4]) {
        let tol = ctx.tolerance.pos_tol;
        let vb0 = ctx.push_vertex(Vertex::new(pt3(0.0, 0.5, -0.5), tol));
        let vb1 = ctx.push_vertex(Vertex::new(pt3(1.0, 0.5, -0.5), tol));
        let vb2 = ctx.push_vertex(Vertex::new(pt3(1.0, 0.5,  0.5), tol));
        let vb3 = ctx.push_vertex(Vertex::new(pt3(0.0, 0.5,  0.5), tol));

        let mk_e = |ctx: &mut SolidModelingContext, a: Point3, b: Point3, va: VertexId, vb: VertexId| {
            let c = ctx.push_curve3(Curve3Kind::Line3(Line3::new(a, b)));
            ctx.push_edge(Edge::new(c, va, vb, 0.0, 1.0))
        };
        let eb01 = mk_e(ctx, pt3(0.,0.5,-0.5), pt3(1.,0.5,-0.5), vb0, vb1);
        let eb12 = mk_e(ctx, pt3(1.,0.5,-0.5), pt3(1.,0.5, 0.5), vb1, vb2);
        let eb23 = mk_e(ctx, pt3(1.,0.5, 0.5), pt3(0.,0.5, 0.5), vb2, vb3);
        let eb30 = mk_e(ctx, pt3(0.,0.5, 0.5), pt3(0.,0.5,-0.5), vb3, vb0);

        let solid_id = ctx.push_solid(crate::brep_kernel::Solid::new(crate::brep_kernel::ShellId(usize::MAX)));
        let shell_id = ctx.push_shell(crate::brep_kernel::Shell::new(solid_id, true));
        ctx.get_mut_solid(solid_id).outer = shell_id;

        let surf_id = ctx.push_surface(SurfaceKind::Plane(crate::geom::Plane::new(
            pt3(0.,0.5,0.), pt3(1.,0.,0.), pt3(0.,0.,1.),
        )));
        let prov = crate::brep_kernel::ProvenanceData::primitive(2, 1);
        let face_id = ctx.push_face(crate::brep_kernel::Face::new(
            shell_id, surf_id, LoopId(usize::MAX), crate::brep_kernel::FaceSense::Aligned, prov,
        ));
        let loop_id = ctx.push_loop(Loop::new(face_id, true));
        ctx.get_mut_face(face_id).outer = loop_id;

        let mk_ce = |ctx: &mut SolidModelingContext, eid: EdgeId, uv0: Point2, uv1: Point2| {
            let pc = ctx.push_curve2(Curve2Kind::Line2(Line2::new(uv0, uv1)));
            let ce = ctx.push_coedge(CoEdge::new(eid, Orientation::Forward, face_id, pc));
            ctx.get_mut_edge(eid).coedges.push(ce);
            ctx.get_mut_loop(loop_id).coedges.push(ce);
        };
        mk_ce(ctx, eb01, pt2(0.,-0.5), pt2(1.,-0.5));
        mk_ce(ctx, eb12, pt2(1.,-0.5), pt2(1., 0.5));
        mk_ce(ctx, eb23, pt2(1., 0.5), pt2(0., 0.5));
        mk_ce(ctx, eb30, pt2(0., 0.5), pt2(0.,-0.5));

        ctx.get_mut_shell(shell_id).faces.push(face_id);
        (solid_id, shell_id, face_id, [eb01, eb12, eb23, eb30])
    }

    #[test]
    fn ssi_parallel_planes_returns_none() {
        // Two parallel XY-plane faces → no intersection line.
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_a, _, _) = make_square(&mut ctx);

        // Second face: unit square at z=1.
        let tol = ctx.tolerance.pos_tol;
        let vb0 = ctx.push_vertex(Vertex::new(pt3(0.,0.,1.), tol));
        let vb1 = ctx.push_vertex(Vertex::new(pt3(1.,0.,1.), tol));
        let vb2 = ctx.push_vertex(Vertex::new(pt3(1.,1.,1.), tol));
        let vb3 = ctx.push_vertex(Vertex::new(pt3(0.,1.,1.), tol));
        let sol2 = ctx.push_solid(crate::brep_kernel::Solid::new(ShellId(usize::MAX)));
        let sh2  = ctx.push_shell(crate::brep_kernel::Shell::new(sol2, true));
        ctx.get_mut_solid(sol2).outer = sh2;
        let surf2 = ctx.push_surface(SurfaceKind::Plane(crate::geom::Plane::new(
            pt3(0.,0.,1.), pt3(1.,0.,0.), pt3(0.,1.,0.),
        )));
        let prov2 = crate::brep_kernel::ProvenanceData::primitive(2, 1);
        let face_b = ctx.push_face(crate::brep_kernel::Face::new(
            sh2, surf2, LoopId(usize::MAX), crate::brep_kernel::FaceSense::Aligned, prov2,
        ));
        let loop2 = ctx.push_loop(Loop::new(face_b, true));
        ctx.get_mut_face(face_b).outer = loop2;
        let mk = |ctx: &mut SolidModelingContext, a: VertexId, b: VertexId, pa: Point3, pb: Point3,
                  uv0: Point2, uv1: Point2| {
            let c3  = ctx.push_curve3(Curve3Kind::Line3(Line3::new(pa, pb)));
            let eid = ctx.push_edge(Edge::new(c3, a, b, 0.0, 1.0));
            let pc  = ctx.push_curve2(Curve2Kind::Line2(Line2::new(uv0, uv1)));
            let ce  = ctx.push_coedge(CoEdge::new(eid, Orientation::Forward, face_b, pc));
            ctx.get_mut_edge(eid).coedges.push(ce);
            ctx.get_mut_loop(loop2).coedges.push(ce);
        };
        mk(&mut ctx, vb0, vb1, pt3(0.,0.,1.), pt3(1.,0.,1.), pt2(0.,0.), pt2(1.,0.));
        mk(&mut ctx, vb1, vb2, pt3(1.,0.,1.), pt3(1.,1.,1.), pt2(1.,0.), pt2(1.,1.));
        mk(&mut ctx, vb2, vb3, pt3(1.,1.,1.), pt3(0.,1.,1.), pt2(1.,1.), pt2(0.,1.));
        mk(&mut ctx, vb3, vb0, pt3(0.,1.,1.), pt3(0.,0.,1.), pt2(0.,1.), pt2(0.,0.));

        assert!(intersect_planar_faces(&ctx, face_a, face_b).is_none());
    }

    #[test]
    fn ssi_crossing_faces_returns_some() {
        // Face A (XY) × Face B (vertical at y=0.5): intersection line y=0.5, z=0.
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_a, _, _) = make_square(&mut ctx);
        let (_, _, face_b, _) = make_vertical_square(&mut ctx);

        assert!(intersect_planar_faces(&ctx, face_a, face_b).is_some());
    }

    #[test]
    fn ssi_crossing_faces_hits_on_correct_edges() {
        // Face A clip: line enters via E30 (x=0 edge) and exits via E12 (x=1 edge).
        // Face B clip: line enters via left edge (x=0) and exits via right edge (x=1).
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_a, _, [_e01, e12, _e23, e30]) = make_square(&mut ctx);
        let (_, _, face_b, [_eb01, eb12, _eb23, eb30]) = make_vertical_square(&mut ctx);

        let result = intersect_planar_faces(&ctx, face_a, face_b)
            .expect("crossing faces must intersect");

        // hits_a: entry at E30 (x=0 edge) t_edge=0.5, exit at E12 (x=1 edge) t_edge=0.5
        let ha = &result.hits_a;
        let small_t = ha.iter().min_by(|a, b| a.t_line.partial_cmp(&b.t_line).unwrap()).unwrap();
        let large_t = ha.iter().max_by(|a, b| a.t_line.partial_cmp(&b.t_line).unwrap()).unwrap();
        assert_eq!(small_t.edge_id, e30, "entry hit should be on E30 (x=0 side)");
        assert_eq!(large_t.edge_id, e12, "exit hit should be on E12 (x=1 side)");
        assert!((small_t.t_edge - 0.5).abs() < 1e-10, "E30 t_edge should be 0.5");
        assert!((large_t.t_edge - 0.5).abs() < 1e-10, "E12 t_edge should be 0.5");

        // hits_b: entry on eb30 (left/x=0 edge), exit on eb12 (right/x=1 edge)
        let hb = &result.hits_b;
        let small_tb = hb.iter().min_by(|a, b| a.t_line.partial_cmp(&b.t_line).unwrap()).unwrap();
        let large_tb = hb.iter().max_by(|a, b| a.t_line.partial_cmp(&b.t_line).unwrap()).unwrap();
        assert_eq!(small_tb.edge_id, eb30, "entry hit on face B should be on eb30 (x=0 side)");
        assert_eq!(large_tb.edge_id, eb12, "exit hit on face B should be on eb12 (x=1 side)");
    }

    #[test]
    fn ssi_hit_points_lie_on_intersection_line() {
        // All four hit points should have y=0.5, z=0 (the intersection line).
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_a, _, _) = make_square(&mut ctx);
        let (_, _, face_b, _) = make_vertical_square(&mut ctx);

        let result = intersect_planar_faces(&ctx, face_a, face_b).unwrap();

        for hit in result.hits_a.iter().chain(result.hits_b.iter()) {
            assert!((hit.point.y - 0.5).abs() < 1e-10, "point.y = {}", hit.point.y);
            assert!(hit.point.z.abs() < 1e-10,          "point.z = {}", hit.point.z);
        }
    }

    #[test]
    fn ssi_non_overlapping_faces_returns_none() {
        // Face A: unit square at z=0, x∈[0,1].
        // Face B: vertical square at y=0.5 but x∈[2,3] — no x-overlap.
        let mut ctx = SolidModelingContext::new();
        let (_, _, face_a, _, _) = make_square(&mut ctx);

        let tol = ctx.tolerance.pos_tol;
        let vb0 = ctx.push_vertex(Vertex::new(pt3(2.,0.5,-0.5), tol));
        let vb1 = ctx.push_vertex(Vertex::new(pt3(3.,0.5,-0.5), tol));
        let vb2 = ctx.push_vertex(Vertex::new(pt3(3.,0.5, 0.5), tol));
        let vb3 = ctx.push_vertex(Vertex::new(pt3(2.,0.5, 0.5), tol));
        let sol2 = ctx.push_solid(crate::brep_kernel::Solid::new(ShellId(usize::MAX)));
        let sh2  = ctx.push_shell(crate::brep_kernel::Shell::new(sol2, true));
        ctx.get_mut_solid(sol2).outer = sh2;
        let surf2 = ctx.push_surface(SurfaceKind::Plane(crate::geom::Plane::new(
            pt3(0.,0.5,0.), pt3(1.,0.,0.), pt3(0.,0.,1.),
        )));
        let prov2 = crate::brep_kernel::ProvenanceData::primitive(2, 1);
        let face_b = ctx.push_face(crate::brep_kernel::Face::new(
            sh2, surf2, LoopId(usize::MAX), crate::brep_kernel::FaceSense::Aligned, prov2,
        ));
        let loop2 = ctx.push_loop(Loop::new(face_b, true));
        ctx.get_mut_face(face_b).outer = loop2;
        let mk = |ctx: &mut SolidModelingContext, va: VertexId, vb: VertexId,
                  pa: Point3, pb: Point3, uv0: Point2, uv1: Point2| {
            let c3  = ctx.push_curve3(Curve3Kind::Line3(Line3::new(pa, pb)));
            let eid = ctx.push_edge(Edge::new(c3, va, vb, 0.0, 1.0));
            let pc  = ctx.push_curve2(Curve2Kind::Line2(Line2::new(uv0, uv1)));
            let ce  = ctx.push_coedge(CoEdge::new(eid, Orientation::Forward, face_b, pc));
            ctx.get_mut_edge(eid).coedges.push(ce);
            ctx.get_mut_loop(loop2).coedges.push(ce);
        };
        mk(&mut ctx, vb0, vb1, pt3(2.,0.5,-0.5), pt3(3.,0.5,-0.5), pt2(2.,-0.5), pt2(3.,-0.5));
        mk(&mut ctx, vb1, vb2, pt3(3.,0.5,-0.5), pt3(3.,0.5, 0.5), pt2(3.,-0.5), pt2(3., 0.5));
        mk(&mut ctx, vb2, vb3, pt3(3.,0.5, 0.5), pt3(2.,0.5, 0.5), pt2(3., 0.5), pt2(2., 0.5));
        mk(&mut ctx, vb3, vb0, pt3(2.,0.5, 0.5), pt3(2.,0.5,-0.5), pt2(2., 0.5), pt2(2.,-0.5));

        assert!(intersect_planar_faces(&ctx, face_a, face_b).is_none(),
            "x-separated faces must not intersect");
    }
}
