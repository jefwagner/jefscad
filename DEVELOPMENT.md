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
│   └── __init__.py             # re-exports public API from ._jefscad
├── tests/                      # pytest test suite
│   └── test_hello.py
├── notebooks/                  # Jupyter notebooks
│   └── playground.ipynb
├── jefscad/                    # Rust crate — compiled into jefscad._jefscad
│   ├── Cargo.toml
│   └── src/lib.rs
└── flint/                      # Rust interval arithmetic library
```

**How the two layers fit together:**
The Rust crate compiles to `jefscad._jefscad` (underscore prefix marks it as an
implementation detail). `python/jefscad/__init__.py` imports from `._jefscad` and
re-exports the public API, so callers write `from jefscad import HelloWorld` rather
than reaching into the private submodule.

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
