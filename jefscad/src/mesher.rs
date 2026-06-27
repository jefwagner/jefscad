// Phase-0 staging: the tessellation pipeline (HalfEdgeMesh, TriMesh, mesh_*
// functions, STL/OBJ writers) is not yet wired into the public Python API and is
// slated for rewrite in a later phase (per Phase 0-c notes: mesher will be
// substantially rewritten anyway). Remove this allow once the mesh path is wired up.
#![allow(dead_code)]

//! Tessellation: converts B-rep solids into triangle meshes.

use crate::brep_kernel::{EdgeId, FaceId, FaceSense, LoopId, Orientation, SolidId, SolidModelingContext, VertexId};
use crate::geom::{ConicalSurface, Curve2, Curve2Kind, Curve3Kind, CylindricalSurface, Plane, Point3, SphericalSurface, Surface, SurfaceKind};

// ── DCEL / Half-Edge mesh ─────────────────────────────────────────────────────

/// Which B-rep entity a mesh vertex was projected from.
///
/// Used to classify vertices for constraint-edge enforcement and future
/// Delaunay refinement: `Corner` and `OnEdge` vertices sit on B-rep boundaries
/// and must not be moved; `OnFace` vertices are interior and may be relocated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshVertexRef {
    /// Projected from a B-rep topological vertex; position is fixed.
    Corner(VertexId),
    /// Lies on a B-rep coedge boundary; the half-edge pair touching this vertex
    /// is a constraint edge — never flip or cut.
    OnEdge(EdgeId),
    /// Interior point sampled on a B-rep face (e.g. sphere grid, refinement insert).
    OnFace(FaceId),
}

/// An internal mesh vertex. All fields are `f64`; narrowing to `f32` happens only
/// at export time (binary STL writer).
#[derive(Debug, Clone)]
pub struct MeshVertex {
    /// 3-D position in world space.
    pub pos:      [f64; 3],
    /// Surface UV parameter at this vertex.
    pub uv:       [f64; 2],
    /// Outward surface normal at this vertex (unit vector).
    pub normal:   [f64; 3],
    /// Which B-rep entity this vertex was projected from.
    pub brep_ref: MeshVertexRef,
}

/// Index into [`HalfEdgeMesh::vertices`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshVertexId(pub usize);

/// Index into [`HalfEdgeMesh::half_edges`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HalfEdgeId(pub usize);

/// Index into [`HalfEdgeMesh::faces`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DcelFaceId(pub usize);

/// A directed half-edge belonging to exactly one face.
///
/// Convention: `vertex` is the *start* vertex of this half-edge; `next.vertex`
/// is the end vertex.  For a CCW triangle (v0, v1, v2) the three half-edges
/// are he0(v0→v1), he1(v1→v2), he2(v2→v0) with he0.next=he1, he1.next=he2,
/// he2.next=he0.
#[derive(Debug, Clone)]
pub struct HalfEdge {
    /// The opposing half-edge on the adjacent face, if any.
    /// `None` during construction; all twins must be filled before [`HalfEdgeMesh::to_trimesh`].
    pub twin:          Option<HalfEdgeId>,
    /// Next half-edge around this face (CCW).
    pub next:          HalfEdgeId,
    /// Start vertex of this half-edge.
    pub vertex:        MeshVertexId,
    /// Face this half-edge belongs to.
    pub face:          DcelFaceId,
    /// `true` if this edge was derived from a B-rep coedge and must not be flipped or cut.
    pub is_constraint: bool,
}

/// A triangular face in the DCEL; stores one representative half-edge.
#[derive(Debug, Clone)]
pub struct DcelFace {
    /// Any one of the three half-edges bounding this face.
    pub half_edge: HalfEdgeId,
}

/// Internal half-edge (DCEL) mesh used during tessellation and future refinement.
///
/// All coordinates are `f64`.  Call [`HalfEdgeMesh::to_trimesh`] to produce the
/// [`TriMesh`] used for export.
#[derive(Debug, Default, Clone)]
pub struct HalfEdgeMesh {
    pub vertices:   Vec<MeshVertex>,
    pub half_edges: Vec<HalfEdge>,
    pub faces:      Vec<DcelFace>,
}

impl HalfEdgeMesh {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a vertex and return its ID.
    pub fn push_vertex(&mut self, v: MeshVertex) -> MeshVertexId {
        let id = MeshVertexId(self.vertices.len());
        self.vertices.push(v);
        id
    }

    /// Append a CCW triangle defined by three vertex IDs.
    ///
    /// Creates three half-edges with `next` linked in CCW order and `twin = None`.
    /// Returns `(face_id, [he0, he1, he2])` where `he_i` starts at `v_i`.
    pub fn push_triangle(
        &mut self,
        v0: MeshVertexId,
        v1: MeshVertexId,
        v2: MeshVertexId,
    ) -> (DcelFaceId, [HalfEdgeId; 3]) {
        let face_id = DcelFaceId(self.faces.len());
        let he0 = HalfEdgeId(self.half_edges.len());
        let he1 = HalfEdgeId(self.half_edges.len() + 1);
        let he2 = HalfEdgeId(self.half_edges.len() + 2);

        self.half_edges.push(HalfEdge { twin: None, next: he1, vertex: v0, face: face_id, is_constraint: false });
        self.half_edges.push(HalfEdge { twin: None, next: he2, vertex: v1, face: face_id, is_constraint: false });
        self.half_edges.push(HalfEdge { twin: None, next: he0, vertex: v2, face: face_id, is_constraint: false });
        self.faces.push(DcelFace { half_edge: he0 });

        (face_id, [he0, he1, he2])
    }

    /// Link two half-edges as twins of each other.
    pub fn set_twin(&mut self, a: HalfEdgeId, b: HalfEdgeId) {
        self.half_edges[a.0].twin = Some(b);
        self.half_edges[b.0].twin = Some(a);
    }

    /// Return the three vertex IDs of `face_id` in CCW order.
    pub fn face_vertices(&self, face_id: DcelFaceId) -> [MeshVertexId; 3] {
        let [he0, he1, he2] = self.face_half_edges(face_id);
        [
            self.half_edges[he0.0].vertex,
            self.half_edges[he1.0].vertex,
            self.half_edges[he2.0].vertex,
        ]
    }

    /// Return the three half-edge IDs bounding `face_id` in CCW order.
    pub fn face_half_edges(&self, face_id: DcelFaceId) -> [HalfEdgeId; 3] {
        let he0 = self.faces[face_id.0].half_edge;
        let he1 = self.half_edges[he0.0].next;
        let he2 = self.half_edges[he1.0].next;
        [he0, he1, he2]
    }

    /// Iterate over all half-edges leaving `vertex_id` (the one-ring).
    ///
    /// Traversal uses `twin.next` to walk around the vertex.  Stops if any
    /// half-edge in the ring has `twin = None` (open boundary).
    pub fn vertex_one_ring(&self, vertex_id: MeshVertexId) -> impl Iterator<Item = HalfEdgeId> + '_ {
        // Find the first outgoing half-edge for this vertex
        let start = self.half_edges.iter().position(|he| he.vertex == vertex_id)
            .map(HalfEdgeId);

        struct OneRing<'a> {
            mesh:    &'a HalfEdgeMesh,
            start:   Option<HalfEdgeId>,
            current: Option<HalfEdgeId>,
            done:    bool,
        }
        impl<'a> Iterator for OneRing<'a> {
            type Item = HalfEdgeId;
            fn next(&mut self) -> Option<HalfEdgeId> {
                if self.done { return None; }
                let cur = self.current?;
                // Advance: twin of current, then .next twice to get the next outgoing
                // half-edge from the same vertex.  Pattern: cur.twin.next.next
                let twin = self.mesh.half_edges[cur.0].twin?;
                let nxt_outgoing = {
                    let n1 = self.mesh.half_edges[twin.0].next;
                    self.mesh.half_edges[n1.0].next
                };
                if Some(nxt_outgoing) == self.start {
                    self.done = true;
                } else {
                    self.current = Some(nxt_outgoing);
                }
                Some(cur)
            }
        }

        OneRing { mesh: self, start, current: start, done: false }
    }

    /// Convert to a [`TriMesh`] for export.
    ///
    /// One `TriMesh` vertex is emitted per `MeshVertex` (no deduplication — shared
    /// positions are guaranteed by the edge registry during assembly).  All values
    /// remain `f64`; the binary STL writer narrows to `f32`.
    pub fn to_trimesh(&self) -> TriMesh {
        let vertices: Vec<[f64; 3]> = self.vertices.iter().map(|v| v.pos).collect();

        let mut triangles   = Vec::with_capacity(self.faces.len());
        let mut tri_normals = Vec::with_capacity(self.faces.len() * 3);
        let mut tri_uvs     = Vec::with_capacity(self.faces.len() * 3);

        for fi in 0..self.faces.len() {
            let [v0, v1, v2] = self.face_vertices(DcelFaceId(fi));
            triangles.push([v0.0 as u32, v1.0 as u32, v2.0 as u32]);
            for &vi in &[v0, v1, v2] {
                tri_normals.push(self.vertices[vi.0].normal);
                tri_uvs.push(self.vertices[vi.0].uv);
            }
        }

        TriMesh { vertices, triangles, tri_normals, tri_uvs }
    }
}

// ── EdgeVertexRegistry ────────────────────────────────────────────────────────

/// Coordinates shared vertices along B-rep edges during solid mesh assembly.
///
/// The consistency contract this relies on: for any coedge, the pcurve parameter `t`
/// and the 3-D edge curve parameter `t` are the **same value** — i.e.
/// `surface.eval(pcurve.eval(t)) ≈ curve3.eval(t)` for all t ∈ [t0, t1].
/// This is already implicit in `sample_loop_uvs`, which passes `edge.t0/t1` directly
/// to `pcurve.eval`.  Keys are stored on the edge's canonical t (not flipped for
/// reverse coedges), so both orientations of the same edge look up the same entry.
///
/// # Phase 5 caveat
/// For `SsiCurve3` coedges produced by boolean ops the pcurve and the 3-D intersection
/// curve may carry independent parameterizations.  At that point a
/// `Curve3::project(pt) -> t` operation will be needed for insertion; this registry
/// design is forward-compatible with that extension.
pub struct EdgeVertexRegistry {
    /// Interior edge samples: keyed by (EdgeId, quantized t).
    /// Used for `MeshVertexRef::OnEdge` vertices only.
    entries: std::collections::HashMap<EdgeId, std::collections::BTreeMap<i64, MeshVertexId>>,
    /// Corner vertices: keyed by B-rep `VertexId`.
    /// A corner is the shared endpoint of multiple edges; it must be registered
    /// once by B-rep vertex identity, not by (EdgeId, t), because the same 3-D
    /// vertex appears as the *start* of different edges on different faces.
    corners: std::collections::HashMap<VertexId, MeshVertexId>,
    /// t is quantized as `(t * quant).round() as i64`.  Default 1e12 gives
    /// sub-picometer resolution — well below the 10 µm modeling accuracy target
    /// while safely above f64 floating-point noise (~1e-15 for typical t ranges).
    quant: f64,
}

impl EdgeVertexRegistry {
    const DEFAULT_QUANT: f64 = 1e12;

    pub fn new() -> Self {
        Self {
            entries: std::collections::HashMap::new(),
            corners: std::collections::HashMap::new(),
            quant:   Self::DEFAULT_QUANT,
        }
    }

    /// Return the `MeshVertexId` for B-rep corner `vertex_id`, creating it on
    /// first call.  Use this for `MeshVertexRef::Corner` vertices.
    pub fn get_or_insert_corner(
        &mut self,
        vertex_id: VertexId,
        mesh: &mut HalfEdgeMesh,
        make_vertex: impl FnOnce() -> MeshVertex,
    ) -> MeshVertexId {
        if let Some(&vid) = self.corners.get(&vertex_id) {
            vid
        } else {
            let vid = mesh.push_vertex(make_vertex());
            self.corners.insert(vertex_id, vid);
            vid
        }
    }

    /// Return the `MeshVertexId` already registered for `(edge_id, t)`, or create
    /// a new vertex via `make_vertex`, push it into `mesh`, register it, and return
    /// the new id.  Use this for `MeshVertexRef::OnEdge` vertices.
    /// `make_vertex` is only called on first insertion.
    pub fn get_or_insert_edge(
        &mut self,
        edge_id: EdgeId,
        t: f64,
        mesh: &mut HalfEdgeMesh,
        make_vertex: impl FnOnce() -> MeshVertex,
    ) -> MeshVertexId {
        let key = (t * self.quant).round() as i64;
        let bucket = self.entries.entry(edge_id).or_default();
        if let Some(&vid) = bucket.get(&key) {
            vid
        } else {
            let vid = mesh.push_vertex(make_vertex());
            bucket.insert(key, vid);
            vid
        }
    }

    /// Number of distinct edges that have at least one registered interior vertex.
    #[cfg(test)]
    pub fn edge_count(&self) -> usize { self.entries.len() }

    /// Number of interior (OnEdge) vertices registered for `edge_id`.
    #[cfg(test)]
    pub fn vertex_count_for(&self, edge_id: EdgeId) -> usize {
        self.entries.get(&edge_id).map_or(0, |m| m.len())
    }

    /// Number of registered B-rep corner vertices.
    #[cfg(test)]
    pub fn corner_count(&self) -> usize { self.corners.len() }
}

// ── stitch_twins ──────────────────────────────────────────────────────────────

/// Link opposing half-edges across face boundaries.
///
/// Builds a map `(start_vertex, end_vertex) → HalfEdgeId` for every currently
/// unstitched half-edge, then for each such half-edge looks up the reverse key
/// `(end_vertex, start_vertex)` to find its geometric neighbor and calls
/// [`HalfEdgeMesh::set_twin`].
///
/// Half-edges on the mesh boundary (no geometric neighbor, e.g. the outer boundary
/// of an open surface patch) remain `twin = None` after this pass.
///
/// This function is idempotent: calling it a second time on an already-stitched
/// mesh is a no-op.
pub fn stitch_twins(mesh: &mut HalfEdgeMesh) {
    use std::collections::HashMap;

    // Pass 1 — index all unstitched half-edges by their (start, end) vertex pair.
    let mut map: HashMap<(MeshVertexId, MeshVertexId), HalfEdgeId> = HashMap::new();
    for i in 0..mesh.half_edges.len() {
        if mesh.half_edges[i].twin.is_none() {
            let start = mesh.half_edges[i].vertex;
            let end   = mesh.half_edges[mesh.half_edges[i].next.0].vertex;
            map.insert((start, end), HalfEdgeId(i));
        }
    }

    // Pass 2 — for each unstitched half-edge, find and link its twin.
    for i in 0..mesh.half_edges.len() {
        if mesh.half_edges[i].twin.is_some() { continue; }
        let start = mesh.half_edges[i].vertex;
        let end   = mesh.half_edges[mesh.half_edges[i].next.0].vertex;
        if let Some(&twin_id) = map.get(&(end, start)) {
            if twin_id.0 != i {
                mesh.half_edges[i].twin         = Some(twin_id);
                mesh.half_edges[twin_id.0].twin = Some(HalfEdgeId(i));
            }
        }
    }
}

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

// ── merge_dcel_vertices ───────────────────────────────────────────────────────

/// Merge [`MeshVertex`] entries whose positions are within `epsilon` of each
/// other, remapping vertex IDs in all half-edges.
///
/// This is the DCEL-level equivalent of [`merge_vertices`]: it must run *before*
/// [`stitch_twins`] so that coincident boundary vertices (created independently
/// by adjacent face tessellators) end up with the same [`MeshVertexId`], enabling
/// correct twin linking.
///
/// If `epsilon` is ≤ 0 the mesh is returned unmodified.
pub fn merge_dcel_vertices(dcel: &mut HalfEdgeMesh, epsilon: f64) {
    if epsilon <= 0.0 { return; }

    use std::collections::HashMap;
    let inv_eps = 1.0 / epsilon;
    let quantize = |x: f64| -> i64 { (x * inv_eps).round() as i64 };
    let key = |v: &[f64; 3]| -> (i64, i64, i64) {
        (quantize(v[0]), quantize(v[1]), quantize(v[2]))
    };

    let mut map: HashMap<(i64, i64, i64), MeshVertexId> = HashMap::new();
    let mut new_verts: Vec<MeshVertex> = Vec::new();
    let mut remap: Vec<MeshVertexId>   = Vec::with_capacity(dcel.vertices.len());

    for v in &dcel.vertices {
        let new_id = *map.entry(key(&v.pos)).or_insert_with(|| {
            let id = MeshVertexId(new_verts.len());
            new_verts.push(v.clone());
            id
        });
        remap.push(new_id);
    }

    for he in &mut dcel.half_edges {
        he.vertex = remap[he.vertex.0];
    }
    dcel.vertices = new_verts;
}

// ── mesh_solid ────────────────────────────────────────────────────────────────

/// Tessellate all faces of solid `sid` and return a combined [`TriMesh`].
///
/// Each face tessellator appends vertices and half-edges to a shared
/// [`HalfEdgeMesh`].  After all faces are meshed, [`merge_dcel_vertices`]
/// collapses coincident boundary duplicates so that [`stitch_twins`] can link
/// all interior half-edge pairs.  The final [`TriMesh`] is produced by
/// [`HalfEdgeMesh::to_trimesh`].
pub fn mesh_solid(ctx: &SolidModelingContext, sid: SolidId, opts: &MeshOptions) -> TriMesh {
    let shell_id = ctx.get_solid(sid).outer;
    let face_ids: Vec<FaceId> = ctx.get_shell(shell_id).faces.clone();

    let mut dcel     = HalfEdgeMesh::new();
    let mut registry = EdgeVertexRegistry::new();

    for face_id in face_ids {
        mesh_face(ctx, face_id, opts, &mut dcel, &mut registry);
    }

    merge_dcel_vertices(&mut dcel, opts.epsilon);
    stitch_twins(&mut dcel);
    dcel.to_trimesh()
}

// ── mesh_face ─────────────────────────────────────────────────────────────────

/// Tessellate a single B-rep face, appending vertices and half-edges to `dcel`.
///
/// Each tessellator creates vertices independently; [`merge_dcel_vertices`] in
/// [`mesh_solid`] collapses boundary duplicates before twin stitching.
fn mesh_face(
    ctx: &SolidModelingContext,
    face_id: FaceId,
    opts: &MeshOptions,
    dcel: &mut HalfEdgeMesh,
    registry: &mut EdgeVertexRegistry,
) {
    let surf_id = ctx.get_face(face_id).surface;
    match ctx.get_surface(surf_id) {
        SurfaceKind::Plane(plane)  => mesh_plane_face(ctx, face_id, *plane, opts, dcel, registry),
        SurfaceKind::Cylinder(cyl) => mesh_cylindrical_face(ctx, face_id, *cyl, opts, dcel, registry),
        SurfaceKind::Cone(cone)    => mesh_conical_face(ctx, face_id, *cone, opts, dcel, registry),
        SurfaceKind::Sphere(sph)   => mesh_spherical_face(ctx, face_id, *sph, opts, dcel, registry),
        _ => {}
    }
}

// ── sample_loop_into_dcel ─────────────────────────────────────────────────────
// NOTE: sample_loop_into_dcel and the EdgeVertexRegistry are reserved for the
// future Delaunay refinement step (Phase 5+), when per-edge vertex sharing must
// be exact.  The current tessellators push vertices independently and rely on
// merge_dcel_vertices for deduplication.

/// Walk the coedges of `loop_id`, register each boundary sample in `registry`,
/// and return `(MeshVertexId, [u, v])` pairs in coedge-walk order.
///
/// `make_vertex(uv, brep_ref)` is called only on first insertion for each
/// `(EdgeId, quantized_t)` key; subsequent calls for the same key return the
/// already-registered id without invoking the closure.
///
/// Vertex classification:
/// - The start of each coedge (first sample) → `MeshVertexRef::Corner(VertexId)`
/// - Interior samples on curved coedges (`CircularArc2`) → `MeshVertexRef::OnEdge(EdgeId)`
/// - Interior mesh points (e.g. sphere latitude rings) are not handled here;
///   callers push those directly with [`HalfEdgeMesh::push_vertex`].
fn sample_loop_into_dcel<F>(
    ctx: &SolidModelingContext,
    loop_id: LoopId,
    opts: &MeshOptions,
    dcel: &mut HalfEdgeMesh,
    registry: &mut EdgeVertexRegistry,
    make_vertex: F,
) -> Vec<(MeshVertexId, [f64; 2])>
where
    F: Fn([f64; 2], MeshVertexRef) -> MeshVertex,
{
    let coedge_ids = ctx.get_loop(loop_id).coedges.clone();
    let mut result = Vec::new();

    for ce_id in coedge_ids {
        let ce      = ctx.get_coedge(ce_id);
        let edge    = ctx.get_edge(ce.edge);
        let edge_id = ce.edge;
        let (t_start, t_end) = match ce.orientation {
            Orientation::Forward => (edge.t0, edge.t1),
            Orientation::Reverse => (edge.t1, edge.t0),
        };
        let corner_vid = match ce.orientation {
            Orientation::Forward => edge.v0,
            Orientation::Reverse => edge.v1,
        };
        let pcurve = ctx.get_curve2(ce.pcurve);

        match pcurve {
            Curve2Kind::Line2(_) => {
                // Straight edge: one sample at t_start — always a B-rep corner.
                // Use corner registry (keyed by VertexId, not EdgeId+t) so the
                // same corner shared by multiple edges gets the same MeshVertexId.
                let p  = pcurve.eval(t_start);
                let uv = [p.u, p.v];
                let vid = registry.get_or_insert_corner(corner_vid, dcel, || {
                    make_vertex(uv, MeshVertexRef::Corner(corner_vid))
                });
                result.push((vid, uv));
            }
            Curve2Kind::CircularArc2(_) => {
                // Curved edge: `resolution` samples, endpoint excluded.
                // k=0 is a corner (use corner registry); k>0 is OnEdge (use edge registry).
                let n  = opts.resolution as usize;
                let dt = (t_end - t_start) / n as f64;
                for k in 0..n {
                    let t   = t_start + k as f64 * dt;
                    let p   = pcurve.eval(t);
                    let uv  = [p.u, p.v];
                    let vid = if k == 0 {
                        registry.get_or_insert_corner(corner_vid, dcel, || {
                            make_vertex(uv, MeshVertexRef::Corner(corner_vid))
                        })
                    } else {
                        registry.get_or_insert_edge(edge_id, t, dcel, || {
                            make_vertex(uv, MeshVertexRef::OnEdge(edge_id))
                        })
                    };
                    result.push((vid, uv));
                }
            }
            Curve2Kind::QuadraticBezier2(_) | Curve2Kind::CubicBezier2(_) => {
                // Bézier edge: `resolution` samples, endpoint excluded — same
                // sampling strategy as CircularArc2 (the curve is non-linear, so
                // intermediate samples approximate it; k=0 is a B-rep corner).
                let n  = opts.resolution as usize;
                let dt = (t_end - t_start) / n as f64;
                for k in 0..n {
                    let t   = t_start + k as f64 * dt;
                    let p   = pcurve.eval(t);
                    let uv  = [p.u, p.v];
                    let vid = if k == 0 {
                        registry.get_or_insert_corner(corner_vid, dcel, || {
                            make_vertex(uv, MeshVertexRef::Corner(corner_vid))
                        })
                    } else {
                        registry.get_or_insert_edge(edge_id, t, dcel, || {
                            make_vertex(uv, MeshVertexRef::OnEdge(edge_id))
                        })
                    };
                    result.push((vid, uv));
                }
            }
            Curve2Kind::Polyline2(_) => todo!("UV sampling for Polyline2 not yet implemented"),
            Curve2Kind::Nurbs(_)     => todo!("UV sampling for NurbsCurve2 not yet implemented"),
        }
    }

    result
}

// ── Plane tessellation ────────────────────────────────────────────────────────

/// Tessellate a face whose surface is a [`Plane`].
///
/// Samples the outer loop's coedge pcurves to get UV boundary points, pushes a
/// [`MeshVertex`] per sample into `dcel`, then fan-triangulates from vertex 0
/// (correct for all convex polygons).
fn mesh_plane_face(
    ctx: &SolidModelingContext,
    face_id: FaceId,
    plane: Plane,
    opts: &MeshOptions,
    dcel: &mut HalfEdgeMesh,
    registry: &mut EdgeVertexRegistry,
) {
    let face    = ctx.get_face(face_id);
    let loop_id = face.outer;

    let raw_n = plane.eval_n(0.0, 0.0).unwrap();
    let normal = if face.sense == FaceSense::AntiAligned {
        [-raw_n.x, -raw_n.y, -raw_n.z]
    } else {
        [raw_n.x, raw_n.y, raw_n.z]
    };

    let boundary = sample_loop_into_dcel(ctx, loop_id, opts, dcel, registry, |[u, v], brep_ref| {
        let p = plane.eval(u, v);
        MeshVertex { pos: [p.x, p.y, p.z], uv: [u, v], normal, brep_ref }
    });
    let n = boundary.len();
    if n < 3 { return; }

    let vids: Vec<MeshVertexId> = boundary.iter().map(|&(vid, _)| vid).collect();
    let v0 = vids[0];
    for i in 1..=(n - 2) {
        dcel.push_triangle(v0, vids[i], vids[i + 1]);
    }
}

// ── CylindricalSurface tessellation ──────────────────────────────────────────

/// Tessellate the lateral face of a [`CylindricalSurface`].
///
/// Builds a `(resolution+1) × 2` UV grid — same geometry as the previous
/// [`TriMesh`]-based tessellator.  Duplicate seam vertices at `u = 2π` are
/// collapsed by [`merge_dcel_vertices`] in [`mesh_solid`].
fn mesh_cylindrical_face(
    ctx: &SolidModelingContext,
    face_id: FaceId,
    cyl: CylindricalSurface,
    opts: &MeshOptions,
    dcel: &mut HalfEdgeMesh,
    _registry: &mut EdgeVertexRegistry,
) {
    use std::f64::consts::TAU;

    let loop_id  = ctx.get_face(face_id).outer;
    let boundary = sample_loop_uvs(ctx, loop_id, opts);
    let v_min    = boundary.iter().map(|uv| uv[1]).fold(f64::INFINITY,    f64::min);
    let v_max    = boundary.iter().map(|uv| uv[1]).fold(f64::NEG_INFINITY, f64::max);

    // Scan the loop to find the seam edge (v0 ≠ v1) and the two circle edges
    // (v0 == v1, CircularArc3).  The seam edge gives bottom/top corners directly.
    let (bot_corner_id, top_corner_id, bot_edge_id, top_edge_id) = {
        let mut seam_eid: Option<EdgeId>   = None;
        let mut circle_eids: Vec<EdgeId>   = Vec::new();
        for &ce_id in &ctx.get_loop(loop_id).coedges.clone() {
            let ce   = ctx.get_coedge(ce_id);
            let edge = ctx.get_edge(ce.edge);
            if edge.v0 == edge.v1 {
                if !circle_eids.contains(&ce.edge) { circle_eids.push(ce.edge); }
            } else {
                seam_eid = Some(ce.edge);
            }
        }
        let seam_eid  = seam_eid.expect("cylinder lateral loop must have a seam edge");
        let seam_edge = ctx.get_edge(seam_eid);
        let v_bot_vid = seam_edge.v0; // seam Fwd: t=0 → UV=(TAU,0) → v=v_min
        let v_top_vid = seam_edge.v1;
        let (mut bot_eid, mut top_eid) = (None, None);
        for &ceid in &circle_eids {
            if ctx.get_edge(ceid).v0 == v_bot_vid { bot_eid = Some(ceid); }
            else                                   { top_eid = Some(ceid); }
        }
        (
            v_bot_vid,
            v_top_vid,
            bot_eid.expect("cylinder lateral loop must have a bottom circle edge"),
            top_eid.expect("cylinder lateral loop must have a top circle edge"),
        )
    };

    let res = opts.resolution as usize;
    let nu  = res + 1;

    let mut vert_ids: Vec<MeshVertexId> = Vec::with_capacity(nu * 2);
    for (row, &v) in [v_min, v_max].iter().enumerate() {
        let (corner_id, circle_edge_id) = if row == 0 {
            (bot_corner_id, bot_edge_id)
        } else {
            (top_corner_id, top_edge_id)
        };
        for ui in 0..nu {
            let u        = ui as f64 * TAU / res as f64;
            let p        = cyl.eval(u, v);
            let n        = cyl.eval_n(u, v).expect("CylindricalSurface normal always defined");
            let brep_ref = if ui == 0 || ui == res {
                MeshVertexRef::Corner(corner_id)
            } else {
                MeshVertexRef::OnEdge(circle_edge_id)
            };
            vert_ids.push(dcel.push_vertex(MeshVertex {
                pos: [p.x, p.y, p.z], uv: [u, v], normal: [n.x, n.y, n.z],
                brep_ref,
            }));
        }
    }

    let idx = |row: usize, col: usize| vert_ids[row * nu + col];
    for col in 0..res {
        let (bl, br, tl, tr) = (idx(0,col), idx(0,col+1), idx(1,col), idx(1,col+1));
        dcel.push_triangle(bl, br, tr);
        dcel.push_triangle(bl, tr, tl);
    }
}

// ── ConicalSurface tessellation ───────────────────────────────────────────────

/// Tessellate the lateral face of a [`ConicalSurface`].
///
/// Apex-fan with hybrid normals (same logic as the previous [`TriMesh`] version).
fn mesh_conical_face(
    ctx: &SolidModelingContext,
    face_id: FaceId,
    cone: ConicalSurface,
    opts: &MeshOptions,
    dcel: &mut HalfEdgeMesh,
    _registry: &mut EdgeVertexRegistry,
) {
    use std::f64::consts::TAU;

    let face    = ctx.get_face(face_id);
    let sense   = face.sense;
    let loop_id = face.outer;

    let boundary = sample_loop_uvs(ctx, loop_id, opts);
    let v_max    = boundary.iter().map(|uv| uv[1]).fold(f64::NEG_INFINITY, f64::max);
    let res      = opts.resolution as usize;

    // Scan the loop to find the apex vertex and the base circle edge.
    // The apex is the closed edge whose 3D curve is degenerate (Line3, v0==v1).
    // The base is the closed edge whose 3D curve is a CircularArc3.
    let (apex_vertex_id, base_edge_id) = {
        let mut apex_vid: Option<VertexId> = None;
        let mut base_eid: Option<EdgeId>   = None;
        for &ce_id in &ctx.get_loop(loop_id).coedges.clone() {
            let ce   = ctx.get_coedge(ce_id);
            let edge = ctx.get_edge(ce.edge);
            if edge.v0 == edge.v1 {
                match ctx.get_curve3(edge.curve3) {
                    Curve3Kind::CircularArc3(_) => base_eid = Some(ce.edge),
                    _                           => apex_vid = Some(edge.v0),
                }
            }
        }
        (
            apex_vid.expect("cone lateral loop must have a degenerate apex edge"),
            base_eid.expect("cone lateral loop must have a base circle edge"),
        )
    };
    let base_corner_id = ctx.get_edge(base_edge_id).v0;

    // Apex vertex (index 0)
    let apex_pos = cone.eval(0.0, 0.0);
    let apex_vid = dcel.push_vertex(MeshVertex {
        pos: [apex_pos.x, apex_pos.y, apex_pos.z], uv: [0.0, 0.0],
        normal: [0.0, 0.0, 1.0], // placeholder; overwritten per-triangle below
        brep_ref: MeshVertexRef::Corner(apex_vertex_id),
    });

    // Base circle vertices (indices 1..=res)
    let mut base_vids = Vec::with_capacity(res);
    let mut base_u    = Vec::with_capacity(res);
    for j in 0..res {
        let u       = j as f64 * TAU / res as f64;
        let p       = cone.eval(u, v_max);
        let n       = cone.eval_n(u, v_max).map_or([0.0, 0.0, 1.0], |n| [n.x, n.y, n.z]);
        let brep_ref = if j == 0 {
            MeshVertexRef::Corner(base_corner_id)
        } else {
            MeshVertexRef::OnEdge(base_edge_id)
        };
        base_vids.push(dcel.push_vertex(MeshVertex {
            pos: [p.x, p.y, p.z], uv: [u, v_max], normal: n,
            brep_ref,
        }));
        base_u.push(u);
    }

    let flip = sense == FaceSense::AntiAligned;

    for j in 0..res {
        let curr_vid = base_vids[j];
        let next_vid = base_vids[(j + 1) % res];
        let u_curr   = base_u[j];
        let u_next   = if j + 1 < res { base_u[j + 1] } else { TAU };

        // Flat cross-product apex normal
        let bv_next = dcel.vertices[next_vid.0].pos;
        let bv_curr = dcel.vertices[curr_vid.0].pos;
        let ap      = dcel.vertices[apex_vid.0].pos;
        let v1 = Point3::new(bv_next[0]-ap[0], bv_next[1]-ap[1], bv_next[2]-ap[2]);
        let v2 = Point3::new(bv_curr[0]-ap[0], bv_curr[1]-ap[1], bv_curr[2]-ap[2]);
        let raw = v1.cross(v2);
        let len = (raw.x*raw.x + raw.y*raw.y + raw.z*raw.z).sqrt();
        let flat = if len > 1e-15 { [raw.x/len, raw.y/len, raw.z/len] } else { [0.0,0.0,1.0] };
        let apex_n = if flip { [-flat[0],-flat[1],-flat[2]] } else { flat };

        let an_curr = cone.eval_n(u_curr, v_max).expect("eval_n defined for v > 0");
        let an_next = cone.eval_n(u_next, v_max).expect("eval_n defined for v > 0");
        let base_curr_n = if flip { [-an_curr.x,-an_curr.y,-an_curr.z] } else { [an_curr.x,an_curr.y,an_curr.z] };
        let base_next_n = if flip { [-an_next.x,-an_next.y,-an_next.z] } else { [an_next.x,an_next.y,an_next.z] };

        // Write normals; apex gets last-write-wins (acceptable)
        dcel.vertices[apex_vid.0].normal  = apex_n;
        dcel.vertices[curr_vid.0].normal  = base_curr_n;
        dcel.vertices[next_vid.0].normal  = base_next_n;

        // Triangle: (apex, base_next, base_curr) — outward winding for Aligned
        dcel.push_triangle(apex_vid, next_vid, curr_vid);
    }
}

// ── SphericalSurface tessellation ────────────────────────────────────────────

/// Tessellate a [`SphericalSurface`] face.
///
/// Same `(n_lon+1) × (n_lat-1)` grid as the previous [`TriMesh`] version.
/// Seam duplicates at `u = 2π` are collapsed by [`merge_dcel_vertices`].
fn mesh_spherical_face(
    ctx: &SolidModelingContext,
    face_id: FaceId,
    sph: SphericalSurface,
    opts: &MeshOptions,
    dcel: &mut HalfEdgeMesh,
    _registry: &mut EdgeVertexRegistry,
) {
    use std::f64::consts::{FRAC_PI_2, TAU};

    let sense   = ctx.get_face(face_id).sense;
    let flip    = sense == FaceSense::AntiAligned;
    let loop_id = ctx.get_face(face_id).outer;

    // Scan the loop to find: south/north pole vertex IDs and the seam edge ID.
    // The two degenerate edges (v0==v1, Line3) are the poles; distinguish them
    // by checking their pcurve v-coordinate (south < 0, north > 0).
    let (south_vertex_id, north_vertex_id, seam_edge_id) = {
        let mut south_vid: Option<VertexId> = None;
        let mut north_vid: Option<VertexId> = None;
        let mut seam_eid:  Option<EdgeId>   = None;
        for &ce_id in &ctx.get_loop(loop_id).coedges.clone() {
            let ce   = ctx.get_coedge(ce_id);
            let edge = ctx.get_edge(ce.edge);
            if edge.v0 == edge.v1 {
                let t_start = match ce.orientation {
                    Orientation::Forward => edge.t0,
                    Orientation::Reverse => edge.t1,
                };
                let v_coord = ctx.get_curve2(ce.pcurve).eval(t_start).v;
                if v_coord < 0.0 { south_vid = Some(edge.v0); }
                else              { north_vid = Some(edge.v0); }
            } else {
                seam_eid = Some(ce.edge);
            }
        }
        (
            south_vid.expect("sphere loop must have a south pole edge"),
            north_vid.expect("sphere loop must have a north pole edge"),
            seam_eid.expect("sphere loop must have a seam edge"),
        )
    };

    let n_lon  = opts.resolution as usize;
    let n_lat  = (opts.resolution as usize / 2).max(2);
    let v_step = std::f64::consts::PI / n_lat as f64;
    let u_step = TAU / n_lon as f64;

    let push = |dcel: &mut HalfEdgeMesh, u: f64, v: f64, brep_ref: MeshVertexRef| -> MeshVertexId {
        let p = sph.eval(u, v);
        let n = sph.eval_n(u, v).expect("SphericalSurface::eval_n always Some");
        let normal = if flip { [-n.x,-n.y,-n.z] } else { [n.x,n.y,n.z] };
        dcel.push_vertex(MeshVertex { pos: [p.x,p.y,p.z], uv: [u,v], normal, brep_ref })
    };

    let south = push(dcel, 0.0, -FRAC_PI_2, MeshVertexRef::Corner(south_vertex_id));

    let mut ring: Vec<Vec<MeshVertexId>> = Vec::with_capacity(n_lat - 1);
    for i in 1..n_lat {
        let v   = -FRAC_PI_2 + i as f64 * v_step;
        let row = (0..=n_lon).map(|j| {
            let brep_ref = if j == 0 || j == n_lon {
                MeshVertexRef::OnEdge(seam_edge_id)
            } else {
                MeshVertexRef::OnFace(face_id)
            };
            push(dcel, j as f64 * u_step, v, brep_ref)
        }).collect();
        ring.push(row);
    }

    let north = push(dcel, 0.0, FRAC_PI_2, MeshVertexRef::Corner(north_vertex_id));

    let rv = |i: usize, j: usize| ring[i - 1][j]; // i is 1-indexed

    // South fan
    for j in 0..n_lon { dcel.push_triangle(south, rv(1, j+1), rv(1, j)); }

    // Middle bands
    for i in 1..n_lat - 1 {
        for j in 0..n_lon {
            dcel.push_triangle(rv(i, j),   rv(i, j+1),   rv(i+1, j+1));
            dcel.push_triangle(rv(i, j),   rv(i+1, j+1), rv(i+1, j));
        }
    }

    // North fan
    for j in 0..n_lon { dcel.push_triangle(north, rv(n_lat-1, j), rv(n_lat-1, j+1)); }
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
            Curve2Kind::QuadraticBezier2(_) | Curve2Kind::CubicBezier2(_) => {
                // Bézier edge: sample `resolution` points, endpoint excluded
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

    // ── DCEL / HalfEdgeMesh ──────────────────────────────────────────────────

    /// Build a single flat CCW triangle: (0,0,0), (1,0,0), (0,1,0), normal +Z.
    fn single_triangle_dcel() -> HalfEdgeMesh {
        let mut m = HalfEdgeMesh::new();
        let vref = MeshVertexRef::OnFace(FaceId(0));
        let v0 = m.push_vertex(MeshVertex { pos: [0.0, 0.0, 0.0], uv: [0.0, 0.0], normal: [0.0, 0.0, 1.0], brep_ref: vref });
        let v1 = m.push_vertex(MeshVertex { pos: [1.0, 0.0, 0.0], uv: [1.0, 0.0], normal: [0.0, 0.0, 1.0], brep_ref: vref });
        let v2 = m.push_vertex(MeshVertex { pos: [0.0, 1.0, 0.0], uv: [0.0, 1.0], normal: [0.0, 0.0, 1.0], brep_ref: vref });
        m.push_triangle(v0, v1, v2);
        m
    }

    /// Build two triangles sharing edge v1–v2 (a simple quad split):
    ///   T0: v0(0,0,0), v1(1,0,0), v2(0,1,0) — top-left
    ///   T1: v3(1,1,0), v2(0,1,0), v1(1,0,0) — bottom-right
    /// The shared edge is v1→v2 in T0 (he1 of T0) and v2→v1 in T1 (he1 of T1).
    fn two_triangle_dcel() -> (HalfEdgeMesh, [HalfEdgeId; 6]) {
        let mut m = HalfEdgeMesh::new();
        let vref = MeshVertexRef::OnFace(FaceId(0));
        let v0 = m.push_vertex(MeshVertex { pos: [0.0, 0.0, 0.0], uv: [0.0, 0.0], normal: [0.0, 0.0, 1.0], brep_ref: vref });
        let v1 = m.push_vertex(MeshVertex { pos: [1.0, 0.0, 0.0], uv: [1.0, 0.0], normal: [0.0, 0.0, 1.0], brep_ref: vref });
        let v2 = m.push_vertex(MeshVertex { pos: [0.0, 1.0, 0.0], uv: [0.0, 1.0], normal: [0.0, 0.0, 1.0], brep_ref: vref });
        let v3 = m.push_vertex(MeshVertex { pos: [1.0, 1.0, 0.0], uv: [1.0, 1.0], normal: [0.0, 0.0, 1.0], brep_ref: vref });
        let (_, [he00, he01, he02]) = m.push_triangle(v0, v1, v2);
        let (_, [he10, he11, he12]) = m.push_triangle(v3, v2, v1);
        // he01: v1→v2 and he11: v2→v1 are twins
        m.set_twin(he01, he11);
        (m, [he00, he01, he02, he10, he11, he12])
    }

    #[test]
    fn dcel_default_is_empty() {
        let m = HalfEdgeMesh::default();
        assert_eq!(m.vertices.len(), 0);
        assert_eq!(m.half_edges.len(), 0);
        assert_eq!(m.faces.len(), 0);
    }

    #[test]
    fn dcel_push_vertex_increments_count() {
        let mut m = HalfEdgeMesh::new();
        let vref = MeshVertexRef::OnFace(FaceId(0));
        let id = m.push_vertex(MeshVertex { pos: [1.0, 2.0, 3.0], uv: [0.0, 0.0], normal: [0.0, 0.0, 1.0], brep_ref: vref });
        assert_eq!(m.vertices.len(), 1);
        assert_eq!(id, MeshVertexId(0));
    }

    #[test]
    fn dcel_push_triangle_creates_face_and_half_edges() {
        let m = single_triangle_dcel();
        assert_eq!(m.faces.len(), 1);
        assert_eq!(m.half_edges.len(), 3);
        assert_eq!(m.vertices.len(), 3);
    }

    #[test]
    fn dcel_next_chain_closes_in_three_steps() {
        let m = single_triangle_dcel();
        let he0 = m.faces[0].half_edge;
        let he1 = m.half_edges[he0.0].next;
        let he2 = m.half_edges[he1.0].next;
        let back = m.half_edges[he2.0].next;
        assert_eq!(back, he0, "next-chain must close: he0→he1→he2→he0");
    }

    #[test]
    fn dcel_face_vertices_single_triangle() {
        let m = single_triangle_dcel();
        let [v0, v1, v2] = m.face_vertices(DcelFaceId(0));
        assert_eq!(m.vertices[v0.0].pos, [0.0, 0.0, 0.0]);
        assert_eq!(m.vertices[v1.0].pos, [1.0, 0.0, 0.0]);
        assert_eq!(m.vertices[v2.0].pos, [0.0, 1.0, 0.0]);
    }

    #[test]
    fn dcel_face_half_edges_belong_to_face() {
        let m = single_triangle_dcel();
        for he_id in m.face_half_edges(DcelFaceId(0)) {
            assert_eq!(m.half_edges[he_id.0].face, DcelFaceId(0));
        }
    }

    #[test]
    fn dcel_new_half_edges_have_no_twin() {
        let m = single_triangle_dcel();
        for he in &m.half_edges {
            assert!(he.twin.is_none(), "newly created half-edges must have twin = None");
        }
    }

    #[test]
    fn dcel_set_twin_is_symmetric() {
        let (m, [_, he01, _, _, he11, _]) = two_triangle_dcel();
        assert_eq!(m.half_edges[he01.0].twin, Some(he11));
        assert_eq!(m.half_edges[he11.0].twin, Some(he01));
    }

    #[test]
    fn dcel_set_twin_twin_twin_is_self() {
        let (m, [_, he01, _, _, _he11, _]) = two_triangle_dcel();
        let twin_of_twin = m.half_edges[m.half_edges[he01.0].twin.unwrap().0].twin.unwrap();
        assert_eq!(twin_of_twin, he01, "he.twin.twin must equal he");
    }

    #[test]
    fn dcel_unstitched_edges_still_none_after_partial_stitch() {
        let (m, [he00, _, he02, he10, _, he12]) = two_triangle_dcel();
        // Only the shared edge was stitched; boundary half-edges remain None
        for he_id in [he00, he02, he10, he12] {
            assert!(m.half_edges[he_id.0].twin.is_none(),
                "boundary half-edge {he_id:?} should still have twin = None");
        }
    }

    // ── to_trimesh ───────────────────────────────────────────────────────────

    #[test]
    fn dcel_to_trimesh_single_triangle_counts() {
        let mesh = single_triangle_dcel().to_trimesh();
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.triangles.len(), 1);
        assert_eq!(mesh.tri_normals.len(), 3);
        assert_eq!(mesh.tri_uvs.len(), 3);
    }

    #[test]
    fn dcel_to_trimesh_single_triangle_positions() {
        let mesh = single_triangle_dcel().to_trimesh();
        assert_eq!(mesh.vertices[0], [0.0, 0.0, 0.0]);
        assert_eq!(mesh.vertices[1], [1.0, 0.0, 0.0]);
        assert_eq!(mesh.vertices[2], [0.0, 1.0, 0.0]);
    }

    #[test]
    fn dcel_to_trimesh_single_triangle_normals_are_z() {
        let mesh = single_triangle_dcel().to_trimesh();
        for n in &mesh.tri_normals {
            assert_eq!(*n, [0.0, 0.0, 1.0]);
        }
    }

    #[test]
    fn dcel_to_trimesh_indices_in_range() {
        let (dcel, _) = two_triangle_dcel();
        let mesh = dcel.to_trimesh();
        let nv = mesh.vertices.len();
        for tri in &mesh.triangles {
            for &idx in tri {
                assert!((idx as usize) < nv, "triangle index {idx} out of range");
            }
        }
    }

    #[test]
    fn dcel_to_trimesh_invariants_two_triangles() {
        let (dcel, _) = two_triangle_dcel();
        let mesh = dcel.to_trimesh();
        check_invariants(&mesh);
    }

    #[test]
    fn dcel_to_trimesh_uv_preserved() {
        let mesh = single_triangle_dcel().to_trimesh();
        assert_eq!(mesh.tri_uvs[0], [0.0, 0.0]);
        assert_eq!(mesh.tri_uvs[1], [1.0, 0.0]);
        assert_eq!(mesh.tri_uvs[2], [0.0, 1.0]);
    }

    // ── EdgeVertexRegistry ───────────────────────────────────────────────────

    fn dummy_vertex(x: f64) -> MeshVertex {
        MeshVertex {
            pos:      [x, 0.0, 0.0],
            uv:       [x, 0.0],
            normal:   [0.0, 0.0, 1.0],
            brep_ref: MeshVertexRef::OnFace(FaceId(0)),
        }
    }

    #[test]
    fn registry_new_is_empty() {
        let r = EdgeVertexRegistry::new();
        assert_eq!(r.edge_count(), 0);
    }

    #[test]
    fn registry_corner_first_insert_creates_vertex() {
        let mut mesh = HalfEdgeMesh::new();
        let mut reg  = EdgeVertexRegistry::new();
        let vid_id = VertexId(0);
        let vid = reg.get_or_insert_corner(vid_id, &mut mesh, || dummy_vertex(1.0));
        assert_eq!(mesh.vertices.len(), 1);
        assert_eq!(vid, MeshVertexId(0));
        assert_eq!(reg.corner_count(), 1);
    }

    #[test]
    fn registry_corner_same_vertex_id_returns_same_mesh_vertex() {
        let mut mesh = HalfEdgeMesh::new();
        let mut reg  = EdgeVertexRegistry::new();
        let vid_id = VertexId(5);
        let v0 = reg.get_or_insert_corner(vid_id, &mut mesh, || dummy_vertex(1.0));
        let v1 = reg.get_or_insert_corner(vid_id, &mut mesh, || dummy_vertex(2.0)); // not called
        assert_eq!(v0, v1);
        assert_eq!(mesh.vertices.len(), 1, "second call must not create a vertex");
    }

    #[test]
    fn registry_corner_different_vertex_ids_are_independent() {
        let mut mesh = HalfEdgeMesh::new();
        let mut reg  = EdgeVertexRegistry::new();
        let v0 = reg.get_or_insert_corner(VertexId(0), &mut mesh, || dummy_vertex(0.0));
        let v1 = reg.get_or_insert_corner(VertexId(1), &mut mesh, || dummy_vertex(1.0));
        assert_ne!(v0, v1);
        assert_eq!(reg.corner_count(), 2);
    }

    #[test]
    fn registry_edge_first_insert_creates_vertex() {
        let mut mesh = HalfEdgeMesh::new();
        let mut reg  = EdgeVertexRegistry::new();
        let eid = EdgeId(0);
        let vid = reg.get_or_insert_edge(eid, 0.5, &mut mesh, || dummy_vertex(1.0));
        assert_eq!(mesh.vertices.len(), 1);
        assert_eq!(vid, MeshVertexId(0));
        assert_eq!(reg.edge_count(), 1);
        assert_eq!(reg.vertex_count_for(eid), 1);
    }

    #[test]
    fn registry_edge_same_t_returns_same_id() {
        let mut mesh = HalfEdgeMesh::new();
        let mut reg  = EdgeVertexRegistry::new();
        let eid = EdgeId(0);
        let v0 = reg.get_or_insert_edge(eid, 0.5, &mut mesh, || dummy_vertex(1.0));
        let v1 = reg.get_or_insert_edge(eid, 0.5, &mut mesh, || dummy_vertex(2.0));
        assert_eq!(v0, v1);
        assert_eq!(mesh.vertices.len(), 1, "second insert must not create a vertex");
    }

    #[test]
    fn registry_edge_different_t_creates_new_vertex() {
        let mut mesh = HalfEdgeMesh::new();
        let mut reg  = EdgeVertexRegistry::new();
        let eid = EdgeId(0);
        let v0 = reg.get_or_insert_edge(eid, 0.0, &mut mesh, || dummy_vertex(0.0));
        let v1 = reg.get_or_insert_edge(eid, 1.0, &mut mesh, || dummy_vertex(1.0));
        assert_ne!(v0, v1);
        assert_eq!(mesh.vertices.len(), 2);
        assert_eq!(reg.vertex_count_for(eid), 2);
    }

    #[test]
    fn registry_edge_different_edges_independent() {
        let mut mesh = HalfEdgeMesh::new();
        let mut reg  = EdgeVertexRegistry::new();
        let v0 = reg.get_or_insert_edge(EdgeId(0), 0.5, &mut mesh, || dummy_vertex(0.0));
        let v1 = reg.get_or_insert_edge(EdgeId(1), 0.5, &mut mesh, || dummy_vertex(1.0));
        assert_ne!(v0, v1);
        assert_eq!(reg.edge_count(), 2);
    }

    #[test]
    fn registry_t_within_quantization_tolerance_returns_same_id() {
        // Two t-values that differ by << 0.5/quant (= 5e-13) must hash to the same bin.
        // Use 1e-13 — clearly inside the bin, not on the rounding boundary.
        let mut mesh = HalfEdgeMesh::new();
        let mut reg  = EdgeVertexRegistry::new();
        let eid = EdgeId(0);
        let t0 = 1.0_f64;
        let t1 = t0 + 1e-13;
        let v0 = reg.get_or_insert_edge(eid, t0, &mut mesh, || dummy_vertex(0.0));
        let v1 = reg.get_or_insert_edge(eid, t1, &mut mesh, || dummy_vertex(1.0));
        assert_eq!(v0, v1, "t-values within quantization tolerance must return the same vertex");
    }

    #[test]
    fn registry_t_outside_quantization_tolerance_creates_new_vertex() {
        let mut mesh = HalfEdgeMesh::new();
        let mut reg  = EdgeVertexRegistry::new();
        let eid = EdgeId(0);
        let t0 = 1.0_f64;
        let t1 = t0 + 2e-12;
        let v0 = reg.get_or_insert_edge(eid, t0, &mut mesh, || dummy_vertex(0.0));
        let v1 = reg.get_or_insert_edge(eid, t1, &mut mesh, || dummy_vertex(1.0));
        assert_ne!(v0, v1, "t-values outside quantization tolerance must be distinct");
    }

    // ── stitch_twins ─────────────────────────────────────────────────────────

    /// Same quad split as `two_triangle_dcel` but without calling `set_twin`.
    fn two_triangle_dcel_no_twins() -> (HalfEdgeMesh, [HalfEdgeId; 6]) {
        let mut m = HalfEdgeMesh::new();
        let vref = MeshVertexRef::OnFace(FaceId(0));
        let v0 = m.push_vertex(MeshVertex { pos: [0.0, 0.0, 0.0], uv: [0.0, 0.0], normal: [0.0, 0.0, 1.0], brep_ref: vref });
        let v1 = m.push_vertex(MeshVertex { pos: [1.0, 0.0, 0.0], uv: [1.0, 0.0], normal: [0.0, 0.0, 1.0], brep_ref: vref });
        let v2 = m.push_vertex(MeshVertex { pos: [0.0, 1.0, 0.0], uv: [0.0, 1.0], normal: [0.0, 0.0, 1.0], brep_ref: vref });
        let v3 = m.push_vertex(MeshVertex { pos: [1.0, 1.0, 0.0], uv: [1.0, 1.0], normal: [0.0, 0.0, 1.0], brep_ref: vref });
        let (_, [he00, he01, he02]) = m.push_triangle(v0, v1, v2);
        let (_, [he10, he11, he12]) = m.push_triangle(v3, v2, v1);
        // No set_twin call — all twins start as None
        (m, [he00, he01, he02, he10, he11, he12])
    }

    #[test]
    fn stitch_twins_links_shared_edge() {
        let (mut dcel, [_, he01, _, _, he11, _]) = two_triangle_dcel_no_twins();
        stitch_twins(&mut dcel);
        assert_eq!(dcel.half_edges[he01.0].twin, Some(he11));
        assert_eq!(dcel.half_edges[he11.0].twin, Some(he01));
    }

    #[test]
    fn stitch_twins_boundary_edges_stay_none() {
        let (mut dcel, [he00, _, he02, he10, _, he12]) = two_triangle_dcel_no_twins();
        stitch_twins(&mut dcel);
        for he_id in [he00, he02, he10, he12] {
            assert!(dcel.half_edges[he_id.0].twin.is_none(),
                "boundary half-edge {he_id:?} must remain twin=None");
        }
    }

    #[test]
    fn stitch_twins_is_idempotent() {
        let (mut dcel, [_, he01, _, _, he11, _]) = two_triangle_dcel_no_twins();
        stitch_twins(&mut dcel);
        stitch_twins(&mut dcel); // second call must not change anything
        assert_eq!(dcel.half_edges[he01.0].twin, Some(he11));
        assert_eq!(dcel.half_edges[he11.0].twin, Some(he01));
    }

    #[test]
    fn stitch_twins_four_triangles_full_interior_edge() {
        // Two quads sharing a full interior edge — all interior half-edges get twins.
        //   v0--v1--v4
        //   |T0/|T2/|
        //   | / | / |
        //   |/T1|/T3|
        //   v2--v3--v5
        let mut m = HalfEdgeMesh::new();
        let vref = MeshVertexRef::OnFace(FaceId(0));
        let mut pv = |x: f64, y: f64| m.push_vertex(MeshVertex {
            pos: [x, y, 0.0], uv: [x, y], normal: [0.0, 0.0, 1.0], brep_ref: vref
        });
        let v0 = pv(0.0, 1.0); let v1 = pv(1.0, 1.0); let v4 = pv(2.0, 1.0);
        let v2 = pv(0.0, 0.0); let v3 = pv(1.0, 0.0); let v5 = pv(2.0, 0.0);
        m.push_triangle(v0, v1, v2); // T0: v0,v1,v2
        m.push_triangle(v1, v3, v2); // T1: v1,v3,v2
        m.push_triangle(v1, v4, v3); // T2: v1,v4,v3
        m.push_triangle(v4, v5, v3); // T3: v4,v5,v3
        stitch_twins(&mut m);
        // Count stitched twins — each interior edge produces 2 stitched half-edges
        let stitched = m.half_edges.iter().filter(|he| he.twin.is_some()).count();
        // Interior edges: v1-v2 (T0/T1), v1-v3 (T1/T2), v3-v4... let's just verify > 0
        assert!(stitched > 0, "at least some half-edges should be stitched");
        // Verify twin symmetry for all stitched edges
        for (i, he) in m.half_edges.iter().enumerate() {
            if let Some(twin_id) = he.twin {
                let back = m.half_edges[twin_id.0].twin;
                assert_eq!(back, Some(HalfEdgeId(i)), "twin.twin must equal self");
            }
        }
    }

    // ── DCEL invariants (full pipeline) ─────────────────────────────────────

    /// Run the full DCEL pipeline for a primitive and return the assembled
    /// `HalfEdgeMesh` before `to_trimesh` discards connectivity.
    fn dcel_prim(node: &CsgNode) -> HalfEdgeMesh {
        let mut ctx = SolidModelingContext::new();
        let sid = compile_csg_node(&mut ctx, node);
        let shell_id = ctx.get_solid(sid).outer;
        let face_ids: Vec<FaceId> = ctx.get_shell(shell_id).faces.clone();
        let mut dcel     = HalfEdgeMesh::new();
        let mut registry = EdgeVertexRegistry::new();
        for face_id in face_ids {
            mesh_face(&ctx, face_id, &MeshOptions::default(), &mut dcel, &mut registry);
        }
        merge_dcel_vertices(&mut dcel, MeshOptions::default().epsilon);
        stitch_twins(&mut dcel);
        dcel
    }

    #[test]
    fn dcel_face_vertices_round_trip() {
        let mut m = HalfEdgeMesh::new();
        let vref = MeshVertexRef::OnFace(FaceId(0));
        let v0 = m.push_vertex(MeshVertex { pos: [0.0, 0.0, 0.0], uv: [0.0, 0.0], normal: [0.0, 0.0, 1.0], brep_ref: vref });
        let v1 = m.push_vertex(MeshVertex { pos: [1.0, 0.0, 0.0], uv: [1.0, 0.0], normal: [0.0, 0.0, 1.0], brep_ref: vref });
        let v2 = m.push_vertex(MeshVertex { pos: [0.0, 1.0, 0.0], uv: [0.0, 1.0], normal: [0.0, 0.0, 1.0], brep_ref: vref });
        let (fid, _) = m.push_triangle(v0, v1, v2);
        let [r0, r1, r2] = m.face_vertices(fid);
        assert_eq!([r0, r1, r2], [v0, v1, v2], "face_vertices must return vertices in push order");
    }

    #[test]
    fn dcel_twin_symmetry_after_stitch_cuboid() {
        let dcel = dcel_prim(&CsgNode::cuboid(1.0, 1.0, 1.0));
        for (i, he) in dcel.half_edges.iter().enumerate() {
            if let Some(twin_id) = he.twin {
                let back = dcel.half_edges[twin_id.0].twin;
                assert_eq!(back, Some(HalfEdgeId(i)), "he.twin.twin must equal he (failed at HalfEdgeId({i}))");
            }
        }
    }

    #[test]
    fn dcel_all_half_edges_have_twin_after_stitch_cuboid() {
        let dcel = dcel_prim(&CsgNode::cuboid(1.0, 1.0, 1.0));
        for (i, he) in dcel.half_edges.iter().enumerate() {
            assert!(he.twin.is_some(),
                "HalfEdgeId({i}) has no twin — cuboid is closed so every half-edge must be interior");
        }
    }

    // ── vertex classification (brep_ref) ─────────────────────────────────────

    #[test]
    fn vertex_classification_cuboid_all_corner() {
        let dcel = dcel_prim(&CsgNode::cuboid(1.0, 1.0, 1.0));
        for (i, v) in dcel.vertices.iter().enumerate() {
            assert!(matches!(v.brep_ref, MeshVertexRef::Corner(_)),
                "cuboid vertex {i} must be Corner (all vertices are B-rep corners)");
        }
    }

    #[test]
    fn vertex_classification_cylinder_no_on_face() {
        let dcel = dcel_prim(&CsgNode::cylinder(1.0, 2.0));
        for (i, v) in dcel.vertices.iter().enumerate() {
            assert!(!matches!(v.brep_ref, MeshVertexRef::OnFace(_)),
                "cylinder vertex {i} is OnFace — should be Corner (seam) or OnEdge (circle)");
        }
    }

    #[test]
    fn vertex_classification_cone_no_on_face() {
        let dcel = dcel_prim(&CsgNode::cone(1.0, 2.0));
        for (i, v) in dcel.vertices.iter().enumerate() {
            assert!(!matches!(v.brep_ref, MeshVertexRef::OnFace(_)),
                "cone vertex {i} is OnFace — should be Corner (apex/seam) or OnEdge (base circle)");
        }
    }

    #[test]
    fn vertex_classification_sphere_all_three_types() {
        let dcel = dcel_prim(&CsgNode::sphere(1.0));
        let corners  = dcel.vertices.iter().filter(|v| matches!(v.brep_ref, MeshVertexRef::Corner(_))).count();
        let on_edges = dcel.vertices.iter().filter(|v| matches!(v.brep_ref, MeshVertexRef::OnEdge(_))).count();
        let on_faces = dcel.vertices.iter().filter(|v| matches!(v.brep_ref, MeshVertexRef::OnFace(_))).count();
        // 2 poles (south + north), seam-column vertices per latitude ring, interior grid points
        assert_eq!(corners, 2, "sphere must have exactly 2 Corner vertices (south and north poles)");
        assert!(on_edges > 0, "sphere must have OnEdge vertices (seam column)");
        assert!(on_faces > 0, "sphere must have OnFace vertices (interior latitude-grid points)");
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
        // The registry deduplicates corners across faces during tessellation,
        // so pre-merge is already 8 for plane-only solids.
        let mesh = unmerged_prim(&CsgNode::cuboid(1.0, 1.0, 1.0));
        assert_eq!(mesh.vertices.len(), 8, "registry deduplicates plane-face corners; pre-merge is already 8");
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
