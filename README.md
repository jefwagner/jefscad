# jefscad

A code-based solid-modeling language for constructive solid geometry (CSG), written in
Rust and exposed to Python via maturin + PyO3. Author CSG solids in a real programming
language (functions, loops, composability for free), keep them analytic until export
time, export to mesh or STEP, and handle coincident faces cleanly under tolerance.

**Status:** early-stage personal project. The first concrete deliverable is custom D&D
dice — a Python program that takes a font + a character and 3D-prints a die face with
that character engraved. See `ROADMAP.md` for the phased plan toward it.

---

## Why build it? (three OpenSCAD pain points)

OpenSCAD is good and worth recommending. But three recurring issues drove starting fresh:

1. **Early meshing.** OpenSCAD meshes each primitive at creation, so non-uniform scaling
   produces severe faceting — a circle becomes a polygon, then gets stretched. jefscad
   keeps an analytic boundary representation (b-rep) with exact surfaces until mesh
   time, so a circle stays a circle (or becomes an exact ellipse) under affine
   transforms. Mesh smoothness (triangle size / dihedral-angle constraints) is a
   *mesh-time* parameter, not a primitive-creation parameter.

2. **No STEP export.** OpenSCAD can't emit STEP, which blocks CNC services and
   interchange with other CAD tools. jefscad's b-rep maps directly to STEP's native
   analytic surface entities where possible (plane, cylinder, cone, sphere, torus,
   surface-of-linear-extrusion, surface-of-revolution), falling back to rational NURBS
   (`b_spline_surface`) for the cases STEP has no native entity for (elliptic cone,
   triaxial ellipsoid, elliptic torus).

3. **Thin slivers from coincident faces.** Differing a cylinder from a block of the
   same thickness can leave an extremely thin layer due to floating-point error.
   Workable with manual epsilon offsets, but it shouldn't be necessary. jefscad handles
   coincident faces explicitly via aligned/anti-aligned outward-normal rules and uses a
   global *relative* tolerance (with an absolute floor, applied against local feature
   size — not raw coordinate magnitudes) to snap near-coincident entities cleanly.

A secondary motivation: use **Python** for CSG authoring instead of a custom DSL, so
functions, loops, and composability come for free.

---

## How it works

1. **CSG tree** authored in Python → each node compiles to a b-rep `SolidSet`. Tree
   nodes can be referenced by multiple parents (shared subnodes), so booleans preserve
   their inputs via a working-copy pattern rather than in-place mutation.
2. **B-rep** with topological structs (SolidSet → Solid → Shell → Face → EdgeLoop →
   Coedge → Edge → Vertex) and geometric structs (Surface, Curve, PCurve, Point), all
   living in a `Context` with unique IDs and all cross-references via IDs. Structs
   carry only *defining* (immutable) content; *convenience* refs live in Context
   side-tables rebuilt by `rebuild_indices()`.
3. **Surfaces** stay STEP-mappable: plane, cylinder, cone, sphere, torus (native
   analytic); ruled/extrusion and revolution (native STEP entities covering elliptic
   cylinder and solids of rotation); NURBS fallback (engine-produced transforms only,
   constrained rational-conic — no author-constructed NURBS). Rational NURBS represent
   conics *exactly*, so a non-uniform scale that triggers NURBS conversion loses zero
   geometry.
4. **Meshing** via a half-edge (DCEL) data structure, allowing local refinement after
   initial meshing. The DCEL traversal is shared with boolean face-splitting.

The **boolean operation pipeline** (union / difference / intersection) is the riskiest
piece. It builds a 2D planar arrangement in each affected face's uv domain — the cells
of that arrangement are the candidate new faces, classified against the other solid via
point-in-solid (PIS). Coincident faces are resolved *before* classification (Phase 4.5)
so the on-boundary case is designed out rather than handled inline.

Full design detail lives in `architecture/`:
- `architecture/index.md` — entry point / session summary
- `architecture/architecture.md` — b-rep data model, surface taxonomy, tolerance
  model, struct lifecycle (defining vs convenience)
- `architecture/boolean-ops.md` — the full boolean pipeline, classification, PIS
  contract, cross-cutting primitives (`pip2d`, `pis`, DCEL cycles)
- `architecture/braindump.md` — the original planning braindump / motivation

---

## Project layout

```
repo root/
├── jefscad/            # Rust crate → compiled to jefscad._jefscad (Python extension)
├── python/jefscad/     # thin pure-Python wrapper package (re-exports from ._jefscad)
├── architecture/       # design narrative — the "what & why" (read for background)
├── docs/               # Sphinx user-facing HTML docs source
├── notebooks/          # Jupyter notebooks (interactive scratch)
├── ROADMAP.md          # long-horizon phased plan
├── TODO.md             # current / next-session actionable items
├── CHANGELOG.md        # curated milestone summaries (reverse-chronological)
├── DEVELOPMENT.md      # environment setup + build/test/jupyter how-to
└── AGENTS.md           # guidance for AI coding agents
```

The Rust crate compiles to `jefscad._jefscad` (the underscore prefix marks it as an
implementation detail); `python/jefscad/__init__.py` re-exports the public API, so
callers write `import jefscad; jefscad.sphere(...)`.

---

## Getting started

See `DEVELOPMENT.md` for the full setup. The essentials:

```bash
# One-time setup
uv venv .venv
uv pip install --python .venv/bin/python maturin pytest jupyterlab ipykernel
source .venv/bin/activate
maturin develop --features extension-module

# Daily loop
maturin develop --features extension-module   # rebuild after Rust edits
cargo test                                    # Rust unit tests (no Python linking)
pytest -v                                      # Python tests
```

**Toolchain note:** `jefscad` builds on stable Rust, edition 2024 (no nightly
features). A standalone `flint` crate — rounded floating-point interval arithmetic —
was co-developed in this workspace and required nightly Rust; it has been spun out to
its own repository as part of the Phase 0 foundation refactor. See `ROADMAP.md` →
Phase 0.

---

## License

MIT.
