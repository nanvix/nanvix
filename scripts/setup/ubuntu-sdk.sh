#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

# Prevent debconf prompts from blocking the script (e.g., grub2 install dialog).
export DEBIAN_FRONTEND=noninteractive

# Update package repository.
apt-get update

# Install packages required to build the Nanvix cross-compilation toolchain
# (Binutils, GCC, Newlib, LLVM, custom Rust, CPython) from source.
# Keep the following list of packages sorted alphabetically.
apt-get install -y        \
    bison                 \
    flex                  \
    g++-multilib          \
    gcc-multilib          \
    libelf-dev            \
    libglib2.0-dev        \
    libgmp-dev            \
    libgmp3-dev           \
    libmpc-dev            \
    libmpfr-dev           \
    libncurses5-dev       \
    libpixman-1-dev       \
    libsdl2-dev           \
    texinfo
