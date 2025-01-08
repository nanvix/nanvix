#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

# Update package repository.
apt-get update

# Install build-essential by default.
apt-get install -y build-essential

# Check if the --extra parameter is supplied.
if [[ "$1" == "--extra" ]]; then
    # Install additional packages.
    apt-get install -y   \
        bison            \
        clang-format     \
        doxygen          \
        flex             \
        graphviz         \
        grub2            \
        libglib2.0-dev   \
        libgmp-dev       \
        libgmp3-dev      \
        libmpc-dev       \
        libmpfr-dev      \
        libncurses5-dev  \
        libncursesw5     \
        libncursesw5-dev \
        libpixman-1-dev  \
        libsdl2-dev      \
        mtools           \
        ninja-build      \
        pkg-config       \
        python3-venv     \
        qemu-system-x86  \
        texinfo          \
        xorriso
fi
