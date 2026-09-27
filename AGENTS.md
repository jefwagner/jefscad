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

Mode switches are explicit — say "pair", "tdd", or "unsupervised" to change modes.
The default, when no mode is stated, is **pair**.

Three nested loops, each with a different unit of approval:

| Mode           | Unit of work              | Writes need approval? |
|----------------|---------------------------|-----------------------|
| `pair`         | single command/change     | yes, per command      |
| `tdd`          | task / commit-sized item  | no                    |
| `unsupervised` | feature / whole goal      | no                    |

The nesting widens the unit of work that may be done unattended: a single command
in pair, a task in tdd, a whole feature or goal in unsupervised. Stepping up is
jef's call, not mine — if a task needs many writes, say so and suggest `tdd`
rather than quietly widening the loop.

### Pair mode (default)

Pair-programming at the keyboard. Short back-and-forth, one or two ideas per
interaction — no long essays presenting a pile of options. Every idea and change
gets discussed as we go: a sentence or two saying what I'm about to do and why,
then act. No autopilot — no multi-file refactors or multi-step plans executed
without checking in between steps.

Freely, without asking:

- read files, search the repo (`rg`, `find`, `grep`), read `architecture/`,
  `notes/`, and the wiki
- non-destructive inspection: `ls`, `cat`, `head`, `tail`, `wc`, `git status`,
  `git log`, `git diff`
- the project's read-only checks: `cargo check`, `cargo test`, `cargo clippy`,
  `uv run pytest`, `uv run pytest --collect-only`, `cargo fmt --check`

Ask for explicit approval, per command, before:

- writing, editing, creating, moving, or deleting any file — including
  `notes/`, `todo.md`, `backlog.md`, `architecture/`, and anything outside this
  repo
- destructive or state-changing commands: `rm`, `mv`, `cp` over an existing
  file, `git commit`/`push`/`checkout`/`reset`, or anything else that changes
  state on disk
- `maturin develop` — it recompiles the extension and reinstalls the `.so` into
  the venv, so it changes the working environment. `cargo build` and
  `maturin build` (release artifact only) do not touch the venv and are fine
  without asking.
- anything touching `~/wiki/`, including local writes; see the Wiki section
- installing packages, editing config outside the repo, sending mail

Approval is per command, not blanket — "yes, go ahead" for one command does not
authorize the next. One approval covers one file: a multi-hunk edit within a
single file is one atomic change, but two files is two approvals. Never commit
unless asked in that same turn, even if every individual write was approved.

Full rules: read `~/tools/llm-instructions/pair.md`.

### TDD mode (interactive)

Structured mode for larger chunks of work. Two sub-modes:

- **Plan** — read `~/tools/llm-instructions/planning.md` first. Produces 1–4
  one-commit-sized items in `todo.md` and triages `backlog.md`.
- **Implement** — read `~/tools/llm-instructions/implementation.md` first. TDD
  loop with hard STOP gates requiring user approval at the task level.

Writes, edits, and commits are **not** gated per command here — the task-level
STOP gates are the approval unit. Commits remain one-item-per-commit, and I still
state intent before each task, but I don't ask before every `edit`.

The **(a)/(b) test split** below applies in both sub-modes and is the most
important project-specific rule here. When unsure, check the tier before
rewriting a test.

### Unsupervised mode

For well-scoped work that jef approves up front and then leaves to run
independently. This is the mode this repo's containment exists to support — see
**Containment** for what it does and does not buy.

1. **Goal stage (conversational)**: work with jef to draft `goal.md` at the repo
   root. It must contain: the goal, constraints, a definition of done, and a
   **spend cap in dollars** agreed during the conversation. Write `goal.md` as
   a complete prompt — an agent with no other context should be able to do the
   work from it alone.
2. **Approval gate**: do not start work until jef explicitly approves `goal.md`.
   Revise and re-submit until approved.
3. **Isolate**: create a worktree before anything else —
   `git worktree add ../jefscad-loop -b agent/<date> dice-plans`, and work in
   `../jefscad-loop`. Never work unattended in the main checkout. This is what
   keeps `~/projects/jefscad` itself untouched.
4. **Independent work**: proceed without further check-ins. Track spend with:
   `uv run ~/tools/spend.py --cap <cap from goal.md>`
   Check periodically. It **reports and exits non-zero**; it does not interrupt,
   so treating that exit code as a stop signal *is* the enforcement. The cap is
   an upper bound on money, not a bound on time — `goal.md` must be small enough
   to finish.
5. **Stop and report**: stop when the goal is met, the cap is exceeded, or the
   work is blocked. Then:

   - run the full test suite and record the result;
   - push the branch — **never merge it, never push to `dice-plans` or `main`**;
   - write a summary email to **jefwagner@gmail.com only** with what was done,
     what was learned, current state, next steps, test results, and the final
     spend from `uv run ~/tools/spend.py`;
   - send it with `msmtp -t < summary-email.txt`, subject prefixed `[jefscad]
     unsupervised` so it is filterable;
   - **also commit the same summary into the branch** as
     `notes/unsupervised-<date>.md`, because email is the notification and the
     file is the durable record;
   - note in the summary what would have been proposed to `~/wiki/` — do not
     propose it. An unmonitored agent does not touch the wiki at all.

**One email per run, to that one address.** The email is not decoration: it is
the signal that pulls jef in to review, so it matters that it is reliable and
that it arrives. If `msmtp` fails, say so in the branch summary rather than
retrying in a loop — a silent email means jef does not know work is waiting.

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
  planning mode decomposes items from here into `todo.md`. Edit it when a phase's
  shape changes, not when an individual task lands.
- **`CHANGELOG.md` is milestones, not commits.** Curated milestone summaries
  (reverse-chronological, coarser than commits) — a phase landing or a shipped
  feature, not every commit. Duplicating `git log` is why CHANGELOGs get abandoned.

---

## Planning and progress files

| File | Horizon | Granularity | Churn |
|------|---------|-------------|-------|
| `ROADMAP.md` | long-term | phases toward full project completion | rarely |
| `todo.md` | current session | commit-sized actionable items (1–4) | every planning session |
| `backlog.md` | persistent | raw ideas, mid-session discoveries | triaged in planning mode |
| `CHANGELOG.md` | history | milestone summaries | at milestones |

`todo.md` is session-scoped: rewritten at each planning session, cleared at the end
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
├── todo.md                  # session-scoped actionable items
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

## Containment

**Work inside the devcontainer.** Open this repo in a devcontainer (VS Code
"Reopen in Container", or `devcontainer up`) and run agent sessions there. The
properties below are enforced by the container, not promised by this file — a
session on the host has the personal SSH key and defeats all of it. If a
session is somehow running outside the container, say so before doing anything
else.

### What the container enforces

Verified by building the image and running these checks inside it. They are
properties of the environment, not claims about behaviour.

- **The personal SSH key is absent**, and no `ssh`, `scp`, `sftp`,
  `ssh-agent`, or `rsync` binary exists. Git has no SSH transport at all, so it
  cannot fall back to `~/.ssh/id_ed25519` — not even if the key were somehow
  mounted, and not even if `GIT_SSH_COMMAND` were unset. This required an
  explicit `apt-get purge` in the Dockerfile: the base image ships
  `openssh-client`, so merely not installing it is not enough.
- **One credential exists**: the repo-scoped PAT, mounted read-only. The
  pixel-world PAT and the lab bot credential are neither mounted nor reachable.
- **No `~/.aws`, `~/.azure`, `~/.config/lab-bot`, or host tool directory.**
- **`~/wiki/` is mounted read-only** — full read access to `kb/`,
  `projects/jefscad/` (9 pages, including `Flint.md` for the interval
  arithmetic this project depends on) and `projects/pixel-world/`, with no
  ability to write.
- **`~/tools/llm-instructions/` and `~/tools/spend.py` are read-only**, so the
  rules this session operates under cannot be edited mid-session.
- **`~/.msmtprc` is read-only** — present only so an unsupervised run can send
  its one summary email. The agent is instructed to send exactly one message,
  to one address.
- `--cap-drop=ALL` and `--security-opt=no-new-privileges:true`.

Consequence: **the worst realistic outcome of a bad unattended run is commits
and branches inside this repository, plus one email.** Nothing else on the
machine is writable, and nothing else is reachable.

### Credentials

Agent sessions push as `jefwagner` with a fine-grained PAT scoped to this repo
only. Setup and the full permission list are in
`~/tools/llm-instructions/pat-setup.md`; the token lives at
`~/.config/jef/jefscad-pat` (mode 600, outside the repo), bind-mounted in at the
same relative path.

`git-askpass.sh` (tracked, repo root) reads it at call time; `devcontainer.json`
sets `GIT_ASKPASS`, `GIT_TERMINAL_PROMPT=0`, and `GIT_SSH_COMMAND=/bin/false`.
This repo's remote is already HTTPS, so the token is genuinely used — which is
the step that was missing in pixel-world and made its PAT decorative.

If a git command prompts interactively, something is misconfigured: stop and
say so rather than working around it.

### Branches

- **The primary branch is `dice-plans`** (89 commits); `main` is the default
  branch holding releases (76 commits). The naming is asymmetric with the other
  projects — pixel-world uses `dev` — and is worth normalising eventually, but
  until then the worktree base in Unsupervised mode step 3 follows
  `dice-plans`, and that is the branch an agent must never commit to.
- Progress files are lowercase (`todo.md`), matching `planning.md`. The old
  `TODO.md` was renamed because a case-mismatched name on a case-sensitive
  filesystem produces two competing todo lists — the one the agent writes and
  the one being read from.
- Work goes on `agent/<short-desc>` branches. Never commit to `dice-plans`.
- `dice-plans` and `main` reject force-pushes and require a pull request.
- Prefer a worktree for anything unattended (see Unsupervised mode, step 3).

### Residual risk — the honest remainder

- **Spend.** `spend.py` reports and exits non-zero; nothing interrupts. The cap
  is enforced by treating that exit as a stop signal.
- **Damage within this repo.** `reset --hard`, `rm`, and a force-push to a
  non-protected branch are all still possible here. Hence worktrees and branch
  discipline.
- **The agent's judgement.** A container cannot tell a good idea from a
  plausible wrong one. This is the *riskiest* repo in the estate for that,
  because the (a)/(b) test contract is a prose contract: nothing mechanically
  stops an agent from rewriting an integration test to make a refactor pass.
  The rule in "Project-specific workflow deltas" is the only thing holding
  that line, so treat it as non-negotiable.
- **One outbound email.** The `msmtp` mount is a real capability, not a
  formality. A goal that would generate many emails is out of scope by design.

## Wiki

`~/wiki/` is **mounted read-only**. Read it freely — that is the point:

- `~/wiki/kb/` — the distilled fundamentals: B-Rep, CSG, NURBS, mesh
  representations, and the physics/visual references.
- `~/wiki/projects/jefscad/` — 9 pages of implementation specifics, including
  `Flint.md` for the interval arithmetic underneath the tolerance model, and
  `JefSCAD-Roadmap.md` / `JefSCAD-Boolean-Operations.md` for the risky parts.
- `~/wiki/projects/<other>/` — **when a decision has cross-project
  implications.** `architecture/` and the wiki are not independent restatements
  of the same design; when they disagree, that is worth surfacing rather than
  silently picking one.

**You cannot write to `~/wiki/`, and must not try.**

### Proposing an update

If the wiki would have changed, write a **proposal** to
`notes/wiki-proposals/YYYY-MM-DD-short-topic.md` (the directory does not exist
yet — create it), structured as the change rather than as a session summary:

- exact target paths under `~/wiki/`,
- for each, the full new text or a precise before/after,
- what `~/wiki/AGENTS.md` would require: new inbound links, index updates, a
  matching edit to `kb/Projects-Summary.md`, cross-links to related pages.

Commit it with the item and point at it in the end-of-chunk summary, so
promotion is mechanical for a human rather than a research project.

**Never promote a proposal yourself**, in any mode. Producing it is the entire
job. The wiki is the one place in this estate where an undetected error gets
*stronger with use* rather than caught: prose has no failing test, and a wrong
claim about the tolerance model or the boolean pipeline would be absorbed into
the next plan and become more entrenched each time it is read.

**While unsupervised, do not propose either** — note it in the summary and stop.

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
