# jefscad

A code-based solid modeling language for constructive solid geometry (CSG), written in
Rust with a Python interface via maturin + PyO3. Addresses three pain points with
OpenSCAD by delaying meshing until export time, supporting STEP interchange, and
handling coincident faces / slivers via tolerance-aware booleans.

This directory is the structured capture of a design + planning session. The original
braindump is in `braindump.md`; these docs supersede and refine it.

## Motivation

Three concrete OpenSCAD pain points drive the design:

1. **Delayed meshing.** OpenSCAD meshes each primitive at creation, so non-uniform
   scaling produces severe faceting (a circle becomes a polygon, then gets stretched).
   jefscad keeps an analytic boundary representation (b-rep) with exact surfaces until
   mesh time, so a circle stays a circle (or becomes an exact ellipse) under affine
   transforms. Mesh smoothness (triangle size / dihedral-angle constraints) is a
   *mesh-time* parameter, not a primitive-creation parameter.

2. **STEP export.** OpenSCAD can't emit STEP, which blocks CNC services and
   interchange with other CAD tools. jefscad's b-rep maps directly to STEP's native
   analytic surface entities (`plane`, `cylindrical_surface`, `conical_surface`,
   `spherical_surface`, `toroidal_surface`, `surface_of_linear_extrusion`,
   `surface_of_revolution`) where possible, falling back to rational NURBS
   (`b_spline_surface`) for the cases STEP has no native entity for.

3. **Tolerance-aware booleans.** OpenSCAD leaves thin slivers when differencing
   coincident-thickness solids (e.g. cylinder minus same-thickness block). jefscad
   handles coincident faces explicitly via aligned/anti-aligned normal rules, and uses
   a global relative tolerance to snap near-coincident entities cleanly.

A secondary motivation: use a **real programming language (Python)** for CSG authoring
instead of a custom DSL — get functions, loops, composability for free.

## First application: custom D&D dice

The first concrete deliverable is a Python program that takes a font + a character and
produces a 3D-printable die face with that character engraved. The first sub-step is a
single cuboid with one inset (boolean-subtracted) font glyph on one face.

This goal sits at the end of the dependency chain (bezier paths → extrusion → boolean
difference → meshing → export), so it validates the whole architecture end-to-end. The
roadmap in `roadmap.md` breaks it into a phased, de-risking build plan rather than
going straight for the end goal.

## Architecture summary

See `architecture.md` for the static structure (b-rep, surfaces, tolerance, CSG tree)
and `boolean-ops.md` for the boolean operation pipeline (the riskiest piece). Key
points:

- **CSG tree** authored in Python → compiled to a b-rep `SolidSet` per node. Tree nodes
  can be referenced by multiple parents (shared subnodes legal), so booleans must
  preserve inputs (working-copy pattern, not in-place mutation).
- **B-rep** with topological structs (SolidSet → Solid → Shell → Face → EdgeLoop →
  Coedge → Edge → Vertex) and geometric structs (Surface, Curve, PCurve, Point), all
  living in a `Context` with unique IDs and all cross-references via IDs.
- **Struct lifecycle**: structs carry only *defining* (immutable) content; *convenience*
  refs (derived lookups like "shell's edge set," "vertex's incident edges") live in
  Context side-tables rebuilt at cleanup. Boolean ops work in a transient DCEL
  arrangement that is its own source of truth, then emit new defining structs into a
  working-copy Context. Originals preserved.
- **Surface taxonomy** stays STEP-mappable: plane, cylinder, cone, sphere, torus
  (native analytic); ruled/extrusion (→ `surface_of_linear_extrusion`, covers elliptic
  cylinder); rotate/revolution (→ `surface_of_revolution`); NURBS fallback for
  elliptic cone, triaxial ellipsoid, elliptic torus (no native STEP entity).
- **Tolerance**: global *relative* tolerance with absolute floor, applied against local
  feature size (not raw coordinate magnitudes). Lives on the Context. Exact predicates
  (Shewchuk-style orient3d/incircle) bookmarked as future robustness work, not needed
  for the dice goal.
- **Cross-cutting geometric primitives** built and tested in isolation before
  assembly: 2D point-in-polygon-with-holes (`pip2d`), point-in-solid via ray casting
  (`pis`), DCEL cycle enumeration. These de-risk the boolean by making it an
  integration of pre-tested pieces.

## Roadmap summary

Phased plan toward the dice first goal:

- **Phase 0** — Foundation refactor (defining-only structs + Context side-tables,
  tolerance on Context) + path/surface gaps (bezier segments, ruled/rotate surface
  extension).
- **Phase 1** — Standalone geometric primitives (`pip2d`, ray-surface, `pis`, DCEL
  cycles) + selection operator as first end-to-end PIS consumer.
- **Phase 2** — Minimal vertical boolean slice (planar only: cube minus axis-aligned
  box, then rotated box). First boolean end-to-end.
- **Phase 3** — Curved surface in the boolean (cube minus cylinder). Multi-solid
  SolidSet output test.
- **Phase 4** — Bezier extrusion boolean = the dice first step. Poke-through (4a) then
  coincident (4b).
- **Phase 5** — STL/3MF export + mesh refinement + font-glyph ingestion via
  `ttf-parser`. The dice application.

See `../ROADMAP.md` for full detail, validation/rendering strategy, and decisions log.

## Open / deferred items (not on dice critical path)

- **STEP export** (motivation #2) — design decisions preserve STEP-mappability, but
  export itself is a later track. Dice only needs STL/3MF.
- **Shell reassembly drill-down** (boolean Phase 6) — standard face-edge-face walk,
  deferred design detail. Phase 2 needs a basic implementation; multi-solid output is
  a Phase 3 test.
- **Tangent/degenerate boolean cases** — deferred; default instinct is "tangent SSI
  curves = non-crossing (no new vertex, no split)." Worth a known-degenerate-case list
  later.
- **Exact predicates** — bookmarked future robustness work for when a real model breaks
  classification. Not needed for the dice.
- **Cubic beziers / `.otf` (CFF) fonts** — quadratic / `.ttf` first; cubic added in
  Phase 5+ once the pipeline works.
- **Text shaping** (kerning/ligatures for multi-glyph) — `rustybuzz` as a future Rust
  track; dice needs outline-only.
- **Cone, sphere, NURBS-fallback surface types** — added as needed beyond the dice.
- **STEP import** with arbitrary `b_spline_surface` of unknown provenance — noted as a
  future problem; current NURBS is constrained to "engine-produced transform fallback"
  only.
- **Solids of rotation with bezier paths** — completeness, not dice-critical.

## Doc index

- `architecture.md` — b-rep struct refactor, surface taxonomy, tolerance, CSG tree,
  struct lifecycle.
- `boolean-ops.md` — full boolean pipeline (phases 1–6 + 4.5 coincident-face
  handling), classification, PIS contract, cross-cutting primitives.
- `../ROADMAP.md` — phased plan, validation strategy, decisions log.
