#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Script Arguments
#===================================================================================================

export PREFIX=${1:-$PWD/toolchain}

#===================================================================================================
# Global Variables
#===================================================================================================

export CONTRIB_DIR=${PREFIX}/src
export CPYTHON_HOME=${CONTRIB_DIR}/cpython
export CPYTHON_REPOSITORY=https://github.com/nanvix/cpython
export CPYTHON_BRANCH=8efd04f0457041f3d56a6a634e8fca73eddad2d0

#===================================================================================================
# Get Sources
#===================================================================================================

mkdir -p ${CONTRIB_DIR}
git clone ${CPYTHON_REPOSITORY} ${CPYTHON_HOME}
cd "${CPYTHON_HOME}" || exit
git checkout "${CPYTHON_BRANCH}"
git clean -fdx

#===================================================================================================
# Build CPython
#===================================================================================================

LDFLAGS='-m32' \
CFLAGS="-m32" \
./configure \
    --build=x86_64-pc-linux-gnux32 \
    --host=x86_64-pc-linux-gnux32 \
    --disable-shared \
    --disable-test-modules \
    --prefix=${PREFIX} \
    --exec-prefix=${PREFIX} \
    --with-ensurepip=no \
    --with-pkg-config=no \
    --disable-ipv6 \
    ac_cv_file__dev_ptmx=no \
    ac_cv_file__dev_ptc=no

make -j "$(nproc)" all
make install
