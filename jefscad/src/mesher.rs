//! Tessellation: converts B-rep solids into triangle meshes.

use crate::brep_kernel::{FaceId, FaceSense, LoopId, Orientation, SolidId, SolidModelingContext};
use crate::geom::{ConicalSurface, Curve2, Curve2Kind, CylindricalSurface, Plane, Point3, SphericalSurface, Surface, SurfaceKind};

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
    pub vertices:    Vec<[f64; 3]>,
    /// Index triples — each triple defines one triangle.
    pub triangles:   Vec<[u32; 3]>,
    /// Per-triangle-corner normals; `tri_normals[t*3 + k]` for triangle `t`, corner `k`.
    pub tri_normals: Vec<[f64; 3]>,
    /// Per-triangle-corner UV params; `tri_uvs[t*3 + k]` for triangle `t`, corner `k`.
    pub tri_uvs:     Vec<[f64; 2]>,
}

// ── MeshOptions ───────────────────────────────────────────────────────────────

/// Options controlling tessellation quality.
#[derive(Debug, Clone, Copy)]
pub struct MeshOptions {
    /// Number of segments per full circle (360°).  Higher values give smoother
    /// curved surfaces at the cost of more triangles.  Default: 32.
    pub resolution: u32,
    /// Vertex-merging tolerance in world units.  After per-face tessellation,
    /// vertices whose positions are within `epsilon` of each other are collapsed
    /// to a single vertex, making the mesh watertight at shared edges.
    ///
    /// Default: `1e-8`.  Set to `0.0` (or any non-positive value) to skip merging.
    ///
    /// **Scale guidance (units = mm):** f64 floating-point noise between two
    /// different surface evaluations of the same geometric point is well below
    /// `1e-10` for geometry up to 300 mm, so the default `1e-8` merges all
    /// genuine seam duplicates while leaving a 10 000× safety margin before
    /// the nearest intentionally-distinct vertices (minimum tessellation spacing
    /// at `resolution=32`, `r=0.01 mm` is ≈ 2×10⁻³ mm).
    pub epsilon: f64,
}

impl Default for MeshOptions {
    fn default() -> Self {
        Self { resolution: 32, epsilon: 1e-8 }
    }
}

// ── STL export ───────────────────────────────────────────────────────────────

/// Write `mesh` as binary STL to `writer`.
///
/// # Binary STL layout
/// ```text
/// [  0.. 80)  80-byte ASCII header
/// [ 80.. 84)  u32 LE — triangle count
/// per triangle (50 bytes):
///   [  0.. 12)  3 × f32 LE — face normal
///   [ 12.. 24)  3 × f32 LE — vertex 0
///   [ 24.. 36)  3 × f32 LE — vertex 1
///   [ 36.. 48)  3 × f32 LE — vertex 2
///   [ 48.. 50)  u16 LE — attribute byte count (0)
/// ```
///
/// The per-triangle normal is the average of the three corner normals from
/// [`TriMesh::tri_normals`], re-normalised.  STL readers commonly recompute
/// normals from vertices anyway, but this produces a correct value for
/// flat-shaded faces and a reasonable approximation for smooth ones.
pub fn write_stl<W: std::io::Write>(mesh: &TriMesh, writer: &mut W) -> std::io::Result<()> {
    use std::io::Write;

    // 80-byte header
    let mut header = [0u8; 80];
    let tag = b"jefscad binary STL";
    header[..tag.len()].copy_from_slice(tag);
    writer.write_all(&header)?;

    // Triangle count
    let n_tris = mesh.triangles.len() as u32;
    writer.write_all(&n_tris.to_le_bytes())?;

    // Per-triangle records
    for (t, tri) in mesh.triangles.iter().enumerate() {
        // Average and renormalise the three corner normals
        let n0 = mesh.tri_normals[t * 3];
        let n1 = mesh.tri_normals[t * 3 + 1];
        let n2 = mesh.tri_normals[t * 3 + 2];
        let nx = (n0[0] + n1[0] + n2[0]) / 3.0;
        let ny = (n0[1] + n1[1] + n2[1]) / 3.0;
        let nz = (n0[2] + n1[2] + n2[2]) / 3.0;
        let len = (nx*nx + ny*ny + nz*nz).sqrt();
        let (nx, ny, nz) = if len > 1e-15 {
            (nx / len, ny / len, nz / len)
        } else {
            (0.0f64, 0.0f64, 1.0f64)
        };

        // Normal — STL format requires f32
        writer.write_all(&(nx as f32).to_le_bytes())?;
        writer.write_all(&(ny as f32).to_le_bytes())?;
        writer.write_all(&(nz as f32).to_le_bytes())?;

        // Three vertices — STL format requires f32
        for &vi in tri {
            let v = mesh.vertices[vi as usize];
            writer.write_all(&(v[0] as f32).to_le_bytes())?;
            writer.write_all(&(v[1] as f32).to_le_bytes())?;
            writer.write_all(&(v[2] as f32).to_le_bytes())?;
        }

        // Attribute byte count
        writer.write_all(&0u16.to_le_bytes())?;
    }

    Ok(())
}

/// Write `mesh` as binary STL to the file at `path`, creating or truncating it.
pub fn write_stl_file(mesh: &TriMesh, path: &std::path::Path) -> std::io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    write_stl(mesh, &mut f)
}

// ── OBJ export ───────────────────────────────────────────────────────────────

/// Write `mesh` as a Wavefront OBJ to `writer`.
///
/// # OBJ layout
/// ```text
/// # jefscad OBJ
/// v  x y z          — one per vertex in mesh.vertices
/// vn x y z          — one per triangle corner (NT×3 total)
/// vt u v            — one per triangle corner (NT×3 total)
/// f  v/vt/vn ...    — one per triangle; all indices 1-based
/// ```
///
/// Because [`TriMesh`] stores normals and UVs per-triangle-corner rather than
/// per-vertex, each corner gets its own `vn`/`vt` entry.  For triangle `t`,
/// corner `k`: vertex index = `triangles[t][k] + 1`, normal/UV index = `t*3 + k + 1`.
pub fn write_obj<W: std::io::Write>(mesh: &TriMesh, writer: &mut W) -> std::io::Result<()> {
    use std::io::Write;

    writeln!(writer, "# jefscad OBJ")?;

    // Vertex positions
    for v in &mesh.vertices {
        writeln!(writer, "v  {} {} {}", v[0], v[1], v[2])?;
    }

    // Per-corner normals
    for n in &mesh.tri_normals {
        writeln!(writer, "vn {} {} {}", n[0], n[1], n[2])?;
    }

    // Per-corner UVs
    for uv in &mesh.tri_uvs {
        writeln!(writer, "vt {} {}", uv[0], uv[1])?;
    }

    // Faces: f v/vt/vn v/vt/vn v/vt/vn  (all 1-indexed)
    for (t, tri) in mesh.triangles.iter().enumerate() {
        let base = t * 3 + 1; // 1-indexed corner offset for this triangle
        writeln!(
            writer,
            "f {}/{}/{} {}/{}/{} {}/{}/{}",
            tri[0] + 1, base,     base,
            tri[1] + 1, base + 1, base + 1,
            tri[2] + 1, base + 2, base + 2,
        )?;
    }

    Ok(())
}

/// Write `mesh` as a Wavefront OBJ to the file at `path`, creating or truncating it.
pub fn write_obj_file(mesh: &TriMesh, path: &std::path::Path) -> std::io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    write_obj(mesh, &mut f)
}

// ── merge_vertices ────────────────────────────────────────────────────────────

/// Merge vertices whose positions are within `epsilon` of each other.
///
/// Uses a quantised hash map: each coordinate is rounded to the nearest multiple
/// of `epsilon` and the resulting `(i64, i64, i64)` triple is used as the key.
/// The first vertex seen for a given key becomes the canonical representative;
/// all later vertices that hash to the same key are remapped to it.
///
/// [`TriMesh::tri_normals`] and [`TriMesh::tri_uvs`] are per-triangle-corner and
/// are copied unchanged — only `vertices` and the indices in `triangles` change.
///
/// If `epsilon` is zero or negative the mesh is returned unmodified.
pub fn merge_vertices(mesh: &TriMesh, epsilon: f64) -> TriMesh {
    if epsilon <= 0.0 {
        return mesh.clone();
    }

    use std::collections::HashMap;

    let inv_eps = 1.0 / epsilon;
    let quantize = |x: f64| -> i64 { (x * inv_eps).round() as i64 };
    let key      = |v: [f64; 3]| -> (i64, i64, i64) {
        (quantize(v[0]), quantize(v[1]), quantize(v[2]))
    };

    let mut map: HashMap<(i64, i64, i64), u32> = HashMap::new();
    let mut new_vertices: Vec<[f64; 3]>        = Vec::new();
    let mut remap: Vec<u32>                    = Vec::with_capacity(mesh.vertices.len());

    for &v in &mesh.vertices {
        let idx = *map.entry(key(v)).or_insert_with(|| {
            let idx = new_vertices.len() as u32;
            new_vertices.push(v);
            idx
        });
        remap.push(idx);
    }

    let new_triangles = mesh.triangles.iter()
        .map(|&[a, b, c]| [remap[a as usize], remap[b as usize], remap[c as usize]])
        .collect();

    TriMesh {
        vertices:    new_vertices,
        triangles:   new_triangles,
        tri_normals: mesh.tri_normals.clone(),
        tri_uvs:     mesh.tri_uvs.clone(),
    }
}

// ── mesh_solid ────────────────────────────────────────────────────────────────

/// Tessellate all faces of solid `sid` and return a combined [`TriMesh`].
///
/// Each face is tessellated independently by the internal `mesh_face` function.
/// The resulting per-face meshes are concatenated with triangle indices adjusted
/// to the global vertex offset, then [`merge_vertices`] is applied to produce a
/// watertight mesh (controlled by [`MeshOptions::epsilon`]).
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

    merge_vertices(&out, opts.epsilon)
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
        SurfaceKind::Sphere(sph)     => mesh_spherical_face(ctx, face_id, *sph, opts),
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
    let normal = [out_n.x, out_n.y, out_n.z];

    // 3-D vertex positions from surface eval
    let vertices: Vec<[f64; 3]> = uvs.iter().map(|&[u, v]| {
        let p = plane.eval(u, v);
        [p.x, p.y, p.z]
    }).collect();

    // Fan triangulation from vertex 0: triangles (0, i, i+1) for i in 1..n-1
    let mut triangles  = Vec::with_capacity(n - 2);
    let mut tri_normals = Vec::with_capacity((n - 2) * 3);
    let mut tri_uvs    = Vec::with_capacity((n - 2) * 3);

    for i in 1..=(n - 2) {
        triangles.push([0u32, i as u32, (i + 1) as u32]);
        for &corner in &[0, i, i + 1] {
            tri_normals.push(normal);
            tri_uvs.push([uvs[corner][0], uvs[corner][1]]);
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
            vertices.push([p.x, p.y, p.z]);
            let n = cyl.eval_n(u, v).expect("CylindricalSurface normal is always defined");
            vert_norms.push([n.x, n.y, n.z]);
            vert_uvs.push([u, v]);
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
/// `resolution` triangles.
///
/// **Normals (hybrid):**
/// - Base-circle corners: analytic `eval_n(u, v_max)` — smooth shading around
///   the circumference.
/// - Apex corner: flat cross-product normal for that triangle — the apex is a
///   geometric singularity where no single outward normal is definable, so the
///   per-face normal is the most honest representation.
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
    vertices.push([apex_pos.x, apex_pos.y, apex_pos.z]);

    let mut base_u = Vec::with_capacity(res);
    for j in 0..res {
        let u = j as f64 * TAU / res as f64;
        let p = cone.eval(u, v_max);
        vertices.push([p.x, p.y, p.z]);
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

        // Apex normal: flat cross-product for this triangle
        //   (base_next - apex) × (base_curr - apex)
        let bv_next = vertices[next_idx as usize];
        let bv_curr = vertices[curr_idx as usize];
        let v1 = Point3::new(
            bv_next[0] - apex_pos.x,
            bv_next[1] - apex_pos.y,
            bv_next[2] - apex_pos.z,
        );
        let v2 = Point3::new(
            bv_curr[0] - apex_pos.x,
            bv_curr[1] - apex_pos.y,
            bv_curr[2] - apex_pos.z,
        );
        let raw_n = v1.cross(v2);
        let len = (raw_n.x*raw_n.x + raw_n.y*raw_n.y + raw_n.z*raw_n.z).sqrt();
        let flat_n = if len > 1e-15 {
            [raw_n.x/len, raw_n.y/len, raw_n.z/len]
        } else {
            [0.0, 0.0, 1.0] // degenerate fallback
        };

        // Base-circle corners: analytic normals (smooth around circumference)
        let an_next = cone.eval_n(u_next, v_max)
            .expect("eval_n is defined for v > 0");
        let an_curr = cone.eval_n(u_curr, v_max)
            .expect("eval_n is defined for v > 0");

        let flip = sense == FaceSense::AntiAligned;
        let sign = |n: [f64; 3]| -> [f64; 3] {
            if flip { [-n[0], -n[1], -n[2]] } else { n }
        };
        let apex_n      = sign(flat_n);
        let base_next_n = sign([an_next.x, an_next.y, an_next.z]);
        let base_curr_n = sign([an_curr.x, an_curr.y, an_curr.z]);

        // Triangle: (apex, base_next, base_curr)
        triangles.push([0u32, next_idx, curr_idx]);
        tri_uvs.push([u_curr, 0.0]);
        tri_uvs.push([u_next, v_max]);
        tri_uvs.push([u_curr, v_max]);
        tri_normals.push(apex_n);
        tri_normals.push(base_next_n);
        tri_normals.push(base_curr_n);
    }

    TriMesh { vertices, triangles, tri_normals, tri_uvs }
}

// ── SphericalSurface tessellation ────────────────────────────────────────────

/// Tessellate a [`SphericalSurface`] face.
///
/// Builds a `(n_lon+1) × (n_lat+1)` latitude/longitude UV grid where
/// `n_lon = resolution` and `n_lat = max(2, resolution/2)`.  The two pole rows
/// collapse to single vertices; all interior rings have `n_lon+1` vertices
/// (including a seam-duplicate at `u = 2π`).
///
/// Triangulation:
/// - **South fan**: `n_lon` triangles connecting the south pole to the first ring.
/// - **Middle bands**: `n_lat-2` bands of `2·n_lon` triangles each (same strip
///   winding as [`mesh_cylindrical_face`]).
/// - **North fan**: `n_lon` triangles connecting the last ring to the north pole.
///
/// Normals are analytic via [`SphericalSurface::eval_n`], which is defined everywhere
/// including the poles — smooth shading with no special-casing required.
fn mesh_spherical_face(
    ctx: &SolidModelingContext,
    face_id: FaceId,
    sph: SphericalSurface,
    opts: &MeshOptions,
) -> TriMesh {
    use std::f64::consts::{FRAC_PI_2, TAU};

    let sense = ctx.get_face(face_id).sense;

    let n_lon = opts.resolution as usize;
    let n_lat = (opts.resolution as usize / 2).max(2);

    let v_step = std::f64::consts::PI / n_lat as f64;
    let u_step = TAU / n_lon as f64;

    // ── Vertices ──────────────────────────────────────────────────────────────
    // Index 0         : south pole
    // Index 1 + (i-1)*(n_lon+1) + j : ring i (1 ≤ i ≤ n_lat-1), column j (0 ≤ j ≤ n_lon)
    // Index 1 + (n_lat-1)*(n_lon+1) : north pole
    let n_verts = 2 + (n_lat - 1) * (n_lon + 1);
    let mut vertices    = Vec::with_capacity(n_verts);
    let mut vert_norms  = Vec::with_capacity(n_verts);
    let mut vert_uvs    = Vec::with_capacity(n_verts);

    let push_vert = |verts: &mut Vec<[f64; 3]>,
                     norms: &mut Vec<[f64; 3]>,
                     uvs:   &mut Vec<[f64; 2]>,
                     u: f64, v: f64| {
        let p = sph.eval(u, v);
        verts.push([p.x, p.y, p.z]);
        let n = sph.eval_n(u, v).expect("SphericalSurface::eval_n is always Some");
        let out_n = if sense == FaceSense::AntiAligned { [-n.x, -n.y, -n.z] } else { [n.x, n.y, n.z] };
        norms.push(out_n);
        uvs.push([u, v]);
    };

    // South pole (u=0 is arbitrary; position and normal are u-independent)
    push_vert(&mut vertices, &mut vert_norms, &mut vert_uvs, 0.0, -FRAC_PI_2);

    // Interior rings
    for i in 1..n_lat {
        let v = -FRAC_PI_2 + i as f64 * v_step;
        for j in 0..=n_lon {
            let u = j as f64 * u_step;
            push_vert(&mut vertices, &mut vert_norms, &mut vert_uvs, u, v);
        }
    }

    // North pole
    push_vert(&mut vertices, &mut vert_norms, &mut vert_uvs, 0.0, FRAC_PI_2);

    // ── Index helpers ─────────────────────────────────────────────────────────
    let south_pole = 0u32;
    let north_pole = (1 + (n_lat - 1) * (n_lon + 1)) as u32;
    // ring i (1-indexed), column j
    let ring = |i: usize, j: usize| (1 + (i - 1) * (n_lon + 1) + j) as u32;

    let n_tris = 2 * n_lon * (n_lat - 1);
    let mut triangles   = Vec::with_capacity(n_tris);
    let mut tri_normals = Vec::with_capacity(n_tris * 3);
    let mut tri_uvs     = Vec::with_capacity(n_tris * 3);

    let mut push_tri = |tri: [u32; 3]| {
        triangles.push(tri);
        for &c in &tri {
            tri_normals.push(vert_norms[c as usize]);
            tri_uvs.push(vert_uvs[c as usize]);
        }
    };

    // ── South fan ─────────────────────────────────────────────────────────────
    // Winding: (south_pole, ring1[j+1], ring1[j]) — CCW in UV ✓
    for j in 0..n_lon {
        push_tri([south_pole, ring(1, j + 1), ring(1, j)]);
    }

    // ── Middle bands ──────────────────────────────────────────────────────────
    // Band between ring i and ring i+1 (for i in 1..n_lat-1)
    for i in 1..n_lat - 1 {
        for j in 0..n_lon {
            let bl = ring(i,     j);
            let br = ring(i,     j + 1);
            let tl = ring(i + 1, j);
            let tr = ring(i + 1, j + 1);
            push_tri([bl, br, tr]);
            push_tri([bl, tr, tl]);
        }
    }

    // ── North fan ─────────────────────────────────────────────────────────────
    // Winding: (north_pole, ring_last[j], ring_last[j+1]) — CCW in UV ✓
    for j in 0..n_lon {
        push_tri([north_pole, ring(n_lat - 1, j), ring(n_lat - 1, j + 1)]);
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
            Curve2Kind::Polyline2(_) => {
                todo!("UV sampling for Polyline2 not yet implemented")
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
        mesh_solid(&ctx, sid, &MeshOptions { resolution, ..MeshOptions::default() })
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

    // ── merge_vertices ───────────────────────────────────────────────────────

    fn unmerged_prim(node: &CsgNode) -> TriMesh {
        // mesh_solid with epsilon=0 to get the pre-merge mesh
        let mut ctx = SolidModelingContext::new();
        let sid = compile_csg_node(&mut ctx, node);
        mesh_solid(&ctx, sid, &MeshOptions { resolution: 32, epsilon: 0.0 })
    }

    #[test]
    fn merge_vertices_no_op_when_epsilon_zero() {
        let mesh = unmerged_prim(&CsgNode::cuboid(1.0, 1.0, 1.0));
        let merged = merge_vertices(&mesh, 0.0);
        assert_eq!(merged.vertices.len(), mesh.vertices.len());
    }

    #[test]
    fn merge_vertices_cuboid_collapses_to_8() {
        let mesh = unmerged_prim(&CsgNode::cuboid(1.0, 1.0, 1.0));
        assert_eq!(mesh.vertices.len(), 24, "pre-merge should be 24");
        let merged = merge_vertices(&mesh, 1e-8);
        assert_eq!(merged.vertices.len(), 8);
    }

    #[test]
    fn merge_vertices_cylinder_collapses_to_64() {
        let mesh = unmerged_prim(&CsgNode::cylinder(1.0, 2.0));
        assert_eq!(mesh.vertices.len(), 130, "pre-merge should be 130");
        let merged = merge_vertices(&mesh, 1e-8);
        assert_eq!(merged.vertices.len(), 64);
    }

    #[test]
    fn merge_vertices_invariants_hold() {
        let mesh = unmerged_prim(&CsgNode::sphere(1.5));
        let merged = merge_vertices(&mesh, 1e-8);
        check_invariants(&merged);
    }

    // ── OBJ export ───────────────────────────────────────────────────────────

    fn obj_string(mesh: &TriMesh) -> String {
        let mut buf = Vec::new();
        write_obj(mesh, &mut buf).expect("write_obj failed");
        String::from_utf8(buf).expect("OBJ output is not valid UTF-8")
    }

    fn count_lines_starting_with(s: &str, prefix: &str) -> usize {
        s.lines().filter(|l| l.starts_with(prefix)).count()
    }

    #[test]
    fn obj_empty_mesh_no_faces() {
        let s = obj_string(&TriMesh::default());
        assert_eq!(count_lines_starting_with(&s, "f "), 0);
    }

    #[test]
    fn obj_cuboid_vertex_line_count() {
        // 8 unique corners after vertex merging
        let mesh = mesh_prim(&CsgNode::cuboid(1.0, 1.0, 1.0));
        let s = obj_string(&mesh);
        assert_eq!(count_lines_starting_with(&s, "v "), 8);
    }

    #[test]
    fn obj_cuboid_face_line_count() {
        let mesh = mesh_prim(&CsgNode::cuboid(1.0, 1.0, 1.0));
        let s = obj_string(&mesh);
        assert_eq!(count_lines_starting_with(&s, "f "), 12);
    }

    #[test]
    fn obj_cuboid_face_indices_valid() {
        let mesh = mesh_prim(&CsgNode::cuboid(1.0, 1.0, 1.0));
        let n_verts  = mesh.vertices.len();       // 8 after merge
        let n_corners = mesh.triangles.len() * 3; // 36
        let s = obj_string(&mesh);

        for line in s.lines().filter(|l| l.starts_with("f ")) {
            // Each token after "f" is "v/vt/vn"
            for token in line.split_whitespace().skip(1) {
                let parts: Vec<usize> = token.split('/')
                    .map(|p| p.parse::<usize>().expect("index must be integer"))
                    .collect();
                assert_eq!(parts.len(), 3, "expected v/vt/vn in token {token}");
                let (vi, vti, vni) = (parts[0], parts[1], parts[2]);
                assert!(vi  >= 1 && vi  <= n_verts,   "vertex index {vi} out of range");
                assert!(vti >= 1 && vti <= n_corners,  "vt index {vti} out of range");
                assert!(vni >= 1 && vni <= n_corners,  "vn index {vni} out of range");
            }
        }
    }

    // ── STL export ───────────────────────────────────────────────────────────

    fn stl_bytes(mesh: &TriMesh) -> Vec<u8> {
        let mut buf = Vec::new();
        write_stl(mesh, &mut buf).expect("write_stl failed");
        buf
    }

    #[test]
    fn stl_empty_mesh_byte_count() {
        let bytes = stl_bytes(&TriMesh::default());
        assert_eq!(bytes.len(), 84); // 80 header + 4 count
    }

    #[test]
    fn stl_cuboid_byte_count() {
        let mesh = mesh_prim(&CsgNode::cuboid(1.0, 1.0, 1.0));
        let bytes = stl_bytes(&mesh);
        assert_eq!(bytes.len(), 84 + 12 * 50); // 684
    }

    #[test]
    fn stl_triangle_count_field() {
        let mesh = mesh_prim(&CsgNode::cuboid(1.0, 1.0, 1.0));
        let bytes = stl_bytes(&mesh);
        let count = u32::from_le_bytes(bytes[80..84].try_into().unwrap());
        assert_eq!(count, 12);
    }

    #[test]
    fn stl_cuboid_normals_axis_aligned() {
        let mesh = mesh_prim(&CsgNode::cuboid(1.0, 1.0, 1.0));
        let bytes = stl_bytes(&mesh);
        // Each triangle record starts at 84 + t*50; normal is the first 12 bytes (3×f32)
        for t in 0..12usize {
            let off = 84 + t * 50;
            let nx = f32::from_le_bytes(bytes[off     ..off +  4].try_into().unwrap());
            let ny = f32::from_le_bytes(bytes[off +  4..off +  8].try_into().unwrap());
            let nz = f32::from_le_bytes(bytes[off +  8..off + 12].try_into().unwrap());
            let is_axis = (nx.abs() > 0.9 && ny.abs() < 0.1 && nz.abs() < 0.1)
                       || (ny.abs() > 0.9 && nx.abs() < 0.1 && nz.abs() < 0.1)
                       || (nz.abs() > 0.9 && nx.abs() < 0.1 && ny.abs() < 0.1);
            assert!(is_axis, "triangle {t} normal ({nx},{ny},{nz}) is not axis-aligned");
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
        // 8 unique corners (3 faces share each corner); merge collapses 24 → 8
        let mesh = mesh_prim(&CsgNode::cuboid(2.0, 3.0, 4.0));
        assert_eq!(mesh.vertices.len(), 8);
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
        // 32 unique base-circle positions + 32 unique top-circle positions = 64
        // (lateral seam duplicate + cap vertices all collapse onto the two circles)
        let mesh = mesh_prim_res(&CsgNode::cylinder(1.0, 2.0), 32);
        assert_eq!(mesh.vertices.len(), 32 + 32);
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
        // 1 apex + 32 base-circle positions = 33
        // (cap vertices collapse onto the lateral base ring)
        let mesh = mesh_prim_res(&CsgNode::cone(1.0, 2.0), 32);
        assert_eq!(mesh.vertices.len(), 1 + 32);
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
    fn mesh_solid_cone_lateral_normals_hybrid() {
        // Hybrid normals: within each lateral triangle the apex corner's normal
        // must differ from the two base-circle corners' normals.
        // (Before this change all three corners had the same flat normal.)
        let mesh = mesh_prim(&CsgNode::cone(1.0, 2.0));
        let mut found_difference = false;
        for t in 0..mesh.triangles.len() {
            let n_apex      = mesh.tri_normals[t * 3];
            let n_base_next = mesh.tri_normals[t * 3 + 1];
            let n_base_curr = mesh.tri_normals[t * 3 + 2];
            if n_apex != n_base_next || n_apex != n_base_curr {
                found_difference = true;
                break;
            }
        }
        assert!(found_difference,
            "apex corner normals should differ from base corner normals");
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

    // ── SphericalSurface tessellation ─────────────────────────────────────────

    #[test]
    fn mesh_solid_sphere_triangle_count() {
        // n_lon=32, n_lat=16: 2 × 32 × 15 = 960
        let mesh = mesh_prim_res(&CsgNode::sphere(1.0), 32);
        assert_eq!(mesh.triangles.len(), 2 * 32 * 15);
    }

    #[test]
    fn mesh_solid_sphere_vertex_count() {
        // 2 + 15 rings × 33 columns = 497 pre-merge;
        // 15 seam duplicate pairs collapse → 497 - 15 = 482
        let mesh = mesh_prim_res(&CsgNode::sphere(1.0), 32);
        assert_eq!(mesh.vertices.len(), 2 + 15 * 33 - 15);
    }

    #[test]
    fn mesh_solid_sphere_normals_are_unit() {
        let mesh = mesh_prim(&CsgNode::sphere(1.0));
        for n in &mesh.tri_normals {
            let len = (n[0]*n[0] + n[1]*n[1] + n[2]*n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-5, "normal {n:?} has length {len}");
        }
    }

    #[test]
    fn mesh_solid_sphere_normals_cover_sphere() {
        // Analytic normals span all directions: verify near-pole and equatorial normals exist.
        let mesh = mesh_prim(&CsgNode::sphere(1.0));
        let near_north  = mesh.tri_normals.iter().filter(|n| n[2] >  0.9).count();
        let near_south  = mesh.tri_normals.iter().filter(|n| n[2] < -0.9).count();
        let near_equator = mesh.tri_normals.iter().filter(|n| n[2].abs() < 0.1).count();
        assert!(near_north  > 0, "expected normals near north pole");
        assert!(near_south  > 0, "expected normals near south pole");
        assert!(near_equator > 0, "expected normals near equator");
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
