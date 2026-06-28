# TODO — current / next-session actionable items

Phase 0-a and 0-b are **complete** (see `CHANGELOG.md` for milestone
summaries and the locked design decisions; the decisions are also captured
in code doc-comments, which are the single source of truth per `AGENTS.md`).
This file now tracks only the remaining Phase 0 work: **0-c**.

For the design narrative behind 0-c, read `architecture/architecture.md`
(the b-rep data model, defining-vs-convenience, tolerance model) and
`architecture/boolean-ops.md` first; this file assumes that context.

References: `architecture.md`, `boolean-ops.md`, `ROADMAP.md`, `CHANGELOG.md`.

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
      *Also unblocks* the multi-outer extrusion case deferred from 0-b (see
      CHANGELOG §0-b "Known limitations"): once `compile_primitive`/
      `compile_csg_node` return a multi-solid handle, `build_extrusion` can
      push multiple `Solid`s for ≥2 top-level outers and lift
      `ExtrusionError::MultiContourNotSupported` into real multi-solid output.
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
      logic.) *Note:* with 0-b's bit-exact `close()`, this check is unreachable through
      the builder; it stays as defense-in-depth + the fuzzy-closure hook.
- [ ] Expose `tolerance` getter/setter to Python later (roadmap) — not required for
      0-c, leave a TODO.

### Tasks — consumer rewrites (aggressive)
- [ ] `brep_compiler.rs`: replace all convenience-field accesses
      (`.shell`, `.coedges`, `.face`, etc. — ~98 `.coedges`, ~82 `.faces`, ~43 `.outer`
      hits across compiler/mesher/bool_ops) with `Context` side-table lookups. Ensure
      `rebuild_indices()` is called after each primitive build.
- [ ] `mesher.rs`: same field-access → side-table migration. (Per the "mesher will be
      rewritten anyway" note: do the minimal migration now to keep it compiling; the
      real rewrite — including the lateral-surface / face-with-holes cap meshing gap
      noted in CHANGELOG §0-b — is a later phase. *Confirm scope — see 0-c.Q3.*)
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
  Stable Rust, edition 2024 (no `+nightly` — `flint` was the only nightly consumer
  and was spun out in 0-a).
- Validation channel (per roadmap): 2D paths via SVG export; 3D via STL/OBJ export
  (STL export is needed for Phase 5 anyway, reusable from Phase 2 onward).
- No `kb/` modifications (it's a read-only reference from this repo).
- Commit style + workflow with AI: see `AGENTS.md` (Mode A plan/brainstorm →
  `TODO.md` items; Mode B TDD cycle per item; `CHANGELOG.md` at milestones only).
