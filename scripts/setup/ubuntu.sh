#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

# Update package repository.
apt-get update

# Install build-essential by default.
apt-get install -y build-essential --no-install-recommends

# Install additional packages.
# Keep the following list of packages sorted alphabetically.
apt-get install -y        \
    bc                    \
    bison                 \
    bridge-utils          \
    build-essential       \
    clang-format          \
    cmake                 \
    codespell             \
    cpio                  \
    curl                  \
    dosfstools            \
    doxygen               \
    flex                  \
    g++-multilib          \
    gawk                  \
    gcc-multilib          \
    git                   \
    graphviz              \
    grub2                 \
    iproute2              \
    jq                    \
    kpartx                \
    libelf-dev            \
    libglib2.0-dev        \
    libgmp-dev            \
    libgmp3-dev           \
    libmpc-dev            \
    libmpfr-dev           \
    libncurses5-dev       \
    libpixman-1-dev       \
    libsdl2-dev           \
    libvirt-clients       \
    libvirt-daemon-system \
    mmdebstrap            \
    mtools                \
    netcat-openbsd        \
    ninja-build           \
    pkg-config            \
    python3-venv          \
    qemu-kvm              \
    qemu-system-x86       \
    shellcheck            \
    texinfo               \
    wget                  \
    xorriso
