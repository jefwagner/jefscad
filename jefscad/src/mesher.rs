//! Tessellation: converts B-rep solids into triangle meshes.

use crate::brep_kernel::{FaceId, FaceSense, LoopId, Orientation, SolidId, SolidModelingContext};
use crate::geom::{ConicalSurface, Curve2, Curve2Kind, CylindricalSurface, Plane, Point3, Surface, SurfaceKind};

// ── TriMesh ───────────────────────────────────────────────────────────────────

/// A triangle mesh produced by tessellating a B-rep solid.
///
/// # Layout
/// - `vertices[i]` — 3-D position of the i-th mesh vertex.
/// - `triangles[t]` — indices `[a, b, c]` into `vertices` for triangle `t`.
/// - `tri_normals[t*3 + k]` — surface normal at corner `k` of triangle `t`.
/// - `tri_uvs[t*3 + k]` — surface UV parameter at corner `k` of triangle `t`.
///
/// # Invariants
/// `tri_normals.len() == triangles.len() * 3`
/// `tri_uvs.len()     == triangles.len() * 3`
/// Every index in `triangles` is `< vertices.len()`.
///
/// # Normals and UV
/// Shared vertex indices mean shared positions, so edge connectivity is directly
/// readable from `triangles`.  Normals and UVs are per-triangle-corner so sharp
/// edges and smooth surfaces are both representable without duplicating vertices.
///
/// UV values are the raw surface parameters (e.g. angle in radians for the u-axis
/// of a cylinder).  Known limitation: seam vertices on periodic surfaces carry a
/// single UV value; duplicating seam vertices for texture-atlas use is deferred.
#[derive(Debug, Default, Clone)]
pub struct TriMesh {
    /// 3-D vertex positions.
    pub vertices:    Vec<[f32; 3]>,
    /// Index triples — each triple defines one triangle.
    pub triangles:   Vec<[u32; 3]>,
    /// Per-triangle-corner normals; `tri_normals[t*3 + k]` for triangle `t`, corner `k`.
    pub tri_normals: Vec<[f32; 3]>,
    /// Per-triangle-corner UV params; `tri_uvs[t*3 + k]` for triangle `t`, corner `k`.
    pub tri_uvs:     Vec<[f32; 2]>,
}

// ── MeshOptions ───────────────────────────────────────────────────────────────

/// Options controlling tessellation quality.
#[derive(Debug, Clone, Copy)]
pub struct MeshOptions {
    /// Number of segments per full circle (360°).  Higher values give smoother
    /// curved surfaces at the cost of more triangles.  Default: 32.
    pub resolution: u32,
}

impl Default for MeshOptions {
    fn default() -> Self {
        Self { resolution: 32 }
    }
}

// ── mesh_solid ────────────────────────────────────────────────────────────────

/// Tessellate all faces of solid `sid` and return a combined [`TriMesh`].
///
/// Each face is tessellated independently by the internal `mesh_face` function.
/// The resulting per-face meshes are concatenated with triangle indices adjusted
/// to the global vertex offset.
pub fn mesh_solid(ctx: &SolidModelingContext, sid: SolidId, opts: &MeshOptions) -> TriMesh {
    let shell_id = ctx.get_solid(sid).outer;
    let face_ids: Vec<FaceId> = ctx.get_shell(shell_id).faces.clone();

    let mut out = TriMesh::default();

    for face_id in face_ids {
        let offset = out.vertices.len() as u32;
        let face_mesh = mesh_face(ctx, face_id, opts);

        out.vertices.extend_from_slice(&face_mesh.vertices);
        out.tri_normals.extend_from_slice(&face_mesh.tri_normals);
        out.tri_uvs.extend_from_slice(&face_mesh.tri_uvs);

        for tri in &face_mesh.triangles {
            out.triangles.push([tri[0] + offset, tri[1] + offset, tri[2] + offset]);
        }
    }

    out
}

// ── mesh_face ─────────────────────────────────────────────────────────────────

/// Tessellate a single face, returning a local [`TriMesh`] with indices starting
/// at 0.  [`mesh_solid`] adjusts the indices to the global vertex offset.
fn mesh_face(ctx: &SolidModelingContext, face_id: FaceId, opts: &MeshOptions) -> TriMesh {
    let face = ctx.get_face(face_id);
    let surf_id = face.surface;
    match ctx.get_surface(surf_id) {
        SurfaceKind::Plane(plane)    => mesh_plane_face(ctx, face_id, *plane, opts),
        SurfaceKind::Cylinder(cyl)   => mesh_cylindrical_face(ctx, face_id, *cyl, opts),
        SurfaceKind::Cone(cone)      => mesh_conical_face(ctx, face_id, *cone, opts),
        _ => TriMesh::default(), // other surface types: stub until Step 2 continues
    }
}

// ── Plane tessellation ────────────────────────────────────────────────────────

/// Tessellate a face whose surface is a [`Plane`].
///
/// Samples the outer loop's coedge pcurves to get UV boundary points, then
/// triangulates with a fan from `boundary[0]` (correct for all convex polygons —
/// rectangles and circles are both convex).
///
/// Normals are taken from [`Plane::eval_n`] and negated for [`FaceSense::AntiAligned`]
/// faces.  The same constant normal is stored at every triangle corner (flat shading).
fn mesh_plane_face(
    ctx: &SolidModelingContext,
    face_id: FaceId,
    plane: Plane,
    opts: &MeshOptions,
) -> TriMesh {
    let face = ctx.get_face(face_id);
    let sense = face.sense;
    let loop_id = face.outer;

    // Sample boundary in UV space
    let uvs = sample_loop_uvs(ctx, loop_id, opts);
    let n = uvs.len();
    if n < 3 {
        return TriMesh::default();
    }

    // Outward-facing normal (constant over the plane)
    let raw_n = plane.eval_n(0.0, 0.0).unwrap();
    let out_n = if sense == FaceSense::AntiAligned {
        Point3::new(-raw_n.x, -raw_n.y, -raw_n.z)
    } else {
        raw_n
    };
    let normal = [out_n.x as f32, out_n.y as f32, out_n.z as f32];

    // 3-D vertex positions from surface eval
    let vertices: Vec<[f32; 3]> = uvs.iter().map(|&[u, v]| {
        let p = plane.eval(u, v);
        [p.x as f32, p.y as f32, p.z as f32]
    }).collect();

    // Fan triangulation from vertex 0: triangles (0, i, i+1) for i in 1..n-1
    let mut triangles  = Vec::with_capacity(n - 2);
    let mut tri_normals = Vec::with_capacity((n - 2) * 3);
    let mut tri_uvs    = Vec::with_capacity((n - 2) * 3);

    for i in 1..=(n - 2) {
        triangles.push([0u32, i as u32, (i + 1) as u32]);
        for &corner in &[0, i, i + 1] {
            tri_normals.push(normal);
            tri_uvs.push([uvs[corner][0] as f32, uvs[corner][1] as f32]);
        }
    }

    TriMesh { vertices, triangles, tri_normals, tri_uvs }
}

// ── CylindricalSurface tessellation ──────────────────────────────────────────

/// Tessellate the lateral face of a [`CylindricalSurface`].
///
/// Builds a `(resolution+1) × 2` UV grid (u sweeps 0→2π in `resolution` steps,
/// two v rows at v_min and v_max derived from the face loop).  Each column strip
/// becomes two triangles.  Normals are the analytic radial normal at each vertex.
///
/// The first and last columns share the same 3-D positions (the seam) but carry
/// different UV values (u=0 vs u=2π) — consistent with the documented seam
/// limitation.
fn mesh_cylindrical_face(
    ctx: &SolidModelingContext,
    face_id: FaceId,
    cyl: CylindricalSurface,
    opts: &MeshOptions,
) -> TriMesh {
    use std::f64::consts::TAU;

    // Derive v range from the boundary UV samples.
    let loop_id = ctx.get_face(face_id).outer;
    let boundary = sample_loop_uvs(ctx, loop_id, opts);
    let v_min = boundary.iter().map(|uv| uv[1]).fold(f64::INFINITY,  f64::min);
    let v_max = boundary.iter().map(|uv| uv[1]).fold(f64::NEG_INFINITY, f64::max);

    let res = opts.resolution as usize;
    let nu  = res + 1;      // columns: u = 0 … 2π (inclusive)
    let nv  = 2usize;       // rows:    v_min, v_max
    let v_vals = [v_min, v_max];

    // Build vertices, normals, UVs
    let mut vertices    = Vec::with_capacity(nu * nv);
    let mut vert_norms  = Vec::with_capacity(nu * nv); // one normal per vertex (reused per corner)
    let mut vert_uvs    = Vec::with_capacity(nu * nv);

    for vi in 0..nv {
        let v = v_vals[vi];
        for ui in 0..nu {
            let u = ui as f64 * TAU / res as f64;
            let p = cyl.eval(u, v);
            vertices.push([p.x as f32, p.y as f32, p.z as f32]);
            let n = cyl.eval_n(u, v).expect("CylindricalSurface normal is always defined");
            vert_norms.push([n.x as f32, n.y as f32, n.z as f32]);
            vert_uvs.push([u as f32, v as f32]);
        }
    }

    // Triangulate: res strips, each split into 2 triangles
    //   BL = vi=0, ui=col     BR = vi=0, ui=col+1
    //   TL = vi=1, ui=col     TR = vi=1, ui=col+1
    //   Triangles: (BL, BR, TR) and (BL, TR, TL)  — outward winding verified
    let mut triangles   = Vec::with_capacity(res * 2);
    let mut tri_normals = Vec::with_capacity(res * 2 * 3);
    let mut tri_uvs     = Vec::with_capacity(res * 2 * 3);

    let idx = |vi: usize, ui: usize| (vi * nu + ui) as u32;

    for col in 0..res {
        let bl = idx(0, col);
        let br = idx(0, col + 1);
        let tl = idx(1, col);
        let tr = idx(1, col + 1);

        for &tri in &[[bl, br, tr], [bl, tr, tl]] {
            triangles.push(tri);
            for &c in &tri {
                tri_normals.push(vert_norms[c as usize]);
                tri_uvs.push(vert_uvs[c as usize]);
            }
        }
    }

    TriMesh { vertices, triangles, tri_normals, tri_uvs }
}

// ── ConicalSurface tessellation ───────────────────────────────────────────────

/// Tessellate the lateral face of a [`ConicalSurface`].
///
/// Uses an apex-fan: 1 apex vertex + `resolution` base-circle vertices forming
/// `resolution` triangles.  Normals are computed via cross product (flat shading)
/// to avoid the singularity where [`ConicalSurface::eval_n`] returns `None` at v=0.
///
/// Triangle winding: `(apex, base_next, base_curr)` produces an outward-facing
/// cross product for [`FaceSense::Aligned`] faces.  The face sense is respected by
/// negating normals for [`FaceSense::AntiAligned`].
///
/// UV at each corner: apex → `(u_j, 0)`, base_next → `(u_{j+1}, v_max)`,
/// base_curr → `(u_j, v_max)`.  The last triangle uses `u = TAU` for base_next
/// instead of `0` to avoid a UV discontinuity at the seam.
fn mesh_conical_face(
    ctx: &SolidModelingContext,
    face_id: FaceId,
    cone: ConicalSurface,
    opts: &MeshOptions,
) -> TriMesh {
    use std::f64::consts::TAU;

    let face = ctx.get_face(face_id);
    let sense = face.sense;
    let loop_id = face.outer;

    // v_max = slant distance from apex to base circle
    let boundary = sample_loop_uvs(ctx, loop_id, opts);
    let v_max = boundary.iter().map(|uv| uv[1]).fold(f64::NEG_INFINITY, f64::max);

    let res = opts.resolution as usize;

    // Vertices: index 0 = apex, indices 1..=res = base circle
    let apex_pos = cone.eval(0.0, 0.0);
    let mut vertices = Vec::with_capacity(res + 1);
    vertices.push([apex_pos.x as f32, apex_pos.y as f32, apex_pos.z as f32]);

    let mut base_u = Vec::with_capacity(res);
    for j in 0..res {
        let u = j as f64 * TAU / res as f64;
        let p = cone.eval(u, v_max);
        vertices.push([p.x as f32, p.y as f32, p.z as f32]);
        base_u.push(u);
    }

    // Fan triangulation from apex
    let mut triangles   = Vec::with_capacity(res);
    let mut tri_normals = Vec::with_capacity(res * 3);
    let mut tri_uvs     = Vec::with_capacity(res * 3);

    for j in 0..res {
        let curr_idx = (j + 1) as u32;
        let next_idx = ((j + 1) % res + 1) as u32;

        let u_curr = base_u[j];
        let u_next = if j + 1 < res { base_u[j + 1] } else { TAU };

        // Outward normal via cross product: (base_next - apex) × (base_curr - apex)
        let bv_next = vertices[next_idx as usize];
        let bv_curr = vertices[curr_idx as usize];
        let v1 = Point3::new(
            bv_next[0] as f64 - apex_pos.x,
            bv_next[1] as f64 - apex_pos.y,
            bv_next[2] as f64 - apex_pos.z,
        );
        let v2 = Point3::new(
            bv_curr[0] as f64 - apex_pos.x,
            bv_curr[1] as f64 - apex_pos.y,
            bv_curr[2] as f64 - apex_pos.z,
        );
        let raw_n = v1.cross(v2);
        let len = (raw_n.x*raw_n.x + raw_n.y*raw_n.y + raw_n.z*raw_n.z).sqrt();
        let n = if len > 1e-15 {
            [raw_n.x/len, raw_n.y/len, raw_n.z/len]
        } else {
            [0.0, 0.0, 1.0] // degenerate fallback
        };
        let out_n: [f32; 3] = if sense == FaceSense::AntiAligned {
            [-n[0] as f32, -n[1] as f32, -n[2] as f32]
        } else {
            [n[0] as f32, n[1] as f32, n[2] as f32]
        };

        // Triangle: (apex, base_next, base_curr)
        triangles.push([0u32, next_idx, curr_idx]);
        // apex corner UV (use u_curr so UV matches the adjacent base edge)
        tri_uvs.push([u_curr as f32, 0.0f32]);
        tri_uvs.push([u_next as f32, v_max as f32]);
        tri_uvs.push([u_curr as f32, v_max as f32]);
        tri_normals.push(out_n);
        tri_normals.push(out_n);
        tri_normals.push(out_n);
    }

    TriMesh { vertices, triangles, tri_normals, tri_uvs }
}

// ── sample_loop_uvs ───────────────────────────────────────────────────────────

/// Walk the coedges of `loop_id` and return UV boundary sample points.
///
/// - `Line2` pcurves contribute one point: the coedge start (endpoint = next coedge start).
/// - `CircularArc2` pcurves contribute `resolution` evenly-spaced points from
///   `t_start` to `t_end` (exclusive of the endpoint, which is the next coedge start).
fn sample_loop_uvs(
    ctx: &SolidModelingContext,
    loop_id: LoopId,
    opts: &MeshOptions,
) -> Vec<[f64; 2]> {
    let coedge_ids = ctx.get_loop(loop_id).coedges.clone();
    let mut uvs = Vec::new();

    for ce_id in coedge_ids {
        let ce   = ctx.get_coedge(ce_id);
        let edge = ctx.get_edge(ce.edge);
        let (t_start, t_end) = match ce.orientation {
            Orientation::Forward => (edge.t0, edge.t1),
            Orientation::Reverse => (edge.t1, edge.t0),
        };
        let pcurve = ctx.get_curve2(ce.pcurve);
        match pcurve {
            Curve2Kind::Line2(_) => {
                // Straight edge: only the start vertex contributes
                let p = pcurve.eval(t_start);
                uvs.push([p.u, p.v]);
            }
            Curve2Kind::CircularArc2(_) => {
                // Curved edge: sample `resolution` points, endpoint excluded
                let n = opts.resolution as usize;
                let dt = (t_end - t_start) / n as f64;
                for k in 0..n {
                    let p = pcurve.eval(t_start + k as f64 * dt);
                    uvs.push([p.u, p.v]);
                }
            }
            Curve2Kind::Nurbs(_) => {
                todo!("UV sampling for NurbsCurve2 not yet implemented")
            }
        }
    }

    uvs
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    use super::*;
    use crate::brep_compiler::compile_csg_node;
    use crate::csg_lang::CsgNode;

    fn mesh_prim(node: &CsgNode) -> TriMesh {
        let mut ctx = SolidModelingContext::new();
        let sid = compile_csg_node(&mut ctx, node);
        mesh_solid(&ctx, sid, &MeshOptions::default())
    }

    fn mesh_prim_res(node: &CsgNode, resolution: u32) -> TriMesh {
        let mut ctx = SolidModelingContext::new();
        let sid = compile_csg_node(&mut ctx, node);
        mesh_solid(&ctx, sid, &MeshOptions { resolution })
    }

    fn check_invariants(mesh: &TriMesh) {
        let nt = mesh.triangles.len();
        assert_eq!(mesh.tri_normals.len(), nt * 3,
            "tri_normals.len() must equal triangles.len() * 3");
        assert_eq!(mesh.tri_uvs.len(), nt * 3,
            "tri_uvs.len() must equal triangles.len() * 3");
        let nv = mesh.vertices.len();
        for tri in &mesh.triangles {
            for &idx in tri {
                assert!((idx as usize) < nv,
                    "triangle index {idx} out of range (vertices.len() = {nv})");
            }
        }
    }

    // ── Scaffold invariants (regression) ─────────────────────────────────────

    #[test]
    fn mesh_options_default_resolution() {
        assert_eq!(MeshOptions::default().resolution, 32);
    }

    #[test]
    fn trimesh_invariants_cuboid() {
        check_invariants(&mesh_prim(&CsgNode::cuboid(2.0, 3.0, 4.0)));
    }

    #[test]
    fn trimesh_invariants_cylinder() {
        check_invariants(&mesh_prim(&CsgNode::cylinder(1.0, 2.0)));
    }

    #[test]
    fn trimesh_invariants_cone() {
        check_invariants(&mesh_prim(&CsgNode::cone(1.0, 2.0)));
    }

    #[test]
    fn trimesh_invariants_sphere() {
        check_invariants(&mesh_prim(&CsgNode::sphere(1.5)));
    }

    // ── Plane tessellation: cuboid ────────────────────────────────────────────

    #[test]
    fn mesh_solid_cuboid_is_nonempty() {
        let mesh = mesh_prim(&CsgNode::cuboid(2.0, 3.0, 4.0));
        assert!(mesh.triangles.len() > 0);
    }

    #[test]
    fn mesh_solid_cuboid_triangle_count() {
        // 6 rectangular faces × (4 boundary pts → fan → 2 triangles) = 12
        let mesh = mesh_prim(&CsgNode::cuboid(2.0, 3.0, 4.0));
        assert_eq!(mesh.triangles.len(), 12);
    }

    #[test]
    fn mesh_solid_cuboid_vertex_count() {
        // 6 faces × 4 vertices each (no sharing across faces) = 24
        let mesh = mesh_prim(&CsgNode::cuboid(2.0, 3.0, 4.0));
        assert_eq!(mesh.vertices.len(), 24);
    }

    #[test]
    fn mesh_solid_cuboid_normals_are_unit() {
        let mesh = mesh_prim(&CsgNode::cuboid(2.0, 3.0, 4.0));
        for n in &mesh.tri_normals {
            let len = (n[0]*n[0] + n[1]*n[1] + n[2]*n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-5, "normal {n:?} has length {len}");
        }
    }

    #[test]
    fn mesh_solid_cuboid_normals_axis_aligned() {
        // Each face of an axis-aligned cuboid must have a normal along ±x, ±y, or ±z.
        let mesh = mesh_prim(&CsgNode::cuboid(2.0, 3.0, 4.0));
        for n in &mesh.tri_normals {
            let [x, y, z] = *n;
            let is_axis = (x.abs() > 0.9 && y.abs() < 0.1 && z.abs() < 0.1)
                       || (y.abs() > 0.9 && x.abs() < 0.1 && z.abs() < 0.1)
                       || (z.abs() > 0.9 && x.abs() < 0.1 && y.abs() < 0.1);
            assert!(is_axis, "normal {n:?} is not axis-aligned");
        }
    }

    // ── CylindricalSurface tessellation ───────────────────────────────────────

    #[test]
    fn mesh_solid_cylinder_triangle_count() {
        // lateral:  resolution × 2 = 32 × 2 = 64
        // 2 caps:   2 × (resolution − 2) = 2 × 30 = 60
        // total: 124
        let mesh = mesh_prim_res(&CsgNode::cylinder(1.0, 2.0), 32);
        assert_eq!(mesh.triangles.len(), 32 * 2 + 2 * (32 - 2));
    }

    #[test]
    fn mesh_solid_cylinder_vertex_count() {
        // lateral:  (resolution + 1) × 2 = 33 × 2 = 66
        // 2 caps:   2 × resolution = 2 × 32 = 64
        // total: 130
        let mesh = mesh_prim_res(&CsgNode::cylinder(1.0, 2.0), 32);
        assert_eq!(mesh.vertices.len(), (32 + 1) * 2 + 2 * 32);
    }

    #[test]
    fn mesh_solid_cylinder_normals_are_unit() {
        let mesh = mesh_prim(&CsgNode::cylinder(1.0, 2.0));
        for n in &mesh.tri_normals {
            let len = (n[0]*n[0] + n[1]*n[1] + n[2]*n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-5, "normal {n:?} has length {len}");
        }
    }

    // ── ConicalSurface tessellation ───────────────────────────────────────────

    #[test]
    fn mesh_solid_cone_triangle_count() {
        // lateral:  resolution = 32 triangles
        // base cap: resolution − 2 = 30 triangles (fan)
        // total: 62
        let mesh = mesh_prim_res(&CsgNode::cone(1.0, 2.0), 32);
        assert_eq!(mesh.triangles.len(), 32 + (32 - 2));
    }

    #[test]
    fn mesh_solid_cone_vertex_count() {
        // lateral:  1 apex + resolution base = 33
        // base cap: resolution = 32
        // total: 65
        let mesh = mesh_prim_res(&CsgNode::cone(1.0, 2.0), 32);
        assert_eq!(mesh.vertices.len(), (32 + 1) + 32);
    }

    #[test]
    fn mesh_solid_cone_normals_are_unit() {
        let mesh = mesh_prim(&CsgNode::cone(1.0, 2.0));
        for n in &mesh.tri_normals {
            let len = (n[0]*n[0] + n[1]*n[1] + n[2]*n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-5, "normal {n:?} has length {len}");
        }
    }

    #[test]
    fn mesh_solid_cone_lateral_normals_not_axial() {
        // Lateral normals should have a radial component; |z| should be well below 1.
        // For cone(r=1, h=2): ha = atan(0.5) ≈ 26.6°; face normal z-component ≈ sin(ha) ≈ 0.45.
        let mesh = mesh_prim(&CsgNode::cone(1.0, 2.0));
        // lateral face has `resolution` triangles × 3 corners = 96 normal entries
        let not_axial = mesh.tri_normals.iter()
            .filter(|n| n[2].abs() < 0.99)
            .count();
        assert!(not_axial >= 32 * 3,
            "expected at least {} non-axial normals, got {}", 32 * 3, not_axial);
    }

    #[test]
    fn mesh_solid_cylinder_lateral_normals_radial() {
        // For an axis-aligned cylinder (axis = +z), lateral normals are radial:
        // their z-component must be ≈ 0.
        let mesh = mesh_prim(&CsgNode::cylinder(1.0, 2.0));
        let radial_count = mesh.tri_normals.iter()
            .filter(|n| n[2].abs() < 0.01)
            .count();
        // lateral face has resolution×2×3 = 32×2×3 = 192 normal entries
        assert!(radial_count >= 32 * 2 * 3,
            "expected at least {} radial normals, got {}", 32 * 2 * 3, radial_count);
    }
}
