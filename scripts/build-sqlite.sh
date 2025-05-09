#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
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
SQLITE_HOME=${OPT_DIR}/sqlite

#===================================================================================================
# Clean
#===================================================================================================

make_clean() {
    make clean
}

#===================================================================================================
# Clean Everything
#===================================================================================================

make_distclean() {
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

#====================================================================================================
# Make
#===================================================================================================

make_all() {
    make all
}

#===================================================================================================
# Make Install
#===================================================================================================

make_install() {
    make install
}

#===================================================================================================
# Build
#===================================================================================================

build() {
    make_distclean

    configure

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
git submodule update --init ${SQLITE_HOME}

# Switch to submodule directory.
cd ${SQLITE_HOME}

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

# Unset environment variables that might interfere with the build.
unset AR
unset AS
unset CC
unset CXX
unset CPP
unset LD
unset CFLAGS
unset CXXFLAGS
unset LDFLAGS

case $RULE in
    build)
        build
        ;;
    clean)
        make_clean
        ;;
    distclean)
        make_distclean
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
