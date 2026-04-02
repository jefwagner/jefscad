# Development Guide

How to set up the environment, build the Rust extension, run tests, and
work interactively in Jupyter.

## Tools

| Tool | Role |
|------|------|
| `uv` | Creates and manages the Python virtualenv and Python dependencies |
| `maturin` | Compiles the Rust extension and installs it as a Python package |
| Rust nightly | Required by `flint` (`portable_simd`, `macro_metavar_expr`) |
| `pytest` | Python test runner |
| `jupyterlab` | Interactive notebook server |

## Project layout

```
repo root/
├── pyproject.toml              # maturin build config + project metadata + pytest config
├── .venv/                      # UV-managed virtualenv (gitignored)
├── python/jefscad/             # thin Python wrapper package
│   ├── __init__.py             # re-exports public API from ._jefscad
│   └── _jefscad/
│       └── __init__.pyi        # generated type stubs (via stub_gen binary)
├── docs/                       # Sphinx documentation source
│   ├── conf.py
│   ├── index.rst
│   ├── getting_started.rst
│   ├── concepts.rst
│   └── api.rst
├── tests/                      # pytest test suite
├── notebooks/                  # Jupyter notebooks
├── jefscad/                    # Rust crate — compiled into jefscad._jefscad
│   ├── Cargo.toml
│   ├── notes.md                # design notes and phase-by-phase TODO list
│   └── src/
│       ├── lib.rs
│       ├── csg_lang.rs         # CSG AST types and constructors
│       ├── py_bindings.rs      # pyo3 Python bindings
│       └── bin/stub_gen.rs     # generates _jefscad/__init__.pyi
└── flint/                      # Rust interval arithmetic library
```

**How the two layers fit together:**
The Rust crate compiles to `jefscad._jefscad` (underscore prefix marks it as an
implementation detail). `python/jefscad/__init__.py` imports from `._jefscad` and
re-exports the public API, so callers write `import jefscad; jefscad.sphere(...)`.

The `extension-module` pyo3 feature is *optional* in `jefscad/Cargo.toml`, which
means `cargo +nightly test` works without linking against Python at all.

---

## One-time setup

Run these once from the repo root after cloning.

```bash
# 1. Create the virtualenv
uv venv .venv

# 2. Install Python dev dependencies
uv pip install --python .venv/bin/python maturin pytest jupyterlab ipykernel

# 3. Activate the virtualenv
source .venv/bin/activate

# 4. Build the Rust extension and install it in editable mode
maturin develop --features extension-module
```

After step 4 you have an editable install: Python-side changes in
`python/jefscad/` take effect immediately; Rust-side changes require
re-running `maturin develop` (step 4).

---

## Daily development loop

Activate the virtualenv once per shell session, then run commands directly:

```bash
source .venv/bin/activate
```

### After editing Rust code

```bash
maturin develop --features extension-module
```

This recompiles only the changed Rust and reinstalls the `.so` in place.
It typically takes a few seconds for incremental builds.

### Run Rust unit tests (no Python required)

```bash
cargo +nightly test
```

### Run Python tests

```bash
pytest -v
```

---

## Jupyter workflow

### Launching the notebook server

Activate the virtualenv, then launch Jupyter from the repo root:

```bash
source .venv/bin/activate
jupyter lab
```

This opens JupyterLab in your browser. Navigate to `notebooks/playground.ipynb`
to start experimenting. You can also open it directly:

```bash
jupyter lab notebooks/playground.ipynb
```

### Reloading after changes

**Python-only changes** (`python/jefscad/__init__.py` or any pure-Python code):

Because the install is editable, the file is already live on disk. Use IPython's
autoreload extension at the top of your notebook to pick up changes without
restarting the kernel:

```python
%load_ext autoreload
%autoreload 2
import jefscad
```

With `%autoreload 2` active, re-running any cell will automatically reload
changed Python modules before executing.

**Rust changes** (anything in `jefscad/src/`):

Compiled extensions (`.so` files) cannot be hot-reloaded by Python — the old
binary stays loaded until the process exits. The workflow is:

1. Edit the Rust source.
2. In a terminal (the notebook server keeps running):
   ```bash
   maturin develop --features extension-module
   ```
3. Back in JupyterLab: **Kernel → Restart Kernel** (or click the ↺ button in the
   toolbar). This unloads the old `.so` and the next `import jefscad` loads the
   new one.
4. Re-run your cells from the top.

You do **not** need to stop the notebook server — just restart the kernel.

---

---

## Documentation

### Three-layer documentation system

| Layer | Source | Audience | Tool |
|-------|--------|----------|------|
| Rust API docs | `///` comments in `.rs` files | Rust developers | `cargo doc` |
| Python docstrings | Same `///` comments — PyO3 maps them to `__doc__` automatically | Python users at runtime (`help()`, `?`) | built-in `help()` |
| Python type stubs | Generated `.pyi` file | IDEs, type checkers (mypy/pyright) | `pyo3-stub-gen` |
| User-facing HTML docs | `docs/*.rst` + `__init__.py` docstring | End users | Sphinx |

The Rust `///` comments are the **single source of truth** for docstrings — there is no
separate Python docstring layer. The `__init__.py` module docstring is the place for
package-level narrative text (what the package is, a quick example).

### Building the Sphinx docs

Install the docs dependencies (once):

```bash
uv pip install --python .venv/bin/python sphinx furo
# or, using the pyproject extras:
uv pip install --python .venv/bin/python ".[docs]"
```

Build and view:

```bash
# The compiled extension must be installed first (maturin develop)
sphinx-build -b html docs/ docs/_build/html/

# Serve locally for review
python -m http.server 8080 --directory docs/_build/html/
# then open http://localhost:8080
```

### Rebuilding the Rust API docs

```bash
cargo +nightly doc --no-deps --features extension-module
# opens target/doc/_jefscad/index.html
```

### Regenerating the .pyi type stubs

Run after any change to the public Python API (new functions, changed signatures):

```bash
cargo +nightly run --bin stub_gen --features extension-module
# writes python/jefscad/_jefscad/__init__.pyi
```

---

## Gotchas

**`.venv/` is gitignored**
Every new clone requires the one-time setup above. The compiled `.so`
(`python/jefscad/_jefscad.cpython-*.so`) is also gitignored and regenerated
by `maturin develop`.

**Always use `cargo +nightly`**
There is no `rust-toolchain.toml` in this repo. Omitting `+nightly` will use
stable Rust, which does not support the features required by `flint`.

**Editable installs and the `.pth` file**
`maturin develop` installs a `.pth` file in `.venv/lib/.../site-packages/`
pointing at `python/`. If you move the repo, re-run `maturin develop` to
update the path.
