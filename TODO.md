# Phase 0 — Foundation refactor (TODO / draft plan)

Detailed actionable breakdown of Phase 0, split into three sub-phases executed in order:
**0-a** (drop flint) → **0-b** (Path2D contour-set + beziers + ruled surface) → **0-c**
(b-rep defining-only structs + Context side-tables + relative tolerance).

Ordering rationale: 0-a is a cheap, mechanical unblock; 0-b delivers the path/surface
gap the dice goal needs and is largely confined to `geom.rs` + the `build_extrusion`
path; 0-c is the widest, riskiest refactor and is deliberately last so its design can
be iterated on while 0-a/0-b are in flight. 0-c's struct refactor will re-touch
`build_extrusion` for the *field/side-table mechanic only* — the contour-set logic is
fully landed in 0-b so 0-c does not re-do it.

References: `architecture.md`, `boolean-ops.md`, `roadmap.md`, `jefscad.md`.

---

## 0-a — Remove the `flint` dependency from CSG-AST authoring

Goal: jefscad no longer depends on the `flint` crate. Two distinct flint usages exist
today and are handled separately.

### Decisions (locked)
- Replace the `[f64; 16]` interval matrix wrapper with a lightweight **`Mat4` newtype**
  (idiomatic Rust, type-system self-documentation). Lives in a new **`jefscad/src/linalg.rs`**
  (not `geom.rs` — it's linear algebra, not geometry).
- **Delete `predicates.rs` entirely.** It is dead code (`classify_node` has no callers)
  representing the deliberately-abandoned pervasive-interval-arithmetic direction.
  Shewchuk-style adaptive exact predicates will be added later (likely an existing Rust
  port/rewrite) when a real model breaks classification — bookmarked, not built now.
  Preserve the `mat4_inv_f64` helper (currently private to `predicates.rs`) by moving a
  plain-f64 version onto `Mat4` before deleting the file — it's a generally useful
  primitive and the only place inverse-transform logic currently lives.
- Remove `flint` from `jefscad/Cargo.toml`. The `flint/` crate stays in the workspace as
  a standalone, no-longer-depended-on crate (housekeeping: optionally delete it outright
  later; not part of 0-a scope).

### API decision (locked this session)
Resolved the open questions on the `Mat4` interface before implementing:

- **`apply_pt`/`apply_vec` take and return `Point3`** (not `[f64; 3]`). Lean on the
  type system to prevent point/vector confusion. `linalg → geom` is a one-directional,
  harmless dependency (`Point3` is a plain `{x,y,z}` struct with no back-edge).
- **`mat_mul(&self, &Mat4) -> Mat4` takes by reference**, not by value. 16 × 64-bit =
  128 bytes is clearly over the copy-by-value threshold. (Rule of thumb recorded: take
  by ref for arrays ≥ ~4 × f64; revisit line per-type as they come up.)
- **`inverse()` panics on singular** (engine-built affine transforms are always
  invertible; a singular matrix is a programmer error, not a runtime condition).
  *Note in the doc-comment:* revisit as `Result`/`Option` when STEP import of arbitrary
  external matrices lands — those can legitimately be singular.
- **`is_identity()` uses the quantize-and-compare-to-identity pattern** (not
  `fuzzy_eq`/`Tolerance` — pulling that in would force the tolerance model in 0-a, which
  0-c owns). **Two quantize scales, both the quantize pattern, labeled by purpose:**
  - `QUANTIZE_SCALE = 1e6` (status quo) for geom-id hashing — deliberately coarse to
    canonicalize near-identical matrices to the same id.
  - `IDENTITY_QUANT_SCALE = 1e12` (new) for the identity test — tight enough that a
    real sub-micron translation (e.g. a user's sliver-avoidance offset, motivation
    #3) is *not* swallowed as "no transform". The 1e6 hash scale is too coarse for
    this purpose; reusing it for identity would be a correctness bug.
  Both scales are named constants with comments explaining their distinct jobs.

### SIMD recommendation (decision: defer)
Build `Mat4` as plain `[f64; 16]` now. Do **not** design for SIMD in 0-a. Reasoning:
the call-site profile (handful of `mat_mul`s during CSG-tree build, one
`apply_pt`/`apply_vec` pass per `compile_primitive`) is not hot-loop; 4×4 matmul is
~64 flops and not the bottleneck even in the boolean/mesher paths (geometry
classification — ray-surface, PIS — dominates). f64-wide SIMD on x86 is awkward
(AVX-512 only for clean 4-wide; AVX2 gets 2-wide) and would re-import a nightly
feature gate — the exact dependency 0-a removes. The method-based interface
(`apply_pt`/`mat_mul`/etc., inner array private) means a future SIMD-backed `Mat4`
swaps in with zero call-site changes. Revisit with `cargo flamegraph` on a real model
*after* Phase 2 booleans work end-to-end — only if matrix math shows up as a hot spot
(it won't). `#[repr(transparent)]` is baked in for free so layout stays FFI-safe.

### Tasks
- [x] Create `jefscad/src/linalg.rs` with a `Mat4` newtype wrapping `[f64; 16]`
      (row-major, column-vector / right-multiply convention — matches current code).
      Members (final, see "API decision" above):
      - `pub const IDENTITY: Mat4`
      - `Mat4::from_array([f64; 16]) -> Mat4` (`const`)
      - `Mat4::mat_mul(&self, &Mat4) -> Mat4` (compose transforms: `self · rhs`)
      - `Mat4::apply_pt(&self, Point3) -> Point3` (w=1, linear + translation)
      - `Mat4::apply_vec(&self, Point3) -> Point3` (w=0, linear part only)
      - `Mat4::inverse(&self) -> Mat4` (panics on singular; ports `mat4_inv_f64` from
        `predicates.rs`; doc-notes a future `Result` split for STEP import)
      - `Mat4::as_array(&self) -> &[f64; 16]` (replaces the dropped `midpoint()`;
        documented as not-for-hot-loops so a future SIMD swap is zero-cost)
      - `Mat4::is_identity(&self) -> bool` — quantize-and-compare-to-identity at
        `IDENTITY_QUANT_SCALE = 1e12` (distinct from the 1e6 hash scale; see API
        decision). 19 unit tests in `linalg.rs`.
      - Constructors `mat_translation`/`mat_scale`/`mat_rot_aa` (free functions,
        ported verbatim from `csg_lang` so behaviour is unchanged on swap-in).
- [x] `jefscad/src/lib.rs`: add `pub mod linalg;`.
- [x] `jefscad/src/csg_lang.rs`:
      - Replace `use flint::{FlintArray, IDENTITY_4X4};` with the new `Mat4`.
      - `CsgNode::flat_transform: FlintArray<f64, 16>` → `Mat4`.
      - `mat_translation`/`mat_scale`/`mat_rot_aa` return `Mat4` (build via `from_array`).
      - `with_transform(... mat: Mat4)`: `self.flat_transform.mat_mul(&mat)`.
      - `quantize_matrix`/`is_identity_transform`: take `&Mat4`, read inner array.
        (Note: `quantize_matrix` currently reads `.lb`; with plain f64 read the array
        directly — for engine-built transforms lb==ub==value so semantics unchanged.)
      - Update all tests using `FlintArray::from_f64(IDENTITY)` → `Mat4::IDENTITY` /
        `Mat4::from_array(...)`.
      *(Done in commit 40bfde2; also collapsed `is_identity_transform` to delegate to
      `Mat4::is_identity`, and re-exported the `mat_*` constructors from `linalg`
      instead of duplicating them.)*
- [x] `jefscad/src/brep_compiler.rs`:
      - Replace `use flint::{FlintArray, IDENTITY_4X4};`.
      - `compile_primitive`'s `transform: &FlintArray<f64, 16>` → `&Mat4`.
      - Replace `transform.midpoint()` with `transform.as_array()` (or inline field
        access) in the linear-part extraction.
      - `is_identity(transform)` → `transform.is_identity()`.
      - Update the `IDENTITY_4X4` test fixture at ~line 2102/2108 to `Mat4::IDENTITY`
        / `Mat4::from_array(transform)`.
      *(Done in commit 40bfde2; local `is_identity` wrapper deleted in favour of the
      `Mat4` method.)*
- [x] `jefscad/src/predicates.rs`: **delete the file**; remove `mod predicates;` and
      `pub mod predicates;` from `lib.rs`. (`mat4_inv_f64` moved onto `Mat4::inverse`.
      File was genuinely dead — `classify_node`/`Classification` had no callers; the
      abandoned pervasive-interval direction.)
- [x] `jefscad/Cargo.toml`: remove `flint = { path = "../flint" }`.
- [x] `cargo test` clean on **stable Rust** (jefscad no longer needs `+nightly` —
      flint was the only nightly-feature consumer). Existing primitive + csg_lang
      tests are the safety net and stayed green throughout.
- [ ] `cargo fmt` — **deferred**. The codebase has pre-existing rustfmt drift across
      many files (predating this work); running `cargo fmt` reformats the whole crate
      and buries the migration edits. Tracked as a separate housekeeping commit after
      the flint crate is spun out (the flint files would otherwise be reformatted
      pointlessly).

### Out of scope for 0-a
- Shewchuk predicates (future, on-demand).
- Any tolerance-model change (that's 0-c).
- Deleting the `flint/` crate itself.

---

## 0-b — Path2D contour-set + beziers + ruled surface

Goal: a Path2D that can express font-glyph-shaped profiles (holes via inner contours,
disconnected sub-glyphs via multiple outer contours), with quadratic + cubic bezier
segment types, compiled through a ruled (`LinearExtrusionSurface`) extrusion.

### Decisions (locked)
- **H3 model**: `Path2D` = a *contour set*, one object expressing holes + disconnected
  parts. Extrusion of one `Path2D` produces a `SolidSet` (multiple `Solid`s for
  disconnected outers; inner cap `EdgeLoop`s for holes). Hole-ness comes from contour
  structure, **not** from CSG boolean ops.
- **Naming: `Path2D` (outer, role-named, author-facing) contains `Vec<Contour>`**
  (inner, structurally-named, mirrors `Solid`/`EdgeLoop`). The b-rep-style parallelism
  lives in the *element* name (`Contour` ↔ `Solid`) — `Path2D` describes the author's
  role ("I'm building a 2D path"), while `ContourSet` as the outer name would describe
  an implementation detail. 3D uses `SolidSet`/`Solid` (set-named outer) because the
  multi-solid result is user-visible (selection operator, booleans); 2D uses
  `Path2D`/`Contour` because the multi-contour structure is an authoring convenience,
  not first-class. Asymmetry recorded as deliberate.
- **No tolerance in the builder.** Validation splits:
  - *Builder-enforced (tolerance-free, exact):* segment chaining (by construction),
    `close()` exactness (`current_pos == start` bit-exact else error), degenerate
    segment rejection, **winding sign** (signed area, exact for non-degenerate
    polygons). Convention: **CCW outer / CW hole** (right-hand rule, matches STEP).
  - *Compile-enforced (needs Context tolerance, lands with 0-c / Phase 1):* fuzzy
    closure snap (end ≈ start within tol), self-intersection, geometric containment
    validation.
- **Winding check split (0-b.Q1 resolved):**
  - *Build-time* (`finish()`): per-contour **nonzero signed area** only. Catches
    degenerate contours (back-and-forth lines, self-cancelling figure-eights) cheaply
    and locally. This is the "capture errors early" Rust-philosophy check.
  - *Compile-time* (nesting in `build_extrusion`): the **role check** — CCW contour
    must be a top-level outer, CW contour must be inside a CCW outer. Cannot be done
    at build time because role depends on the *other* contours (circular dep).
    Reject on mismatch per the reject-don't-swap rule below.
- **Minimal `pip2d`** (point-in-polygon, single level) implemented in 0-b and used by
  `build_extrusion` to determine contour nesting via winding. Rejects malformed nesting
  (e.g. a CW contour not inside any CCW outer) as a compile error — delivers the
  "can't author a bad solid" guarantee. Full pip2d-with-holes (scanline, used by boolean
  classification) stays a Phase 1 deliverable; the minimal version is standalone and
  testable, and either feeds into or is replaced by the full one (no wasted work).
- **Winding: reject, do not auto-swap.** ttf-parser documents correct winding, so
  rejection rarely bites. Auto-swap is contour-role-aware (must not "fix" a CW hole into
  CCW) and therefore needs nesting → defer to Phase 1+.
- **Ruled surface extended to bezier profiles** in 0-b — clean vertical slice
  (path → lift → surface → eval), no topo dependency, well-placed. No blocking dep on
  0-c.
- `build_extrusion` is **fully rewritten for contour-sets in 0-b** (multi-contour →
  multi-Solid; holes → inner cap loops) so 0-c only re-touches it for the
  field/side-table mechanic, not the contour logic.

### Builder API (0-b.Q2 resolved)
- **Drop `Path2D::start(p)`.** `Path2D::new()` creates the empty contour set; the
  first `start_contour(p)` opens contour 0. No special "first contour" path — every
  contour opens the same way. (`Path2D::start` only made sense when a path was a single
  contour; the multi-contour model contradicts its "one start" implication.)
- **`start_contour(p: Point2) -> Result<&mut Self, PathError>`** opens a new contour.
  **Strict, not silent:** errors with `PathError::UnclosedContour` if the previous contour
  is open (non-empty + not closed). This catches the "forgot to close" omission at the
  call site where it's made (Rust philosophy, per Q1), rather than downstream.
  - *Why not `move_to` (SVG convention):* SVG auto-closes-on-`move_to` is fine for
    rendering (fill rules) but violates our bit-exact-close invariant. The strict
    `move_to` variant is the contender, but the name carries the silent-close baggage
    authors may misremember. `start_contour`/`close` is an unambiguous verb pair.
  - *Font-pipeline fit:* `ttf-parser` emits `move_to` events; the Phase-5 Rust font
    binding shims that event to `start_contour` internally. Machine consumer adapts;
    the author-facing API optimizes for authors.
- **`close() -> Result<&mut Self, PathError>` / `line_to_close() -> Result<&mut Self,
  PathError>`** close the current contour (see Q3 below).
- **`line_to`/`arc_to`/`quad_to`/`cubic_to` are infallible (`&mut Self`)** — they
  append to the current contour and panic on programmer error (no open contour),
  which is the "unreachable in correct usage" case. This split is self-documenting:
  fallible methods (`start_contour`, `close`, `line_to_close`, `finish`) carry real
  preconditions (structural state + geometric exactness); infallible appends only
  fail on a misuse that `finish()` could never recover from anyway (0-b.Q4 resolved:
  option 2 — pragmatic fallibility split).
- **`finish() -> Result<Path2D, PathError>`** consumes the builder, runs structural +
  winding validation.
- Full builder API:
  ```rust
  Path2D::new()                                              // empty contour set
      .start_contour(p: Point2) -> Result<&mut Self, PathError>  // open (errors if prev unclosed)
      .line_to(end)             -> &mut Self                  // infallible append
      .arc_to(center, sweep)    -> &mut Self
      .quad_to(c1, end)         -> &mut Self                  // (0-b adds the curve type)
      .cubic_to(c1, c2, end)    -> &mut Self                  // (0-b adds the curve type)
      .close()                  -> Result<&mut Self, PathError>  // mark closed (errors if current_pos != start)
      .line_to_close()          -> Result<&mut Self, PathError>  // append final Line2 to start, then close
      .finish()                 -> Result<Path2D, PathError>      // consume + validate
  ```

### Close-method family (0-b.Q3 resolved)
- **Keep `line_to_close`.** It's load-bearing ergonomics for the 90% case: the author
  built up the curve chain but the last point isn't bit-exactly at `start`, and lines
  have no control points to think about — delegating the final segment is unambiguous.
  Stays exact because the explicit `Line2` makes `current_pos == start` hold for the
  subsequent `close()` check.
- **Do NOT add** `arc_to_close` / `quad_to_close` / `cubic_to_close`. With bezier
  closes, the author is *already specifying* the endpoint as a parameter — at which
  point `end = start` is just `quad_to(start)` + `close()`, and the `_to_close`
  variant saves one call while hiding tangent-matching the author should own (a smooth
  close wants the closing segment's incoming tangent matched to the first segment's
  start tangent — a real geometric computation, not something to bury in a "close"
  method). Document the pattern instead: *to close smoothly, call `*_to(start)` with
  control points chosen for tangent continuity, then `close()`.*

### Tasks — data model & builder
- [x] `geom.rs`: redefine
      `Path2D { contours: Vec<Contour> }`, `Contour { start: Point2, segments: Vec<Curve2Kind>, closed: bool }`.
      `Contour` carries a private `current_pos` during building (decided: private field,
      O(1) append, mirrors the pre-refactor approach).
- [x] Builder API on `Path2D` per the locked "Builder API" decision above
      (0-b.Q4 resolved: option 2 — pragmatic fallibility split):
      - `new()` (empty set, infallible).
      - `start_contour(p) -> Result<&mut Self, PathError>` — open; errors
        `UnclosedContour` if previous contour is open.
      - `line_to`/`arc_to`/`quad_to`/`cubic_to` — infallible `&mut Self` appends;
        panic on programmer error (no open contour) rather than return `Result`.
        *(quad_to/cubic_to deferred to the bezier-segment cycle — the curve types
        don't exist yet.)*
      - `close() -> Result<&mut Self, PathError>` — errors `NoOpenContour` (no open
        contour) or `CloseNotAtStart` (`current_pos != start`, bit-exact).
      - `line_to_close() -> Result<&mut Self, PathError>` — append final `Line2` to
        `start`, then close; same errors as `close()` (plus `NoOpenContour`).
      - `finish() -> Result<Path2D, PathError>` — consume + validate.
      No `Path2D::start(p)` alias. No `move_to`. No `arc_to_close`/`quad_to_close`/
      `cubic_to_close`. Added a public `Curve2Kind::end()` method (replaces the
      private `curve2_end` free fn in `brep_compiler.rs`).
- [x] `finish()` validation (build-time, tolerance-free, exact):
      * every contour non-empty → else `EmptyContour`,
      * every contour closed (no open contour left at finish) → else `UnclosedContour`,
      * every closed contour has `current_pos == start` (bit-exact) → else
        `CloseNotAtStart`,
      * no degenerate segments → else `DegenerateSegment`,
      * each contour has **nonzero signed area** → else `ZeroAreaContour`.
      (Role check — CCW outer / CW hole — is *compile-time*, not here.)
      *(Also: `ExtrusionError::GeometricallyOpen` is now unreachable through the
      builder because `close()` is bit-exact; kept as defense-in-depth + the 0-c
      fuzzy-closure hook. The `extrude_err_geometrically_open` test was dropped —
      the state can't be constructed via the public API.)*
- [x] `PathError` enum: `NoOpenContour`, `UnclosedContour`, `EmptyContour`,
      `DegenerateSegment`, `ZeroAreaContour`, `CloseNotAtStart`, plus compile-time
      variants (`WindingRoleMismatch`, `HoleOutsideOuter`,
      `SelfIntersection`(future)). `Display` + `std::error::Error` impls.

### Tasks — bezier segment types
- [ ] `geom.rs`: add
      `QuadraticBezier2 { p0, p1, p2: Point2, t_min, t_max }` and
      `CubicBezier2 { p0, p1, p2, p3: Point2, t_min, t_max }`, both `t ∈ [0,1]`
      (matches the edge t-parameter convention in `boolean-ops.md`).
- [ ] Implement `Curve2` for both (`eval`, `eval_dt`, `is_degenerate`).
- [ ] Add variants to `Curve2Kind` and arms in:
      `curve2_end`, `curve2_t_range`, `lift_curve2`, `lift_xz_curve2`, `Path2D::Display`.
- [ ] Add `Bezier3` (`QuadraticBezier3`/`CubicBezier3`) to `Curve3Kind` + the lift arms,
      or lift beziers directly into existing 3D bezier structs (decide: a 3D bezier
      struct pair is cleaner and needed for ruled-surface profile curves anyway).

### Tasks — minimal pip2d
- [ ] New module `jefscad/src/pip2d.rs` (or fold into `geom.rs` — lean separate module,
      it's a standalone primitive per the roadmap's de-risking ethos).
      `pip2d(point: Point2, outer: &[Point2] /* polygon vertices in order */) -> bool`
      using even-odd ray casting (or scanline). Single level (no holes-within-holes
      recursion) — sufficient for nesting via winding.
- [ ] Inline tests on hand-built polygons (square, triangle, concave, point-on-edge
      behavior documented).

### Tasks — extrusion compile (contour-set)
- [ ] `build_extrusion` rewrite in `brep_compiler.rs`:
      - Input `path: &Path2D` (contour set).
      - **Nest contours**: for each contour, pick a representative point (e.g. first
        vertex), run `pip2d` against every other contour to build a containment tree.
        Outers = top-level CCW contours → each becomes one `Solid`. Holes = CW
        contours → assigned as inner cap loops to the nearest enclosing `Solid`'s cap
        `Face`s.
      - **Reject malformed**: a CW contour not inside any CCW outer →
        `ExtrusionError::HoleOutsideOuter`; a contour whose winding sign doesn't match
        its nesting role → `ExtrusionError::WindingRoleMismatch`.
      - Per `Solid`: outer contour → lateral faces (one `LinearExtrusionSurface` per
        segment) + bottom/top cap `Plane` faces; each hole contour → inner `Loop` on
        both caps.
      - Multi-outer → push multiple `Solid`s into one `NodeBRep` (the SolidSet).
      - Keep the existing `ExtrusionError` variants; add the new ones above.
- [ ] Extend `LinearExtrusionSurface` to accept bezier profile segments (the surface
      eval already delegates to the profile `Curve2`/`Curve3`; ensure the profile curve
      type carries beziers and eval/deriv are correct). Add tests exercising a
      quadratic-bezier extrusion end-to-end (path → solid → mesh or struct count).
- [ ] Update `py_bindings.rs` `PyPath2D` to the new builder API
      (`start_contour`/`quad_to`/`cubic_to`/etc.); update docstrings; regenerate stubs.

### Open questions
(none — all 0-b open questions resolved: Q1 winding split, Q2 builder API,
Q3 close family, Q4 fallibility split.)

### Out of scope for 0-b
- Fuzzy closure snap + self-intersection (need Context tolerance → 0-c / Phase 1).
- Full pip2d-with-holes (Phase 1).
- Auto-swap winding (Phase 1+).
- `Revolve`/rotate-surface bezier extension (roadmap: completeness, not dice-critical;
  could slip).
- The 0-c struct refactor (build_extrusion keeps current struct field access for now;
  0-c swaps it for side-table lookups).

---

## 0-c — b-rep defining-only structs + Context side-tables + relative tolerance

Goal: rearchitect the b-rep to the `architecture.md` model — structs carry *only*
defining (immutable) fields; convenience refs live in `Context` side-tables rebuilt by
`rebuild_indices()`; tolerance becomes global *relative* + absolute floor against local
feature size. `bool_ops.rs` and `mesher.rs` will be substantially rewritten later
anyway (justified by lessons learned since first implementation), so the aggressive
wide refactor is less costly than it appears — most convenience-field consumers will be
rewritten, not patched.

### Decisions (locked)
- **Keep index-into-`Vec` IDs** (no generation handles / slab) for now. Working-copy
  booleans build a fresh `Context`, so "abandon old structs" = "leave the old Context
  alone"; within-Context abandonment is not needed for the dice goal. Revisit only if
  Phase 2+ booleans demand it.
- **Aggressive, big-bang side-table refactor**: every struct loses convenience fields;
  every `face.shell` / `shell.faces` / `edge.coedges` / `loop.face` / `coedge.face` /
  `solid.outer`+`inners` traversal becomes a `Context` lookup. Gated behind one
  PR-sized commit with the existing primitive tests as the safety net. Aggressive is
  justified because bool_ops + mesher are slated for rewrite anyway.
- **Relative tolerance + absolute floor, call-site feature size.** Replace
  `KernelTolerance { pos_tol, ang_tol, param_tol }` with
  `Tolerance { eps_rel, abs_floor }` and a `fuzzy_eq(a, b, ref_scale)` (free function
  taking `&Tolerance`, or method on Context). `ref_scale` passed explicitly at each
  call site (edge length, bbox diagonal, etc.) — more honest, verbosity is
  engine-internal, not user-facing.
- **`ProvenanceData` / `CsgSource` / `attr` stay as defining fields on `Face`**
  (provenance is identity, not derivable). Leave them alone.
- **`Vertex { point, tol }` → `Vertex { point }`** (drop per-vertex tolerance — the
  OCCT-style per-entity tolerance `architecture.md` explicitly rejects).

### Defining vs convenience mapping (target, from architecture.md)
| Struct | Defining (kept in struct) | Convenience (→ Context side-table) |
|---|---|---|
| SolidSet | down: Solids | — |
| Solid | down: outer shell + void shells | up: SolidSet |
| Shell | down: (face, sense) list | down-cache: edges, vertices; up: Solid |
| Face | down: outer loop + inner loops + surface + sense + prov (+attr) | up: Shell |
| EdgeLoop | down: ordered Coedges | up: Face |
| Coedge | down: edge + orientation + pcurve | up: Face |
| Edge | down: start v + end v + curve + t-range | up: Coedges, Faces |
| Vertex | down: point | up: Shell, Faces, Edges |

**Wrinkle to preserve**: Shell's down edge/vertex list is *convenience* (derived from
faces), not defining — a shell is fully defined by its faces+senses.

### Tasks — structs
- [ ] **Rename `NodeBRep` → `SolidSet`** (lands *first* in 0-c, before the
      side-table migration, so all subsequent work uses the final vocabulary).
      Mechanical find-replace across `brep_kernel.rs`, `brep_compiler.rs`,
      `mesher.rs`, `bool_ops.rs`, `py_bindings.rs`: `NodeBRep` → `SolidSet`,
      `NodeBRepId` → `SolidSetId`, `push_node`/`get_node`/`get_mut_node` →
      `push_solidset`/`get_solidset`/`get_mut_solidset` (and the `nodes` arena
      field → `solidsets`). Primitive tests are the safety net and stay green.
      *Design note:* the struct becomes `SolidSet { solids: Vec<SolidId>,
      source_csg_id: u64 }` — `source_csg_id` is kept as a **defining** field
      (provenance is identity, per the same rule that keeps `CsgSource`/`attr`
      on `Face`). This aligns the code with `architecture.md`'s data-model
      diagram, which already names the top-level struct `SolidSet`.
- [ ] `brep_kernel.rs`: strip convenience fields:
      - `Face`: drop `shell` (→ `Context::shell_of_face`).
      - `Shell`: drop `solid` (→ `Context::solid_of_shell`); keep `faces` as defining
        *or* move to side-table per the wrinkle. **Decision**: the wrinkle says
        shell's *edges/vertices* are convenience, but `faces` is defining (a shell is
        defined by its faces+senses). So keep `Shell { faces: Vec<FaceId>, is_outer }`
        as defining; add `Context::edges_of_shell`/`vertices_of_shell` side-tables
        derived from faces. *Confirm — see 0-c.Q1.*
      - `Edge`: drop `coedges` (→ `Context::coedges_of_edge`).
      - `CoEdge`: drop `face` (→ `Context::face_of_coedge`).
      - `Loop`: drop `face` (→ `Context::face_of_loop`); keep `coedges`, `is_outer`.
      - `Solid`: keep `outer`, `inners` (defining); add up-ref side-table
        `Context::solidset_of_solid` (or skip — SolidSet already lists its solids;
        decide if the up-ref is worth it).
      - `Vertex`: drop `tol`; keep `point`.
- [ ] Add `Context` side-tables (start with `HashMap<K, V>`, revisit if hot):
      `faces_by_shell` (if faces move off Shell — see 0-c.Q1), `edges_by_shell`,
      `vertices_by_shell`, `coedges_by_edge`, `face_of_coedge`, `face_of_loop`,
      `shell_of_face`, `solid_of_shell`, etc.
- [ ] `Context::rebuild_indices(&mut self)` — single method that walks defining
      content and (re)populates all side-tables. Called once after every
      topological-mutating op. Document: side-tables may be stale *during* an op;
      ops that need them must call `rebuild_indices` first or read defining content
      directly.
- [ ] Audit `push_*`/`get_*`/`get_mut_*` macro — still fine for defining storage.

### Tasks — tolerance
- [ ] `brep_kernel.rs`: replace `KernelTolerance` with
      `Tolerance { eps_rel: f64, abs_floor: f64 }` (defaults `eps_rel ~ 1e-10..1e-12`,
      `abs_floor ~ 1e-12`; tune).
- [ ] `fuzzy_eq(a: f64, b: f64, ref_scale: f64, tol: &Tolerance) -> bool`:
      `|a - b| <= tol.eps_rel * ref_scale.max(tol.abs_floor)`. (Decide: is the
      `abs_floor` a floor on `ref_scale` or an absolute on `|a-b|`? Per architecture
      doc it's a floor so values near zero still snap — `ref_scale.max(abs_floor)` in
      the multiplier. *Confirm — see 0-c.Q2.*)
- [ ] Replace all current `ctx.tolerance.pos_tol` / `ang_tol` / `param_tol` call sites
      with explicit `ref_scale` calls. Catalogue call sites first (grep
      `tolerance.pos_tol` etc.).
- [ ] `build_extrusion`'s geometrically-open check (`sqrt(dx²+dy²) > pos_tol`) →
      `fuzzy_eq(cp.u, start.u, ref_scale, ...) && fuzzy_eq(cp.v, start.v, ref_scale, ...)`
      with `ref_scale` = path bbox diagonal or segment length. (Lands here even though
      contour logic is from 0-b — this is the tolerance-mechanic touch, not contour
      logic.)
- [ ] Expose `tolerance` getter/setter to Python later (roadmap) — not required for
      0-c, leave a TODO.

### Tasks — consumer rewrites (aggressive)
- [ ] `brep_compiler.rs`: replace all convenience-field accesses
      (`.shell`, `.coedges`, `.face`, etc. — ~98 `.coedges`, ~82 `.faces`, ~43 `.outer`
      hits across compiler/mesher/bool_ops) with `Context` side-table lookups. Ensure
      `rebuild_indices()` is called after each primitive build.
- [ ] `mesher.rs`: same field-access → side-table migration. (Per the "mesher will be
      rewritten anyway" note: do the minimal migration now to keep it compiling; the
      real rewrite is a later phase. *Confirm scope — see 0-c.Q3.*)
- [ ] `bool_ops.rs`: same. (Likely mostly rewrite later; minimal migration now to
      keep green. *Confirm scope.*)
- [ ] All existing primitive b-rep tests must pass unchanged — they are the safety net.

### Open questions (resolve before implementing 0-c)
- **0-c.Q1** Does `Shell` keep `faces: Vec<FaceId>` as a defining field, or do faces
  move entirely to a `Context::faces_by_shell` side-table (so `Shell` carries only
  `is_outer` + maybe nothing else)? Architecture doc says "a shell is fully defined by
  its faces+senses" — which makes `faces` defining and *kept on the struct*. The
  wrinkle is only that shell's *edges/vertices* are convenience. Lean: keep `faces` on
  `Shell`; side-table only edges/vertices + up-refs. Confirm.
- **0-c.Q2** `fuzzy_eq` floor semantics: `|a-b| <= eps_rel * ref_scale.max(abs_floor)`
  (floor on the scale multiplier) vs `|a-b| <= max(eps_rel*ref_scale, abs_floor)`
  (absolute floor on the distance). Architecture doc writes the former. Confirm former.
- **0-c.Q3** Scope of mesher/bool_ops changes in 0-c: minimal field-access migration
  (keep compiling, defer real rewrite) vs full rewrite now. Lean: minimal migration
  now (the aggressive refactor is about the *struct/Context schema*, not about
  rewriting the algorithms); algorithm rewrites are separate later phases. Confirm.

### Out of scope for 0-c
- Generation-handle IDs / within-Context abandonment (revisit if Phase 2+ demands).
- bool_ops / mesher *algorithm* rewrites (schema migration only in 0-c).
- Python tolerance getter/setter (later).
- Exact predicates (future).

---

## Cross-cutting
- Every sub-phase ends with `cargo check` + `cargo test` green and `cargo fmt`.
  (Stable Rust, edition 2024 — no `+nightly` since 0-a; the former cross-cutting note
  referenced nightly and is updated here.)
- Validation channel (per roadmap): 2D paths via SVG export; 3D via STL/OBJ export
  (STL export is needed for Phase 5 anyway, reusable from Phase 2 onward). Add a
  throwaway SVG dump for Path2D in 0-b if it helps visualize glyph outlines — optional.
- No `kb/` modifications.
- Update `notes.md` / `flint/notes.md` checkboxes as work lands (per AGENTS.md).
