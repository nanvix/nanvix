#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Script Arguments
#===================================================================================================

export PREFIX=${1:-$PWD/toolchain}

#===================================================================================================
# Get Sources
#===================================================================================================

mkdir -p $PREFIX/src && cd $PREFIX/src

git clone https://github.com/nanvix/binutils.git --branch nanvix/binutils-2.40 binutils
git clone https://github.com/nanvix/gcc.git --branch nanvix/gcc-12.4.0 gcc
git clone https://github.com/nanvix/newlib.git --branch nanvix/newlib-4.4.0 newlib

#===================================================================================================
# Build Standalone Binutils
#===================================================================================================

export TARGET=i686-elf

cd $PREFIX/src/binutils

git clean -fdx

./configure \
    --target=$TARGET \
    --prefix=$PREFIX \
    --disable-multilib \
    --disable-nls \
    --disable-sim

make -j `nproc` all
make install

#===================================================================================================
# Build Standalone GCC
#===================================================================================================

export TARGET=i686-elf

cd $PREFIX/src/gcc

git clean -fdx

./contrib/download_prerequisites

mkdir -p build && cd build

../configure \
    --target=$TARGET \
    --prefix=$PREFIX \
    --disable-multilib \
    --disable-nls \
    --enable-languages=c  \
    --without-headers

make -j `nproc` all-gcc all-target-libgcc
make install-gcc install-target-libgcc

#===================================================================================================
# Create Symbolic Links to Standalone Toolchain to Fool Newlib
#===================================================================================================

export TARGET=i686-elf

cd $PREFIX/bin

for f in $TARGET-*; do
    filename=`echo $f | sed -e 's/-elf-/-nanvix-/g'`
    ln -s $f $filename
done

#===================================================================================================
# Build Newlib
#===================================================================================================

export TARGET=i686-nanvix

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
# Populate SYSROOT
#===================================================================================================

export TARGET=i686-nanvix
export SYSROOT=$PREFIX

mkdir -p $PREFIX/usr/include
cp -r $PREFIX/$TARGET/include/* $PREFIX/usr/include

#===================================================================================================
# Build Binutils for Nanvix
#===================================================================================================

export TARGET=i686-nanvix
export SYSROOT=$PREFIX

cd $PREFIX/src/binutils

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

export TARGET=i686-nanvix
export SYSROOT=$PREFIX

cd $PREFIX/src/gcc

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

make -j `nproc` all-gcc all-target-libgcc all-target-libstdc++-v3
make -j install-gcc install-target-libgcc install-target-libstdc++-v3
