#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# This script builds our fork of cloud-hypervisor as well as the linuxd kernel used for direct boot.

set -euo pipefail

#===================================================================================================
# Script Arguments
#===================================================================================================

PREFIX=${1:-$PWD/toolchain}

#===================================================================================================
# Global Variables
#===================================================================================================

SRC_DIR="${PREFIX}/src"
BIN_DIR="${PREFIX}/bin"
SHARE_DIR="${PREFIX}/share/cloud-hypervisor"
L2_SYSTEM_VM_KERNEL="${SHARE_DIR}/l2_sysvm_vmlinux.bin"

# Cloud-hypervisor variables.
CLOUD_HYPERVISOR_HOME="${SRC_DIR}/cloud-hypervisor"
CLOUD_HYPERVISOR_REPOSITORY="https://github.com/nanvix/cloud-hypervisor"
CLOUD_HYPERVISOR_COMMIT="4681d5eba0e63fb010bf4ca962eb3c9f163d3288"

# Cloud-hypervisor linuxd kernel.
CLOUD_HYPERVISOR_LINUX_HOME="${SRC_DIR}/cloud-hypervisor-linux"
CLOUD_HYPERVISOR_LINUX_REPOSITORY="https://github.com/cloud-hypervisor/linux"
CLOUD_HYPERVISOR_LINUX_TAG="ch-6.12.8"

#===================================================================================================
# Build
#===================================================================================================

build_clh() {
    mkdir -p ${SRC_DIR}
    if [ ! -d "${CLOUD_HYPERVISOR_HOME}" ];
    then
        git clone ${CLOUD_HYPERVISOR_REPOSITORY} ${CLOUD_HYPERVISOR_HOME}
        pushd ${CLOUD_HYPERVISOR_HOME} >> /dev/null
        git checkout ${CLOUD_HYPERVISOR_COMMIT}
    else
        pushd ${CLOUD_HYPERVISOR_HOME} >> /dev/null
        git fetch origin
        git reset --hard ${CLOUD_HYPERVISOR_COMMIT}
    fi

    cargo build --release

    # Copy built binaries.
    mkdir -p ${BIN_DIR}
    cp ./target/release/cloud-hypervisor "${BIN_DIR}/cloud-hypervisor"
    cp ./target/release/ch-remote "${BIN_DIR}/ch-remote"

    # Give cloud-hypervisor binary CAP_NET_ADMIN to create TAP devices without sudo.
    # Make the operation resilient to builds in a docker environment without sudo.
    if command -v sudo >/dev/null 2>&1; then
        sudo setcap cap_net_admin+ep "${BIN_DIR}/cloud-hypervisor"
    else
        if ! setcap cap_net_admin+ep "${BIN_DIR}/cloud-hypervisor"; then
            echo "WARNING: failed to set cap_net_admin on ${BIN_DIR}/cloud-hypervisor. You may need to run this script as root or install sudo." >&2
        fi
    fi

    popd >> /dev/null
}

build_clh_linux() {
    mkdir -p ${SRC_DIR}
    if [ ! -d "${CLOUD_HYPERVISOR_LINUX_HOME}" ]; then
        git clone \
            --depth 1 \
            ${CLOUD_HYPERVISOR_LINUX_REPOSITORY} \
            -b ${CLOUD_HYPERVISOR_LINUX_TAG} \
            ${CLOUD_HYPERVISOR_LINUX_HOME}
        pushd ${CLOUD_HYPERVISOR_LINUX_HOME} >> /dev/null
    else
        pushd ${CLOUD_HYPERVISOR_LINUX_HOME} >> /dev/null
        git reset --hard ${CLOUD_HYPERVISOR_LINUX_TAG}
    fi

    # Build linuxd kernel.
    make ch_defconfig
    KCFLAGS="-Wa,-mx86-used-note=no" make bzImage -j "$(nproc)"

    # Copy built image.
    mkdir -p ${SHARE_DIR}
    cp "${CLOUD_HYPERVISOR_LINUX_HOME}/arch/x86/boot/bzImage" "${L2_SYSTEM_VM_KERNEL}"

    popd >> /dev/null
}

build() {
    build_clh
    build_clh_linux
}

build
