# Boolean Operations

The riskiest piece of the project and the one that required the most design work. This
doc captures the full pipeline, the classification design, the cross-cutting
primitives that de-risk it, and the t-parameter / orientation conventions that make it
all hang together.

## The conceptual reframe (read this first)

The naive mental model — "find a cutting plane path that completely splits the face" —
is wrong and is the source of the stuckness the author hit. The right model:

> On each face that participates in the boolean, build a **planar arrangement** in the
> face's 2D uv domain, formed by the *original edge-loops* of the face plus *all the
> new edges* clipped to that face. The **cells (cycles) of this arrangement are the
> candidate new faces.** Some become new faces; some get discarded by classification.

Under this model:
- A new edge doesn't have to "go all the way across" the face. It just has to land its
  endpoints on the existing graph.
- Multiple new edges from multiple surface-surface intersection (SSI) pairs all get
  inserted into the same arrangement; they don't need to connect a priori.
- "Face split into two," "hole punched in a face," "slit," "island" — all produced
  uniformly by the arrangement machinery. No special-casing.

This is why the b-rep has **pcurves** on coedges: the pcurve is the 2D image of the edge
on the face's surface, and the arrangement is built in that 2D uv space.

## The full pipeline

```
Phase 1: Geometric intersection
  For each (face_a, face_b) pair with overlapping bbox:
    compute SSI curve(s) C_ab  (surface_a ∩ surface_b in 3D)
  For each (edge_a, face_b) and (edge_b, face_a) pair:
    compute edge-face intersection points    ← THE MISSING PIECE the author originally had
  For each (edge_a, edge_b) pair:
    compute edge-edge intersection points

Phase 2: Restrict to faces → produce NEW EDGES
  For each SSI curve C_ab:
    clip to face_a's domain → pcurve P_a on surface_a
    clip to face_b's domain → pcurve P_b on surface_b
    Keep only contiguous segments where C_ab lies INSIDE BOTH domains simultaneously
    Each such segment = one new Edge (curve C, pcurves P_a and P_b, start/end vertices)
  Edge-face points and edge-edge points → new VERTICES (used to split existing edges in phase 3)

Phase 3: Insert new edges into each affected face's loop structure
  For each face F that received new edges or new vertices-on-its-edges:
    For each existing edge of F that has a new vertex landing on it (parametrically):
      SPLIT the edge at that vertex → two sub-edges, rewire coedge/edge topology
    Now F's edge-loops + new edges form a 2D planar graph on F's uv domain, all nodes consistent

Phase 4: Find cycles → candidate new faces
  On F's uv arrangement, walk the half-edge (coedge) structure to enumerate all face-cycles
  Each cycle → one candidate new face: same surface as F, new edge-loop from the cycle's coedges
  Tag each candidate with its PARENT face ID (transient scratch, consumed by Phase 5, discarded after)

Phase 4.5: Coincident-face handling
  Identify face pairs whose surfaces coincide within tolerance
  Apply aligned/anti-aligned rules (see below) to produce hole-bounded or removed faces
  Remove resolved coincident candidate sub-faces from the set Phase 5 will process

Phase 5: Classify each candidate new face against the OTHER solid
  For each candidate, sample an interior point (uv → 3D), run PIS against the other solid
  Apply operation rule (see table below)
  PIS must return inside/outside only — on-boundary would signal a Phase 2/3 or 4.5 bug (assert)

Phase 6: Shell reassembly  (design deferred; basic implementation needed for Phase 2 of roadmap)
  From surviving faces, walk face-edge-face connectivity to assemble Shells
  → Solid per connected shell; possibly multiple Solids in the SolidSet
    (the "difference cuts a cylinder in two" case)
```

## The crucial clipping rule in Phase 2 (common bug source)

A real new edge is a contiguous part of the SSI curve that lies **inside BOTH face_a's
domain AND face_b's domain at the same time.** Not just one of them.

The SSI is between *surfaces*, but faces are *subsets* of their surfaces bounded by
edge loops. The SSI curve can run through regions where:
- face_a exists but face_b doesn't (face_b's boundary excludes that part) → surfaces
  intersect there, faces don't. **Not a real edge. Discard for both.**
- face_b exists but face_a doesn't → same, discard.
- both exist → real edge, keep, with pcurves on both surfaces.

Implementation: compute the SSI curve; compute its restriction to face_a's domain (a
set of t-intervals); compute its restriction to face_b's domain (another set of
t-intervals); intersect the interval-sets; each contiguous resulting interval is one
new edge.

**Edge-face intersection points are the endpoints of those new edges.** The places
where the SSI curve enters/exits a face's domain are exactly where it crosses that
face's boundary edges. So edge-face and edge-edge points are not "extra data" — they're
the vertices that terminate new edges. *This was the missing piece in the author's
original mental model.* Without them, SSI segments have endpoints that don't land on
anything and Phase 3 can't close the graph.

## Phase 3–4 in detail: the arrangement / cycle-finding

### Node consistency (non-negotiable)

After inserting all new edges and splitting all existing edges at new vertices, the
graph on the face's uv domain must satisfy: **every vertex has all its incident edges
properly accounted for, no edge ending in mid-air.** Every endpoint of every new edge
is either (a) an existing face vertex, (b) a new vertex on an existing edge (which you
then split), or (c) a new vertex on another new edge's endpoint. If a new edge endpoint
lands in the *interior* of the face touching nothing, that's a bug — the SSI segment
was under- or over-clipped, or FP error lost an endpoint.

This is one of the most common boolean-op bug sources in real kernels: an SSI segment
endpoint lands 1e-13 off an existing edge, you fail to split, and a new edge dangles.
**The global *relative* tolerance against local feature size is exactly what snaps
"endpoint 1e-13 off the edge" to "endpoint on the edge"** so the split happens
correctly. Tolerance machinery directly enables this step.

### DCEL face-traversal (Phase 4)

Once node-consistent, you have a 2D planar graph on the face's uv domain. Finding new
faces is standard DCEL face-traversal:

```
For every directed coedge D in the face's graph not yet used in a cycle:
  Start a new cycle C = [D]
  Loop:
    Let T = twin of the last coedge in C   (same edge, opposite orientation)
    Let N = the coedge incident to T's vertex that is "next" around that vertex in
            the appropriate rotational direction (CCW for outer loops, CW for inner)
    Append N to C
    If N == D: cycle complete, one face's boundary loop. Mark all coedges used.
  Emit a new candidate face with this edge-loop, sharing the parent face's surface.
```

The "next around the vertex" step requires a per-vertex cyclic-incident-coedge list,
ordered by pcurve tangent angle at the vertex — the only nontrivial data structure to
add. "Next around vertex in CCW direction after the twin" is a constant-time lookup.

**Cycle orientation convention (locked)**: outer loops CCW, inner loops CW (right-hand
rule, outward normal). Matches STEP convention — b-rep aligns with STEP export. DCEL
"next around vertex" uses CCW for outer-loop traversal, CW for inner. **Test on the
"square cut by a line into two rectangles" case before generalizing.** Getting this
backwards produces inside-out faces that look correct in isolation but break
classification.

### Why this is the same machinery as the mesh

The DCEL / half-edge data structure is exactly what the mesh uses (HalfEdgeMesh with
half-edges, twins, nexts, faces). Boolean face-splitting is a 2D arrangement problem;
meshing is a later 2D-ish arrangement on each face for triangulation; mesh refinement
is local re-meshing. **The half-edge traversal code is reusable across at least three
places.** Worth implementing the core traversal once.

## Coincident-face rules (Phase 4.5)

Each face has an *outward* normal pointing away from its solid's interior.

- **Anti-aligned** outward normals at a coincident face = the two solids sit on
  *opposite sides* of the shared face. They touch/kiss but don't interpenetrate. (Two
  stacked cubes.)
- **Aligned** outward normals = the two solids sit on the *same side* of the shared
  face. One is glued to the other's wall from the inside, or one is inside the other
  along that face. (The glyph-inset case: cube top face outward = +z, glyph extrusion
  top face outward = +z, both point up, glyph hangs below.)

Rules (corrected from the original doc — "perpendicular" was a misnomer for
"anti-aligned"):

1. **Difference, aligned** (inset case) → overlapping patch destroyed: subtracted
   solid's face goes away, base face gets a hole where the overlap was. The dice
   glyph-inset behavior.
2. **Difference, anti-aligned** (merely touching) → base face left unchanged, as if
   the subtracted solid didn't overlap there. Subtracting something that just kisses
   the base shouldn't carve into it.
3. **Union, anti-aligned** (touching-and-joinable) → shared face becomes interior,
   removed; solids join cleanly. Standard "stack two solids into one."

(Union + aligned is degenerate/overlapping, not a clean join — error or containment
case, handled elsewhere.)

**Important**: rule 1 says "base face gets a hole" — that hole is bounded by *new
edges* (the footprint of the subtracted solid's side walls meeting the base face),
produced by the regular edge/face intersection machinery, *not* by the coincident-face
logic. The coincident-face rule only decides the *fate of the shared patch*; the hole
boundary comes from the standard intersection pipeline.

## Classification (Phase 5)

After Phase 3–4 (+ 4.5), each affected face is split into candidate sub-faces, each
with a known parent face (so a known "I'm on A's boundary" or "I'm on B's boundary").
Phase 5 decides keep/discard:

| Op | Sub-face on boundary of… | Keep if its interior is… |
|---|---|---|
| Union | A | outside B |
| Union | B | outside A |
| Difference (A−B) | A | outside B |
| Difference (A−B) | B | inside A |
| Intersection | A | inside B |
| Intersection | B | inside A |

Plus the coincident-face overrides above (resolved in Phase 4.5, before
classification).

Reduces to one primitive asked many times: **point-in-solid (PIS)** — same primitive
the selection operator uses. Design once, consume twice.

### Two PIS families

**Family 1 — Ray casting.** Shoot a ray from p; count intersections with the solid's
faces; odd = inside, even = outside. Degeneracy hell at grazing rays / vertices, but
it's a pure query on a finished b-rep, localizes to "ray vs face" per surface type,
testable independently of booleans.

**Family 2 — Topological traversal.** Don't classify each sub-face independently. The
sub-faces on a given original face form a connected partition; in/out status
*transitions across shared new edges* (SSI edges — crossing the other solid's boundary)
and *doesn't transition across shared original edges* (moving within the same face's
interior). Pick one unambiguous seed sub-face per connected component of the SSI graph,
flood-fill the rest. Robustness comes from topology, not from FP ray-surface
intersection. Fast: O(sub-faces) vs O(sub-faces × faces). Requires the intersection
graph to be correct and complete (which Phase 2–3 must be anyway).

### Decision: hybrid — Family 2 workhorse, Family 1 for seeds + standalone queries

- **Topological traversal (Family 2)** as the workhorse within a boolean. Robust where
  it counts (transitions), fast. Leans on Phase 2–3 correctness (needed anyway).
- **Ray casting (Family 1)** for seed classifications — one per connected component of
  the SSI graph (typically 1–4 per boolean). Degeneracy rare; retry with a different
  ray direction on detected degeneracy.
- **Ray casting as the standalone PIS** for the selection operator and any user-facing
  "is this point in this solid" query. These are one-off queries against a stable
  b-rep, not part of a boolean — ray casting is the natural API.

**Implementation order**: ray casting first (standalone deliverable, testable on a cube
and a cylinder immediately). Add topological traversal as a performance/robustness
upgrade to the boolean path once Phase 2–3 are solid.

### PIS contract (resolved)

**PIS is a single primitive, trinary: inside / outside / on-boundary. No wrapper.**
Used directly by classification and by the selection operator.

The "on-boundary" case is *designed out* of classification by phase ordering, not
handled by either layer:
- **For the selection operator / user-facing PIS**: the user may pass a point genuinely
  on the solid's boundary. PIS correctly returns on-boundary. Trinary return is right.
- **For classification**: PIS is called with the sub-face's *interior* sample point
  against the *other* solid. In a correctly-split b-rep (Phase 2–3 correct + Phase 4.5
  resolved coincident faces), an interior point of a sub-face on A's boundary cannot
  lie on B's boundary — the splitting would have separated them. So PIS-as-called-by-
  classification can never return on-boundary. **An on-boundary return is a bug signal**
  (Phase 2/3 missed a split, or Phase 4.5 missed a coincident face) — assert in debug,
  degrade gracefully (perturb sample + retry, or treat as outside) in release.

This is cleaner than a wrapper (no extra layer) and cleaner than classification
handling it inline (classification's code stays binary). The on-boundary case is
designed out by phase ordering, not handled by either layer. **This is why Phase 4.5
exists as a distinct phase between splitting and classification.**

### Interior sample point (Phase 5)

Pick a point strictly inside the sub-face's uv loop (centroid of loop vertices is *not*
guaranteed inside — think C-shaped face). **Decision: scanline point-in-polygon-with-
holes** in uv, then map to 3D via `surface.eval(u, v)`. Simple to implement now, no
coupling to meshing. Replaceable later with triangulate-and-centroid (reusing the mesh
triangulator) without changing classification's interface, if desired.

### Ray direction (Family 1)

**Decision: fixed unlikely direction + retry on degeneracy.** Default ray direction
e.g. normalized (1, √2, √3) (hits no axis-aligned cube face dead-on). If the ray hits a
vertex or grazes an edge (detected as two ray-face intersections at t-values within
relative tolerance), pick a new random direction and recast. Robust, simple, graceful
fallback.

## Cross-cutting geometric primitives (build + test in isolation)

Three reusable primitives that show up across the boolean pipeline and beyond. Each
independently buildable and testable; assembled into the boolean later. **This is the
de-risking strategy**: the boolean becomes an integration of pre-tested pieces, not a
monolith debugged from scratch.

1. **`pip2d` — 2D point-in-polygon-with-holes** (`pip2d(point, outer_loop, inner_loops)
   -> bool`). Used by:
   - Phase 2 (clip SSI segments to face domains — "is this uv point inside this face's
     domain")
   - Phase 5 (find sub-face interior sample point via scanline)
   - Any "is this point in this face" query.
   Unit-testable on hand-built loops with no b-rep in sight.

2. **`pis` — point-in-solid** (`pis(point, solid) -> {Inside, Outside, OnBoundary}`).
   Used by:
   - Phase 5 seeds (one per connected component of the SSI graph)
   - The selection operator
   - Any user-facing "does this point lie in this solid" query.
   Decomposes into per-surface-type ray-surface intersection (ray-vs-plane, ray-vs-
   cylinder, ray-vs-cone, ray-vs-sphere, ray-vs-ruled, ray-vs-rotate, ray-vs-NURBS),
   each independently testable. Build plane + cylinder first (covers cube, extrusion
   of a rectangle, cylinder primitive — most early models). Add surface types as the
   taxonomy grows.

3. **DCEL cycle enumeration** (`enumerate_cycles(half_edge_graph) -> Vec<FaceLoop>`).
   Used by:
   - Phase 4 (sub-face loops from the split arrangement)
   - Meshing (mesh face loops from a face's triangulation)
   - Mesh refinement (local re-meshing after vertex insertion).
   Testable on hand-built half-edge graphs. One core, three consumers.

**Testability order**: `pip2d` → ray-vs-each-surface-type → `pis` (assembled from
ray-surface) → DCEL cycles → boolean (assembled from all). Each step is a standalone
deliverable with standalone tests.

## t-parameter convention (locked)

The b-rep struct already encodes this; the author's "t=0 to t=1 per edge" instinct
would fight it. Lock the struct's design:

- **Curve-global t.** Each curve type has its own natural parametrization with its own
  domain (`[0, 17.3]`, `[-π, π]`, whatever the intersection march produces).
- **Edge = `(curve_id, t_start, t_end)`** — a sub-range on the curve. One curve serves
  many edges (an SSI curve clipped into three segments by two intermediate vertices →
  three edges, all referencing the same curve struct).
- **PCurve = same t-domain as the curve** over the edge's range. The load-bearing
  invariant `curve.eval(t) == surface.eval(pcurve.eval(t))` is a one-liner you can
  assert in tests. No remap function, no per-edge parametrization.
- **Coedge orientation** (forward/reversed) controls traversal direction only; eval
  functions are orientation-agnostic. Reversed coedge → traverse t from `t_end` down
  to `t_start`. Pcurve and curve still take the curve's t directly. Keeps geom structs
  orientation-agnostic; orientation lives purely in the topo layer (coedge), which is
  where the struct already puts it.

## Shell reassembly (Phase 6) — design deferred

Standard face-edge-face walk from surviving faces to assemble Shells, then Solids, then
the SolidSet. Genuinely lower-risk than Phases 1–5; the face-edge-face walking is
standard DCEL. The key test case is the doc's "cylinder cut in two by a thin block" →
multi-solid SolidSet output (the case the selection operator exists to serve).

**Deferred the design drill-down**; basic implementation is needed for Phase 2 of the
roadmap, multi-solid output is a Phase 3 test.

## Tangent / degenerate cases — deferred

Default instinct (capture, refine later): **classify tangent SSI curves as non-crossing
(no new vertex inserted, no edge split).** Conservative; avoids phantom topology. Risk
case is a *real* crossing misclassified as tangency due to FP — relative tolerance
against local feature size earns its keep here; exact predicates (the bookmarked future
work) would give certainty. Worth a known-degenerate-case list once the main pipeline
runs.
