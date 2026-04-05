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
- [ ] Build B-rep for each primitive (initially without CSG booleans)
- [ ] Apply `flat_transform` from the CSG node during compilation (see brep_notes.md for
      the transform-handling strategy):
  - `Cuboid`: any affine transform produces planar faces — always absorb directly into
    `Plane` parameters. No check needed.
  - `Sphere`: check if linear part M satisfies `M^T · M = s²·I` (uniform scale ×
    rotation). If yes, absorb into `SphericalSurface` (new center, new radius). If no,
    fall back to `NurbsSurf`.
  - `Cylinder`, `Cone`: check if M restricted to the plane perpendicular to the axis is
    isotropic (two equal eigenvalues in the projected 2×2 block). If yes, absorb into
    `CylindricalSurface`/`ConicalSurface`. If no, fall back to `NurbsSurf`.
  - Edges on NURBS-fallback surfaces: circular arc edges become `NurbsCurve3` (degree-2
    rational NURBS represents the ellipse exactly; apply the full transform to the
    control points).

Deliverable for Phase 3:
- `compile_primitive_to_brep(prim)` works and produces a valid trimmed-surface B-rep.

---

### Phase 4 — Meshing from B-rep (first real output)
- [ ] Tessellate trimmed parametric faces
  - chordal deviation / curvature-based refinement knobs
  - consistent sampling along shared edges (watertightness)
- [ ] Generate triangle mesh for preview/export
- [ ] Export to STL/OBJ (later glTF for preview)

Deliverable for Phase 4:
- Full pipeline for primitives: Python AST -> B-rep -> mesh -> file.

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


