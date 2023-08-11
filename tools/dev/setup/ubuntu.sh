#!/usr/bin/env bash

# Copyright(c) 2011-2023 The Maintainers of Nanvix.
# Licensed under the MIT License.

# Update package repository.
apt-get update

# Install required packages.
apt-get install -y   \
    bison            \
    build-essential  \
    clang-format     \
    doxygen          \
    flex             \
    g++              \
    gdb              \
    genisoimage      \
    graphviz         \
    grub2            \
    libglib2.0-dev   \
    libgmp-dev       \
    libncurses5-dev  \
    libncursesw5     \
    libncursesw5-dev \
    libpixman-1-dev  \
    libsdl2-dev      \
    make             \
    mtools           \
    ninja-build      \
    pkg-config       \
    texinfo          \
    xorriso          \
    xz-utils         \
