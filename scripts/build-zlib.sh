#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================

# Fast fail on errors, unset variables, and pipe failures.
set -euo pipefail

#===================================================================================================
# Script Arguments
#===================================================================================================

RULE=${1:-init}
TOOLCHAIN_DIR=${2:-$PWD/toolchain}
SYSROOT_DIR=${3:-$PWD/sysroot}

#===================================================================================================
# Global Variables
#===================================================================================================

CONTRIB_DIR="${SYSROOT_DIR}/src"
REPOSITORY_HOME="${CONTRIB_DIR}/zlib"

#===================================================================================================
# Global Constants
#===================================================================================================

REPOSITORY=https://github.com/nanvix/zlib
COMMIT=fe7fae43935133eedf20a1d1e4dafe397d42a9c5

#===================================================================================================
# Functions
#===================================================================================================

init() {
    mkdir -p "${CONTRIB_DIR}"
    if [ ! -d "${REPOSITORY_HOME}/.git" ];
    then
        git clone "${REPOSITORY}" "${REPOSITORY_HOME}"
        cd "${REPOSITORY_HOME}" || exit 1
    else
        cd "${REPOSITORY_HOME}" || exit 1
        git fetch origin
        git reset --hard
    fi
    git checkout ${COMMIT}
}

build() {
    cd "${REPOSITORY_HOME}" || exit 1

    ./z configure --toolchain-path="${TOOLCHAIN_DIR}" --sysroot-path="${SYSROOT_DIR}"
    ./z build
    ./z install
}

clean() {
    cd "${REPOSITORY_HOME}" || exit 1

    ./z clean
}

#===================================================================================================
# Main Script
#===================================================================================================

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
    init)
        init
        ;;
esac

#===================================================================================================

# Save current environment variables.
OLD_PATH=$PATH
OLD_CC=$CC
OLD_CXX=$CXX
OLD_CFLAGS=$CFLAGS
OLD_CXXFLAGS=$CXXFLAGS
OLD_LD_FLAGS=$LDFLAGS

# Prepend SCCACHE's directory to PATH if the SCCACHE variable is set.
if [[ -n "${SCCACHE:-}" ]]; then
    sccache_dir="$(dirname "${SCCACHE}")"

    # Add sccache_dir to PATH if directory exists.
    if [[ -d "${sccache_dir}" ]]; then
        export PATH="${sccache_dir}:${PATH}"
    fi
fi

# Unset variables that might interfere with the build process.
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
        distclean
        ;;
    init)
        init
        ;;
esac

# Restore original environment variables.
export PATH=$OLD_PATH
export CC=$OLD_CC
export CXX=$OLD_CXX
export CFLAGS=$OLD_CFLAGS
export CXXFLAGS=$OLD_CXXFLAGS
export LDFLAGS=$OLD_LD_FLAGS
