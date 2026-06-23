# Roadmap

Phased build plan toward the first application: a Python program that takes a font + a
character and produces a 3D-printable die face with that character engraved. The first
sub-step is a single cuboid with one inset (boolean-subtracted) font glyph on one face.

The plan is shaped by two principles from the design work:

1. **De-risk via standalone primitives first** — `pip2d`, ray-surface, `pis`, DCEL
   cycles are each independently buildable and testable. The boolean becomes an
   *integration* of pre-tested pieces, not a monolith debugged from scratch.
2. **Refactor the b-rep structs to defining-only + Context side-tables before
   booleans** — that's where the struct-lifecycle complexity bites hardest, and doing
   it *during* boolean dev would be a mess.

See `architecture.md` and `boolean-ops.md` for the design this plan builds on.

## Phase 0 — Foundation refactor + path/surface gaps

No boolean code yet. Cleanup + the path/surface gaps the dice needs.

- **Refactor b-rep structs to defining-only** with Context side-tables
  (`rebuild_indices()` cleanup method). Regression risk on existing primitives,
  mitigated by the existing primitive b-rep tests as a safety net.
- **Add tolerance to Context**: global relative + absolute floor, applied against local
  feature size. Reasonable default; Python getter/setter later if needed.
- **Add quadratic bezier segment** to the path module (cubic deferred to Phase 5+).
- **Extend ruled surface** to extrude bezier paths (for the glyph extrusion).
- **Extend rotate surface** to bezier paths (completeness, not dice-critical — could
  slip if time-pressured).

## Phase 1 — Geometric primitives (each independently testable)

The de-risking phase. No boolean yet; each primitive is a standalone deliverable with
standalone tests.

- **`pip2d`** — 2D point-in-polygon-with-holes (scanline). Test on hand-built loops.
- **Ray-surface intersection**: ray-vs-plane, ray-vs-cylinder, ray-vs-ruled-surface.
  Each per-surface-type, each independently testable.
- **`pis`** assembled from ray-surface + odd-parity + degeneracy retry (fixed unlikely
  direction (1, √2, √3) + random retry on detected degeneracy).
- **DCEL cycle enumeration** on hand-built half-edge graphs.
- **Selection operator** (contains point) as the first end-to-end consumer of PIS,
  tested on a cube and a cylinder.

**Deliverable**: a user can ask "is this point in this solid" and get a correct trinary
answer. The whole PIS + ray + primitive stack validated without a single boolean.

**Flexibility note**: Phase 1 primitives are mostly independent of the Phase 0 struct
refactor (`pip2d` is pure 2D; ray-surface and DCEL are standalone; only `pis` needs
solid/shell/face structs, which already exist). Could parallelize, but lean sequential
— refactor once on stable primitive code, then build everything on the clean base.

## Phase 2 — Minimal vertical boolean slice (planar only)

First boolean end-to-end. All planar SSI, trivial splitting, trivial classification.

- **Target**: cube minus an axis-aligned box poking into one face.
- Implement boolean phases 1–6 + 4.5 for this case:
  - Phase 1: plane-plane SSI (lines), edge-face and edge-edge intersection points.
  - Phase 2: clip SSI segments to face domains, produce new edges.
  - Phase 3: split existing edges at new vertices, build node-consistent graph.
  - Phase 4: DCEL cycle enumeration → candidate sub-faces with parentage tags.
  - Phase 4.5: coincident-face handling (anti-aligned touching, aligned inset).
  - Phase 5: classification (ray-vs-plane PIS, binary inside/outside, assert no
    on-boundary).
  - Phase 6: shell reassembly — basic face-edge-face walk, single-solid output.
  - Initial meshing of the new (split) faces.
- **Quick follow-up**: cube minus a *rotated* box, to exercise non-parallel plane-plane
  SSI without curved surfaces.

**Deliverable**: a renderable cube with a rectangular hole. First boolean works
end-to-end.

## Phase 3 — Curved surface in the boolean

- **Target**: cube minus a cylinder poking through one face.
- Adds: ray-vs-cylinder PIS, cylinder-vs-plane SSI (circle), curved-edge face splitting
  (circular arc on a planar face).
- **Shell reassembly test**: the doc's "cylinder cut in two by a thin block" →
  multi-solid SolidSet output. The case the selection operator exists to serve.

**Deliverable**: cube with a circular hole; multi-solid output validated.

## Phase 4 — Bezier extrusion boolean (the dice first step)

- **Target**: cube minus a glyph-extrusion (ruled surface from a quadratic bezier path)
  poking into the top face.
- **Two sub-steps** (poke-through first to de-risk, coincident second for the clean
  engraving look):
  - **4a — Poke-through**: glyph top *above* cube top, standard splitting, no
    coincident handling needed. De-risks the bezier-in-boolean path. **This is the main
    path for the first deliverable** (single cuboid with one inset glyph).
  - **4b — Coincident**: glyph top *flush* with cube top, aligned normals, Phase 4.5
    punches a clean hole = the glyph footprint. The clean engraving look.
- Adds: ray-vs-ruled PIS, ruled-vs-plane SSI (bezier curve on a planar face), bezier-
  edge face splitting; exercises Phase 4.5 on a real case.

**Deliverable**: a cube with a glyph-shaped engraving — the dice first step,
renderable. **4b (coincident faces) must work before starting the full dice-application
Python path** (per author's requirement).

## Phase 5 — Mesh export + glyph-from-font (the dice application)

- **STL/3MF export** from the boolean-result mesh. Mesh refinement per the size/angle
  constraint (motivation #1, delivered — smoothness is a mesh-time parameter).
- **Font-glyph extraction via `ttf-parser`** (Rust side, separate `jefscad-font` crate
  — see decisions log). `.ttf` quadratic beziers only for now; `.otf`/CFF cubic added
  later. Outline-only (no shaping); `rustybuzz` is a future track for multi-glyph
  kerning/ligatures.
- **The dice application**: a Python program that takes a font + a character and 3D-
  prints a die face with that character engraved.

**Deliverable**: the dice first application, end-to-end.

## Validation / rendering strategy

Every phase needs a way to visualize results to confirm correctness — this is on the
critical path of *every* phase, not a nicety.

- **2D paths**: SVG export + external viewer (author's existing workflow).
- **3D objects**: `.stl` or wavefront `.obj` export + external viewer (author's
  existing workflow).
- STL export is needed for Phase 5 anyway, so it's reusable from Phase 2 onward as the
  primary 3D validation channel.

## Decisions log

Captured design decisions that shape the plan, with one-line rationale. Full reasoning
in `architecture.md` and `boolean-ops.md`.

### Surface types & non-uniform scaling
- Cylinder non-uniform-xy-scaled → elliptic cylinder as ruled/extrusion of ellipse
  (reuses a type we build; native STEP). NURBS not used for cylinders.
- Cone non-uniform-xy-scaled → elliptic cone → NURBS fallback (no native STEP entity).
- Sphere any non-uniform scaled → NURBS fallback (uniform code path; minor waste for
  z-aligned spheroids, accepted).
- NURBS scope: engine-produced transform-fallback only, constrained rational-conic
  representation. No author-constructed NURBS. STEP import of arbitrary NURBS = future
  problem.

### Tolerance
- Global *relative* tolerance + absolute floor, against local feature size (not raw
  coordinate magnitudes). Lives on Context. Single knob, no per-entity bookkeeping.
- Exact predicates (Shewchuk-style) bookmarked as future robustness work, not built for
  the dice. Trigger to revisit: a real model breaks classification randomly.

### Coincident faces
- "Perpendicular normals" in original doc was a misnomer; corrected to **aligned vs
  anti-aligned** outward normals.
- Difference + aligned → destroy shared patch, base gets hole. Difference + anti-
  aligned → base unchanged. Union + anti-aligned → shared face interior, removed.
- The hole boundary comes from the regular edge/face intersection pipeline, not the
  coincident-face logic.

### Boolean pipeline
- Reframe: build a 2D planar arrangement in each face's uv domain; cells are candidate
  new faces. No "cutting plane path must completely split" mental model.
- **Edge-face and edge-edge intersection points are topologically essential** — they
  are the vertices that terminate new edges and let the arrangement close. (Author's
  original missing piece.)
- SSI segment must lie inside *both* face domains simultaneously to be a real new edge.
- Cycle orientation: outer CCW, inner CW (right-hand rule, matches STEP).
- t-parameter: curve-global t; edge = `(curve_id, t_start, t_end)`; pcurve same
  t-domain; coedge orientation controls traversal direction only.
- Working-copy booleans (option C): shallow-copy inputs into a working Context, mutate
  freely, emit as result. Originals preserved (AST nodes can be shared).

### Struct lifecycle
- Defining content (immutable, struct identity) vs convenience refs (derived, cached
  in Context side-tables, rebuilt at cleanup).
- Defining lives in the struct; convenience lives in Context side-tables — type-system
  enforced.
- Shell's down edge/vertex list is convenience (derived from faces), the main wrinkle
  vs pure "down = defining."
- Boolean ops work in a transient DCEL arrangement (own source of truth); they do not
  read Context convenience refs during the op. Cleanup = `rebuild_indices()` once after
  op end.
- Splitting produces new faces sharing the parent's surface; parentage tracked as
  transient scratch during the op, not stored permanently.

### Classification
- Hybrid: topological traversal (Family 2) as boolean workhorse, ray casting (Family 1)
  for seeds + standalone PIS (selection operator, user queries).
- PIS is trinary (inside/outside/on-boundary), single primitive, no wrapper.
- Phase 4.5 (coincident-face handling) runs *before* classification and *removes* the
  on-boundary case by construction. Classification asserts no on-boundary return;
  on-boundary = bug signal.
- Interior sample point via scanline `pip2d` in uv → 3D. Replaceable later with
  triangulate-and-centroid without interface change.
- Ray direction: fixed (1, √2, √3) + random retry on degeneracy.

### Font parsing
- **Rust-side via `ttf-parser`**, in a separate `jefscad-font` crate (not Python +
  `fonttools`).
- Reasons: (a) cargo-testable with lower friction than maturin+pytest for the same
  amount of glue code (the parser is a dep, not our code); (b) keeps the geometry core
  pure-Rust and reusable from non-Python contexts (CLI, WASM, other bindings) —
  aligns with how serious CAD kernels are structured.
- Crate split: `jefscad-core` (b-rep, booleans, meshing, paths, surfaces — no font dep)
  / `jefscad-font` (depends on core + ttf-parser) / `jefscad-py` (PyO3 bindings).
- Scope: outline-only (cmap + glyph outline → jefscad Path). No shaping. `rustybuzz`
  is a future track for multi-glyph.

### Export format
- Dice target export is STL/3MF (3D printing). STEP is a *later track*, not on the
  dice critical path. Design decisions preserve STEP-mappability for when STEP export
  is built.

### Bezier scope
- Quadratic bezier + `.ttf` first (covers most everyday fonts, simpler). Cubic bezier
  + `.otf`/CFF added in Phase 5+ once the pipeline works.

## Deferred / future tracks (not on dice critical path)

- **STEP export** (motivation #2) — for CNC/interchange. Design preserves mappability;
  export itself is later.
- **Shell reassembly drill-down** (Phase 6 design) — basic impl needed for roadmap
  Phase 2; multi-solid output is a roadmap Phase 3 test. Full design detail deferred.
- **Tangent / degenerate boolean cases** — default instinct: tangent SSI = non-
  crossing. Known-degenerate-case list to be built once the main pipeline runs.
- **Exact predicates** — bookmarked future robustness work.
- **Cubic beziers / `.otf` fonts** — Phase 5+.
- **Text shaping** (`rustybuzz`) — for multi-glyph engraving.
- **Cone, sphere, NURBS-fallback surface types** — added as needed beyond the dice.
- **STEP import** of arbitrary `b_spline_surface` — requires the general NURBS struct;
  future problem.
- **Solids of rotation with bezier paths** — completeness, not dice-critical.
