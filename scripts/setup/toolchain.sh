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
export NEWLIB_HOME=${CONTRIB_DIR}/newlib
export BINUTILS_HOME=${CONTRIB_DIR}/binutils
export GCC_HOME=${CONTRIB_DIR}/gcc

#===================================================================================================
# Get Sources
#===================================================================================================

git submodule update --init ${NEWLIB_HOME}
git submodule update --init ${BINUTILS_HOME}
git submodule update --init ${GCC_HOME}

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

#===================================================================================================
# Build GCC for Nanvix
#===================================================================================================

cd ${GCC_HOME}

git clean -fdx

./contrib/download_prerequisites

mkdir -p build && cd build

../configure \
    --target=$TARGET \
    --prefix=$PREFIX \
    --with-sysroot=$SYSROOT \
    --disable-multilib \
    --disable-nls \
    --enable-languages=c,c++  \
    --with-newlib

make -j `nproc` all-gcc all-target-libgcc
make install-gcc install-target-libgcc

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

#===================================================================================================
# Rebuild GCC for Nanvix
#===================================================================================================

# We must rebuild GCC so fix-includes are actually fixed.
# Note this time we also enable libstdc++ compilation.

cd ${GCC_HOME}/build

../configure \
    --target=$TARGET \
    --prefix=$PREFIX \
    --with-sysroot=$SYSROOT \
    --disable-multilib \
    --disable-nls \
    --enable-languages=c,c++  \
    --with-newlib

make -j `nproc` all-gcc all-target-libgcc all-target-libstdc++-v3
make install-gcc install-target-libgcc install-target-libstdc++-v3
