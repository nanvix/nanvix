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
export OPENSSL_HOME=${CONTRIB_DIR}/openssl
export OPENSSL_REPOSITORY=https://github.com/nanvix/openssl
export OPENSSL_COMMIT=b4773b592552f9d3c77fbd0e36509e3fcd030536

#===================================================================================================
# Configure
#===================================================================================================

configure() {
    CFLAGS="-I $TOOLCHAIN_DIR/usr/include/" \
    CXX=$TOOLCHAIN_DIR/bin/i686-nanvix-g++ \
    AR=$TOOLCHAIN_DIR/bin/i686-nanvix-ar \
    RANLIB=$TOOLCHAIN_DIR/bin/i686-nanvix-ranlib \
    CC=$TOOLCHAIN_DIR/bin/i686-nanvix-gcc \
    ./Configure \
        --openssldir=$SYSROOT_DIR \
        --prefix=$SYSROOT_DIR \
        nanvix \
        no-shared \
        threads \
        no-dso \
        no-apps \
        no-docs \
        no-rdrand \
        no-posix-io \
        no-asm \
        no-ui-console
}

#===================================================================================================
# Clean
#===================================================================================================

make_clean() {
    cd ${OPENSSL_HOME}
    make clean
}

#===================================================================================================
# Clean Everything
#===================================================================================================

distclean() {
    cd ${OPENSSL_HOME}
    git clean -fdx
}

#===================================================================================================
# Make
#===================================================================================================

make_all() {
    cd ${OPENSSL_HOME}

    make -j $(nproc) all
}

#===================================================================================================
# Install
#===================================================================================================

make_install() {
    cd ${OPENSSL_HOME}
    make install
}

#===================================================================================================
# Build
#===================================================================================================


build() {
    cd ${OPENSSL_HOME}
    configure
    make_all
    make_install
}

#===================================================================================================
# Init
#===================================================================================================

init() {
    mkdir -p ${CONTRIB_DIR}
    if [ ! -d "${OPENSSL_HOME}/.git" ];
    then
        git clone ${OPENSSL_REPOSITORY} ${OPENSSL_HOME}
        cd ${OPENSSL_HOME}
    else
        cd ${OPENSSL_HOME}
        git fetch origin
        git reset --hard
    fi
    git checkout ${OPENSSL_COMMIT}
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
