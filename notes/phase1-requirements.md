# Phase 1 requirements layout

> **Status: draft** (pair session, 2026-10-05). Working planning document for
> Phase 1 (standalone geometric primitives). Fills in the three-bucket
> inventory that drives the design: every geometric decision point is tagged
> **predicate-shaped**, **tolerance-shaped**, or **necessarily approximate**, so
> the deferred Shewchuk exact-predicate upgrade has a seam and we know which
> behaviour can change under it.
>
> Promote to `architecture/` once settled. Feeds the Phase 1 `goal.md`(s).

## 1. The three categories

Every geometric decision in a primitive is exactly one of:

- **Predicate-shaped** — a *discrete topology* question: "which side of this
  line/plane?"; "is this exactly degenerate in the stored coordinates?".
  Answered by a sign predicate (`orient2d`/`orient3d`), eventually exact.
  The predicate returns **only a sign** — never a magnitude. Never fuzzed by
  tolerance.
- **Tolerance-shaped** — a *continuous closeness* question: "is this point too
  close to the boundary to treat as on it?"; "is this hit grazing/tangent?".
  Answered by a distance/angle metric compared against `Tolerance` (global) and
  a locally-derived `ref_scale`. Computed **independently of the predicate**.
- **Necessarily approximate** — root-finding/algebra with no finite exact
  determinant (arc/bezier crossings, ray-vs-cylinder/ruled). Uses interval
  isolation/analytic solving plus the tolerance layer, and must still yield a
  *definite* answer by feeding a degeneracy/retry path.

### The rule (non-negotiable)

- **Never derive a tolerance check from a predicate.** A future exact predicate
  returns no usable magnitude; `Sign::Pos` is consistent with distance `1e-30`
  or `1e30`.
- **Never fuzz a predicate with a tolerance band.** Topology must be a clean
  sign; closeness is handled by *steering* (retry another probe) or *snapping*
  (declare on-boundary), never by blurring the sign.
- **Exact predicates do not replace tolerance.** An exact `orient2d` returns
  `Zero` only when the configuration is degenerate to the last bit of the stored
  f64s; a mathematically-degenerate case that computes to `1e-17` returns a
  nonzero sign. Near-degenerate geometry still needs the tolerance layer (or
  symbolic perturbation). Exactness buys *consistency* of sign decisions;
  tolerance buys *"too close to call."* They are complementary.

## 2. Predicate seam (`predicates` module)

Phase 1 delivers a small module so the sign decisions are isolated from the
start. Interface now; exact body later.

```rust
enum Sign { Neg, Zero, Pos }

fn sign_of(x: f64) -> Sign;                                  // sign of a scalar
fn orient2d(a: Point2, b: Point2, c: Point2) -> Sign;        // side of line ab
fn orient3d(a: Point3, b: Point3, c: Point3, d: Point3) -> Sign; // side of plane abc
```

- **Implementation now:** naive float determinant → `Sign`. `Zero` only when the
  determinant is exactly `0.0`. No error bound, no adaptive fallback.
- **Deferred upgrade:** filtered/adaptive exact (Shewchuk) behind the *same
  signature*. Because no caller reads a magnitude, the swap is local.
- **Target beyond that (not Phase 1):** symbolic perturbation (SoS), where the
  predicate never returns `Zero`, so degeneracy retry can be dropped for the
  linear cases.
- Which functions: only what Phase 1 consumes — `orient2d`, `orient3d`
  (`incircle`/`insphere` are later). `sign_of` is for `t > 0`-style tests.
- Visibility: `pub` within the crate is sufficient; decide `pub` vs `pub(crate)`
  when the module lands (no Python exposure needed).

## 3. Tolerance and `ref_scale` conventions

- `Tolerance { eps_rel, abs_floor }` is **global kernel state** — passed as one
  object (`&Tolerance`), never split into separate `eps`/`abs` arguments.
- `ref_scale` is **local feature size** — a property of the input geometry, not
  a kernel setting. It is **derived inside the primitive**, never added to
  `Tolerance` and never required from the caller.

| Primitive | `ref_scale` source |
|---|---|
| `pip2d` | contour-set bbox diagonal |
| ray-surface / `pis` | solid bbox diagonal |
| DCEL cycle enumeration | face uv-domain bbox diagonal |

`fuzzy_eq(a, b, ref_scale, &tol) = |a-b| <= eps_rel * max(ref_scale, abs_floor)`.
The floor is a minimum *reference scale* (option A); effective minimum distance
is `eps_rel * abs_floor`.

## 4. Per-primitive inventory

### 4.1 `pip2d` (full, with holes) — 2-D containment

Interface (locked in discussion):

```rust
// module pip2d
pub fn classify(point, contour_set, tol: &Tolerance) -> Containment;
pub fn contains(point, contour_set, tol: &Tolerance) -> bool;  // == Inside
enum Containment { Inside, Outside, OnBoundary }
```

- The trinary `classify` is primary; `contains` is the thin boolean wrapper
  (`matches!(.., Inside)`), preserving current on-boundary → `false` semantics.
- Internal `CrossResult::{Count(n), Degenerate}` stays **private**; `classify`
  retries and always resolves.
- Input is a contour-set (outer + holes), not a single `&Contour` — this is the
  Phase 1 extension of the 0-b minimal version.

| Decision point | Category | Predicate / metric |
|---|---|---|
| Which side of the ray line a curve point is on | predicate | `orient2d(P, P+D, Q)` sign |
| Crossing parity (even/odd) accumulation | topology | integer parity; no predicate beyond the sign |
| Ray is tangent to a segment/arc (grazing) | tolerance | `|cross(ray_dir, tangent)| <= tol·|ray_dir|·|tangent|` |
| Crossing coincides with a segment endpoint | tolerance | crossing parameter within `tol·ref_scale` of `t=0`/`t=n` |
| Point on boundary (for `OnBoundary`) | tolerance | nearest-point distance `<= tol·ref_scale` |
| Arc/bezier crossing parameter | approximate | analytic arc solve / quadratic-cubic root isolation |
| Curved on-boundary nearest point | approximate | closest-point root isolation within the tolerance band |

- Detection is currently the constant `DEGEN_EPS = 1e-12`; Phase 1 replaces it
  with `Tolerance` + derived `ref_scale`. This threads `ctx.tolerance` through
  `build_extrusion`'s nesting calls (the 0-b → 0-c leftover).
- Deterministic ray-direction retry stays (no PRNG). The `Count`/`Degenerate`
  branch is the single choke point a future SoS predicate could remove.

### 4.2 Ray-surface intersection — *(to fill)*

Three surface types, each independently testable:

- **Ray vs plane** — line/plane solve; parallel test is a sign predicate
  (`sign_of(dot(n, dir))`) with a tolerance band only for *near*-parallel
  steering; hit `t > 0` sign.
- **Ray vs cylinder** — quadratic in `t`; discriminant sign is predicate-shaped
  (exact `Zero` = tangent); root selection is `t > 0`; then `(u,v)` → trim via
  `pip2d` in the face's uv domain.
- **Ray vs ruled/extrusion surface** — solve `profile(s) + z·dir` against the
  ray; root isolation; trim via `pip2d`.

For each: list predicate / tolerance / approximate decisions, plus the per-hit
result (`Crossing(t)` vs `Tangent`/`Degenerate`) that the `pis` parity assembler
consumes.

### 4.3 `pis` — point-in-solid (trinary) — *(to fill)*

Contract: `pis(point, solid) -> Containment` (Inside/Outside/OnBoundary), a
single primitive with no wrapper. Assembled from ray-surface hits over the
solid's faces + odd-parity; on-boundary detected from surface distance **and**
face-domain containment. Fixed unlikely ray direction `(1, √2, √3)` + retry on
detected degeneracy. Classification callers assert no `OnBoundary`.

### 4.4 Selection operator — *(to fill)*

First end-to-end `pis` consumer: pick the solid of a `SolidSet` that contains a
point. Resolve the deferred questions: on-boundary point, and no-solid-contains
(error vs empty).

### 4.5 DCEL cycle enumeration — *(separate run)*

Not consumed by Phase 1; a Phase 2 prerequisite. First design question: reuse
`mesher::HalfEdgeMesh` or a new structure (the half-edge traversal is shared with
the mesher per `boolean-ops.md`). Inventory:

- next-around-vertex cyclic ordering by tangent angle → **predicate**
  (`orient2d` of tangents; tie-break for coincident angles = tolerance or SoS).
- outer-CCW / inner-CW cycle orientation → **predicate** (signed-area sign).
- twin/next traversal → pure topology, no predicate.
- coincident tangent angles at a vertex → tolerance (or SoS later).

## 5. Degeneracy / retry policy

- Every primitive has **one** choke point that decides "this probe is
  degenerate → retry/steer," expressed via the tolerance layer (magnitude-based),
  not via a predicate.
- `pip2d`/`pis` use deterministic direction retry. DCEL uses an angular
  tie-break. Record per primitive what happens when all probes are degenerate.

## 6. Exact-upgrade impact (what Shewchuk will and won't change)

- **Will** make `orient2d`/`orient3d` sign decisions mutually consistent for the
  stored f64s → removes roundoff-induced parity/orientation contradictions.
- **Will not** change: root-finding for curves; on-boundary/grazing detection;
  `ref_scale` choices; any tolerance metric. Those remain approximate/tolerance.

## 7. Open questions

- `predicates`: `pub` vs `pub(crate)`; module location; whether `Sign` should
  carry a `bool is_exact` for tests.
- Does the full `pip2d` take a contour-set type that already exists
  (`Path2D`/`Vec<Contour>`), or a dedicated `Loops`/`ContourSet` view?
- `Containment` shared between `pip2d` and `pis`, or two enums? (Shared is
  probably right; decide placement.)
- Run split: `predicates` + full `pip2d` as Run A; `ray-surface` + `pis` +
  selection as Run B; DCEL as its own run.
- Does `OnBoundary` detection run always, or only when requested (cost)?
