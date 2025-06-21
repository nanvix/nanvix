#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Script Arguments
#===================================================================================================

export STAGE=${1:-stage0}
export PREFIX=${2:-$PWD/toolchain}

#===================================================================================================
# Environment Variables
#===================================================================================================

export TARGET=i686-nanvix
export SYSROOT=$PREFIX
export CONTRIB_DIR=${PREFIX}/src
export GCC_HOME=${CONTRIB_DIR}/gcc
export GCC_REPOSITORY=https://github.com/nanvix/gcc
export GCC_COMMIT=b0222fe731e2888cb26737e12c528818ec92cb81

#===================================================================================================
# stage0
#===================================================================================================

stage0() {
    mkdir -p ${CONTRIB_DIR}
    git clone ${GCC_REPOSITORY} ${GCC_HOME}
    cd ${GCC_HOME}
    git checkout ${GCC_COMMIT}
    git clean -fdx

    ./contrib/download_prerequisites

    mkdir -p build && cd build

    ../configure \
        --target=$TARGET \
        --prefix=$PREFIX \
        --with-sysroot=$SYSROOT \
        --disable-multilib \
        --disable-nls \
        --enable-languages=c,c++,fortran  \
        --disable-libquadmath \
        --disable-libquadmath-support \
        --with-newlib

    make -j `nproc` all-gcc all-target-libgcc
    make install-gcc install-target-libgcc
}

#===================================================================================================
# stage1
#===================================================================================================

stage1() {

    # We must rebuild GCC to have fix-includes are fixed.
    # Note this time we also enable libstdc++ compilation.

    cd ${GCC_HOME}/build

    ../configure \
        --target=$TARGET \
        --prefix=$PREFIX \
        --with-sysroot=$SYSROOT \
        --disable-multilib \
        --disable-nls \
        --enable-languages=c,c++,fortran  \
        --disable-libquadmath \
        --disable-libquadmath-support \
        --with-newlib

    make -j `nproc` all-gcc all-target-libgcc all-target-libgfortran all-target-libstdc++-v3
    make install-gcc install-target-libgcc install-target-libgfortran install-target-libstdc++-v3

}

#===================================================================================================
# usage
#===================================================================================================

usage() {
    echo "Usage: $0 {stage0|stage1} [prefix]"
}

#===================================================================================================
# Main
#===================================================================================================


case $STAGE in
    stage0)
        stage0
        ;;
    stage1)
        stage1
        ;;
    *)
        usage
        exit 1
        ;;
esac
