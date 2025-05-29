#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Script Arguments
#===================================================================================================

export PREFIX=${1:-$PWD/toolchain}

#===================================================================================================
# Environment Variables
#===================================================================================================

export TARGET=i686-nanvix
export SYSROOT=$PREFIX
export NANVIX_HOME=`git rev-parse --show-toplevel`
export CONTRIB_DIR=${NANVIX_HOME}/contrib
export BINUTILS_HOME=${CONTRIB_DIR}/binutils

#===================================================================================================
# Get Sources
#===================================================================================================

git submodule update --init ${BINUTILS_HOME}

#===================================================================================================
# Build Binutils for Nanvix
#===================================================================================================

cd ${BINUTILS_HOME}

git clean -fdx

./configure \
    --target=$TARGET \
    --prefix=$PREFIX \
    --with-sysroot=$SYSROOT \
    --disable-multilib \
    --disable-nls \
    --disable-sim

make -j `nproc` all
make install
