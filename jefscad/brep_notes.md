# Notes for B-Rep design and corresponding data-structurs

I want to keep geometric information (surface, curve, point coordinates) separate from`
topological information (how a solid is constructed from faces, which edges bound a
face, which faces share an edge, which vertices bound an edge, etc.)

## Geometry Kernel Context

The BRep and meshing operations will take place within a context, this can hold caches
of the topology and geometry we'll need for further computation as well as some
parameters that we migth want set for all operations (tolerance snapping...)

* SolidModelingContext
  - surfaces: Vec<Surface>
  - curves3: Vec<Curve3>
  - curves2: Vec<Curve2>
  - solids: Vec<Solid>
  - shells: Vec<Shell>
  - faces: Vec<Face>
  - loops: Vec<Loop>
  - coedges: Vec<Coedge>
  - edges: Vec<Edge>
  - vertices: Vec<Vertex>
  - attributes: Vec<Arc<CsgMetadata>>

For each distinct type we can use the newtype pattern <TypeName>Id(usize), and implement
a push_<TypeName>(x: <TypeName>) -> <TypeName>Id that inserts the type into the
container, and get_<TypeName>(x: <TypeName>Id) -> &<TypeName> and get_mut_<TypeName>(x:
<TypeName>Id) -> &mut <TypeName> for accessing a given element. I'm not sure what other
stuff we'll need, but this is reasonable for now

## Topological Types

* NodeBRep: This corresponds to a single node in the CSG tree, and can (and often does)
  represent zero or more than one solid
  - solids: Vec<SolidId>
  - source_csg_id: usize (CsgNode prov_id)
  - (more to store cached computed values maybe)
* Solid: This is a continuous solid. It will correspond to one outer shell and a
  possibly zero-length set of inner shells that would represent voids inside.
  - outer: ShellId
  - inners: Vec<ShellId>
* Shell: This is a list or faces that define a single manifold surface
  - solid: SolidId (back-link to containing solid)
  - faces: Vec<FaceId>
  - is_outer: bool (is this the outer shell or an inner shell around a void)
* Face: This is a surface and one outer loop co-edges and an possibly zero-length set of
  inner loops that would represent holes in the face.
  - shell: ShellId (back-link to containing shell)
  - surface: SurfaceId
  - outer: LoopId
  - inners: Vec<LoopId>
  - sense: FaceSense (does the surface normal match the face normal)
  - prov: ProvenanceData
  - attr: AttrId
* Loop: This is a closed loop of co-edges.
  - coedges: Vec<CoedgeId>
  - face: FaceId (id of containing face)
  - is_outer: bool (is the the outer loop, or an inner loop around a hole)
  - note: there is an invariant where the edges are an ordered set that go from end
    of one = start of next and form a closed loop (end of last = start of first)
* CoEdge: This is an edge associated with a face, and will contain an edge and an
  orientation
  - edge: EdgeId
  - orientation: Orientation (does direction of co-edge match direction of edge)
  - face: FaceId (id of containing face)
  - pcurve: Curve2Id (curve in face's surface's u,v coordinates)
    - note: it is important to define the Curve2 to have the same orientation as the
    Curve3 so that the oritentation applies to both.
* Edge: This is curve and boundary vertices, and should also contain a list of co-edges
  - curve: Curve3Id
  - v0: VertexId (start point of edge)
  - v1: VertexId (end point of edge)
  - t0: f64 (curve.eval(t0) = v0)
  - t1: f64 (curve.eval(t1) = v1)
  - coedges: SmallVec<[CoedgeId; 2]> (coedges associated with the edge)
    - note: in intermediate steps, the full surface might not be a proper manifold so
    being able to store more than two coedges is useful.
* Vertex:
  - point: Point3
  - tol: f64 (tolerance used for snappoing vertices together, snap to edges, etc.
  - Question: should I store the point or add a points container to the
  SolidModelingContext and store the Point3Id?
  - We COULD store the adjacenty data here, but it that would be harder to keep
  consistent, so for now we'll leave adjacency data out of the Vertex struct

### Provenance

It is useful to track provenance of faces back through to the CSG nodes where they came
from. For that we've introduced a new ProvenanceData datastructure

* ProvenanceData:
  - sources: SmallVec<[CsgSource; 1]>
    - a primative would have a single CsgSource of 1
    - a coincident face from a union could have more than one source
    - a new face from intersection could have 2 source
  - last_op: Option<OpType>
* CsgSource (a face from a CSG primative)
  - prov_id: u64 (prov_id of original CSG node)
  - geom_id: u64 (geom_id of the original CSG node)
* OpType { Union, Difference, Intersection } (simple enum of boolean operators)

### FaceSense and Orientation

A Face has a corresponding surface, and there is a natural normal direction cross(d
surf/ dx, d surf/dy). If the sense of a face corresponds to whether the face normal
(pointing OUT of the solid) is aligned or anti-aligned with the surface normal.

* FaceSense { Aligned, AntiAligned }

Similarly, a curve has a natural direction corresponding to the direction of increasing
`t` parameter. If the start -> end corresponds to same increasing `t` the edge is in 
a forward direction, otherwise it is reversed.

* Orientation { Forward, Reverse }

### Degenerate Edges (Poles and Apices)

Some primitives have faces whose boundary degenerates to a point in 3D space — the
north/south pole of a sphere and the apex of a cone. These are handled using *degenerate
edges*: an edge where `v0 == v1` (same `VertexId`) and the 3D curve `eval(t)` is constant
for all `t` in `[t0, t1]`. The 3D derivative `eval_dt(t)` is the zero vector everywhere.

The degenerate edge still participates in the containing loop's CoEdge list normally. The
`pcurve` (Curve2Id) is *not* degenerate — it is a full line segment in the surface's UV
domain (e.g. the meridian line running from the equator to the pole at u=const). This
means UV-space traversal of the loop is well-defined even though the 3D image degenerates.

**Surface normals at degenerate points:**

- **Sphere pole**: `eval_n(u, v)` is well-defined and continuous as `v → ±π/2`. Both
  `eval_du` and `eval_dv` go to zero, but their cross product still points along ±z after
  normalization, giving a unique, correct outward normal. No special casing required.

- **Cone apex**: `eval_n(u, v)` is *not* well-defined at the apex. Different meridians
  approach from different directions and give different normals — the limit depends on
  path. The apex is a genuine geometric singularity (a sharp corner). In practice:
  - The mesher should not place a sample exactly at the apex; approach it within some
    tolerance and cap with a small polygon instead.
  - The B-rep kernel should treat the apex vertex as having no well-defined normal.
    Downstream operations (e.g. rendering normals, offset surfaces) need to handle this
    case explicitly. A reasonable approach: `eval_n` returns `None` (or an error) when
    evaluated at the apex parameter.
  - Boolean operations involving the apex need careful treatment, but this is deferred
    until Phase 5.

**Implementation note**: A degenerate edge can be identified by checking
`v0 == v1 && curve3.is_degenerate()` (or by a dedicated `is_degenerate: bool` flag on
`Edge` if the check turns out to be expensive).

### CSG→B-rep Transform Handling

When compiling a CSG primitive to B-rep, the primitive's `flat_transform` (a 4×4 affine
matrix) must be applied to the resulting geometry. The CSG layer accepts **any** affine
transform at any time; the question is only how the B-rep compiler handles them.

Let M be the 3×3 linear part of `flat_transform` (the top-left block; excludes
translation). The strategy:

**Cuboid** (planar faces only): Any M maps a plane to a plane — absorb the full transform
directly into each `Plane`'s `p0`, `u_dir`, `v_dir`. Always analytic, no check needed.

**Sphere**: Analytic iff M is a uniform scale times an orthogonal matrix, i.e.
`M^T · M = s²·I` for some scalar s. If yes, absorb into `SphericalSurface`: transform
center by the full 4×4, set `radius = s * original_radius`, re-orthogonalize `ref_dir`
and `axis` by applying M and normalizing. If no, fall back to `NurbsSurf`.

**Cylinder and Cone**: The axis direction transforms as `â' = normalize(M · â)`. The
cross-section stays circular iff M restricted to the plane perpendicular to `â` is
isotropic — that is, the 2×2 projected block has two equal eigenvalues. If yes, absorb
into `CylindricalSurface`/`ConicalSurface` (new origin, new axis, new ref_dir, scaled
radius or half-angle). If no, fall back to `NurbsSurf`.

**NURBS fallback**: A degree-2 rational NURBS represents circles (and hence ellipses,
after a linear transform applied to the control points) exactly — no approximation is
introduced. Applying the full affine transform to all control points of the canonical
NURBS circle/cylinder/sphere produces an exact elliptic/ellipsoidal representation.

**Edges on NURBS-fallback surfaces**: When the lateral surface of a cylinder or cone
falls back to `NurbsSurf`, the top and bottom circular edges become elliptic arcs.
Represent these as `NurbsCurve3` (degree-2 rational); apply the affine transform to the
arc's control points directly.

**Analytic edges on analytic surfaces**: When the analytic path is taken, circular arc
edges remain `CircularArc3` — absorb the transform into `center`, `normal`, `ref_dir`,
`radius` as for the surface above.

**Coincidence detection note**: When the NURBS fallback is taken, analytic coincidence
detection is unavailable for those faces. This is acceptable: coincident non-uniformly-
scaled curved surfaces are extremely rare in practice, and they fall back to the same
NURBS comparison path as any freeform surface pair.

### Curve3 and Surface Representation Strategy

`Curve3` and `Surface` will be **enums**, not trait objects. Dynamic dispatch (`dyn Trait`)
adds indirection and lifetime complexity that tends to compound in Rust geometry kernels.
The set of concrete curve and surface types is closed so an enum is the right fit.

**Decision**: Use analytic types for circles/cylinders/cones/spheres rather than
representing them as NURBS. Rationale:
- NURBS can represent circles exactly (rational B-splines with `w = cos(θ/2)`), but the
  representation is non-canonical — the same circle has infinitely many valid knot vectors,
  making coincident-face detection impossible without shape recognition.
- Analytic implicit forms (`x²+y²=r²` for cylinder) work directly with Flint arithmetic
  for certified inside/outside predicates. NURBS inversion requires iterative Newton steps
  where Flint interval widths compound and certification is lost.
- Many analytic surface-pair intersections have closed-form results (plane∩sphere →
  CircularArc3, plane∩cylinder → CircularArc3 or ellipse), avoiding the general SsiCurve3
  fallback for the common cases.
- STEP AP242 has first-class entities for all four analytic surface types; analytic types
  map directly with no reverse-engineering step.

```rust
enum Curve3 {
    Line3(Line3),
    CircularArc3(CircularArc3),
    Nurbs(NurbsCurve3),
    Ssi(SsiCurve3),           // Phase 5: general surface-surface intersection
}

enum Surface {
    Plane(Plane),
    Cylinder(CylindricalSurface),
    Cone(ConicalSurface),
    Sphere(SphericalSurface),
    Nurbs(NurbsSurf),
}
```

`Curve2` does **not** need a `CircularArc2`. The standard parameterization for all four
analytic surfaces uses `u = angle ∈ [0, 2π)`, so circles on those surfaces become
horizontal lines in UV space — already covered by `Line2`.

All eval/derivative calls dispatch through a `match` in a single `impl` block. Adding a
new variant requires touching that `match` everywhere — which is the point: the compiler
enforces exhaustiveness.

## Geometric Types

### Traits

For surfaces and curves there are multiple types that can be used and they should all
implement the following traits

* Surface: represents a parametric surface (u,v) -> R^3
  - eval(u: f64, v: f64) -> Point3 (evaluate point on surface)
  - eval_du(u: f64, v: f64) -> Point3 (evaluate u-derivative (un-normalized u-tangent))
  - eval_dv(u: f64, v: f64) -> Point3 (evaluate v-derivative (un-normalized v-tangent))
  - eval_n(u: f64, v: f64) -> Option<Point3> (evaluate normalized surface normal; None at geometric singularities like the cone apex)
* Curve3:
  - eval(t: f64) -> Point3 (evaluate point on curve)
  - eval_dt(t: f64) -> Point3 (evaluate t-derivative (un-normalized tangent) on curve)
* Curve2:
  - eval(t: f64) -> (f64, f64) (evaluate u,v point on curve)
  - eval_dt(t: f64) -> (f64, f64) (evaluate t-derivative (un-normalized tangent in uv space))

### Structs

The following structs implement the `Surface`, `Curve3`, or `Curve2` traits.

* Line2 - implements Curve2
  - p0: (f64, f64) - the point corresponding to t=0
  - p1: (f64, f64) - the point corresponding to t=1
  - t_min: f64 - lower bound of t space domain
  - t_max: f64 - upper bound of t space domain
* NurbsCurve2 - implements Curve2
  - p: u8 - The order of the nurbs curve
  - k: SmallVec<[f64; N]> - the knot-vector (See note about `N`)
  - cp: SmallVec<[(f64, f64); N]> - the control points
  - w: SmallVec<[f64; N]> - the weights

* Line3 - implements Curve3
  - p0: Point3 - the point corresponding to t=0
  - p1: Point3 - the point corresponding to t=1
  - t_min: f64 - lower bound of t space domain
  - t_max: f64 - upper bound of t space domain
* NurbsCurve3 - implements Curve3
  - p: u8 - The order of the nurbs curve
  - k: SmallVec<[f64; N]> - the knot-vector (See note about `N`)
  - cp: SmallVec<[Point3; N]> - the control points
  - w: SmallVec<[f64; N]> - the weights
* CircularArc3 - implements Curve3
  - center: Point3 - center of the circle
  - normal: Point3 - unit vector normal to the plane of the circle; sweep direction follows
    the right-hand rule around this normal
  - radius: f64 - radius of the circle
  - t0: f64 - start angle in radians; eval(t0) gives the start point
  - t1: f64 - end angle in radians; t1 > t0; t1 - t0 = 2π for a full circle
  - note: a reference direction (the point at t=0) must be established consistently; we
    define it as an arbitrary unit vector perpendicular to `normal` (e.g. the first
    axis of a frame built from `normal` using a fixed convention). This needs to be
    stored or derived deterministically to avoid ambiguity during STEP export and
    coincidence checks.

* Plane - implements Surface
  - p0: Point3 - the point corresponding to (u,v) = (0,0)
  - u_dir: Point3 - unit vector in the u direction; eval(u,v) = p0 + u*u_dir + v*v_dir
  - v_dir: Point3 - unit vector in the v direction; must be perpendicular to u_dir
  - note: normal = u_dir × v_dir (right-hand rule); domain is defined by the trimming loop
* CylindricalSurface - implements Surface
  - origin: Point3 - center of the base circle (point at v=0 on the axis)
  - axis: Point3 - unit vector along the cylinder axis (direction of increasing v)
  - ref_dir: Point3 - unit vector perpendicular to axis; defines the u=0 meridian
  - radius: f64 - radius of the cylinder
  - parameterization: u = angle in radians [0, 2π), v = height along axis
  - eval(u,v) = origin + v*axis + radius*(cos(u)*ref_dir + sin(u)*(axis × ref_dir))
* ConicalSurface - implements Surface
  - apex: Point3 - the apex of the cone (the degenerate point)
  - axis: Point3 - unit vector along the cone axis pointing away from the apex toward the
    base; direction of increasing v
  - ref_dir: Point3 - unit vector perpendicular to axis; defines the u=0 meridian
  - half_angle: f64 - half-angle of the cone in radians (0 < half_angle < π/2)
  - parameterization: u = angle in radians [0, 2π), v = distance from apex along the
    slant surface (i.e. the slant height, not the axial height)
  - eval(u,v) = apex + v*(cos(half_angle)*axis + sin(half_angle)*(cos(u)*ref_dir +
    sin(u)*(axis × ref_dir)))
  - note: eval_n is None at v=0 (the apex); see Degenerate Edges section
* SphericalSurface - implements Surface
  - center: Point3 - center of the sphere
  - radius: f64 - radius of the sphere
  - ref_dir: Point3 - unit vector defining the u=0, v=0 reference direction (equatorial
    point at longitude 0)
  - axis: Point3 - unit vector from center toward north pole; defines v = +π/2
  - parameterization: u = longitude [0, 2π), v = latitude [-π/2, +π/2]
  - eval(u,v) = center + radius*(cos(v)*(cos(u)*ref_dir + sin(u)*(axis × ref_dir)) +
    sin(v)*axis)
  - note: eval_n is well-defined everywhere including the poles; see Degenerate Edges
    section
* NurbsSurf - implements Surface
  - pu: the order in u
  - pv: the order in p
  - ku: SmallVec<[f64; N]> - the knot-vector in u-space
  - kv: SmallVec<[f64; N]> - the knot-vector in v-space
  - cp: SmallVec<[Point3; N*N]> - the control points
  - w: SmallVec<[f64; N*N]> - the weights

**Note**: We should reasonably assume that most Nurbs curves/surfaces will have a max
order, usualy 2 or 3 depending on how its defined to support exact representation of
circular arcs. the `N` term in the smallvec would be the size expected in such a case,
and the `N` for the knot-vector is NOT the same as the `N` in the number of control
points and weights.

And a simple structs

* Point3
  - x: f64
  - y: f64
  - z: f64

* Rect2
  - u_min: f64
  - v_min: f64
  - u_max: f64
  - v_max: f64

* We will also need a more complicated Surface-Surface intersection curve. I want to
tackle this soon, but not immediately.

* I already have a full implementation of a numeric type as rounded floating point
intervals, where all the interval is grown for every arithmatic and basic math (sin,
cos, etc) operations such that the true result is always within the interval. I would
like to use this type/implementation to robustly answer geometry queries PRIMARILY
focused on the case of identifying coincident faces to avoid the 'thin sliver' or
'objects not joined' issue you occationaly get with OpenSCAD meshes. I am currently not
exactly sure how to do that, but it is important that I make sure my design for the BRep
kernel supports this feature.


