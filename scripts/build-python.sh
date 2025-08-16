#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================

# Fast fail on errors, unset variables, and pipe failures.
set -euo pipefail

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
export CPYTHON_COMMIT=8efd04f0457041f3d56a6a634e8fca73eddad2d0

export NANVIX_HOME=${NANVIX_HOME:-$(git rev-parse --show-toplevel)}

#===================================================================================================
# Configure
#===================================================================================================

configure() {
    CC="${SCCACHE} $TOOLCHAIN_DIR/bin/i686-nanvix-gcc" \
    CXX="${SCCACHE} $TOOLCHAIN_DIR/bin/i686-nanvix-g++" \
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
        --with-build-python="${TOOLCHAIN_DIR}"/bin/python3 \
        --disable-test-modules \
        --with-libc="${TOOLCHAIN_DIR}"/i686-nanvix/lib/libc.a \
        --with-libm="${TOOLCHAIN_DIR}"/i686-nanvix/lib/libm.a \
        --prefix="${SYSROOT_DIR}" \
        --exec-prefix="${SYSROOT_DIR}" \
        --with-ensurepip=no \
        --with-pkg-config=no \
        --with-openssl="${SYSROOT_DIR}" \
        --disable-ipv6 \
        ac_cv_file__dev_ptmx=no \
        ac_cv_file__dev_ptc=no \
        ac_cv_pthread_is_default=yes \
        ac_cv_pthread=yes \
        ac_cv_kthread=no
}

#===================================================================================================
# Clean
#===================================================================================================

make_clean() {
    if [ ! -d "${CPYTHON_HOME}" ];
    then
        return 0
    fi
    cd "${CPYTHON_HOME}"
    make clean
}

#===================================================================================================
# Clean Everything
#===================================================================================================

distclean() {
    if [ ! -d "${CPYTHON_HOME}" ];
    then
        return 0
    fi
    cd "${CPYTHON_HOME}"
    git clean -fdx
}

#===================================================================================================
# Make
#===================================================================================================

make_all() {
    cd "${CPYTHON_HOME}"
    make -j "$(nproc)" all
}

#===================================================================================================
# Install
#===================================================================================================

make_install() {
    cd "${CPYTHON_HOME}"
    make install
}

#===================================================================================================
# Build
#===================================================================================================

build() {
    cd "${CPYTHON_HOME}"

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
mkdir -p "${CONTRIB_DIR}"
    if [ ! -d "${CPYTHON_HOME}/.git" ];
    then
        git clone "${CPYTHON_REPOSITORY}" "${CPYTHON_HOME}"
        cd "${CPYTHON_HOME}"
    else
        cd "${CPYTHON_HOME}"
        git fetch origin
        git reset --hard
    fi
    git checkout ${CPYTHON_COMMIT}
}

#===================================================================================================

# Save current environment variables.
OLD_CC=$CC
OLD_CXX=$CXX
OLD_CFLAGS=$CFLAGS
OLD_CXXFLAGS=$CXXFLAGS
OLD_LD_FLAGS=$LDFLAGS

# Unset variables that might interfere with the build process.
unset CC
unset CXX
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
        distclean
        ;;
    init)
        init
        ;;
esac

# Restore original environment variables.
export CC=$OLD_CC
export CXX=$OLD_CXX
export CFLAGS=$OLD_CFLAGS
export CXXFLAGS=$OLD_CXXFLAGS
export LDFLAGS=$OLD_LD_FLAGS
