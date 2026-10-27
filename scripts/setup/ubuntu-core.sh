#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

# Prevent debconf prompts (e.g., tzdata on fresh images) from blocking the script.
export DEBIAN_FRONTEND=noninteractive

# Update package repository.
apt-get update

# Install build-essential on its own, without recommended packages (e.g., gnupg,
# fakeroot, and libc-devtools), which are not needed to build Nanvix.
apt-get install -y build-essential --no-install-recommends

# Install core development packages needed to build, run, and test Nanvix.
# Keep the following list of packages sorted alphabetically.
apt-get install -y        \
    bzip2                 \
    clang                 \
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
