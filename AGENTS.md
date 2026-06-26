# AGENTS.md

Guidance for agentic coding assistants (AI helpers, automated tools) working in this
repository. Read this before making changes.

---

## Project Overview

`jefscad` is an early-stage personal project: a code-based solid-modeling language for
constructive solid geometry (CSG), written in Rust and exposed to Python via
maturin + PyO3. It targets three pain points with OpenSCAD:

1. **Delayed meshing.** Primitives are kept as an analytic boundary representation
   (b-rep) with exact surfaces until export time; a circle stays a circle (or becomes
   an exact ellipse) under affine transforms. Mesh smoothness is a *mesh-time*
   parameter, not a creation-time one.
2. **STEP export.** The b-rep maps to STEP's native analytic surface entities where
   possible (plane / cylinder / cone / sphere / torus / ruled / revolution), falling
   back to rational NURBS for the cases STEP has no native entity for.
3. **Tolerance-aware booleans.** Coincident faces are handled explicitly
   (aligned/anti-aligned normal rules); a global *relative* tolerance with an absolute
   floor snaps near-coincident entities cleanly so differencing coincident-thickness
   solids leaves no thin slivers.

CSG is authored in **Python** (a real programming language, not a custom DSL), compiled
to a b-rep `SolidSet` per tree node. The first concrete deliverable is custom D&D dice:
a Python program that takes a font + glyph and produces a 3D-printable die face with
the glyph boolean-subtracted from one cuboid face.

**Design narrative lives in `architecture/`** (the renamed `jefscad-plan/` directory):
`architecture.md` (b-rep structs, surface taxonomy, tolerance, struct lifecycle),
`boolean-ops.md` (the boolean pipeline — the riskiest piece), and the original
braindump. For the full *what & why*, read `architecture/index.md` first.

---

## Repository Layout (target state after the Phase 0 refactor)

```
repo root/
├── Cargo.toml               # workspace, member: jefscad
├── Cargo.lock
├── pyproject.toml           # maturin build config + pytest config
├── uv.lock
├── .venv/                   # UV-managed virtualenv (gitignored)
│
├── jefscad/                 # the Rust crate → compiled to jefscad._jefscad
│   ├── Cargo.toml           # edition 2024; pyo3 optional (extension-module feature)
│   └── src/
│       ├── lib.rs
│       ├── csg_lang.rs      # CSG AST types and constructors
│       ├── geom.rs          # 2D/3D geometry: paths, curves, surfaces
│       ├── linalg.rs        # Mat4 newtype (linear algebra, not geometry)
│       ├── brep_kernel.rs   # b-rep topological + geometric structs, Context
│       ├── brep_compiler.rs # CSG tree → b-rep SolidSet
│       ├── bool_ops.rs      # boolean union / difference / intersection
│       ├── mesher.rs        # b-rep → triangular mesh
│       ├── predicates.rs    # classification predicates (deleted in 0-a)
│       ├── py_bindings.rs   # pyo3 Python bindings
│       └── bin/stub_gen.rs  # generates python/jefscad/_jefscad/__init__.pyi
│
├── python/jefscad/          # thin pure-Python wrapper package
│   ├── __init__.py          # re-exports public API from ._jefscad
│   └── _jefscad/
│       └── __init__.pyi     # generated type stubs
│
├── architecture/           # design narrative — the "what & why" (read-only-ish)
│   ├── index.md
│   ├── architecture.md
│   ├── boolean-ops.md
│   └── jefscad.md           # original braindump / motivation
│
├── docs/                    # Sphinx user-facing HTML docs source
├── notebooks/               # Jupyter notebooks (interactive scratch)
│
├── ROADMAP.md               # long-horizon phased plan (the wellspring)
├── TODO.md                  # current / next-session actionable items (churns often)
├── CHANGELOG.md             # curated milestone summaries (reverse-chronological)
│
├── README.md                # short public summary / motivation
├── DEVELOPMENT.md           # environment setup + build/test/jupyter how-to
└── AGENTS.md                # this file
```

### Phase 0 in flight

The project is mid-refactor. Phase 0 (see `ROADMAP.md` → Phase 0, and
`architecture/` for the detailed breakdown) is the foundation refactor. Its first
sub-step **0-a** removes the `flint` dependency from this repo:

- The standalone `flint/` crate (rounded floating-point interval arithmetic, nightly
  Rust) is being **spun out to its own independent repository**. It stays buildable in
  isolation but is no longer a dependency of `jefscad`.
- `jefscad` itself uses **no nightly features**; once `flint` is removed the toolchain
  is **stable Rust, edition 2024** (no more `cargo +nightly`).
- `jefscad/src/predicates.rs` is dead code (the abandoned pervasive-interval direction)
  and is deleted in 0-a; its `mat4_inv_f64` helper is preserved as `Mat4::inverse` in a
  new `linalg.rs`.

Until 0-a lands, `cargo +nightly` is still required because `jefscad` still depends on
`flint`. After 0-a, drop the `+nightly` everywhere. The build commands below are written
for the **target (stable)** state; prefix `+nightly` only while the `flint` dependency
remains.

The `_AGENTS.md` file (the renamed former `AGENTS.md`) is the `flint`-specific guidance
being kept for the spin-out; it is **not** authoritative for this repo once 0-a lands.

---

## Build, Test, and Tooling Commands

Tools: **`uv`** (Python virtualenv + deps), **`maturin`** (Rust → Python extension),
**`cargo`** (Rust), **`pytest`** (Python tests), **`jupyterlab`** (notebooks).
Detailed setup is in `DEVELOPMENT.md`; the essentials:

### Daily development loop

```bash
source .venv/bin/activate          # once per shell session

# After editing Rust:
maturin develop --features extension-module   # recompile + reinstall the .so in place

# After editing pure-Python (python/jefscad/__init__.py):
# nothing — the install is editable, changes are live

# Run Rust unit tests (no Python linking required — extension-module feature is optional):
cargo test
# (pre-0-a only: cargo +nightly test)

# Run Python tests:
pytest -v
# single file:   pytest tests/test_nodes.py -v
# single test:   pytest tests/test_nodes.py::test_sphere_returns_node -v
```

### Check, lint, format, docs

```bash
cargo check                       # fast type-check, no linking
cargo clippy                      # lint
cargo fmt                         # format before committing (rustfmt defaults)

# Rust API docs:
cargo doc --no-deps --features extension-module

# Regenerate Python type stubs after any public-API change:
cargo run --bin stub_gen --features extension-module
# writes python/jefscad/_jefscad/__init__.pyi

# Sphinx user docs (one-time: uv pip install ".[docs]"):
sphinx-build -b html docs/ docs/_build/html/
```

### Key gotchas

- **`extension-module` is an optional pyo3 feature** in `jefscad/Cargo.toml`. This means
  `cargo test` works **without linking against Python at all** — Rust unit tests run
  standalone. Only `maturin develop` activates the feature.
- **No `rust-toolchain.toml`** in the repo. Pre-0-a you must use `cargo +nightly`
  (flint needs `portable_simd` / `macro_metavar_expr`). Post-0-a plain `cargo` on stable
  is correct.
- `.venv/` and the compiled `.so` are gitignored; every fresh clone needs the one-time
  setup in `DEVELOPMENT.md` (`uv venv`, `uv pip install ...`, `maturin develop`).

---

## Workflow with AI

This is the workflow AGENTS.md exists to support. Two modes, used in sequence within a
session.

### Mode A — Plan / brainstorm

Use AI to talk through a problem, work through design options, and produce a concrete,
actionable plan. The output of this mode is **`TODO.md`** updated (or created) with
checkbox items — each item commit-sized and individually testable. Before leaving Mode A:

- Decompose the work from `ROADMAP.md` into specific, actionable `TODO.md` items.
- Resolve open design questions *before* implementing (record decisions in `architecture/`
  if they change the design, or inline in `TODO.md` if they're scoped to the task).
- Each TODO item should be small enough that one TDD cycle (below) closes it.

### Mode B — TDD development cycle

For each `TODO.md` item, in this order:

1. **Decide the API specifics first.** Agree on struct layouts, trait definitions, and
   method signatures *before* writing tests or implementation. Capture the decision (in
   `architecture/` if it's a lasting design point, or in the commit if it's local).
2. **Write the tests** — split into two classes by intent:
   - **(a) Unit tests for *internal* interfaces.** These live inline in Rust
     (`#[cfg(test)] mod test { ... }` at the bottom of the source file) and as focused
     Python tests. They pin behaviour of a single module's internals. **These MAY be
     updated or changed during a refactor** — they are part of the implementation, not
     a contract.
   - **(b) Integration tests for *cross-module / public* interfaces.** These live in
     `tests/` (Python, pytest) and `jefscad/tests/` (Rust integration tests, when
     added). They pin the public contract between modules. **These MUST NOT be updated
     or changed during a refactor** — a refactor moves internals around while keeping
     these green. If a refactor genuinely needs to change one of these, that is a signal
     the public contract is changing and should be called out explicitly, not silently
     edited.
3. **Implement** the code until the tests pass.
4. **Run the full suite** (`cargo test` + `pytest -v`) to confirm nothing else broke.
5. **Commit once tests are green** (see commit style below). Check off the TODO item.

The point of the (a)/(b) split: a refactor is *allowed* to rewrite every unit test, and
*forbidden* from rewriting integration tests. Integration tests are the safety net that
makes an aggressive refactor safe.

---

## Planning and Progress Files

Three top-level files, split by time horizon:

| File | Horizon | Granularity | Churn |
|------|---------|-------------|-------|
| `ROADMAP.md` | long-term | phases toward full project completion | rarely |
| `TODO.md` | current / next session | commit-sized actionable items | often |
| `CHANGELOG.md` | history | **milestones** (phase completions, shipped features) | at milestones |

### `ROADMAP.md`

The long-horizon phased plan (migrated from `architecture/roadmap.md`). This is the
wellspring: Mode A decomposes items from here into `TODO.md`. Edit it when a phase's
shape changes, not when an individual task lands.

### `TODO.md`

The current and (optionally) next-session actionable items, each commit-sized and
individually testable. Created/updated during Mode A. Worked through (and checked off)
during Mode B. Use **standard Markdown checkboxes**:

```
- [ ] pending item
- [x] completed item
```

### `CHANGELOG.md` — milestones, not commits

`CHANGELOG.md` is **curated milestone summaries**, reverse-chronological — coarser than
commits. It is **not** a per-commit log (that is `git log`'s job, and duplicating it is
why CHANGELOGs get abandoned). Most commits do not touch `CHANGELOG.md`; only milestone
boundaries (a phase landing, a shipped feature) do.

### The session ritual

- **Session start (Mode A):** review leftover `TODO.md` items → read `ROADMAP.md` →
  decompose the next chunk of work into fresh `TODO.md` checkbox items.
- **During the session (Mode B):** TDD-cycle each TODO item; commit when green; check off
  the item in `TODO.md`.
- **Session end (milestone boundary):** when a milestone has been reached (e.g. a phase
  completes), write a `CHANGELOG.md` entry summarising what was built, and clear the
  completed items from `TODO.md`. If no milestone crossed, just leave `TODO.md` with its
  checkmarks for next session — no mandatory CHANGELOG churn every session.

---

## Commit Message Style

Format-agnostic and human-readable. Two parts:

1. **Subject line** — a short, general description that reads well from
   `git log --oneline`. Imperative mood preferred. No required prefix or convention
   (the existing `Phase N: ...` history is fine to continue or not — your call).
2. **Body** — a longer message detailing the changes: what was added/changed and *why*,
   notable design decisions, and (for tests) what the new tests cover. Wrap at a
   readable width.

Example shape:

```
Add Mat4 newtype and move matrix inverse out of predicates

Introduces jefscad/src/linalg.rs with a Mat4 newtype wrapping [f64;16]
(row-major, column-vector / right-multiply, matching the existing convention).
Members: IDENTITY, from_array, mat_mul, apply_pt, apply_vec, inverse, is_identity,
as_array.

mat4_inv_f64 is moved from predicates.rs onto Mat4::inverse (panic on singular,
preserving the existing logic) so the helper survives the predicates.rs deletion
in the next step.

csg_lang.rs and brep_compiler.rs switch from flint::FlintArray<f64,16> to Mat4;
.midpoint() calls become .as_array() since plain f64 has no interval.

6 unit tests: identity, compose, apply_pt vs apply_vec, inverse round-trip,
is_identity true/false, as_array round-trip. Existing primitive/csg_lang tests
unchanged and green.
```

**Couple CHANGELOG to milestones, not commits.** Only touch `CHANGELOG.md` when a
milestone lands; otherwise the commit message itself is the record of an atomic change.

---

## Code Style

### Rust

- Edition **2024**. Run `cargo fmt` before committing (rustfmt defaults; no
  `rustfmt.toml`).
- `///` doc-comments on all public items (types, traits, methods, macros, constants).
  These doc-comments are the **single source of truth** for documentation: PyO3 maps
  them to Python `__doc__` automatically, and `pyo3-stub-gen` renders them into the `.pyi`
  stubs. There is no separate Python docstring layer — keep the Rust `///` comments good
  and both layers stay correct.
- PascalCase for types/traits, snake_case for functions/methods, SCREAMING_SNAKE_CASE
  for constants. Domain abbreviations (`lb`, `ub`) are canonical where they apply.
- Keep trait bounds minimal — only what the body uses.
- Inline `#[cfg(test)] mod test { use super::*; ... }` for unit tests; name each test
  `test_<what_is_being_tested>`.

### Python

- The pure-Python layer (`python/jefscad/__init__.py`) is a thin re-export wrapper; real
  logic lives in Rust. Keep it thin.
- Python tests in `tests/` use pytest, plain `assert`, one logical behaviour per test.

---

## External Knowledge Base

A separate personal knowledge base lives at `~/kb` (outside this repo). It holds
JefSCAD wiki pages and notes (e.g. `~/kb/wiki/JefSCAD*.md`,
`~/kb/notes/jefscad-plan-*.md`). **Reference it when useful** for background and prior
planning context. **Do not write to it from this repo** — it is a read-only reference
from here.

---

## Things to Avoid

- Do not run `cargo build` / `cargo test` with `+nightly` once Phase 0-a has landed —
  `jefscad` is stable-Rust. (Pre-0-a, `+nightly` is still required because of `flint`.)
- Do not modify `architecture/` design narrative as a side-effect of an implementation
  task unless the task genuinely changes the design — then update it deliberately.
- Do not rewrite integration tests (cross-module / `tests/` / `jefscad/tests/`) during a
  refactor. If a refactor seems to require it, that is a contract change: call it out
  explicitly. Unit tests (inline `mod test`) are fair game to rewrite.
- Do not add `unwrap()` in user-visible library code; use `expect("reason")` or return
  `Option`/`Result`.
- Do not regenerate the `.pyi` stubs and forget to commit them when the public Python API
  changes — run `cargo run --bin stub_gen --features extension-module` and commit the
  resulting `python/jefscad/_jefscad/__init__.pyi` together with the API change.
- Do not duplicate every commit into `CHANGELOG.md` — CHANGELOG is milestones only.
