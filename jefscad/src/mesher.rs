//! Tessellation: converts B-rep solids into triangle meshes.

use crate::brep_kernel::{FaceId, SolidId, SolidModelingContext};

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

// ── mesh_face (stub — filled in during Step 2 per surface type) ───────────────

/// Tessellate a single face, returning a local [`TriMesh`] with indices starting
/// at 0.  [`mesh_solid`] adjusts the indices to the global vertex offset.
fn mesh_face(_ctx: &SolidModelingContext, _face_id: FaceId, _opts: &MeshOptions) -> TriMesh {
    // Stub: returns an empty mesh until per-surface tessellation is implemented.
    TriMesh::default()
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
}
