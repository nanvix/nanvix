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
export CONTRIB_DIR=${PREFIX}/src
export NEWLIB_HOME=${CONTRIB_DIR}/newlib
export NEWLIB_REPOSITORY=https://github.com/nanvix/newlib
export NEWLIB_COMMIT=3e6c5f6d86b5fbfb7019f5895f7c5ffd3e8dcbaa

#===================================================================================================
# Get Sources
#===================================================================================================

mkdir -p ${CONTRIB_DIR}
git clone ${NEWLIB_REPOSITORY} ${NEWLIB_HOME}
cd ${NEWLIB_HOME}
git checkout ${NEWLIB_COMMIT}
git clean -fdx

#===================================================================================================
# Build Newlib
#===================================================================================================

export OLD_PATH=$PATH
export PATH=$PREFIX/bin:$PATH


./configure \
    --target=$TARGET \
    --prefix=$PREFIX \
    --disable-multilib

make -j `nproc` all
make install

export PATH=$OLD_PATH

#===================================================================================================
# Copy Headers
#===================================================================================================

mkdir -p $PREFIX/usr/include
cp -r $PREFIX/$TARGET/include/* $PREFIX/usr/include/
