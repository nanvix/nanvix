#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Script Arguments
#===================================================================================================

RULE=${1:-build}
TOOLCHAIN_DIR=${2:-$PWD/toolchain}
SYSROOT_DIR=${3:-$PWD/sysroot}

#===================================================================================================
# Global Variables
#===================================================================================================

export CONTRIB_DIR=${SYSROOT_DIR}/src
export OPENBLAS_HOME=${CONTRIB_DIR}/openblas
export OPENBLAS_REPOSITORY=https://github.com/nanvix/openblas
export OPENBLAS_COMMIT=f6026cdcc72df936edee97c9b3e628f9735a0a14

#===================================================================================================
# Configure
#===================================================================================================

configure() {
    # OpenBLAS uses make variables instead of configure script
    export OPENBLAS_MAKE_OPTIONS="\
        CC=${TOOLCHAIN_DIR}/bin/i686-nanvix-gcc \
        FC=${TOOLCHAIN_DIR}/bin/i686-nanvix-gfortran \
        PREFIX=${SYSROOT_DIR} \
        HOSTCC=gcc \
        TARGET=P2 \
        BINARY=32 \
        CROSS=1 \
        NO_SHARED=1 \
        USE_OPENMP=0 \
        USE_THREAD=0 \
        USE_LOCKING=1 \
        USE_TLS=0"
}

#===================================================================================================
# Clean
#===================================================================================================

make_clean() {
    if [ ! -d "${OPENBLAS_HOME}" ];
    then
        return 0
    fi
    cd "${OPENBLAS_HOME}"
    make ${OPENBLAS_MAKE_OPTIONS} clean
}

#===================================================================================================
# Clean Everything
#===================================================================================================

distclean() {
    if [ ! -d "${OPENBLAS_HOME}" ];
    then
        return 0
    fi
    cd "${OPENBLAS_HOME}"
    git clean -fdx
}

#===================================================================================================
# Make
#===================================================================================================

make_all() {
    cd "${OPENBLAS_HOME}"
    make ${OPENBLAS_MAKE_OPTIONS} all
}

#===================================================================================================
# Install
#===================================================================================================

make_install() {
    cd "${OPENBLAS_HOME}"
    make ${OPENBLAS_MAKE_OPTIONS} install
}

#===================================================================================================
# Build
#===================================================================================================


build() {
    cd "${OPENBLAS_HOME}"
    configure
    make_all
    make_install
}

#===================================================================================================
# Init
#===================================================================================================

init() {
    mkdir -p ${CONTRIB_DIR}
    if [ ! -d "${OPENBLAS_HOME}/.git" ];
    then
        git clone ${OPENBLAS_REPOSITORY} ${OPENBLAS_HOME}
        cd "${OPENBLAS_HOME}"
    else
        cd "${OPENBLAS_HOME}"
        git fetch origin
        git reset --hard
    fi
    git checkout ${OPENBLAS_COMMIT}
}

#===================================================================================================

# Save current environment variables.
OLD_AR=$AR
OLD_AS=$AS
OLD_CC=$CC
OLD_CXX=$CXX
OLD_CPP=$CPP
OLD_LD=$LD
OLD_CFLAGS=$CFLAGS
OLD_CXXFLAGS=$CXXFLAGS
OLD_LD_FLAGS=$LDFLAGS
OLD_LIBC=$LIBC
OLD_LIBM=$LIBM

# Unset variables that might interfere with the build process.
unset AR
unset AS
unset CC
unset CXX
unset CPP
unset LD
unset CFLAGS
unset CXXFLAGS
unset LDFLAGS
unset LIBC
unset LIBM

case $RULE in
    build)
        build
        ;;
    clean)
        make_clean
        ;;
    distclean)
        distclean
        ;;
    init)
        init
        ;;
esac

# Restore original environment variables.
export AR=$OLD_AR
export AS=$OLD_AS
export CC=$OLD_CC
export CXX=$OLD_CXX
export CPP=$OLD_CPP
export LD=$OLD_LD
export CFLAGS=$OLD_CFLAGS
export CXXFLAGS=$OLD_CXXFLAGS
export LDFLAGS=$OLD_LD_FLAGS
export LIBC=$OLD_LIBC
export LIBM=$OLD_LIBM
