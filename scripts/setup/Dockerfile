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

# Update the package list and install dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    bc \
    bison \
    bridge-utils \
    build-essential \
    clang-format \
    curl \
    dosfstools \
    doxygen \
    flex \
    g++-multilib \
    gcc-multilib \
    git \
    graphviz \
    grub2 \
    iproute2 \
    jq \
    kpartx \
    libglib2.0-dev \
    libgmp-dev \
    libgmp3-dev \
    libmpc-dev \
    libmpfr-dev \
    libncurses5-dev \
    libpixman-1-dev \
    libsdl2-dev \
    libvirt-clients \
    libvirt-daemon-system \
    mtools \
    netcat-openbsd \
    ninja-build \
    pkg-config \
    python3-venv \
    qemu-kvm \
    qemu-system-x86 \
    texinfo \
    xorriso \
    wget && \
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
