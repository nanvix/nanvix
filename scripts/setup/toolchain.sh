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

#===================================================================================================
# Get Sources
#===================================================================================================

mkdir -p $PREFIX/src && cd $PREFIX/src

git clone https://github.com/nanvix/binutils.git --branch nanvix/binutils-2.40 binutils
git clone https://github.com/nanvix/gcc.git --branch nanvix/gcc-12.4.0 gcc
git clone https://github.com/nanvix/newlib.git --branch nanvix/newlib-4.4.0 newlib

#===================================================================================================
# Build Binutils for Nanvix
#===================================================================================================

cd $PREFIX/src/binutils

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

cd $PREFIX/src/gcc

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

make -j `nproc` all-gcc all-target-libgcc all-target-libstdc++-v3
make install-gcc install-target-libgcc install-target-libstdc++-v3

#===================================================================================================
# Build Newlib
#===================================================================================================

export OLD_PATH=$PATH
export PATH=$PREFIX/bin:$PATH

cd $PREFIX/src/newlib

./configure \
    --target=$TARGET \
    --prefix=$PREFIX \
    --disable-multilib

make -j `nproc` all
make install

export PATH=$OLD_PATH

#===================================================================================================
# Build Libstdc++ for Nanvix
#===================================================================================================

cd $PREFIX/src/gcc/build

make -j `nproc` all-target-libstdc++-v3
make install-target-libstdc++-v3
