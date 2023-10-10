#!/usr/bin/env bash

# Copyright(c) 2011-2023 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================
# Script Arguments
#===============================================================================

PREFIX=$1 # Prefix

#===============================================================================
# Global Variables
#===============================================================================

export TARGET=i386

#==============================================================================
# Setup QEMU
#==============================================================================

function setup_qemu
{
    local TARGET="$1-softmmu"
    local PREFIX=$2/qemu
    local VERSION=8.1.1

    pushd $PWD

    # Create build directory.
    mkdir -p build-qemu && cd build-qemu

    # Get the sources.
    wget "http://wiki.qemu-project.org/download/qemu-$VERSION.tar.bz2"
    tar -xjvf qemu-$VERSION.tar.bz2
    rm -f qemu-$VERSION.tar.bz2

    # Build and install.
    cd qemu-$VERSION
    ./configure \
        --prefix=$PREFIX --target-list=$TARGET --enable-sdl --enable-curses
    make all
    make install

    popd

    # Cleanup build files.
    rm -rf build-qemu
}

#===============================================================================
# Main
#===============================================================================

if [ -z "$PREFIX" ];
then
    export PREFIX=$PWD
fi

setup_qemu "i386" "$PREFIX/toolchain"
