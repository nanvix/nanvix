#!/usr/bin/env bash

# Copyright(c) 2011-2023 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================
# Script Arguments
#===============================================================================

PREFIX=$1 # Prefix

#==============================================================================
# Setup Bochs
#==============================================================================

function setup_bochs
{
    local PREFIX=$1/bochs
    local VERSION=2.7

    pushd $PWD

    # Create build directory.
    mkdir -p build-bochs && cd build-bochs

    # Get the sources.
    wget "https://sourceforge.net/projects/bochs/files/bochs/$VERSION/bochs-$VERSION.tar.gz"
    tar -xvf bochs-$VERSION.tar.gz
    rm -f bochs-$VERSION.tar.gz

    # Build and install.
    cd bochs-$VERSION
    ./configure --prefix=$PREFIX --with-term --enable-gdb-stub --disable-docbook
    make all
    make install

    popd

    # Cleanup build files.
    rm -rf build-bochs
}

#===============================================================================
# Main
#===============================================================================

if [ -z "$PREFIX" ];
then
    export PREFIX=$PWD
fi

setup_bochs "$PREFIX/toolchain"
