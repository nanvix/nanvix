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
export SQLITE_HOME=${CONTRIB_DIR}/sqlite
export SQLITE_REPOSITORY=https://github.com/nanvix/sqlite
export SQLITE_COMMIT=eefd69769fef1cb3d29d63e89a62a8b2d0ef8b0d
export NANVIX_HOME=${NANVIX_HOME:-`git rev-parse --show-toplevel`}

#===================================================================================================
# Clean
#===================================================================================================

make_clean() {
    if [ ! -d "${SQLITE_HOME}" ];
    then
        return 0
    fi
    cd "${SQLITE_HOME}"
    make clean
}

#===================================================================================================
# Clean Everything
#===================================================================================================

distclean() {
    if [ ! -d "${SQLITE_HOME}" ];
    then
        return 0
    fi
    cd "${SQLITE_HOME}"
    git clean -fdx
}

#===================================================================================================
# Configure
#===================================================================================================

configure() {
    AR="$TOOLCHAIN_DIR/bin/i686-nanvix-ar" \
    AS="$TOOLCHAIN_DIR/bin/i686-nanvix-as" \
    CC_FOR_BUILD=gcc \
    CC="$TOOLCHAIN_DIR/bin/i686-nanvix-gcc" \
    CXX="$TOOLCHAIN_DIR/bin/i686-nanvix-g++" \
    CPP="$TOOLCHAIN_DIR/bin/i686-nanvix-cpp" \
    LD="$TOOLCHAIN_DIR/bin/i686-nanvix-ld" \
    CFLAGS="-I $SYSROOT_DIR/include -DSQLITE_OMIT_WAL=1" \
    LDFLAGS="-static -T $NANVIX_HOME/build/user/linker/x86/user.ld -L $SYSROOT_DIR/lib -Wl,--start-group $NANVIX_HOME/lib/libposix.a $TOOLCHAIN_DIR/i686-nanvix/lib/libc.a $TOOLCHAIN_DIR/i686-nanvix/lib/libm.a $SYSROOT_DIR/lib/libz.a -Wl,--end-group" \
    LIBS="-Wl,--start-group $NANVIX_HOME/lib/libposix.a $TOOLCHAIN_DIR/i686-nanvix/lib/libc.a $TOOLCHAIN_DIR/i686-nanvix/lib/libm.a $SYSROOT_DIR/lib/libz.a -Wl,--end-group" \
    ./configure \
        --disable-shared \
        --sysroot=$SYSROOT_DIR \
        --prefix=$SYSROOT_DIR \
        --host=i686-nanvix \
        --disable-tcl \
        --disable-threadsafe
}

#===================================================================================================
# Make
#===================================================================================================

make_all() {
    cd "${SQLITE_HOME}"
    make all
}

#===================================================================================================
# Install
#===================================================================================================

make_install() {
    cd "${SQLITE_HOME}"
    make install
}

#===================================================================================================
# Build
#===================================================================================================

build() {
    cd "${SQLITE_HOME}"
    configure
    make_all
    make_install
}

#===================================================================================================
# Init
#===================================================================================================

init() {
    mkdir -p ${CONTRIB_DIR}
    if [ ! -d "${SQLITE_HOME}/.git" ];
    then
        git clone ${SQLITE_REPOSITORY} ${SQLITE_HOME}
        cd "${SQLITE_HOME}"
    else
        cd "${SQLITE_HOME}"
        git fetch origin
        git reset --hard
    fi
    git checkout ${SQLITE_COMMIT}
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
