# Architecture

Static structure of jefscad: the b-rep data model, surface taxonomy, tolerance model,
CSG tree, and struct lifecycle. For the boolean operation pipeline see
`boolean-ops.md`; for the build plan see `roadmap.md`.

## CSG tree → b-rep compilation

A solid is authored in Python as a CSG tree:

- **Leaf nodes**: primitives (cuboid, cylinder, sphere, cone), 2D→3D extrusions, and
  solids of rotation (open or closed path).
- **Transform nodes**: translation, rotation, scaling — applied to any subtree.
- **Boolean nodes**: union, difference, intersection — over any subtrees.
- **Selection node**: pick a single solid from a multi-solid result by "contains point"
  (uses the same PIS primitive as boolean classification — see `boolean-ops.md`).

Each CSG node compiles to a b-rep `SolidSet`. **Tree nodes may be referenced by
multiple parents** (e.g. define `wheel`, use it four times), so booleans must preserve
inputs: a boolean op shallow-copies its input structs into a *working-copy* Context,
mutates freely within, and emits the working copy as the result. Originals stay valid
for reuse. (See "Struct lifecycle" below.)

## B-rep data model

### Topological structs

```
SolidSet  ─down─▶  collection of Solids
Solid     ─down─▶  outer Shell + list of inner Shells (around voids)
Shell     ─down─▶  collection of (Face, face-sense)s, Edges, Vertices
Face      ─down─▶  outer EdgeLoop + list of inner EdgeLoops (holes); geom: Surface
EdgeLoop  ─down─▶  ordered list of Coedges
Coedge    ─down─▶  Edge + orientation; geom: PCurve
Edge      ─down─▶  start Vertex, end Vertex; geom: Curve + t-range
Vertex    ─down─▶  geom: Point
```

### Geometric structs

- **Surface** — uv → R³ mapping with rectangular max domain. Traits: `eval`, `du`,
  `dv`, `norm`.
- **Curve** — t → R³ mapping with finite t-domain. Traits: `eval`, `dt`.
- **PCurve** — t → uv mapping for a given surface, with finite t-domain. Traits:
  `eval`, `dt`.
- **Point** — a point in R³.

**Load-bearing invariant**: for any Edge and its Coedge's PCurve on a Face, over the
edge's t-range, `curve.eval(t) == surface.eval(pcurve.eval(t))` to within tolerance.
Same t-parameter on both. (See "t-parameter convention" in `boolean-ops.md`.)

### Context and ID-based references

All structs live in a `Context` object with unique IDs per struct. **All connections**
— downward (SolidSet → Solids) and upward (Coedge → Face) — are via IDs, never raw
pointers. The Context is the sole owner; everything else is ID lookups.

## Struct lifecycle: defining vs convenience (key refactor)

This is the most important architectural decision in the session. Two categories of
data on every struct:

- **Defining content** — the struct's *identity*, set at construction, never mutated.
  Changing any defining field means abandoning the struct (ID dropped) and creating a
  new one with a new ID. Example: a `Face`'s surface + edge-loops; an `Edge`'s curve +
  t-range + start/end vertices; a `Vertex`'s point.
- **Convenience refs** — derivable from the defining content of the whole b-rep;
  cached for traversal speed; can be stale during ops; rebuilt at cleanup. Example:
  Shell's down edge/vertex set (derivable from faces), Vertex's up edge set, Face's up
  shell.

### Rule: defining content lives in the struct; convenience refs live in Context side-tables

Structs carry *only* defining fields. Convenience refs are **not** struct fields — they
live in derived lookup tables on the Context (e.g. `Context::faces_by_shell:
HashMap<ShellId, Vec<FaceId>>`). The type system enforces the split for free: "is it
a struct field or a Context index?" == "is it defining or convenience?"

Benefits:
- You physically cannot accidentally mutate a convenience ref "on the struct" — there
  isn't one.
- Defining structs are trivially pure data + ID; immutability is enforced by simply
  not exposing setters.
- Stale-ness is scoped to "between ops" by construction. Cleanup is one method:
  `Context::rebuild_indices()`, called once after every topological-mutating op.
- Makes the boolean working-copy clean: shallow-copy *defining* structs (immutable
  data) into a new Context, build fresh derived tables, mutate freely.

Cost: every "up" or "shell's edges" traversal does a Context lookup instead of a struct
field access. Zero additional indirection in practice because the design is already
Context-mediated via IDs.

### Defining vs convenience mapping

| Struct | Defining (immutable, in struct) | Convenience (derived, in Context) |
|---|---|---|
| SolidSet | down: Solids | — |
| Solid | down: outer shell + void shells | up: SolidSet |
| Shell | down: (face, sense) list | down-cache: edges, vertices; up: Solid |
| Face | down: outer loop + inner loops + surface | up: Shell |
| EdgeLoop | down: ordered Coedges | up: Face |
| Coedge | down: edge + orientation + pcurve | up: Face |
| Edge | down: start v + end v + curve + t-range | up: Coedges, Faces |
| Vertex | down: point | up: Shell, Faces, Edges |

**Wrinkle**: Shell's down edge/vertex list is *convenience* (derived from faces), not
defining — a shell is fully defined by its faces+face-senses. This is the main place
"down = defining" intuition breaks; use the test "if this field changed, is it a
different struct or the same struct with a refreshed view?" to classify.

### Transient working data (not in the permanent schema)

Boolean ops use scratch data that is *not* part of the b-rep schema: the DCEL
arrangement on each face's uv domain, candidate sub-face lists, parentage tags (which
parent face each candidate came from). This scratch is consumed during the op and
discarded; surviving faces have no parentage tag. **General principle**: ops carry
their own transient data; the permanent schema stays minimal (defining content +
rebuilt indices). Meshing will follow the same pattern.

### Immutability + working-copy for booleans

Because AST nodes can be shared, inputs cannot be consumed. Booleans use **option C**:
shallow-copy input defining structs into a working Context at op start (remap IDs,
share geometry), mutate freely within via approach-A-style splicing (abandon old
structs, create new ones for any defining change), emit the working Context as the
result. Originals preserved for future reuse.

## Surface taxonomy and non-uniform scaling

Analytic surfaces, kept STEP-mappable:

| Surface | Internal type | STEP target | When |
|---|---|---|---|
| Plane | planar | `plane` | always |
| Circular cylinder | cylindrical | `cylindrical_surface` | no breaking affine |
| Circular cone | conical | `conical_surface` | no breaking affine |
| Sphere | spherical | `spherical_surface` | no affine break |
| Torus | toroidal | `toroidal_surface` | no affine break |
| Elliptic cylinder | ruled/extrusion of ellipse | `surface_of_linear_extrusion` | non-uniform xy scale of cylinder |
| Ruled/extrusion (any path) | ruled | `surface_of_linear_extrusion` | extrusions, incl. elliptic cylinder |
| Rotate/revolution (any path) | rotate | `surface_of_revolution` | solids of rotation |
| Elliptic cone | NURBS (rational) | `b_spline_surface` | non-uniform xy scale of cone |
| Triaxial ellipsoid | NURBS (rational) | `b_spline_surface` | any non-uniform scale of sphere |
| Spheroid (any axis) | NURBS (rational) | `b_spline_surface` | any non-uniform scale of sphere |
| Elliptic torus | NURBS (rational) | `b_spline_surface` | non-uniform scale of torus |

### Key decisions

- **Cylinder** non-uniform-in-xy scaled → elliptic cylinder → represented as **ruled
  surface = surface_of_linear_extrusion of an `ellipse` curve**. Reuses a type we
  already build. Native STEP. NURBS not used for cylinders at all.
- **Cone** non-uniform-in-xy scaled → elliptic cone → no native STEP entity, no clean
  ruled/rotate reuse → **NURBS fallback**.
- **Sphere** any non-uniform scale → **NURBS fallback**. Chose uniform NURBS even for
  z-aligned prolate/oblate spheroids (which would be clean `surface_of_revolution` of
  an ellipse) to avoid the "extra rotation when the symmetry axis isn't z" confusion.
  Known minor trade-off: slightly wasteful representation for the clean case, but
  uniform code path. Worth noting, not a problem.
- **Ruled surface** = surface_of_linear_extrusion (extrusions along any path, including
  ellipse → covers elliptic cylinder).
- **Rotate surface** = surface_of_revolution (solids of rotation along any path;
  internal axis always z).

### NURBS scope (constrained, not general)

NURBS surfaces are **only ever created by the engine** as a transform-fallback of a
circular surface. Authors never construct one by hand. This means the NURBS struct can
be a *constrained* rational-conic representation (low degree, fixed structure, easy
eval/deriv) — no general knot vectors from author input, no knot insertion, no
control-point/weight manipulation.

**Rational NURBS represent conics exactly** (not approximately) via weights. So the
NURBS fallback loses zero geometry — it's a circle/ellipse, exactly, just under a more
complex representation. The "circle stays a circle until meshing" property (motivation
#1) is preserved even when a non-uniform scale triggers NURBS conversion.

**Future problem noted**: STEP *import* of arbitrary `b_spline_surface` of unknown
provenance would require the *general* NURBS struct. Deferred; not needed for the
project's current direction.

## Tolerance model

### Decision: global *relative* tolerance + absolute floor, against local feature size

Single parameter on the Context (the "modern global variable" — reasonable default,
getter/setter exposed to Python later if needed). Implemented as a function, not a
constant:

```
fuzzy_eq(a, b, ref_scale):
    return |a - b| <= eps_rel * max(ref_scale, abs_floor)
```

- `eps_rel` ~ 1e-10 to 1e-12 (tune; captures accumulated FP error with margin).
- `abs_floor` ~ 1e-12 (so values near zero — where relative tolerance blows up — still
  have a sane floor).
- **`ref_scale` = local feature size**, not raw coordinate magnitudes. E.g. bounding-box
  diagonal of the two faces being compared, or length of the edge being tested. This
  is the crucial nuance: relative-to-coordinate-value breaks when models are translated
  far from origin (giant solid at 1e6 from origin, feature is 1mm — relative-to-coord
  uses 1e6, wrong). Relative-to-local-feature-size handles it correctly.

Still one global knob, still no per-entity bookkeeping. Achieves the stated goal
("capture FP rounding for values between 1e-5 and 1e5") because the tolerance scales
with the error, which itself scales with the value.

### Why not global *absolute* tolerance

Absolute FP error swings ~7 orders of magnitude across 1e-5 to 1e5 values. No single
absolute constant works for both ends: pick small → fails to recognize engine-produced
equal points as equal at large magnitudes (phantom slivers); pick large → snaps
distinct tiny features together at small magnitudes. Exactly the OpenSCAD failure mode
when modeling at unusual scales.

### Why not per-entity tolerance (OpenCASCADE-style)

Avoid it for now; probably avoid it forever. Per-entity tolerance exists to handle the
case where a boolean op creates a new vertex via an error-amplifying op (intersection
of nearly-parallel edges has huge error bar). But:
- jefscad doesn't have OpenCASCADE's long-lived-evolving-model workflow that forces
  evolving tolerances.
- jefscad's tolerance questions are mostly at coincident-face / sliver-removal level,
  where compared entities were created by the same op (correlated, similar error bars)
  — global relative handles that fine.
- Per-entity machinery is the source of a lot of OCCT's robustness *and* a lot of its
  bugs (tolerance-only-grows-never-shrinks pathology). Not worth importing.

Caveat: if a real model breaks classification and the cause is "comparing a 1-ulp-error
primitive vertex against a 1e-8-error intersection point," that's the symptom that
says reconsider per-entity. Bookmark, don't build now.

### Robustness vs tolerance — two different problems, not to be conflated

- **Tolerance** = "these two faces are 1e-13 apart; that's below my noise floor; I'll
  *deliberately model* them as coincident." An *engineering* decision to snap. Used for
  coincident-face detection, sliver removal, "are these two entities the same."
  Delivers motivation #3.
- **Exact predicates** = "I must *know with certainty* whether point p is above/below
  this plane." For geometric classification that must never be wrong (orient3d,
  incircle, edge-edge). Tolerance makes these *worse* (returns "ambiguous" exactly
  where it matters most). Right tool is filtered/adaptive exact arithmetic
  (Shewchuk-style). Not a pervasive numeric type; a few functions called at the 5
  sites where "I'm not sure" is unacceptable.

**Bookmarked as future work, not built for the dice goal.** If a real model breaks
classification randomly, that's the trigger to add exact-predicate versions of
orient3d/incircle/edge-edge. (The author's earlier rounded-interval-arithmetic crate
was the right instinct applied pervasively; the value is concentrated in those 5 sites
which exact predicates cover more cleanly. Pervasive interval type abandoned
correctly.)

## 2D paths (for extrusion and solid-of-rotation leaves)

A path is an ordered list of segments, where each segment's start = previous segment's
end. Segment types:

- Line
- Circular arc
- **Quadratic bezier** (added Phase 0 — needed for `.ttf` font glyphs)
- **Cubic bezier** (added later — needed for `.otf`/CFF fonts)

Current state: lines + circular arcs only. Phase 0 adds quadratic bezier; cubic is a
later track.

## 2D→3D non-primitive leaf nodes

- **Extrusion**: closed path in x-y plane, extruded linearly along z → ruled surface.
- **Solid of rotation, open path**: open path in x-z half-plane (positive x), rotated
  around z-axis, endpoints extended to z-axis as circular caps.
- **Solid of rotation, closed path**: closed path in x-z half-plane, rotated around
  z-axis → torus-like shape.

jefscad is a *solid* modeler — no infinitely-thin sheets.

## Selection operator

Picks a single solid from a multi-solid `SolidSet` (e.g. difference that cuts a
cylinder in two). Rule: **"contains point"** — caller provides a point, operator
returns the solid that contains it. Uses the same PIS primitive as boolean
classification (see `boolean-ops.md`).

Open question (deferred): what happens when the point is *on* a boundary of a
candidate (ambiguous), or when no solid contains the point (error vs empty result)?
Decide when implementing.
