#!/usr/bin/env bash

# Copyright(c) 2011-2023 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================
# Script Arguments
#===============================================================================

PREFIX=$1 # Prefix

#===============================================================================
# Setup Binutils
#===============================================================================

function setup_binutils
{
    local TARGET="$1-elf"
    local PREFIX=$2
    local VERSION=2.40

    pushd $PWD

    # Create build directory.
    mkdir -p build-binutils && cd build-binutils

    # Get sources.
    wget https://ftp.gnu.org/gnu/binutils/binutils-$VERSION.tar.xz
    tar -xvf binutils-$VERSION.tar.xz
    cd binutils-$VERSION

    # Build and install.
    ./configure --target=$TARGET --prefix=$PREFIX --disable-nls --disable-sim
    make all
    make install

    popd

    # Cleanup build files.
    rm -rf build-binutils
}

#===============================================================================
# Main
#===============================================================================

if [ -z "$PREFIX" ];
then
    export PREFIX=$PWD
fi

setup_binutils "i486" "$PREFIX/toolchain"
