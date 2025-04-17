#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Script Arguments
#===================================================================================================

RULE=${1:-build}
NANVIX_HOME=${2:-`git rev-parse --show-toplevel`}
TOOLCHAIN_DIR=${3:-$PWD/toolchain}
SYSROOT_DIR=${4:-$PWD/sysroot}

#===================================================================================================
# Global Variables
#===================================================================================================

OPT_DIR=${NANVIX_HOME}/opt
OPENBLAS_HOME=${OPT_DIR}/openblas
OPENBLAS_MAKE_OPTIONS="\
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

#===================================================================================================
# Clean
#===================================================================================================

make_clean() {
    make ${OPENBLAS_MAKE_OPTIONS} clean
}

#===================================================================================================
# Clean Everything
#===================================================================================================

distclean() {
	git clean -fdx
}

#===================================================================================================
# Make
#===================================================================================================

make_all() {
    make ${OPENBLAS_MAKE_OPTIONS} all
}

#===================================================================================================
# Install
#===================================================================================================

make_install() {
    make ${OPENBLAS_MAKE_OPTIONS} install
}

#===================================================================================================
# Build
#===================================================================================================


build() {
	make_all

	make_install
}

#===================================================================================================
# Init
#===================================================================================================

init() {
	# Nothing to do here.
	return
}

#===================================================================================================

# Fetch submodule if needed.
git submodule update --init $OPENBLAS_HOME

# Switch to submodule directory.
cd ${OPENBLAS_HOME}

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
