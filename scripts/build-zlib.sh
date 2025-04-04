#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Script Arguments
#===================================================================================================

RULE=${1:-build}
NANVIX_HOME=${2:-$PWD}
TOOLCHAIN_DIR=${3:-$PWD/toolchain}
SYSROOT_DIR=${4:-$PWD/sysroot}

#===================================================================================================
# Global Variables
#===================================================================================================

ZLIB_VERSION=1.3.1
OPT_DIR=$NANVIX_HOME/opt
ZLIB_HOME=$OPT_DIR/zlib
BRANCH_NAME=nanvix/zlib-$ZLIB_VERSION

#===================================================================================================
# Clean
#===================================================================================================

clean() {
    make clean
}

#===================================================================================================
# Clean Everything
#===================================================================================================

distclean() {
    git clean -fdx
}

#===================================================================================================
# Build
#===================================================================================================

build() {
    # Configure.
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

    # Build.
    make all

    # Install.
    make install
}

#===================================================================================================

# Fetch submodule if needed and enter source directory.
git submodule update --init $ZLIB_HOME && cd $ZLIB_HOME

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
        clean
        ;;
    distclean)
        distclean
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
