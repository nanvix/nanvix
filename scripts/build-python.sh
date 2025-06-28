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
export CPYTHON_HOME=${CONTRIB_DIR}/cpython
export CPYTHON_REPOSITORY=https://github.com/nanvix/cpython
export CPYTHON_BRANCH=nanvix/v3.12.3

export NANVIX_HOME=${NANVIX_HOME:-`git rev-parse --show-toplevel`}
export CROSS_DIR=${SYSROOT_DIR}/cross

#===================================================================================================
# Configure
#===================================================================================================

configure() {
    CC="$TOOLCHAIN_DIR/bin/i686-nanvix-gcc" \
    CXX="$TOOLCHAIN_DIR/bin/i686-nanvix-g++" \
    LD="$TOOLCHAIN_DIR/bin/i686-nanvix-ld" \
    LDFLAGS="-static -T $NANVIX_HOME/build/user/linker/x86/user.ld" \
    CFLAGS="-static -L $SYSROOT_DIR/lib" \
    LIBS="-Wl,--start-group $NANVIX_HOME/lib/libposix.a $TOOLCHAIN_DIR/i686-nanvix/lib/libc.a $TOOLCHAIN_DIR/i686-nanvix/lib/libm.a -lsqlite3 -lssl -lcrypto -Wl,--end-group" \
    LIBSQLITE3_LIBS="-L $SYSROOT_DIR/lib -lsqlite3" \
    LIBSQLITE3_CFLAGS="-I $SYSROOT_DIR/include" \
    ZLIB_LIBS="-L $SYSROOT_DIR/lib -lz" \
    ZLIB_CFLAGS="-I $SYSROOT_DIR/include" \
    ./configure \
        --disable-shared \
        --build=x86_64-pc-linux-gnux32 \
        --host=i686-nanvix \
        --with-build-python=${CROSS_DIR}/bin/python3 \
        --disable-test-modules \
        --with-libc=$TOOLCHAIN_DIR/i686-nanvix/lib/libc.a \
        --with-libm=$TOOLCHAIN_DIR/i686-nanvix/lib/libm.a \
        --prefix=$SYSROOT_DIR \
        --exec-prefix=$SYSROOT_DIR \
        --with-ensurepip=no \
        --with-pkg-config=no \
        --with-openssl=$SYSROOT_DIR \
        --disable-ipv6 \
        ac_cv_file__dev_ptmx=no \
        ac_cv_file__dev_ptc=no \
        ac_cv_pthread_is_default=yes \
        ac_cv_pthread=yes \
        ac_cv_kthread=no
}

#===================================================================================================
# Configure Cross
#===================================================================================================

configure_cross() {
    LDFLAGS='-m32' \
    CFLAGS="-m32" \
    ./configure \
         --build=x86_64-pc-linux-gnux32 \
        --host=x86_64-pc-linux-gnux32 \
        --disable-shared \
        --disable-test-modules \
        --prefix=${CROSS_DIR} \
        --exec-prefix=${CROSS_DIR} \
        --with-ensurepip=no \
        --with-pkg-config=no \
        --disable-ipv6 \
        ac_cv_file__dev_ptmx=no \
        ac_cv_file__dev_ptc=no
}

#===================================================================================================
# Clean
#===================================================================================================

make_clean() {
    cd ${CPYTHON_HOME}
    make clean
}

#===================================================================================================
# Clean Everything
#===================================================================================================

distclean() {
    cd ${CPYTHON_HOME}
    git clean -fdx
}

#===================================================================================================
# Make
#===================================================================================================

make_all() {
    cd ${CPYTHON_HOME}
    make -j `nproc` all
}

#===================================================================================================
# Install
#===================================================================================================

make_install() {
    cd ${CPYTHON_HOME}
    make install
}

#===================================================================================================
# Build
#===================================================================================================

build_cross() {
    cd ${CPYTHON_HOME}
    configure_cross
    make_all
    make_install
    distclean
}

build() {
    cd ${CPYTHON_HOME}

    # Check if we need to configure or not.
    if [ ! -f "${CPYTHON_HOME}/Makefile" ]; then
        configure
    else
        # Remove the existing binary to ensure it links with the updated system libraries.
        # Note: Running 'make clean' would remove all object files, which is unnecessary here.
        rm -f python
    fi

    make_all
    make_install
}

#===================================================================================================
# Init
#===================================================================================================

init() {
    mkdir -p ${CONTRIB_DIR}
    if [ ! -d "${CROSS_DIR}/bin/python3" ];
    then
        git clone ${CPYTHON_REPOSITORY} ${CPYTHON_HOME}
        git checkout ${CPYTHON_BRANCH}
        build_cross
    else
        cd ${CPYTHON_HOME}
        git fetch origin
        git reset --hard
        git checkout ${CPYTHON_BRANCH}
    fi
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
OLD_LIBS=$LIBS

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
unset LIBS

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
export LIBS=$OLD_LIBS
