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

**Design narrative lives in `architecture/`**: `index.md` (start here for the full
what & why), `architecture.md` (b-rep structs, surface taxonomy, tolerance, struct
lifecycle), `boolean-ops.md` (the boolean pipeline — the riskiest piece), and
`jefscad.md` (original braindump / motivation).

---

## Modes of work

Mode switches are explicit — say "plan" or "implement" to change modes.

- **Planning mode**: read `~/tools/llm-instructions/planning.md` before starting.
  Produces 1–4 one-commit-sized items in `TODO.md` and triages `backlog.md`.
- **Implementation mode**: read `~/tools/llm-instructions/implementation.md` before
  starting. TDD loop with hard STOP gates requiring user approval.

### Project-specific workflow deltas

These modify the generic loop in the shared instruction files:

- **The (a)/(b) test split.** Tests come in two classes by intent:
  - **(a) Unit tests for *internal* interfaces.** Inline in Rust
    (`#[cfg(test)] mod test { ... }` at the bottom of the source file) and focused
    Python tests. They pin a single module's internals. **These MAY be updated or
    changed during a refactor** — they are part of the implementation, not a contract.
  - **(b) Integration tests for *cross-module / public* interfaces.** These live in
    `tests/` (Python, pytest) and `jefscad/tests/` (Rust integration tests, when
    added). They pin the public contract between modules. **These MUST NOT be updated
    or changed during a refactor** — a refactor moves internals around while keeping
    these green. If a refactor genuinely needs to change one of these, that is a
    signal the public contract is changing and must be called out explicitly, not
    silently edited.

  The point: a refactor is *allowed* to rewrite every unit test and *forbidden* from
  rewriting integration tests. The integration tests are the safety net that makes an
  aggressive refactor safe.

- **Planning consults `ROADMAP.md`.** The long-horizon phased plan is the wellspring:
  planning mode decomposes items from here into `TODO.md`. Edit it when a phase's
  shape changes, not when an individual task lands.
- **`CHANGELOG.md` is milestones, not commits.** Curated milestone summaries
  (reverse-chronological, coarser than commits) — a phase landing or a shipped
  feature, not every commit. Duplicating `git log` is why CHANGELOGs get abandoned.

---

## Planning and progress files

| File | Horizon | Granularity | Churn |
|------|---------|-------------|-------|
| `ROADMAP.md` | long-term | phases toward full project completion | rarely |
| `TODO.md` | current session | commit-sized actionable items (1–4) | every planning session |
| `backlog.md` | persistent | raw ideas, mid-session discoveries | triaged in planning mode |
| `CHANGELOG.md` | history | milestone summaries | at milestones |

`TODO.md` is session-scoped: rewritten at each planning session, cleared at the end
of a work chunk (see `~/tools/llm-instructions/wiki-sync.md` — the changelog entry
goes to `CHANGELOG.md` at milestones).

---

## Repository Layout

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
│       ├── py_bindings.rs   # pyo3 Python bindings
│       └── bin/stub_gen.rs  # generates python/jefscad/_jefscad/__init__.pyi
│
├── python/jefscad/          # thin pure-Python wrapper package
│   ├── __init__.py          # re-exports public API from ._jefscad
│   └── _jefscad/
│       └── __init__.pyi     # generated type stubs
│
├── architecture/            # design narrative — the "what & why"
│   ├── index.md
│   ├── architecture.md
│   ├── boolean-ops.md
│   └── jefscad.md           # original braindump / motivation
│
├── docs/                    # Sphinx user-facing HTML docs source
├── notebooks/               # Jupyter notebooks (interactive scratch)
├── tests/                   # Python integration tests (contract tier (b))
│
├── ROADMAP.md               # long-horizon phased plan (the wellspring)
├── TODO.md                  # session-scoped actionable items
├── backlog.md               # persistent idea parking lot
├── CHANGELOG.md             # curated milestone summaries (reverse-chronological)
│
├── README.md                # short public summary / motivation
├── DEVELOPMENT.md           # environment setup + build/test/jupyter how-to
└── AGENTS.md                # this file
```

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
- **No `rust-toolchain.toml`** in the repo. `jefscad` targets stable Rust, edition
  2024 — plain `cargo` everywhere.
- `.venv/` and the compiled `.so` are gitignored; every fresh clone needs the one-time
  setup in `DEVELOPMENT.md` (`uv venv`, `uv pip install ...`, `maturin develop`).

---

## Commit Message Style

Two parts:

1. **Subject line** — a short, general description that reads well from
   `git log --oneline`. Imperative mood preferred. No required prefix or convention
   (the existing `Phase N: ...` history is fine to continue or not — your call).
2. **Body** — a longer message detailing the changes: what was added/changed and *why*,
   notable design decisions, and (for tests) what the new tests cover. Wrap at a
   readable width.

Lasting design decisions go in `architecture/` (if they change the design) or the
commit body (if they're local to the task).

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

## Wiki

This project has an LLM-managed wiki at `~/wiki/projects/jefscad/` (background,
design rationale, and planning history distilled from `architecture/` and past
planning notes). Before ending a session that produced commits, check whether the
wiki needs updating — full instructions in `~/wiki/AGENTS.md`. The wiki is a
*reference*, not a write target from implementation tasks: wiki updates happen as
their own step (wiki-sync mode), not as side-effects.

---

## Things to Avoid

- Do not run `cargo build` / `cargo test` with `+nightly` — `jefscad` is stable-Rust
  (edition 2024).
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
- Do not write to `~/wiki/` from implementation tasks — wiki updates happen in
  wiki-sync mode.
