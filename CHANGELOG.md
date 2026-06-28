# Changelog

Curated milestone summaries, reverse-chronological. Coarser than `git log`
(don't duplicate per-commit detail here) — captures *what was built, key
decisions, and where things stand* at each milestone boundary. For the
actionable next steps see `TODO.md`; for the design narrative see
`architecture/`.

---

## 0-b — Path2D contour-set + beziers + ruled surface (complete)

Phase 0-b delivered the 2-D path/surface gap the dice goal needs: a `Path2D`
that can express font-glyph-shaped profiles (holes via inner contours) with
quadratic + cubic bezier segments, compiled through a ruled
(`LinearExtrusionSurface`) extrusion that supports holes. Four commits,
each one TDD cycle:

1. `207e499` — `Path2D` restructured from single-contour
   `{ start, segments, closed }` to a contour-set `{ contours: Vec<Contour> }`
   with a strict builder API.
2. `945df7d` — `QuadraticBezier2`/`CubicBezier2` (+ 3-D analogues), wired
   end-to-end through the path builder, curve enums, lift functions, hashing,
   transform absorption, and mesher sampling.
3. `6fbca0f` — minimal `pip2d` module (even-odd ray casting, analytic
   per-segment curve-ray intersection, deterministic retry).
4. `9ac1fe4` — `build_extrusion` rewritten for contour-set nesting
   (single-outer + holes → one `Solid` with inner cap `EdgeLoop`s).

### Key decisions (locked during 0-b, recorded in `TODO.md` and code docs)

- **Naming**: `Path2D` (outer, role-named, author-facing) contains
  `Vec<Contour>` (inner, structurally-named, mirrors `Solid`/`EdgeLoop`).
  The b-rep-style parallelism lives in the *element* name (`Contour` ↔
  `Solid`); the outer name describes the author's role, not the
  implementation. Asymmetry vs. 3-D's `SolidSet`/`Solid` is deliberate
  (3-D's multi-solid result is user-visible; 2-D's multi-contour structure
  is an authoring convenience).
- **Builder API** (Q2): drop `Path2D::start(p)`; `new()` creates the empty
  set; `start_contour(p)` opens each contour (strict — errors
  `UnclosedContour` if the previous is open). No `move_to` (SVG silent-close
  baggage; the Phase-5 font pipeline will shim `ttf-parser`'s `move_to`
  event to `start_contour` internally).
- **Close-method family** (Q3): keep `line_to_close` (load-bearing for the
  90% "didn't return exactly to start" case); do NOT add
  `arc_to_close`/`quad_to_close`/`cubic_to_close` (they duplicate the
  endpoint the author already specifies and hide tangent-matching the
  author should own).
- **Winding split** (Q1): build-time (`finish()`) enforces per-contour
  *nonzero signed area* only (the cheap local check); the *role* check
  (CCW outer / CW hole) is compile-time in `build_extrusion` (needs nesting,
  which depends on the other contours — circular dep, so can't be at build
  time). Reject on mismatch (don't auto-swap).
- **Fallibility split** (Q4, option 2): appends (`line_to`/`arc_to`/
  `quad_to`/`cubic_to`) are infallible `&mut Self` (panic on no-open-contour
  programmer error); structural ops (`start_contour`/`close`/
  `line_to_close`/`finish`) return `Result`. Self-documenting of where
  errors arise.
- **`pip2d` design**: input `&Contour` (analytic segments, no sampling —
  preserves "circle is a circle" at the topology level); even-odd rule
  (matches the committed Phase-1 full `pip2d`-with-holes); **deterministic
  fixed-sequence retry, no PRNG, no new crate dep** — a `PIP2D_RAY_DIRECTIONS:
  [Point2; 4]` compile-time constant; on *detected* degeneracy (tangent /
  endpoint-coincidence / point-on-curve) retry the next direction. Fully
  reproducible by construction. The degeneracies are characterizable, so
  randomness is the wrong tool (and a PRNG would need consistent seeding for
  reproducibility, adding fragility + a dependency for no benefit).
- **Extrusion nesting scope**: implement **single-outer + holes** (the
  dice-critical case — a glyph is one outer + holes); defer **multi-outer**
  (disconnected glyphs like "i") to 0-c, since it genuinely needs the
  multi-solid plumbing (single-`SolidId` signature can't represent multiple
  solids). That's the 0-c `NodeBRep → SolidSet` rename task.

### Bug fixed during 0-b

- `contour_signed_area` was rewritten as the exact per-segment Green's-theorem
  integral (`∫(x·y'−y·x')dt` with closed forms for line/quadratic/cubic/arc/
  polyline). The pre-contour-set chord-only shoelace *zeroed area* for any
  contour whose chord endpoints were collinear — including exactly the
  line+quadratic glyph profile the dice goal needs. The arc form also fixes
  undercounting for arcs subtending < 2π (latent, no test exercised it).

### Known limitations carried forward

- **Multi-outer extrusion** (≥2 top-level outers, or nesting depth ≥ 2
  island-in-hole) → `ExtrusionError::MultiContourNotSupported`. Lands with
  the 0-c `SolidSet` plumbing. The nesting/role validation still runs first,
  so malformed multi-contour paths get the specific `WindingRoleMismatch`/
  `HoleOutsideOuter` error.
- **Mesher is cap-only for extrusions/revolutions**: the mesher currently
  triangulates only the planar cap faces (a no-hole square prism meshes to
  8V/4T = 2 caps × 2 tris); lateral `Extrusion`/`Revolution` surfaces and
  face-with-holes caps aren't fully meshed yet. This is a *pre-existing*
  limitation, unchanged by 0-b — the b-rep `build_extrusion` produces is
  structurally correct (manifold edges, closed loops, inner cap loops
  present, all verified by Rust tests). The real mesher rewrite is 0-c/later
  (the TODO's 0-c scope explicitly says "minimal migration now to keep
  compiling; the real rewrite is a later phase").

### Test posture at end of 0-b

594 Rust + 49 Python, all green. The Rust suite includes: full primitive
b-rep structural tests (cuboid/cylinder/cone/sphere/extrusion/revolution),
`Path2D` builder + `finish` validation, bezier eval/deriv/degenerate,
`pip2d` (lines/arcs/beziers/concave/nested/degeneracy-retry), and the
contour-set nesting (entity counts, inner cap loops, manifold hole edges,
closed loop chains, the three error cases).

---

## 0-a — Remove the `flint` dependency; `Mat4` newtype (complete)

Phase 0-a was the foundation cleanup: jefscad no longer depends on the
`flint` crate (rounded floating-point interval arithmetic, nightly Rust).
The standalone `flint` crate was spun out to its own repo; `jefscad` is now
**stable Rust, edition 2024** (no `+nightly` anywhere).

- `jefscad/src/linalg.rs` added with a `Mat4` newtype wrapping `[f64; 16]`
  (row-major, column-vector / right-multiply). Members: `IDENTITY`,
  `from_array`, `mat_mul`, `apply_pt`/`apply_vec` (take/return `Point3`),
  `inverse` (panics on singular), `as_array`, `is_identity` (two distinct
  quantize scales: `QUANTIZE_SCALE=1e6` for geom-id hashing,
  `IDENTITY_QUANT_SCALE=1e12` for the identity test).
- `predicates.rs` **deleted** (dead code — the abandoned pervasive-interval
  direction). Its `mat4_inv_f64` helper moved onto `Mat4::inverse` first.
- `csg_lang.rs` and `brep_compiler.rs` switched from
  `FlintArray<f64, 16>` to `Mat4`.
- `flint` removed from `jefscad/Cargo.toml`.

Shewchuk-style adaptive exact predicates are bookmarked as future work
(add when a real model breaks classification), not built now.

---

## Next up: 0-c

The remaining Phase 0 work is **0-c** — the widest, riskiest refactor,
deliberately last. Goal: rearchitect the b-rep to the `architecture.md`
model — structs carry *only* defining (immutable) fields; convenience refs
live in `Context` side-tables rebuilt by `rebuild_indices()`; tolerance
becomes global *relative* + absolute floor against local feature size.

Three open questions to resolve before implementing 0-c (see `TODO.md`
§0-c Open questions): `Shell.faces` defining-vs-side-table, `fuzzy_eq` floor
semantics, and mesher/bool_ops rewrite scope. The first 0-c task is the
mechanical **`NodeBRep → SolidSet` rename** (lands before the side-table
migration so all subsequent work uses the final vocabulary) — and it
unblocks the multi-outer extrusion case deferred from 0-b.
