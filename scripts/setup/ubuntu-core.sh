#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

# Prevent debconf prompts (e.g., tzdata on fresh images) from blocking the script.
export DEBIAN_FRONTEND=noninteractive

# Update package repository.
apt-get update

# Install build-essential by default.
apt-get install -y build-essential --no-install-recommends

# Install core development packages needed to build, run, and test Nanvix.
# Keep the following list of packages sorted alphabetically.
apt-get install -y        \
    build-essential       \
    bzip2                 \
    clang                 \
    codespell             \
    cpu-checker           \
    curl                  \
    gawk                  \
    gdb-multiarch         \
    git                   \
    iproute2              \
    jq                    \
    netcat-openbsd        \
    python3-venv          \
    shellcheck            \
    wget
