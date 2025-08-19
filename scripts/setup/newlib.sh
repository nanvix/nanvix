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
export NEWLIB_COMMIT=2093e7bd26f7b6bebd1462266413d00580a6c88b

#===================================================================================================
# Get Sources
#===================================================================================================

mkdir -p ${CONTRIB_DIR}
git clone ${NEWLIB_REPOSITORY} ${NEWLIB_HOME}
cd "${NEWLIB_HOME}" || exit
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

make -j "$(nproc)" all
make install

export PATH=$OLD_PATH

#===================================================================================================
# Copy Headers
#===================================================================================================

mkdir -p $PREFIX/usr/include
cp -r $PREFIX/$TARGET/include/* $PREFIX/usr/include/
