#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

# Prevent debconf prompts from blocking the script (e.g., grub2 install dialog).
export DEBIAN_FRONTEND=noninteractive

# Update package repository.
apt-get update

# Install packages required to build Cloud Hypervisor and the L2 system VM
# kernel from source.
# Keep the following list of packages sorted alphabetically.
apt-get install -y        \
    bison                 \
    flex                  \
    libelf-dev            \
    libssl-dev
