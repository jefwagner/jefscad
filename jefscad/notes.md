# Notes and ToDo for jefscad

jefscad should eventual be a constructive solid geometry language similar to openscad
that is exposed as a python package.

## Hello-world style first implementation

The first step is to scaffold the project and build/testing tools to create a rust
backed local python package that can be loaded into a python environment (or jupyter
notebook) in editable mode with pip install -e (or the UV equivalent).

[x] Document the environmental setup for making a rust backed python package
[x] Create simple HelloWorld rust struct with new that takes a string
[x] Create a method for the HelloWorld struct that prints a greeting with the string
[x] Use pyo3 macros to expose HelloWorld struct and method to python
[x] Scaffold the setup to create the rust backed python library
[x] Create a simple python unit test (using pytest package)
[x] Create a jupyter notebook the demos the use of the python package

See [DEVELOPMENT.md](../DEVELOPMENT.md) for environment setup, build commands,
running tests, and the Jupyter workflow.

## Construct Solid Geometry Layer

[x] Design/Architect the constructive solid geometry (csg) layer in rust

Notes for the above todo item:
  * A CsgNode is a struct with
    - geom_id: A hash built from canonicalized CsgBaseNode and flat_transform
    - prov_id: A unique node id that can be used to trace info flow through the system
    - base: A CsgBaseNode (the geometry of the node before applying transforms)
    - transforms: A set of affine transforms applied to the base node
    - flat_transform: the 4x4 matrix rep of combined set of transforms
    - meta: An optional set of metadata to carry with the node
  * A CsgNodeBase is an enum with:
    - Op(CsgOp) where CsgOp is an enum with:
      - Union
      - Difference
      - Intersection
      - Selection -> select a single solid from multiple disjoint solids using a ranking
        function/method.
    - Prim(CsgPrimative) where CsgPrimative is an enum with:
      - Cuboid
      - Cylinder
      - Sphere
      - Cone
      - Extrusion(ClosedPath2d, length) (do these later when we tackle 2D path creation)
      - SolidOfRotation(Path2d) (do these later with 2D path creation)
  * AffineTransform is an enum with:
    - Generic(matrix) a generic affine transformation given by a 4x4 matrix
    - Translation(delta) or Move(delta) -> A translation by the vector delta
    - RotationAA(axis, angle) -> A rotation around axis by angle
    - Scale(sx, sy, sz) axis-alligned non-uniform scaling
    - Shear(to-be-determinied) A shear transformation
  * CsgMetadata - additional data for a CsgNode, right now the only things that
    come to mind are a color for rendering, a material specifier for printing/rendering. 
    Maybe a label or texture info?

A CSG tree is traditionally made with the operators (union, difference, intersection) as
internal nodes, and leaf nodes being the solid primatives. But there are several
questions I have about this:

1. Can this system work without needing a context or global workspace? Ergonomically
I would prefer not needing to keep global variables in my head while designing, but
it somehow feels like this will be a necessary evil... we'll see. If so, I we night have
to add a CsgContext or CsgWorkspace struct to the system.

2. In rust this raises some questions about ownership of nodes - does a
internal node own it's children? If not, who does own the children, and if so, how would
we have a separate inner node that uses the same child node for a different operation
(say we want to subract a the same sphere from multiple different cubes)? Right now I'm
leaning towards No - an operator just gets an Arc ref to the child nodes.

3. Will the operators be strictly binary operators, or do we allow them to
have multiple inputs? I'm leaning towards allowing multiple inputs - at least for order
independent unions and intersections -  because it's logically equivalent, and it's nice
when the data structure reflects reality.

4. In a perfect world, a given node in a CSG tree would correspond to a
single connected solid, but in reality that is often not the case. I would like to be
able to have some method of choosing a single solid from set of disconnected solids
from a single node in a simple to understand and easy to use manor. How might one create
some selection operation like this?

After thoughts - I have the following plan:

## Rust-backed Python CSG Solid Modeling Language — Plan / TODO (next few weeks)

### High-level architecture (3 layers)
1. **CSG / Modeling AST**
   - User-facing authoring model (OpenSCAD-like)
   - Cheap structural sharing, transforms, metadata
   - No heavy geometry here

2. **B-rep Kernel**
   - Topology + geometry objects (NURBS + analytic surfaces)
   - Eventually supports booleans, queries (intersection tests), and (stretch) STEP export

3. **Meshing**
   - Triangulate B-rep faces (trimmed parametric surfaces)
   - Ensure watertightness + quality (eventually Chew-style refinement)
   - Export for rendering / 3D printing (STL/OBJ/etc.)

---

### Phase 0 — Repo structure and scaffolding
- [x] Create jefscad crate layout
  - [x] `csg_lang` (CSG AST + canonicalization)
  - [x] `geom` (vectors/matrices, NURBS eval, curve/surface types)
  - [x] `brep_kernel` (topology + kernel algorithms)
  - [x] `mesher` (tessellation + mesh quality)
  - [x] `py_bindings` (pyo3/maturin)
- [x] Choose math types for internal transforms
  * The transforms will stay as objects with parameters in the AST phase AND there will
    will be a single 4x4 matrix of f64 floats for the flattened transform. We will
    quantize the matrix to i64 for hashing to try and deal with the what should be
    equivalent transforms that were off because of floating point shenanagons.
- [x] Decide initial error handling approach (Rust errors -> Python exceptions)
  * The error strategy: Internally, use rust-error with explicit error types
    (InvalidInput, ConvergenceFailure, etc) with a translation layer to python 
    exceptions

---

### Phase 1 — CSG AST in Rust + Python authoring API (first milestone)
#### CSG core types
- [x] Define `CsgNode { geom_id, prov_id, base, transform, flat-transform, meta }`
  - transforms stored as transform-stack
  - meta is cheap to clone (e.g., `Option<Arc<CsgMetadata>>`)
- [x] Define `NodeRef = Arc<CsgNode>` and make nodes immutable
- [x] Define `CsgNodeBase`
  - [x] `Prim(CsgPrimitive)`
  - [x] `Op(CsgOp)` where ops include children explicitly:
    - [x] `Union { children: Vec<NodeRef> }` (n-ary)
    - [x] `Intersection { children: Vec<NodeRef> }` (n-ary)
    - [x] `Difference { base: NodeRef, subtract: Vec<NodeRef> }` (base minus rest)
    - [x] `Select { input: NodeRef, policy: SelectPolicy }` (component selection)
- [x] Define `CsgPrimitive`
  - [x] Cuboid, Cylinder, Sphere, Cone
  - [_] Extrusion(ClosedPath2d, length) (leave these till later when we want to tackle
        two dimensional path stuff)
  - [_] SolidOfRotation(Path2d) (leave these till later)
- [x] Define transforms:
  - [x] translation/rotation/scale/shear
  - [_] generic
  - At the AST level, leave the translations as user-friendly types
  - can be smushed to generic 4x4 matrix for affine transformations for homogenous vecs
    when transitioning to next layer
- [_] Define metadata:
  - color, material name/id, label, texture info
  - keep as optional to reduce overhead


Possible rust interface for authoring CSG solids
```rust
let ball = CsgNode::sphere(2.5).translate(v3![0, 0, 1.5));
let base = CsgNode::cuboid(2.0, 2.0, 0.5).translate(v3![-1.0, -1.0, 0])
let statue = CsgNode::union(ball, base);
```

#### AST ergonomics / canonicalization
- [x] Implement primative constructors (cuboid, cylinder, sphere, cone)
- [x] Implement transform methods (translate, rot_x, rot_y, rot_z, rot_aa, scale)
  - returns a new node (functional style, immutable)
- [x] Implement quantization for flat_matrix so similar matrices give same hash
- [x] Implement operator constructors (union, intersection, difference)
  - creation preserves authored structure (no autoflattening; that's for CanonicalCsgNodeView)
- [x] Implement a CanonicalCsgNodeView allow manipulation without mutating authored AST
  - [x] allow flattenting of ops
  - [x] allow sorting of children
- [x] Implement structural hashing to support evaluation caches

#### Python interface (authoring)
- [x] Expose Python `Node` class that holds `Arc<CsgNode>`
- [x] Provide Python constructors: `sphere()`, `cuboid()`, `cylinder()`, `cone()`
- [x] Provide chainable transforms: `node.translate(...)`, `rot_x/y/z(...)`, `rot_aa(...)`, `scale(...)`
  - return new nodes (functional style)
- [x] Provide ops: `union(a,b,...)`, `intersection(a,b,...)`, `difference(base, sub...)`, `select_largest`, `select_closest_to`, `select_contains`
- [x] Provide `__repr__` showing full AST detail (Rust Debug style)
- [x] Implement `__str__` (Python) and `Display` (Rust) with condensed output when
      transform stack is long (hide stack, just show count)

#### Rust and Python documentation
Documentation approach:
- Rust `///` doc-comments are the **single source of truth**. PyO3 maps them directly
  to Python `__doc__`, so there is no separate Python docstring layer.
- Format: plain prose for self-evident items; Google-style `Args:` blocks only where
  parameter semantics are non-obvious (e.g. `rot_aa` axis normalisation).
- Type information lives in `.pyi` stub files, not in docstrings.
- Stubs are generated with pyo3-stub-gen:
  `cargo +nightly run --bin stub_gen --features extension-module`
  → output: `python/jefscad/_jefscad/__init__.pyi`

- [x] Add Rust doc-comments to all public constructors and methods (csg_lang.rs, py_bindings.rs)
  - Primitive placement convention: sphere centered at origin; cuboid corner at origin;
    cylinder/cone base circle at z=0 centered at origin, height extends to z=h
- [x] Python docstrings come from Rust doc-comments (no separate Python layer needed)
- [x] Set up pyo3-stub-gen for .pyi type stub generation
- [x] Build and review `cargo +nightly doc` (rustdoc for the Rust API)
  - Rebuild: `cargo +nightly doc --no-deps --features extension-module`
  - csg_lang is `pub mod`; CsgNode, NodeRef, SelectPolicy re-exported at crate root
- [x] Python discoverability from Jupyter — no extra work needed:
  - `import jefscad; jefscad.<TAB>` — works via `__all__` in `__init__.py`
  - `node.<TAB>` on a Node instance — works via PyO3's automatic `__dir__`
  - `help(jefscad.sphere)`, `help(jefscad.Node)` — works via Rust `///` → `__doc__`
  - `jefscad.sphere?` Jupyter magic — works via `__doc__`
  - `.pyi` stub covers static type checking in VS Code / pyright / mypy (not runtime)
- [x] Plan and build sphinx/read-the-docs style Python docs
  - Stack: sphinx + furo theme + napoleon (Google-style Args:)
  - Source: docs/{index,getting_started,concepts,api}.rst
  - Build: sphinx-build -b html docs/ docs/_build/html/
  - concepts.rst intentionally stubbed — fill in as project matures

Deliverable for Phase 1:
- Python can build AST trees/graphs (with sharing) and introspect them. 

---

### Phase 2 — Geometry foundation (`geom` crate)
- [x] Verify that storage is appropriate to allow proper provenance of full rounded
  interval arithmetic/mathematics from creation through to predicate tests. (I'm not
  yet conviced that the storage type for the combined translation matrices in the CSG
  AST phase shouldn't be Flint<f64> types to make sure the 'is this point
  inside/outside/indeterminant this solid queries possible without making some
  assumption about tolerance.)

  **Decision (2026-04-03):** Replace `flat_transform: [f64; 16]` in `CsgNode` with
  `FlintArray<f64, 16>` (Option B). Rationale:
  - f64 accumulates ~5 ULPs of error per composition step. Converting to Flint at
    evaluation time only captures the last step — silently discarding accumulated error
    from prior compositions, making predicates look rigorous but not be so.
  - For typical models (short chains, unit scale) the error is ~1e-13 m — negligible
    against a 0.01 mm feature size by 8 orders of magnitude. But two cases matter:
      1. Coincident surfaces (e.g. cube face-on-face): boundary classification can flip.
      2. Near-singular transforms (tiny scales, nearly-coplanar rotations): error
         can reach thousands of ULPs.
  - FlintArray<f64,16> is the cleanest single source of truth: compose with Flint
    mat_mul, extract midpoint for quantization/hashing. Marginal cost since `flint`
    already exists.
  - **Deferral:** The refactor is self-contained (CsgNode struct + composition code
    only) and is not needed until Phase 5 boolean ops require inside/outside predicates.
    Do the refactor as the first step of Phase 5.
- [ ] Implement vector/matrix utilities + affine transform application
- [ ] Implement NURBS curve/surface data structures:
  - [ ] evaluation `(u,v)->R^3`
  - [ ] first derivatives (normals/tangents)
  - [ ] bounding boxes (coarse, then refine later)
- [ ] Integrate rounded-interval numeric type for:
  - robust predicates & bounding certification (not necessarily for all geometry storage)

Deliverable for Phase 2:
- Surfaces/curves can be evaluated + differentiated; basic bounding is available.

---

### Phase 3 — B-rep core types (no booleans yet)
Goal: “primitive -> B-rep -> mesh” pipeline working.

#### B-rep representation principles (STEP-friendly later)
- [x] Keep **topology** separate from **geometry**
  - [x] NodeBRep as collection of BRepSolid's
  - [x] BRepSolid (Solid) as outer shell + optional inner shells (voids)
  - [x] Face as surface + outer loop + optional inner loops (holes); yes, faces can have
        holes — inner loops wind opposite to the outer loop
- [x] Plan for faces as **trimmed parametric surfaces**
  - loops defined in surface UV domain via per-coedge pcurves (Curve2)
  - edges carry both a 3D curve (Curve3) and per-face 2D p-curves (Curve2)

#### Core B-rep types
- [x] Geometry — `Surface` trait + `SurfaceKind` enum (geom.rs):
  - [x] `Surface` trait — eval(u,v)->Point3, eval_du/dv->Point3, eval_n->Option<Point3>
  - [x] `Plane` — p0, u_dir, v_dir; all eval methods closed-form; eval_n always Some
  - [x] `SurfaceKind` enum — Plane, Cylinder, Cone, Sphere implemented; Nurbs stub remains
  - [x] `CylindricalSurface` — origin, axis, ref_dir, radius; outward normal = r̂(u)
  - [x] `ConicalSurface` — apex, axis, ref_dir, half_angle; eval_n=None at v=0 (apex)
  - [x] `SphericalSurface` — center, radius, ref_dir, axis; eval_n always Some, unit
  - [ ] `NurbsSurf` — freeform; for future use
- [x] Geometry — `Curve3` trait + `Curve3Kind` enum (geom.rs):
  - [x] `Point3` — x, y, z; Add/Sub/Mul<f64>, length/normalize/cross
  - [x] `Curve3` trait — eval(t)->Point3, eval_dt(t)->Point3, is_degenerate()->bool
  - [x] `Line3` — p0, p1, t_min, t_max; t=0 at p0, t=1 at p1; implements Curve3
  - [x] `Curve3Kind` enum — Line3, CircularArc3, Polyline3 implemented; Nurbs/Ssi stubs remain
  - [x] `CircularArc3` — center, normal, ref_dir, radius, t0, t1; eval via angle param;
        eval_dt is tangent scaled by radius; is_degenerate when radius==0
  - [x] `Polyline3` — Vec<Point3>; t∈[0, n_segments]; segment i covers t∈[i, i+1]
        **Knot convention:** `eval`/`eval_dt` use `floor(t)` clamped to [0, n-1] to select
        the segment. At an integer knot t=k, `eval_dt` returns the direction of segment k-1
        (the segment *ending* at the knot) because floor(k) == k is clamped into [0, n-1],
        i.e. for t=1.0 on a 3-point polyline, segment 0 (not segment 1) gives the tangent.
        This is intentional: callers requiring the outgoing tangent should query t=k+ε.
  - [ ] `NurbsCurve3` — freeform; for future use
  - [ ] `SsiCurve3` — Phase 5: general surface-surface intersection curve
- [x] Geometry — `Curve2` trait + `Curve2Kind` enum (geom.rs):
  - [x] `Point2` — u, v; Add/Sub/Mul<f64>
  - [x] `Curve2` trait — eval(t)->Point2, eval_dt(t)->Point2, is_degenerate()->bool
  - [x] `Line2` — p0, p1, t_min, t_max; implements Curve2
  - [x] `Curve2Kind` enum — Line2, CircularArc2, Polyline2 implemented; Nurbs stub remains
  - [x] `Polyline2` — Vec<Point2>; same knot convention as Polyline3 (floor(t) selects segment)
  - [ ] `NurbsCurve2` — for freeform surface pcurves; future use
- [x] Topology (brep_kernel.rs):
  - [x] Newtype IDs: VertexId, EdgeId, CoEdgeId, LoopId, FaceId, ShellId, SolidId,
        NodeBRepId, SurfaceId, Curve3Id, Curve2Id, AttrId — via `define_id!` macro
  - [x] `FaceSense { Aligned, AntiAligned }`, `Orientation { Forward, Reverse }`,
        `OpType { Union, Difference, Intersection }`
  - [x] `CsgSource { prov_id: u64, geom_id: u64 }`
  - [x] `ProvenanceData { sources: SmallVec<[CsgSource; 1]>, last_op: Option<OpType> }`
        with `primitive(prov_id, geom_id)` convenience constructor
  - [x] `KernelTolerance { pos_tol, ang_tol, param_tol }` with Default impl
  - [x] `Vertex { point: Point3, tol: f64 }`
  - [x] `Edge { curve3, v0, v1, t0, t1, coedges: SmallVec<[CoEdgeId; 2]> }`
        degenerate edges (v0==v1, constant 3D curve) handle sphere poles and cone apex
  - [x] `CoEdge { edge, orientation, face, pcurve: Curve2Id }`
  - [x] `Loop { coedges, face, is_outer }` — coedges starts empty
  - [x] `Face { shell, surface, outer, inners, sense, prov, attr: Option<AttrId> }`
        inners/attr start empty/None; attr set after creation
  - [x] `Shell { solid, faces, is_outer }` — faces starts empty
  - [x] `Solid { outer: ShellId, inners: Vec<ShellId> }` — inners starts empty
  - [x] `NodeBRep { solids: Vec<SolidId>, source_csg_id }` — solids starts empty
- [x] `SolidModelingContext` arena — typed Vecs, push/get/get_mut via `impl_push_get!`
      macro; `new()` creates empty context with `KernelTolerance::default()`

#### Primitive -> B-rep construction
- [x] Build B-rep for each primitive (initially without CSG booleans)
  - [x] `build_cuboid` — 8V, 12E, 24CE, 6 faces; all Line3 edges, all FaceSense::Aligned
  - [x] `build_cylinder` — 2V, 3E (2 closed CircularArc3 + 1 Line3 seam), 6CE, 3 faces
  - [x] `build_cone`  — 2V, 3E (degenerate apex edge), 5CE, 2 faces
  - [x] `build_sphere` — 2V, 3E (2 degenerate pole edges), 4CE, 1 face; UV Mercator diagram in source
  - [x] `CircularArc2` added to `Curve2Kind` (needed for cap pcurves)
  - [x] `Point3::dot` added to `geom.rs`
  - [x] `compile_primitive` — dispatcher + transform absorption; 275 tests
        - build-then-transform strategy: snapshot arena index ranges, walk freshly-added slices
        - Vertices: apply full 4×4 as points
        - Line3: transform p0/p1 as points; t_min/t_max unchanged
        - CircularArc3: isotropic check; transform center, rotate ref_dir/normal, radius *= s
        - Plane: transform p0/u_dir/v_dir; pcurves unchanged (scaling absorbed into direction vecs)
        - Cylindrical/Conical: isotropic check; transform origin/apex/axis/ref_dir; lateral pcurve v-coords *= s via topology traversal; half_angle scale-invariant for cone
        - SphericalSurface: isotropic check; transform center/ref_dir/axis; radius *= s; pcurves unchanged (u,v are angles)
        - Non-isotropic on curved primitives: todo!() until NURBS fallback implemented
- [x] `compile_primitive` absorbs `flat_transform` (isotropic check; todo!() for NURBS fallback)
- [x] `compile_csg_node(ctx, node) -> SolidId`
  - Match on `CsgBaseNode::Prim` → call `compile_primitive` with node's `flat_transform`,
    `prov_id`, `geom_id`
  - Boolean ops (`Op`) → `todo!()` for now; 282 tests

Deliverable for Phase 3:
- `compile_csg_node` works for all four primitives with arbitrary isotropic transforms.

---

### Phase 4 — Meshing from B-rep (first real output)

#### Step 1 — TriMesh type + meshing scaffold

**TriMesh representation decisions (2026-04-11):**
- Shared vertex positions (`vertices[NV]`) + index triangles (`triangles[NT]`) — vertex index
  equality means shared position, enabling future connectivity/watertightness queries
- Per-triangle-vertex normals (`tri_normals[NT×3]`): sharp edges get different normals at
  the same vertex without duplicating positions; smooth surfaces interpolate naturally
- Per-triangle-vertex UV coords (`tri_uvs[NT×3]`): always populated (values are free during
  tessellation since we sample from UV grid points anyway); enables future texture mapping
  - Known limitation: seam vertices (u=0/2π on cylinder/sphere/cone) carry a single UV value;
    proper texture mapping at seams requires seam vertex duplication — deferred until needed
- Invariants: `tri_normals.len() == triangles.len() * 3`; same for `tri_uvs`

```rust
pub struct TriMesh {
    pub vertices:    Vec<[f32; 3]>,   // NV positions; shared across triangles
    pub triangles:   Vec<[u32; 3]>,   // NT index triples into vertices
    pub tri_normals: Vec<[f32; 3]>,   // NT×3; tri t corner k → tri_normals[t*3+k]
    pub tri_uvs:     Vec<[f32; 2]>,   // NT×3; surface UV params at each triangle vertex
}
pub struct MeshOptions { pub resolution: u32 }   // segments per full circle; default 32
```

- [x] Implement `TriMesh`, `MeshOptions` (with `Default`), `mesh_solid(ctx, sid, opts) -> TriMesh`
      — `mesh_solid` walks shell faces, calls internal `mesh_face` per face, concatenates
      — `mesh_face` stub returns empty `TriMesh`; 287 tests

#### Step 2 — Per-surface tessellation (one surface type at a time)
UV domains for our four surface types — all simple, no general polygon trimming needed yet:
- [x] `Plane` (cuboid faces + caps): fan triangulation from boundary[0]; analytic normal
      adjusted for FaceSense; sample_loop_uvs handles Line2 (start pt) + CircularArc2
      (resolution samples); 293 tests
- [x] `CylindricalSurface`: (resolution+1)×2 UV grid; v range from loop boundary samples;
      radial analytic normals; seam at u=0/2π has duplicate positions, separate UVs; 296 tests
- [x] `ConicalSurface`: apex-fan: 1 apex + resolution base vertices; cross-product flat normals
      (avoids apex singularity); winding (apex, base_next, base_curr) for outward normal; 300 tests
  - [DONE] Hybrid normals implemented: base-circle corners use analytic eval_n
    (smooth shading around circumference); apex corner uses flat cross-product
    normal per triangle (singularity — no single outward normal definable).
    Verified by mesh_solid_cone_lateral_normals_hybrid test. 317 Rust tests.
  - [REVISIT — lower priority] Further improvement possible if needed:  The lateral surface is smooth
    everywhere except the apex (eval_n returns None at v=0).  Base-circle vertices can
    use analytic normals from eval_n(u, v_max).  The apex is the hard case: no single
    well-defined normal exists there — one option is to omit the apex vertex from the
    normal buffer (use the triangle face normal for apex corners only) while using
    smooth analytic normals at base corners; another is to duplicate the apex vertex
    once per triangle so each copy can carry the face normal for its triangle.
    Worth revisiting before export/rendering work begins.
- [x] `SphericalSurface`: (n_lon+1)×(n_lat-1) grid + 2 pole vertices; n_lon=resolution,
      n_lat=max(2, resolution/2); south/north fans + middle strips; analytic eval_n smooth
      everywhere including poles (no special-casing); 304 tests
- [x] Watertightness: `merge_vertices(mesh, epsilon)` — quantised hash map (i64 key),
      epsilon=1e-8 default in MeshOptions; called automatically by `mesh_solid`;
      cuboid 24→8, cylinder 130→64, cone 65→33, sphere 497→482 vertices; 316 Rust tests

#### Step 3 — File export
- [x] Binary STL: `write_stl<W: Write>(mesh, writer)` + `write_stl_file(mesh, path)`;
      per-triangle normal = averaged+renormalised corner normals; 308 tests
- [x] OBJ: `write_obj<W: Write>(mesh, writer)` + `write_obj_file(mesh, path)`;
      shared vertex positions, NT×3 per-corner vn/vt entries, f v/vt/vn syntax; 312 tests
- [_] glTF — deferred until STL/OBJ are working

#### Step 4 — Python binding

**Design decision: hide `SolidModelingContext` from Python users.**

`SolidModelingContext` is not exposed as a Python type.  `PyNode.mesh()` creates
a fresh context internally on every call, compiles the CSG tree, tessellates, and
returns a `PyMesh`.  Rationale:

- *Jupyter ergonomics*: the full round-trip is a single expression —
  `sphere(r=1.5).mesh().save_stl("out.stl")` — with no setup object to manage.
- *Clean-slate semantics*: re-running a notebook cell always starts fresh; no
  stale B-rep data accumulates in a long-lived context.
- *Future-proof*: sub-tree caching (keyed on `geom_id`) can be added inside
  `.mesh()` transparently later without changing the Python API.
- *Current cost is zero*: `compile_csg_node` rebuilds from scratch anyway, so a
  persistent context provides no speedup today.

- [x] `PyMesh` class wrapping `TriMesh`: `triangle_count`, `vertex_count` properties,
      `save_stl(path)`, `save_obj(path)` methods, `__repr__`
- [x] `PyNode::mesh(resolution=32) -> PyMesh`: creates fresh `SolidModelingContext`
      internally, calls `compile_csg_node` + `mesh_solid`, drops context on return
- [x] `Mesh` exported from `jefscad.__init__`; 49 pytest + 312 Rust tests passing

Deliverable for Phase 4: ✓
- `sphere(2.5).translate(0,0,1).mesh().save_stl("out.stl")` works end-to-end.

---

### Phase 4.5 — Extrusion and Revolution primitives

#### Design decisions (2026-04-12)

**New surface types** — two new `SurfaceKind` variants:

**`LinearExtrusionSurface`**
- `profile: Curve3Kind` — generatrix curve in the base plane
- `direction: Point3` — unit extrusion vector
- `S(u, v) = profile.eval(u) + direction * v`
- u = profile parameter, v = world-space extrusion distance
- Normal = `cross(profile.eval_dt(u), direction).normalize()`
- *Consistency check*: a cylinder is this type with a `CircularArc3` profile and
  `direction = +Z`. On `CylindricalSurface`, `u = angle` and `v = height`, which
  matches u=profile-param and v=extrusion-distance. ✓

**`RevolutionSurface`**
- `profile: Curve3Kind` — generatrix curve in the meridional half-plane (X ≥ 0 when axis = Z)
- `axis_origin: Point3`, `axis_dir: Point3` (unit)
- `S(u, v)` = rotate `profile.eval(v)` around axis by angle `u`
- u = angle ∈ [0, 2π], v = profile parameter
- *Consistency check*: cone is this type with a `Line3` profile; `ConicalSurface` has
  `u = angle`, `v = slant distance`. Sphere is this type with a semicircular
  `CircularArc3`; `SphericalSurface` has `u = longitude`, `v = latitude`. ✓

**Coordinate conventions**
- Extrusion: `Path2D` is in the X-Y plane; extrude along +Z. Consistent with
  `build_cylinder`/`build_cone` placing base at z=0.
- Revolution: `Path2D` profile in the X-Z half-plane (x ≥ 0); rotate around Z-axis.
  Consistent with `build_cylinder`/`build_cone` having axis = +Z.

**`Path2D`** — a Rust struct (in `geom.rs`), exposed to Python via a wrapper.
Stores a `Vec<Curve2Kind>` of segments plus `start: Point2`, `current_pos: Point2`
(private), and `closed: bool`. Canvas-style builder API: `line_to`, `arc_to`,
`close` (topological only, no segment), `line_to_close` (adds closing segment + sets
closed). Geometric closure validated by the B-rep compiler using `KernelTolerance`.
Each segment becomes its own face in the B-rep (one surface per segment).

**Solid validity rules**

*Extrusion*: path must be closed; raise `ValueError` (Python) / `Err` (Rust) otherwise.

*Revolution*: only three cases allowed (this is a solid modeler — no infinitely thin shells):
- Both profile endpoints on the Z-axis → full solid; cap faces degenerate to points
  (like sphere poles)
- Exactly one endpoint on the Z-axis → full solid; one cap is a point, other is a disk
- Closed path with all x-coordinates ≥ 0 (and path does not touch x=0) → solid of
  revolution with a hole (torus-like); no cap faces needed since the swept path is closed
  (e.g. a circle of radius 0.5 centred at x=0.6, z=0 → solid donut)
- All other cases (open path with neither endpoint on axis; path crosses axis; closed
  path that touches or crosses x=0) → raise

*Partial revolution*: deferred. Full 360° only for now.

#### Implementation status
- [x] Add `LinearExtrusionSurface` and `RevolutionSurface` to `SurfaceKind` in `geom.rs`
  - `todo!()` stubs in brep_compiler transform match arm
  - `#[derive(Debug, Clone)]` added to `Curve3Kind`, `Curve2Kind`, and stub types
  - 27 tests (13 LES + 14 RS), all passing
- [x] Add `Path2D` struct to `geom.rs`
  - `new`, `line_to`, `arc_to`, `close`, `line_to_close`, `current_pos` methods
  - 13 tests, all passing
- [x] `build_extrusion(ctx, path, height, prov_id, geom_id) -> SolidId`
  - Validates closed path (via KernelTolerance)
  - One lateral face per segment (LinearExtrusionSurface)
  - Bottom and top cap faces (Plane)
  - 27 tests, all passing
- [x] `build_revolution(ctx, path, prov_id, geom_id) -> SolidId`
  - Validates endpoint-on-axis rule and case classification
  - One lateral face per segment (RevolutionSurface)
  - Disk cap face(s) where endpoint is off-axis; degenerate (point) caps where on-axis
  - 19 tests, all passing
  - All lateral faces: FaceSense::Aligned (RevolutionSurface natural normal is outward for
    standard profiles). End cap: Fwd circle + Aligned (outward +Z). Start cap: Rev + AntiAligned.
- [x] Add `CsgPrim::Extrude { path, height }` and `CsgPrim::Revolve { path }` variants
  - `hash_path2d` helper for bit-pattern hashing of Path2D in `hash_primitive`
  - `compile_primitive` dispatch for both via `build_extrusion`/`build_revolution`
- [x] Python `Path2D` wrapper with `extrude(height) -> Node` and `revolve() -> Node`
  - `PyPath2D` class with `line_to`, `arc_to`, `close`, `line_to_close` builder methods
  - `extrude(height)` and `revolve()` do eager validation via temp context
  - `__str__` (tree-style Display) and `__repr__` (Rust Debug)
  - Module-level `path2d(u, v)` constructor function
- [x] `fmt::Display` for `Path2D` — tree-style output matching `CsgNode` style

---

### Phase 4.5 → Phase 5 prerequisite — flat_transform refactor (COMPLETE 2026-04-12)

**Decision**: refactored `flat_transform` from `[f64; 16]` to `FlintArray<f64, 16>` before
Phase 5 work begins (as noted in Phase 2 decision record).

**Changes to `flint` crate** (commits dfc6b25, 4f57ea1):
- `FlintArray.lb` / `.ub` made `pub` (enables const struct-literal construction)
- `pub const IDENTITY_4X4: FlintArray<f64, 16>` added to `linalg.rs`, re-exported from crate root
- `FlintArray<T,16>::det2()` — upper-left 2×2 determinant (for x-y isotropy checks in revolution)
- `FlintArray<T,N>::midpoint() -> [T; N]` — elementwise (lb+ub)/2 for extracting best-estimate f64
- `FlintArray::<f64,N>::from_f64(arr)` — 1-ULP-widening constructor from plain f64 array;
  correct for non-representable values like trig outputs (vs. zero-width `lb==ub` struct literal)

**Changes to `jefscad` crate** (commit 4f57ea1):
- `flint` added as a Cargo dependency
- `CsgNode::flat_transform`: `[f64;16]` → `FlintArray<f64,16>`
- Local `IDENTITY_4X4` and `mat_mul` removed; use `flint::IDENTITY_4X4` and `FlintArray::mat_mul`
- `mat_translation`/`mat_scale`/`mat_rot_aa` return `FlintArray` via `from_f64` (correct ULP
  widening for all inputs, including trig values in `mat_rot_aa`)
- `quantize_matrix` and `is_identity_transform` operate on `FlintArray.lb`
- `compile_primitive` takes `&FlintArray<f64,16>`; extracts `.midpoint()` once for all f64
  geometry operations; `is_identity` uses midpoint comparison

---

### Phase 4.6 — DCEL / Half-Edge mesh refactor

**Motivation:** The current `TriMesh` is an unstructured triangle soup — adequate for STL/OBJ
output but insufficient for the meshing pipeline that Phase 5 boolean ops will require.
Delaunay refinement, edge-flipping, point insertion, and constraint-edge preservation all need
local connectivity. The wiki (`kb/wiki/Mesh-Representations.md`) specifies the target structure.

**Architecture:** Two-layer design:
- **`HalfEdgeMesh`** — internal DCEL, used during construction and refinement. All coordinates
  are `f64`. Each vertex carries a back-reference to the B-rep entity it was projected from.
- **`TriMesh`** — output/export format; unchanged in shape, but `f64 → f32` narrowing now
  happens only here. Generated from `HalfEdgeMesh` at the end of the pipeline.

**Precision rule:** All `MeshVertex` fields (`pos`, `uv`, `normal`) are `f64` internally.
Conversion to `f32` happens only inside `HalfEdgeMesh::to_trimesh()` (and the export writers).
The existing `merge_vertices` epsilon logic moves into the DCEL assembly step.

**B-rep back-reference (full three-way from the start):**
`SphericalSurface` meshing generates interior latitude/longitude grid vertices that have no
corresponding B-rep edge or corner — `OnFace` is required immediately.
```rust
pub enum MeshVertexRef {
    Corner(VertexId),   // projected from a B-rep topological vertex; position is fixed
    OnEdge(EdgeId),     // lies on a coedge; this half-edge pair is a constraint — never flip/cut
    OnFace(FaceId),     // interior point sampled on a face (e.g. sphere grid, refinement insert)
}
```

**Edge vertex registry — concept (needs design before implementation):**
Each face is meshed independently in its own parametric UV domain, then assembled into a
single solid `HalfEdgeMesh`. The problem is that two faces sharing a B-rep edge must
reference the *same* `MeshVertexId`s along that boundary so that twin half-edges can be
stitched. The proposed mechanism is a per-solid registry:

```
EdgeVertexRegistry: one entry per B-rep EdgeId
    BTreeMap<t_val, MeshVertexId>   ← ordered by edge parameter
```

As each face is meshed, it looks up or inserts into the registry for each coedge it borders.
Adjacent faces sharing an edge retrieve the same `MeshVertexId`s, enabling twin stitching
after all faces are assembled. Open design questions to resolve before coding:

- **Key type for t_val:** `OrderedFloat<f64>` (requires `ordered-float` dep) vs. quantized
  `i64` (avoids dep but needs a stable epsilon strategy)?
- **Vertex pool scope:** Global-to-solid during construction (IDs directly comparable across
  faces) vs. per-face with a remapping pass on assembly? Global pool is simpler for stitching.
- **Refinement interaction:** Refinement inserts new edge vertices after initial triangulation.
  Those new vertices must be added back into the registry. When is the registry "finalized"?
- **Coedge orientation:** The registry is keyed on the B-rep edge's own t-parameter (not the
  coedge's), so Forward and Reverse coedges on the same edge look up the same entries correctly.

#### Tasks

- [x] **Design step**: resolved four open edge-registry questions:
      - Key type: quantized `i64` (quant=1e12, bin width=1e-12); no external dep
      - Vertex pool: global-to-solid; IDs directly comparable across faces
      - Corner vertices: separate `corners: HashMap<VertexId, MeshVertexId>` — same B-rep corner
        appears under different (EdgeId, t) pairs from different faces, so must key by VertexId
      - Phase 4.6 tessellators push directly to global DCEL; registry reserved for Delaunay refinement
      - Assembly: `merge_dcel_vertices(epsilon)` deduplicates coincident boundary vertices BEFORE
        `stitch_twins`; two-pass twin linking via `HashMap<(start,end), HalfEdgeId>`

##### Core types
- [x] Define `MeshVertexRef { Corner(VertexId), OnEdge(EdgeId), OnFace(FaceId) }` in `mesher.rs`
- [x] Define `MeshVertex { pos: [f64;3], uv: [f64;2], normal: [f64;3], brep_ref: MeshVertexRef }`
- [x] Define half-edge newtypes: `HalfEdgeId`, `DcelFaceId`, `MeshVertexId`
- [x] Define `HalfEdge { twin: Option<HalfEdgeId>, next: HalfEdgeId, vertex: MeshVertexId,
      face: DcelFaceId, is_constraint: bool }`
      (`twin` is `Option` during construction; all twins filled before `to_trimesh` is called)
- [x] Define `DcelFace { half_edge: HalfEdgeId }` (one representative half-edge per triangle)
- [x] Define `HalfEdgeMesh { vertices: Vec<MeshVertex>, half_edges: Vec<HalfEdge>,
      faces: Vec<DcelFace> }` with arena-style push helpers

##### Conversion and navigation
- [x] Implement `HalfEdgeMesh::to_trimesh(&self) -> TriMesh`
      — narrows `f64 → f32` for positions/normals/UVs
- [x] Implement `HalfEdgeMesh` navigation helpers:
  - `face_vertices(id) -> [MeshVertexId; 3]` — via next-chain
  - `face_half_edges(id) -> [HalfEdgeId; 3]`
  - `vertex_one_ring(id) -> impl Iterator<HalfEdgeId>` — orbit via twin+next

##### Edge vertex registry
- [x] Define `EdgeVertexRegistry` — split-key: `corners: HashMap<VertexId, MeshVertexId>` +
      `entries: HashMap<EdgeId, BTreeMap<i64, MeshVertexId>>`; quant=1e12
- [x] Implement `get_or_insert_corner` and `get_or_insert_edge` registry methods with tests

##### Twin stitching and vertex merging
- [x] Implement `stitch_twins(mesh)` — two-pass: build `HashMap<(start,end), HalfEdgeId>`,
      then link twins; idempotent; boundary edges stay `twin = None`
- [x] Implement `merge_dcel_vertices(dcel, epsilon)` — quantized position hash deduplication;
      must run BEFORE `stitch_twins` so coincident boundary vertices get same `MeshVertexId`

##### Pipeline refactor
- [x] Refactor all four tessellators (`Plane`, `Cylindrical`, `Conical`, `Spherical`) to push
      vertices directly into global DCEL via `dcel.push_vertex` / `dcel.push_triangle`;
      all vertices currently classified as `OnFace` (full classification deferred to Delaunay phase)
- [x] Update `mesh_solid` to: build global DCEL per face → `merge_dcel_vertices` → `stitch_twins`
      → `to_trimesh()`

##### Validation
- [x] All 467 existing meshing tests pass after pipeline swap (commit 4eadd17)
- [x] Targeted DCEL invariant tests added: `face_vertices` round-trip, twin symmetry
      (`he.twin.twin == he`), every half-edge has a twin after stitching on a cuboid.
      Constraint-edge flag test deferred (flag is always false until vertex classification lands).
- [x] Full vertex classification (`Corner`/`OnEdge`/`OnFace`) — all four tessellators updated:
      - `mesh_plane_face`: uses `sample_loop_into_dcel` + `EdgeVertexRegistry`; all vertices
        are `Corner` or `OnEdge`; cuboid pre-merge count drops 24→8 (registry deduplicates corners)
      - `mesh_conical_face`: loop scan finds apex (degenerate edge) and base circle edge;
        apex → `Corner`, base seam j=0 → `Corner`, base ring j>0 → `OnEdge`
      - `mesh_cylindrical_face`: loop scan finds seam (v0≠v1) and two circle edges;
        seam corners → `Corner`, circle interior → `OnEdge`
      - `mesh_spherical_face`: loop scan finds south/north pole edges (by pcurve v-sign) and
        seam edge; poles → `Corner`, seam columns → `OnEdge`, interior grid → `OnFace`
      - 4 classification tests added (cuboid all-Corner, cylinder/cone no-OnFace,
        sphere all-three-types); 474 tests total

**Deliverable:** `mesh_solid` pipeline passes through DCEL internally; STL/OBJ output is
geometrically equivalent to current output; B-rep back-references fully classified
(`Corner`/`OnEdge`/`OnFace`) for all four primitive tessellators.

---

### Phase 5 — Add boolean ops gradually

#### Predicate infrastructure (COMPLETE 2026-04-18)
- [x] Implement point-in-primitive predicates using Flint transforms (`jefscad/src/predicates.rs`):
  - `Classification { Inside, Outside, Indeterminate }` enum
  - `classify_sphere_local`, `classify_cuboid_local`, `classify_cylinder_local`, `classify_cone_local`
    — each evaluates the primitive's implicit function with Flint outward rounding
  - `classify_primitive_local` — dispatcher over `CsgPrimitive` variants
  - `mat4_inv_f64` — affine 4×4 inverse via 3×3 cofactor (valid for bottom row `[0,0,0,1]`)
  - `classify_node(p_world: [f64;3], node: &CsgNode) -> Classification` — top-level entry point:
    inverts `flat_transform`, transforms query point to local frame (midpoint extraction), dispatches
    to primitive classifier; 55 tests passing
  - Known approximation: midpoint of Flint-transformed local point is passed to primitive classifiers
    (interval width from transform not propagated through implicit); acceptable for Phase 5 start
- [x] Tolerance/indeterminate-zone policy:
  - Flint `PartialOrd`: `Some(Less)` = Inside, `Some(Greater)` = Outside, `None`/`Some(Equal)` = Indeterminate
  - `Outside` dominates `Indeterminate` when combining sub-tests (lateral × axial for cylinder/cone)

#### Boolean ops — planar-polyhedra subset (`jefscad/src/bool_ops.rs`)

Strategy doc: `kb/Boolean-Op-SSI-Strategy.md`

##### Data structures *(define the API contract first)*
- [x] Define `FaceFaceIntersection { v_start: VertexId, v_end: VertexId, curve3: Curve3Id,
      pcurve_a: Curve2Id, pcurve_b: Curve2Id }` — all arena IDs; strategy-doc canonical form
- [x] Define `SsiTable` newtype wrapping `HashMap<(FaceId, FaceId), Option<FaceFaceIntersection>>`
      with `insert`/`get` that canonicalize key as `(min_id, max_id)`; 2 tests

##### SSI dispatcher *(structural anchor — surface-type routing)*
- [x] `intersect_faces(ctx, face_a, face_b) -> Option<FaceFaceIntersection>` — dispatches on
      `(tag_a, tag_b)` via `SurfTag` enum (Copy discriminant mirror of `SurfaceKind`); non-planar
      arms return `None`; `intersect_plane_plane` stub returns `None`; 2 tests

##### AABB fast-reject *(prerequisite for SSI table enumeration)*
- [x] `Aabb { min, max }` with `new`, `unbounded` (±∞ conservative fallback), `empty`, `expand`
- [x] `face_aabb(ctx, face_id) -> Aabb` — SurfTag dispatch; Plane: walk outer-loop edges
      by curve type: `Line3` uses endpoints; `CircularArc3` uses endpoints + per-axis extrema
      (d/dt = 0 at atan2(v_i, u_i) ± π, checked against arc span via rem_euclid); other
      curves → unbounded; non-Plane surfaces → unbounded; 2 tests
- [x] `aabb_overlap(a, b) -> bool` — overlap on all three axes; `<=` so touching counts; 3 tests

##### Plane/plane SSI *(first concrete surface-pair implementation)*
- [ ] `intersect_plane_plane(ctx, face_a, face_b) -> Option<FaceFaceIntersection>` — computes
      intersection line, clips independently to each face, pushes `Vertex`/`Curve3`/`Curve2` into
      context; called from `(Plane, Plane)` arm of `intersect_faces`

##### SSI table
- [ ] `compute_ssi_table(ctx, solid_a, solid_b) -> SsiTable` — enumerate all face pairs, AABB
      reject first, then call `intersect_faces`; canonical key `(min_id, max_id)`

##### Phase 2: Face subdivision
- [ ] `EdgeChain` type — ordered list of `FaceFaceIntersection` refs, `v_start`/`v_end: VertexId`
- [ ] `stitch_edge_chains(ssi_table, solid) -> HashMap<FaceId, SmallVec<EdgeChain>>` — adjacency
      map per face, trace boundary-node → boundary-node paths
- [ ] `split_edge(ctx, edge_id, t_split) -> (VertexId, EdgeId, EdgeId)` — insert vertex at t,
      update every loop referencing the edge
- [ ] `split_face(ctx, face_id, entry_edge, t_entry, exit_edge, t_exit) -> (FaceId, FaceId)` —
      calls `split_edge` twice, creates cut coedge pair, rebinds loop coedges and shell
- [ ] `apply_edge_chains_to_face(ctx, face_id, chains) -> SmallVec<FaceId>` — 0 chains: `[face_id]`;
      1 chain: one `split_face`; multiple: sequential splits with fragment routing
- [ ] `subdivide_faces(ctx, solid, edge_chains) -> FaceFragments`
      — `HashMap<FaceId, SmallVec<FaceId>>`; unmodified face maps to `[face_id]`

##### Phase 3: Fragment classification
- [ ] `uv_centroid_of_face(ctx, face_id) -> (f64, f64)` — average UV coords of outer-loop vertices
- [ ] `sample_point_on_face(ctx, face_id) -> Point3` — `surface.eval` at UV centroid
- [ ] `classify_all_fragments(ctx, face_fragments, opposing_node) -> HashMap<FaceId, Classification>`

##### Phase 4: Assembly
- [ ] `select_kept_fragments(classified_a, classified_b, op) -> Vec<FaceId>` — apply
      Union/Difference/Intersection keep rules (Outside-B, Outside-A, Inside-B, Inside-A)
- [ ] `sew_cut_edges(ctx, kept_faces)` — link cut coedge pairs sharing a `Curve3Id` as twins
- [ ] `find_result_shells(ctx, kept_faces) -> SmallVec<SolidId>` — BFS over twin pointers;
      one `Shell`/`Solid` per connected component
- [ ] `boolean_op(ctx, op, solid_a, solid_b) -> SmallVec<SolidId>` — top-level entry point
      wiring all 4 phases

##### Integration
- [ ] Wire `compile_csg_node` Op branch to call `boolean_op` (replace `todo!()`)

##### End-to-end tests *(one at a time)*
- [ ] `union` of two partially-overlapping cuboids
- [ ] `difference` of two partially-overlapping cuboids
- [ ] `intersection` of two partially-overlapping cuboids

##### Deferred cases (from `kb/Boolean-Op-SSI-Strategy.md`)
- Holes: closed SSI loop entirely interior to a face (requires inner-loop creation)
- Coincident/coplanar faces
- Degree-3 interior node (vertex of one solid projects inside a face of the other)
- Curved surface pairs (cylinder, sphere, NURBS)
- Indeterminate fragment classification (Flint interval refinement)

Deliverable for Phase 5:
- `Union/Difference/Intersection` works for two overlapping planar polyhedra with stable output mesh.

---

### Phase 6 — “Selection” op for disconnected results
- [ ] Define `SelectPolicy` (deterministic, simple):
  - `LargestByVolume` (mesh- or brep-derived)
  - `ClosestToPoint(p)`
  - `ContainsPoint(p)` (very intuitive)
  - axis-based ranking
- [ ] Implement selection as a post-boolean step:
  - mesh connected components (initially easiest)
  - later: B-rep shell selection

Deliverable for Phase 6:
- Users can reliably pick one connected component from disconnected boolean results.

---

### Phase 7 — Stretch: STEP export
- [ ] Confirm B-rep has enough info (surfaces + trimming loops + p-curves)
- [ ] Map B-rep entities to STEP entities
- [ ] Start with limited geometry subset; expand as kernel matures

---

## Notes / guiding principles
- Authoring should be context-free: build CSG graphs with `Arc` sharing; caches live in an `EvalContext`.
- Keep nodes immutable; store evaluation caches outside the nodes.
- Use interval arithmetic for robust predicate certification + refinement triggers; keep core geometry storage in `f64` unless proven necessary.
- Keep provenance (`NodeId`) flowing into B-rep faces/edges to support debugging, selection, and metadata assignment.


