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
export NANVIX_HOME=`git rev-parse --show-toplevel`
export CONTRIB_DIR=${NANVIX_HOME}/contrib
export NEWLIB_HOME=${CONTRIB_DIR}/newlib

#===================================================================================================
# Get Sources
#===================================================================================================

git submodule update --init ${NEWLIB_HOME}

#===================================================================================================
# Build Newlib
#===================================================================================================

export OLD_PATH=$PATH
export PATH=$PREFIX/bin:$PATH

cd ${NEWLIB_HOME}

git clean -fdx

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
