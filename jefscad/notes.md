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
  - [x] `Curve3Kind` enum — Line3 and CircularArc3 implemented; Nurbs/Ssi stubs remain
  - [x] `CircularArc3` — center, normal, ref_dir, radius, t0, t1; eval via angle param;
        eval_dt is tangent scaled by radius; is_degenerate when radius==0
  - [ ] `NurbsCurve3` — freeform; for future use
  - [ ] `SsiCurve3` — Phase 5: general surface-surface intersection curve
- [x] Geometry — `Curve2` trait + `Curve2Kind` enum (geom.rs):
  - [x] `Point2` — u, v; Add/Sub/Mul<f64>
  - [x] `Curve2` trait — eval(t)->Point2, eval_dt(t)->Point2, is_degenerate()->bool
  - [x] `Line2` — p0, p1, t_min, t_max; implements Curve2
  - [x] `Curve2Kind` enum skeleton — Line2 (implemented), Nurbs (stub)
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
- [_] `ConicalSurface`: uniform UV grid `[0,2π] × [0,v_max]`; apex at v=0 collapses to point
- [_] `SphericalSurface`: uniform UV grid `[0,2π] × [−π/2,π/2]`; poles collapse to points
- [_] Watertightness: post-merge coincident vertices by position after per-face meshing

#### Step 3 — File export
- [_] Binary STL (`write_stl(mesh, path)`) — flat triangle list, face normals; no vertex sharing needed
- [_] OBJ (`write_obj(mesh, path)`) — shared vertices + normals; Blender-friendly
- [_] glTF — deferred until STL/OBJ are working

#### Step 4 — Python binding
- [_] `PyMesh` class wrapping `TriMesh`
- [_] `PyNode::mesh(resolution=32) -> PyMesh`
  - creates `SolidModelingContext`, calls `compile_csg_node`, then `mesh_solid`
- [_] `PyMesh::save_stl(path)`, `PyMesh::save_obj(path)`

Deliverable for Phase 4:
- `sphere(2.5).translate(0,0,1).mesh().save_stl("out.stl")` works end-to-end.

---

### Phase 5 — Add boolean ops gradually
#### Prerequisite — flat_transform refactor
- [ ] Refactor `CsgNode` to use `FlintArray<f64, 16>` for `flat_transform`
  - Replace `[f64; 16]` with `FlintArray<f64, 16>`
  - Compose transforms via Flint mat_mul so accumulated rounding is tracked outward
  - Extract midpoint of each entry for quantization/hashing (geom_id logic unchanged)
  - This is the prerequisite for correct inside/outside classification at coincident
    surfaces and under near-singular transform chains (see Phase 2 decision record)

#### Predicate infrastructure
- [ ] Implement point-in-primitive predicates using Flint transforms:
  - `classify(p: FlintArray<f64,4>, node: &CsgNode) -> Classification`
    where `Classification` is `Inside | Outside | Indeterminate`
  - For each primitive: transform query point to local frame via Flint mat_mul,
    then evaluate the primitive's implicit function with outward rounding
  - Sphere: `||T⁻¹p||² < r²` (with Flint arithmetic)
  - Cuboid: AABB test in local frame
  - Cylinder/Cone: analytic implicit in local frame
- [ ] Implement tolerance/indeterminate-zone policy:
  - When Flint interval straddles the boundary, return `Indeterminate`
  - Caller decides: refine, fallback to subdivision, or treat as on-surface

#### Start with restricted subset
- [ ] Implement boolean scaffolding:
  - surface/surface intersection infrastructure
  - face splitting + trimming update machinery
  - classification (inside/outside) framework using Flint predicates above
  - sewing/healing basics + snapping with tolerances
- [ ] First boolean targets:
  - [ ] either: planar-only polyhedra subset
  - [ ] or: analytic pairs (plane/cyl/sphere) before full NURBS
- [ ] Use interval arithmetic as a robustness filter:
  - predicates + bounding checks using FlintArray<f64,16> transforms
  - if uncertain (Indeterminate) -> subdivision/refinement/fallback path

Deliverable for Phase 5:
- `Union/Difference/Intersection` works for a limited subset with stable output mesh.

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


