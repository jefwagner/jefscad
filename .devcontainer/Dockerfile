# syntax=docker/dockerfile:1
FROM mcr.microsoft.com/devcontainers/base:1-ubuntu-24.04

# The single most important line in this file is the openssh purge below.
#
# openssh is deliberately REMOVED. Without an ssh binary on PATH, git has no SSH
# transport available at all: it cannot fall back to a personal key even if one
# were somehow mounted, and even if GIT_SSH_COMMAND were unset. This repo's
# containment is exactly one credential — the repo-scoped PAT mounted in by
# devcontainer.json — and no other transport by which to use any credential.
# That is a wall, not a speed bump.
#
# This has to be an explicit purge, not an omission. devcontainers/base ships
# openssh-client already, so simply not listing it leaves /usr/bin/ssh in place
# and the property silently does not hold. That was learned the hard way in
# pixel-world: the omission was verified by building the image and running the
# checks in it, not by reading the Dockerfile. Do not "fix" a git problem here
# by reinstalling openssh-client.

ENV DEBIAN_FRONTEND=noninteractive

# python3-dev is insurance, not ceremony: pyo3 resolves a Python interpreter at
# build time, and a missing header set produces a confusing maturin failure
# rather than a clear one. Ubuntu 24.04 ships 3.12, which satisfies the
# project's requires-python >=3.9 and matches what uv provisions.
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    cmake \
    ca-certificates \
    curl \
    git \
    python3 \
    python3-dev \
    msmtp \
    msmtp-mta \
    && rm -rf /var/lib/apt/lists/*

# Strip every openssh component, not just the client, so nothing can be
# re-enabled by pulling in a package that happens to depend on one. rsync goes
# too — not a credential transport (its remote mode needs the ssh binary the
# purge removes), so that is hygiene rather than a fix, but a file arguing for
# a minimal attack surface should not leave unexplained binaries in it.
RUN apt-get purge -y 'openssh-*' rsync && apt-get autoremove -y \
    && rm -rf /var/lib/apt/lists/* \
    && rm -f /usr/bin/ssh /usr/bin/scp /usr/bin/sftp /usr/bin/ssh-agent /usr/bin/ssh-add /usr/bin/rsync

# uv, copied from the official Astral distroless image (fast, no extra cruft).
# Required: the project is uv-managed (venv, maturin, pytest, jupyterlab) and
# the unsupervised spend cap is invoked as `uv run ~/tools/spend.py`.
COPY --from=ghcr.io/astral-sh/uv:latest /uv /uvx /bin/

ENV UV_LINK_MODE=copy \
    UV_COMPILE_BYTECODE=1 \
    UV_PYTHON_DOWNLOADS=automatic

# Rust stable. jefscad is edition 2024 with no nightly features and no
# rust-toolchain.toml, so plain stable is correct — do not add a toolchain file
# or a nightly component. DEVELOPMENT.md is explicit that `cargo test` works
# without linking Python, which is why the optional extension-module feature
# exists.
RUN mkdir -p /home/vscode/.rustup /home/vscode/.cargo \
    && chown -R vscode:vscode /home/vscode/.rustup /home/vscode/.cargo
USER vscode
ENV RUSTUP_HOME=/home/vscode/.rustup \
    CARGO_HOME=/home/vscode/.cargo
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable --profile default
ENV PATH=/home/vscode/.cargo/bin:$PATH

# Node.js (NodeSource LTS) + pi coding agent, installed natively in the image.
USER root
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/* \
    && npm install -g @earendil-works/pi-coding-agent

# pi agent config dir. devcontainer.json mounts the repo-scoped OpenRouter
# auth.json at ~/.pi/agent/auth.json; the parent dir must exist and be
# writable by vscode so pi can store models-store.json and sessions too.
RUN mkdir -p /home/vscode/.pi/agent && chown -R vscode:vscode /home/vscode/.pi

# Create workspace mountpoint
RUN mkdir -p /workspace && chown vscode:vscode /workspace

USER vscode
WORKDIR /workspace

CMD ["sleep", "infinity"]
