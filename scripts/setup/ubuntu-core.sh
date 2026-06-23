#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

# Prevent debconf prompts from blocking the script (e.g., grub2 install dialog).
export DEBIAN_FRONTEND=noninteractive

# Update package repository.
apt-get update

# Install build-essential by default.
apt-get install -y build-essential --no-install-recommends

# Install core development packages needed to build, run, and test Nanvix.
# Keep the following list of packages sorted alphabetically.
apt-get install -y        \
    bc                    \
    bridge-utils          \
    build-essential       \
    bzip2                 \
    clang                 \
    cmake                 \
    codespell             \
    cpio                  \
    curl                  \
    dosfstools            \
    doxygen               \
    gawk                  \
    gdb-multiarch         \
    git                   \
    graphviz              \
    grub2                 \
    iproute2              \
    jq                    \
    kpartx                \
    libvirt-clients       \
    libvirt-daemon-system \
    mmdebstrap            \
    mtools                \
    netcat-openbsd        \
    ninja-build           \
    pkg-config            \
    python3-venv          \
    shellcheck            \
    unzip                 \
    wget                  \
    xorriso
