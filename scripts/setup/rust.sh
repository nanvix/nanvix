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

NANVIX_HOME=$(git rev-parse --show-toplevel)

CHANGE_ID=138986
COMMIT_ID=0a7e34697b651d08007108b54e7143744a8115a3
REPOSITORY_NAME=rust
REPOSITORY=https://github.com/nanvix/rust.git

#===================================================================================================
# Sanity Checks
#===================================================================================================

# Check if $TOOLCHAIN_DIR is within the current repository.
if [[ "${TOOLCHAIN_DIR}" == "${NANVIX_HOME}"/* || "${TOOLCHAIN_DIR}" == "${NANVIX_HOME}" ]];
then
    echo -e "\033[0;31mError: Building the Rust toolchain inside the Nanvix repository is not supported.\033[0m"
    echo -e "\033[0;31m       The toolchain directory must be outside the Nanvix repository.\033[0m"
    exit 1
fi

# Check if rustup is installed.
if ! command -v rustup >/dev/null 2>&1;
then
    echo -e "\033[0;31mError: rustup is not installed. Please install rustup before running this script.\033[0m"
    exit 1
fi

mkdir -p "${TOOLCHAIN_DIR}"/src
cd "${TOOLCHAIN_DIR}/src" || exit

WASI_OS=linux
WASI_ARCH=x86_64
WASI_VERSION=25
WASI_VERSION_FULL=${WASI_VERSION}.0
WASI_SDK_FILE=wasi-sdk-${WASI_VERSION_FULL}-${WASI_ARCH}-${WASI_OS}.tar.gz

# Get WASI SDK.
if [ ! -f "${WASI_SDK_FILE}" ];
then
    wget https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-${WASI_VERSION}/${WASI_SDK_FILE}
fi

# Check file was downloaded successfully.
if [ ! -f "${WASI_SDK_FILE}" ];
then
    echo -e "\033[0;31mError: Failed to download ${WASI_SDK_FILE}.\033[0m"
    exit 1
fi

if [ ! -d "wasi-sdk-${WASI_VERSION_FULL}-${WASI_ARCH}-${WASI_OS}" ];
then
    tar xvf "${WASI_SDK_FILE}"
fi

WASI_SDK_PATH=
WASI_SDK_PATH="$(pwd)/wasi-sdk-${WASI_VERSION_FULL}-${WASI_ARCH}-${WASI_OS}"
export WASI_SDK_PATH

# Clone repository.
if [ ! -d "${REPOSITORY_NAME}" ];
then
    git clone "${REPOSITORY}" "${REPOSITORY_NAME}"
fi
cd "${REPOSITORY_NAME}" || exit
git checkout ${COMMIT_ID}
export DESTDIR=${TOOLCHAIN_DIR}

# Configure the build.
./configure \
    --release-channel=nightly \
    --disable-docs \
    --disable-compiler-docs \
    --set llvm.download-ci-llvm=false \
    --enable-cargo-native-static \
    --target=x86_64-unknown-linux-gnu,wasm32-wasip1,i686-unknown-nanvix \
    --set change-id=$CHANGE_ID

# Build the toolchain.
./x build --incremental --target x86_64-unknown-linux-gnu,wasm32-wasip1,i686-unknown-nanvix

# Install nightly toolchain if not installed and set it as the default.
if ! rustup toolchain list | grep -q '^nightly'; then
    rustup install nightly
fi
rustup override set nightly

# Check if the toolchain was built successfully.
if [ -d "build/host/stage2" ]; then
    rustup toolchain link nanvix-x86 build/host/stage2
else
    echo -e "\033[0;31mError: build/host/stage2 directory does not exist. Toolchain link skipped.\033[0m"
    exit 1
fi
