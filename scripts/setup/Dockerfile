# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Use an official Ubuntu as a parent image
FROM ubuntu:24.04

# Set default environment variables.
ARG USER=root
ARG HOME=/root
USER $USER

# Rust version.
ARG RUST_VERSION=nightly-2025-03-18

# Nanvix version.
ARG NANVIX_VERSION=b7d8b4accafd30d8ed15a99f51cc5b77ea32a4ba

# Update the package list and install dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    build-essential \
    bison \
    clang-format \
    curl \
    doxygen \
    flex \
    gcc-multilib \
    g++-multilib \
    git \
    graphviz \
    grub2 \
    libglib2.0-dev \
    libgmp-dev \
    libgmp3-dev \
    libmpc-dev \
    libmpfr-dev \
    libncurses5-dev \
    libpixman-1-dev \
    libsdl2-dev \
    mtools \
    ninja-build \
    pkg-config \
    python3-venv \
    qemu-system-x86 \
    texinfo \
    xorriso && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Install Rust.
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH=$HOME/.cargo/bin:$PATH
RUN rustup toolchain install $RUST_VERSION
RUN rustup default $RUST_VERSION
RUN rustup component add rust-src
RUN rustup target add wasm32-wasip1

# Build toolchain.
RUN git clone https://github.com/nanvix/nanvix /opt/src/nanvix
RUN cd /opt/src/nanvix && git checkout -f $NANVIX_VERSION
RUN cd /opt/src/nanvix && scripts/setup/toolchain.sh /opt
RUN rm -rf /opt/nanvix/src
