#!/usr/bin/env bash

# Copyright(c) 2011-2023 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================
# Script Arguments
#===============================================================================

PREFIX=$1 # Prefix

#===============================================================================
# Setup GCC
#===============================================================================

function setup_gcc
{
    local TARGET="$1-elf"
    local PREFIX=$2
    local VERSION=13.1.0

    pushd $PWD

    # Create build directory.
    mkdir -p build-gcc && cd build-gcc

    # Get sources.
    wget https://ftp.gnu.org/gnu/gcc/gcc-$VERSION/gcc-$VERSION.tar.xz
    tar -xvf gcc-$VERSION.tar.xz
    cd gcc-$VERSION

    ./contrib/download_prerequisites

    # Build and install.
    mkdir build && cd build
    ../configure \
        --target=$TARGET  --prefix=$PREFIX \
        --disable-nls --enable-languages=c --without-headers \
        --disable-multilib $ABI
    make all-gcc all-target-libgcc
    make install-gcc install-target-libgcc

    popd

    # Cleanup build files.
    rm -rf build-gcc
}

#===============================================================================
# Main
#===============================================================================

if [ -z "$PREFIX" ];
then
    export PREFIX=$PWD
fi

setup_gcc "i486" "$PREFIX/toolchain"
