#!/usr/bin/env bash

# Copyright(c) 2011-2023 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================
# Script Arguments
#===============================================================================

PREFIX=$1 # Prefix

#===============================================================================
# Setup GDB
#===============================================================================

function setup_gdb
{
    local TARGET="$1-elf"
    local PREFIX=$2
    local VERSION=13.1

    pushd $PWD

    # Create build directory.
    mkdir -p build-gdb && cd build-gdb

    # Get sources.
	wget https://ftp.gnu.org/gnu/gdb/gdb-$VERSION.tar.xz
	tar -xvf gdb-$VERSION.tar.xz
	cd gdb-$VERSION

	# Build and install.
	./configure \
        --target=$TARGET --prefix=$PREFIX \
        --with-auto-load-safe-path=/ --with-guile=no
	make
	make install

    popd

    # Cleanup build files.
    rm -rf build-gdb
}

#===============================================================================
# Main
#===============================================================================

if [ -z "$PREFIX" ];
then
    export PREFIX=$PWD
fi

setup_gdb "i486" "$PREFIX/toolchain"
