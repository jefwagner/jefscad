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
- [ ] Implement primative constructors (cuboid, cylinder, sphere, code)
- [ ] Implement trasform methods (translate, rot_x, rot_y, rot_z, rot_aa, scale,
 eventually shear)
  - the should return a new node? or mutate the existing node?
- [ ] Implement quantization for flat_matrix to so similar matrices give same hash
- [ ] Implement operator constructors (union, intersection, difference)
  - should creation create authored structure, or autoflatten/sort children
- [ ] Implement a CanonicalCsgNodeView allow manipulation without mutating authored AST
  - [ ] allow flattenting of ops
  - [ ] allow sorting of children
- [ ] Implement structural hashing to support evaluation caches

#### Python interface (authoring)
- [ ] Expose Python `Node` class that holds `Arc<CsgNode>`
- [ ] Provide Python constructors: `cube()`, `sphere()`, etc.
- [ ] Provide chainable transforms: `node.translate(...)`, `rotate(...)`, `scale(...)`
  - return new nodes (functional style)
- [ ] Provide ops: `union(a,b,...)`, `difference(base, sub...)`, etc.
- [ ] Provide a way to dump/inspect the AST for debugging (`repr`, `to_json`, etc.)

Deliverable for Phase 1:
- Python can build AST trees/graphs (with sharing) and introspect them. 

---

### Phase 2 — Geometry foundation (`geom` crate)
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
- [ ] Keep **topology** separate from **geometry**
- [ ] Plan for faces as **trimmed parametric surfaces**
  - loops defined in surface UV domain
  - long-term edges carry both 3D curves and per-face 2D p-curves

#### Core B-rep types
- [ ] Geometry handles:
  - [ ] `Surface` enum: NurbsSurface + analytic surfaces (plane/cyl/cone/sphere)
  - [ ] `Curve3` enum: NurbsCurve + analytic (line/circle/ellipse)
  - [ ] `Curve2` enum for trimming curves in UV space
- [ ] Topology:
  - [ ] `Vertex { position, tol, ... }`
  - [ ] `Edge { v0, v1, curve3, ... }`
  - [ ] `Coedge/Halfedge` (edge-use with orientation)
  - [ ] `Loop { coedges... }`
  - [ ] `Face { surface, loops, pcurves(per coedge), provenance(NodeId), ... }`
  - [ ] `Shell`, `Solid`
- [ ] Kernel-wide tolerance struct (in evaluation context, not per-node)
  - `pos_tol`, `ang_tol`, `param_tol`, merge policies

#### Primitive -> B-rep construction
- [ ] Build B-rep for each primitive (initially without CSG booleans)
- [ ] Ensure transforms are applied correctly (prefer push-down + compose early)

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
#### Start with restricted subset](#)
- [ ] Implement boolean scaffolding:
  - surface/surface intersection infrastructure
  - face splitting + trimming update machinery
  - classification (inside/outside) framework
  - sewing/healing basics + snapping with tolerances
- [ ] First boolean targets:
  - [ ] either: planar-only polyhedra subset
  - [ ] or: analytic pairs (plane/cyl/sphere) before full NURBS
- [ ] Use interval arithmetic as a robustness filter:
  - predicates + bounding checks
  - if uncertain -> subdivision/refinement/fallback path

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


