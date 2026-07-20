# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Use an official Ubuntu as a parent image
FROM ubuntu:24.04

# Set default environment variables.
ARG USER=root
ARG HOME=/root
USER $USER

# Rust version.
ARG RUST_VERSION

# Nanvix version.
ARG NANVIX_VERSION

# Validate required build arguments.
RUN if [ -z "$RUST_VERSION" ] || [ -z "$NANVIX_VERSION" ]; then \
    echo "RUST_VERSION and NANVIX_VERSION must be set"; exit 1; \
    fi

# Install minimal dependencies to clone repository.
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    sudo && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Clone Nanvix repository.
RUN git clone https://github.com/nanvix/nanvix /opt/nanvix/src/nanvix && \
    cd /opt/nanvix/src/nanvix && \
    git checkout -f $NANVIX_VERSION

# Install dependencies for development tools.
RUN cd /opt/nanvix/src/nanvix && \
    ./scripts/setup/ubuntu-core.sh && \
    ./scripts/setup/ubuntu-sdk.sh

# Install Rust.
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH=$HOME/.cargo/bin:$PATH

# Install Rust toolchain and components.
RUN rustup toolchain install $RUST_VERSION && \
    rustup default $RUST_VERSION

# Install sccache for build caching.
ARG SCCACHE_VERSION=0.10.0
RUN curl -fsSL "https://github.com/mozilla/sccache/releases/download/v${SCCACHE_VERSION}/sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
    | tar xz -C /usr/local/bin --strip-components=1 --wildcards '*/sccache' && \
    chmod +x /usr/local/bin/sccache

# Build toolchain.
RUN cd /opt/nanvix/src/nanvix && \
    ./z setup --nanvix-sdk --toolchain-dir /opt/nanvix
