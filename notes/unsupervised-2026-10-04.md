# Unsupervised run — 2026-10-04

**Goal:** Phase 0-c — b-rep defining-only structs + `Context` side-tables +
relative tolerance, per `/workspace/goal.md` (approved 2026-10-04 14:57).

**Branch:** `agent/2026-10-04`, worktree `worktree/agent-2026-10-04`, based on
`dev` @ `6b4e5c8` (which carries both golden safety nets).

## Status: goal met

Two commits on top of `dev`:

1. `Rename NodeBRep to SolidSet; compiler returns SolidSetId` (0-c.Q4).
2. `Make b-rep structs defining-only with Context side-tables and relative
   tolerance` (the main migration).

## Decisions as implemented

- **Q1 — `Shell.faces` stays defining.** Only `Face.shell`, `Loop.face`,
  `CoEdge.face`, `Shell.solid`, `Edge.coedges`, and `Vertex.tol` were removed;
  `Shell { faces, is_outer }` keeps `faces`.
- **Q2 — option A tolerance.** `Tolerance { eps_rel: 1e-10, abs_floor: 1e-12 }`
  and `fuzzy_eq(a, b, ref_scale, tol) = |a-b| <= eps_rel * max(ref_scale,
  abs_floor)`. `abs_floor` is documented as a *minimum reference scale*; the
  effective minimum distance is `eps_rel * abs_floor`. Orientation uses
  `angle_between` = `atan2(|u×v|, u·v)` plus `fuzzy_angle_eq` (comparison of
  `ref_scale = 1`), never `acos`. No angular floor/epsilon added.
- **Q3 — minimal, mechanical migration.** No algorithm changes, iteration order
  preserved (side-table builders walk arenas in order, so `coedges_of_edge`
  matches the old creation order), `#![allow(dead_code)]` staging markers
  untouched.
- **Q4 — compiler returns `SolidSetId`.** `compile_primitive`/`compile_csg_node`
  push exactly one `SolidSet` per compiled node (one solid for a primitive),
  `source_csg_id` from the node's `prov_id`. A `sole_solid` helper extracts the
  single solid for primitive-only callers. No multi-outer extrusion, no boolean
  evaluation.

## Non-goals respected

No `mesher`/`bool_ops` algorithm rewrites; no cap-meshing fix; no multi-outer
extrusion; no exact predicates; no Python tolerance API; no generation handles;
the cubic-Bezier `contour_signed_area` 2× bug (backlog) was left alone.

## Test results

- `cargo test`: **598** lib tests pass (594 prior + new `rebuild_indices` and
  `fuzzy_eq`/`fuzzy_angle_eq` tests). Both integration golden guards pass:
  `golden_brep` (12 fixtures) and `golden_mesh` (10 fixtures) **byte-identical**;
  both regenerators still `#[ignore]`d.
- `pytest`: **49 passed** (after `maturin develop --features extension-module`).
- `cargo check --features extension-module`: clean.
- `rustfmt`: changed files clean (`brep_compiler`, `brep_kernel`, `inspect`,
  `py_bindings`). `bool_ops`/`mesher` keep their pre-existing baseline drift
  (diff count unchanged: 48 and 114).
- `clippy`: no new warnings; remaining warnings are pre-existing.

## Surprises

- The plan estimated ~98 `.coedges` convenience-field accesses to migrate. Most
  were `Loop.coedges` (defining, kept) or arena iteration. **No production code
  read any of the removed up-refs** — only the compiler *wrote* `Edge.coedges`.
  The migration was far smaller than feared.
- A real scale-invariance bug surfaced in the revolution on-axis test. Sizing
  `ref_scale` from the knots alone gave a semicircle endpoint `u = cos(π/2) ≈
  6e-17` a threshold of `1e-22`, so it looked off-axis and the topology changed.
  Fixed by using the profile bbox diagonal as `ref_scale`. Exactly the call-site
  discipline 0-c.Q2 anticipates.
- `build_extrusion_contour_skeleton` carried an unused `tol` parameter; removed.
- `py_bindings.rs` sits behind the `extension-module` feature, so `cargo test`
  did not catch its rename in commit 1; `cargo check --features extension-module`
  did, and commit 1 was amended.

## Spend

**$0.91** of the $10.00 cap.

## Wiki proposal (NOT written — unsupervised runs do not touch the wiki)

A proposal would have touched, under `~/wiki/projects/jefscad/`:

- `JefSCAD-Tolerance-Model.md`: status "design" → "implemented"; record that
  `abs_floor` is a minimum reference scale, and the two call-site `ref_scale`
  choices (contour extent for the extrusion closure check; profile bbox diagonal
  for the revolution on-axis test).
- `JefSCAD-Roadmap.md`: mark 0-c complete.
- `Boundary-Representation-BRep.md`: document the new `Context` side-table API
  (`rebuild_indices`, `*_of_*` accessors) and the defining-vs-convenience split
  as landed.

Left for a human or a wiki-sync session.
