# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# =============================================================================
# Nanvix CI container image.
#
# This image pre-installs every system package, Rust toolchain component, and
# Python dependency required by GitHub-hosted CI jobs so that workflows can skip
# the expensive apt-get update / install cycle at runtime.
#
# Build:
#   docker build -t ghcr.io/nanvix/ci:ubuntu-24.04 -f docker/ci/Dockerfile .
#
# The image is rebuilt automatically by .github/workflows/ci-container.yml
# whenever the Dockerfile, setup scripts, rust-toolchain, or requirements.txt
# change.
# =============================================================================

FROM ubuntu:24.04

# Prevent interactive prompts during package installation.
ENV DEBIAN_FRONTEND=noninteractive

# ---------------------------------------------------------------------------
# System packages — reuses scripts/setup/ubuntu-core.sh to avoid duplication.
#
# The extra packages installed here (python3-pip, sudo) are CI-container
# specific and therefore not part of the shared setup script.
# ---------------------------------------------------------------------------
COPY scripts/setup/ubuntu-core.sh /tmp/ubuntu-core.sh
RUN bash /tmp/ubuntu-core.sh \
    && apt-get install -y --no-install-recommends \
        python3-pip \
        sudo \
    && rm -rf /var/lib/apt/lists/* /tmp/ubuntu-core.sh

# ---------------------------------------------------------------------------
# Rust toolchain — derived from rust-toolchain at build time.
# ---------------------------------------------------------------------------
COPY rust-toolchain /tmp/rust-toolchain

ENV RUSTUP_HOME=/opt/rust/rustup \
    CARGO_HOME=/opt/rust/cargo
ENV PATH="${CARGO_HOME}/bin:${PATH}"

RUN RUST_CHANNEL="$(sed -n 's/^channel[[:space:]]*=[[:space:]]*"\(.*\)"/\1/p' /tmp/rust-toolchain | head -n 1)" \
    && if [ -z "${RUST_CHANNEL}" ]; then echo "error: failed to parse Rust channel from rust-toolchain" >&2; exit 1; fi \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --default-toolchain "${RUST_CHANNEL}" \
    && rustup component add rust-src rustfmt clippy \
    && rm /tmp/rust-toolchain

# ---------------------------------------------------------------------------
# Python virtual environment — mirrors requirements.txt.
# ---------------------------------------------------------------------------
COPY requirements.txt /tmp/requirements.txt
RUN python3 -m venv /opt/nanvix-venv \
    && /opt/nanvix-venv/bin/pip install --no-cache-dir -r /tmp/requirements.txt \
    && rm /tmp/requirements.txt
ENV PATH="/opt/nanvix-venv/bin:${PATH}" \
    VIRTUAL_ENV="/opt/nanvix-venv"

# ---------------------------------------------------------------------------
# Marker so native-setup can detect it is running inside the CI container.
# ---------------------------------------------------------------------------
RUN touch /etc/nanvix-ci-container

LABEL org.opencontainers.image.source="https://github.com/nanvix/nanvix" \
      org.opencontainers.image.description="Nanvix CI build environment"
