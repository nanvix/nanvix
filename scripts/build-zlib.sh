#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
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
export ZLIB_HOME=${CONTRIB_DIR}/zlib
export ZLIB_REPOSITORY=https://github.com/nanvix/zlib
export ZLIB_COMMIT=1780482083a52ff172f661658801fa34f1541fe7
export NANVIX_HOME=${NANVIX_HOME:-`git rev-parse --show-toplevel`}

#===================================================================================================
# Clean
#===================================================================================================

make_clean() {
    cd ${ZLIB_HOME}
    make clean
}

#===================================================================================================
# Clean Everything
#===================================================================================================

distclean() {
    cd ${ZLIB_HOME}
    git clean -fdx
}

#===================================================================================================
# Configure
#===================================================================================================

configure() {
    AR="$TOOLCHAIN_DIR/bin/i686-nanvix-ar" \
    AS="$TOOLCHAIN_DIR/bin/i686-nanvix-as" \
    CC="$TOOLCHAIN_DIR/bin/i686-nanvix-gcc" \
    CXX="$TOOLCHAIN_DIR/bin/i686-nanvix-g++" \
    CPP="$TOOLCHAIN_DIR/bin/i686-nanvix-cpp" \
    LD="$TOOLCHAIN_DIR/bin/i686-nanvix-ld" \
    CFLAGS="-Wno-error" \
    LDFLAGS="-static -T $NANVIX_HOME/build/user/linker/x86/user.ld" \
    EXTRA_LIBS="-Wl,--start-group $NANVIX_HOME/lib/libposix.a $TOOLCHAIN_DIR/i686-nanvix/lib/libc.a $TOOLCHAIN_DIR/i686-nanvix/lib/libm.a -Wl,--end-group" \
    ./configure \
        --static \
        --prefix=$SYSROOT_DIR
}

#====================================================================================================
# Make
#===================================================================================================

make_all() {
    cd ${ZLIB_HOME}
    make all
}

#===================================================================================================
# Install
#===================================================================================================

make_install() {
    cd ${ZLIB_HOME}
    make install
}

#===================================================================================================
# Build
#===================================================================================================

build() {
    cd ${ZLIB_HOME}
    configure
    make_all
    make_install
}

#===================================================================================================
# Init
#===================================================================================================

init() {
    mkdir -p ${CONTRIB_DIR}
    if [ ! -d "${ZLIB_HOME}/.git" ];
    then
        git clone ${ZLIB_REPOSITORY} ${ZLIB_HOME}
        cd ${ZLIB_HOME}
    else
        cd ${ZLIB_HOME}
        git fetch origin
        git reset --hard
    fi
    git checkout ${ZLIB_COMMIT}
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
