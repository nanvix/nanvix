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
export CONTRIB_DIR=${PREFIX}/src
export BINUTILS_HOME=${CONTRIB_DIR}/binutils
export BINUTILS_REPOSITORY=https://github.com/nanvix/binutils
export BINUTILS_COMMIT=bf7e9ff67059a35c927cc8b598b8d7f974b7d55d

#===================================================================================================
# Get Sources
#===================================================================================================

mkdir -p ${CONTRIB_DIR}
git clone ${BINUTILS_REPOSITORY} ${BINUTILS_HOME}
cd ${BINUTILS_HOME}
git checkout ${BINUTILS_COMMIT}
git clean -fdx

#===================================================================================================
# Build Binutils for Nanvix
#===================================================================================================

./configure \
    --target=$TARGET \
    --prefix=$PREFIX \
    --with-sysroot=$SYSROOT \
    --disable-multilib \
    --disable-nls \
    --disable-sim

make -j `nproc` all
make install
