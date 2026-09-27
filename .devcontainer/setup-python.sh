#!/usr/bin/env bash
# .devcontainer/setup-python.sh
# Post-creation setup: uv-managed venv + dependency groups.
#
# Unlike the lab templates, this project already declares its own dependencies
# in pyproject.toml:
#
#   [project.optional-dependencies]
#   dev  = pytest, jupyterlab, ipykernel
#   docs = sphinx>=7, furo
#
# so this is just `uv sync --all-extras` — no ad-hoc package list to drift out
# of date against pyproject.toml. Idempotent: uv resolves from the existing
# lockfile and leaves the tree alone if nothing changed.
#
# --all-extras, not --all-groups: those keys are *extras* (PEP 621
# optional-dependencies), not PEP 735 dependency-groups, and the two flags are
# not interchangeable. Using --all-groups silently installs none of them and
# uv reports success, so the failure is quiet. Caught by running this script in
# the built image rather than by reading it.

set -euo pipefail

cd /workspace

echo "→ Syncing the uv-managed environment (extras: dev, docs)..."
uv sync --all-extras

# maturin is a PEP 517 *build backend* (build-backend = "maturin"), which means
# uv uses it in an isolated build environment and does NOT leave it in the
# project venv. But DEVELOPMENT.md's daily loop is literally
# `maturin develop --features extension-module`, so it has to be on PATH.
# DEVELOPMENT.md step 3 prescribes exactly this install; without it the
# documented workflow fails with "Failed to spawn: maturin".
#
# Arguably maturin belongs in the [dev] extra in pyproject.toml, which would
# make `uv sync --all-extras` sufficient and drop this line. That is a change
# to the project's declared dependencies, so it is not made here unilaterally.
echo "→ Installing maturin into the venv (build backend; needed by the daily loop)..."
uv pip install --python .venv/bin/python "maturin>=1.0,<2.0"

echo
echo "→ Registering a Jupyter kernel for the notebooks/ directory..."
uv run python -m ipykernel install \
    --user \
    --name "jefscad-dev" \
    --display-name "Python (jefscad)"

echo
echo "=== Verification ==="
rustc --version
cargo --version
uv --version
uv run python --version
uv run pytest --version
uv run maturin --version
uv run python -c "import sphinx; print('sphinx', sphinx.__version__)"

echo
echo "=== Reminder: the Rust extension is not built yet ==="
echo "  maturin develop --features extension-module   # rebuild the .so after Rust edits"
echo "  cargo test                                     # Rust unit tests; no Python needed"
echo "  uv run pytest -v                               # Python tests"
echo
echo "✅ Devcontainer setup complete"
