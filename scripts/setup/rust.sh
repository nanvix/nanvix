#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Script Arguments
#===================================================================================================

TOOLCHAIN_DIR=${1:-$PWD/toolchain}

#===================================================================================================
# Global Variables
#===================================================================================================

NANVIX_HOME=`git rev-parse --show-toplevel`

CHANGE_ID=138986
RUST_VERSION=v1.87.0
COMMIT_ID=17067e9ac6d7ecb70e50f92c1944e545188d2359
REPOSITORY_NAME=rust
REPOSITORY=https://github.com/nanvix/rust.git
RUST_HOME=${TOOLCHAIN_DIR}/src/rust

#===================================================================================================
# Sanity Checks
#===================================================================================================

# Check if $TOOLCHAIN_DIR is within the current repository.
if [[ ${TOOLCHAIN_DIR} =~ ${NANVIX_HOME} ]]; then

    echo -e "\033[0;31mError: Build the Rust toolchain inside the Nanvix repository is not supported.\033[0m"
    echo -e "\033[0;31m       The toolchain directory must be outside the Nanvix repository.\033[0m"
    exit 1
fi

mkdir -p ${TOOLCHAIN_DIR}/src
cd ${TOOLCHAIN_DIR}/src

WASI_OS=linux
WASI_ARCH=x86_64
WASI_VERSION=25
WASI_VERSION_FULL=${WASI_VERSION}.0
wget https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-${WASI_VERSION}/wasi-sdk-${WASI_VERSION_FULL}-${WASI_ARCH}-${WASI_OS}.tar.gz
tar xvf wasi-sdk-${WASI_VERSION_FULL}-${WASI_ARCH}-${WASI_OS}.tar.gz

export WASI_SDK_PATH=`pwd`/wasi-sdk-${WASI_VERSION_FULL}-${WASI_ARCH}-${WASI_OS}

# Clone repository.
git clone ${REPOSITORY} && cd ${REPOSITORY_NAME}
git checkout ${COMMIT_ID}

export DESTDIR=${TOOLCHAIN_DIR}

# Configure the build.
./configure \
    --release-channel=nightly \
    --disable-docs \
    --disable-compiler-docs \
    --set llvm.download-ci-llvm=true \
    --enable-cargo-native-static \
    --target=x86_64-unknown-linux-gnu,wasm32-wasip1 \
    --set change-id=$CHANGE_ID

# Build the toolchain.
./x build --incremental --target x86_64-unknown-linux-gnu,wasm32-wasip1

# Set nightly cargo and link toolchain to make it available.
rustup override set nightly
rustup toolchain link nanvix-x86 build/host/stage2
