#!/usr/bin/env bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# In this script we generate the initramfs for the System (L2) VM. The initramfs is
# minimal, and intended to boot fast into linuxd.elf, which is executed as the
# /init process.

set -euo pipefail

#===================================================================================================
# Global Variables
#===================================================================================================

NANVIX_HOME=$(git rev-parse --show-toplevel)
IMAGES_DIR="${NANVIX_HOME}/images"
INITRAMFS_IMAGE="${IMAGES_DIR}/l2_sysvm_initramfs.img"
INITRAMFS_DIR="${IMAGES_DIR}/l2-sysvm-rootfs"

#===================================================================================================
# Utilities
#===================================================================================================

source "${NANVIX_HOME}/scripts/common/logging.sh"
source "${NANVIX_HOME}/scripts/common/utils.sh"

#===================================================================================================
# Command line arguments
#===================================================================================================

# Parse command line arguments.
CLEAN=false
for arg in "$@"; do
  if [ "$arg" = "--clean" ]; then
    CLEAN=true
  fi
done

# Do a clean build if requested.
if ${CLEAN}; then
    if [ -d ${INITRAMFS_DIR} ]; then
        print_warning "removing initramfs from ${INITRAMFS_DIR}"
        rm -rf ${INITRAMFS_DIR}
    fi
fi

#===================================================================================================
# Socket address parsing
#===================================================================================================

LINUXD_CONFIG_TOML="${NANVIX_HOME}/build/linuxd_config.toml"
GUEST_TAP_IP_ADDRESS=$(get_value_from_toml "${LINUXD_CONFIG_TOML}" "guest_tap_ip_address")
HOST_TAP_IP_ADDRESS=$(get_value_from_toml "${LINUXD_CONFIG_TOML}" "host_tap_ip_address")
CONTROL_PLANE_PORT=$(get_value_from_toml "${LINUXD_CONFIG_TOML}" "control_plane_port")
USER_VM_PORT=$(get_value_from_toml "${LINUXD_CONFIG_TOML}" "user_vm_port")

CONTROL_PLANE_SOCKADDR="${HOST_TAP_IP_ADDRESS}:${CONTROL_PLANE_PORT}"
USER_VM_SOCKADDR="${GUEST_TAP_IP_ADDRESS}:${USER_VM_PORT}"

#===================================================================================================
# Build initramfs
#===================================================================================================

UBUNTU_VERSION="jammy"
UBUNTU_COMPONENTS="main,restricted,universe,multiverse"
# Robust mirror detection, and fallback to a safe default.
UBUNTU_MIRROR="$(
  grep -m1 -Po '^URIs:\s*\K(\S+)' /etc/apt/sources.list.d/ubuntu.sources 2>/dev/null \
  | sed -E 's/^\[|]$//g' | awk '{print $1}'
)"
: "${UBUNTU_MIRROR:=http://archive.ubuntu.com/ubuntu}"
export DEBIAN_FRONTEND=noninteractive

build_initramfs() {
    mkdir -p ${IMAGES_DIR}

    if [ ! -d "$INITRAMFS_DIR" ]; then
        print_info "Creating minimal ubuntu (${UBUNTU_VERSION}, mirror: ${UBUNTU_MIRROR}) initramfs..."
        sudo -E mmdebstrap \
            --variant=minbase \
            --components="${UBUNTU_COMPONENTS}" \
            --include=busybox-static,iputils-arping,libc6,libgcc-s1,libstdc++6,iproute2,jq \
            "${UBUNTU_VERSION}" "${INITRAMFS_DIR}" "${UBUNTU_MIRROR}"
        sudo chown -R "$(id -u):$(id -g)" "${INITRAMFS_DIR}"

        print_info "Creating /dev and /proc mount points..."
        mkdir -p "${INITRAMFS_DIR}"/{proc,bin,sbin,sys,dev,etc}
        mkdir -p "${INITRAMFS_DIR}/usr"/{bin,lib}
        mkdir -p "${INITRAMFS_DIR}/etc/scripts"
    fi

    # Copy pre-built files and libraries into the L2 initramfs.
    cp "${SYSROOT_DIR}/bin/linuxd.elf" "${INITRAMFS_DIR}/usr/bin/linuxd.elf"
    cp -r "${SYSROOT_DIR}/lib/python3.12" "${INITRAMFS_DIR}/usr/lib/"

cat >/tmp/init <<EOF
#!/bin/sh

echo "[init] Nanvix L2 System VM init wrapper started!"

# Set-up any resources that linuxd needs when running in the L2-VM.

# We must bind to the same IP, as it is the only one available in the guest.
RUST_LOG=${LOG_LEVEL:-warn} /usr/bin/linuxd.elf \
    -control-plane-addr ${CONTROL_PLANE_SOCKADDR} \
    -control-plane-socket-type tcp \
    -user-vm-bind-addr ${USER_VM_SOCKADDR} \
    -user-vm-bind-socket-type tcp \
    -l2

echo "[init] Nanvix L2 System VM shutting down!"
busybox poweroff -f
EOF
    cp /tmp/init "$INITRAMFS_DIR/init"
    chmod +x "$INITRAMFS_DIR/init"

    print_info "Creating initramfs..."
    cd "${INITRAMFS_DIR}"
    find . | sudo cpio -H newc -o | gzip -9 > "$INITRAMFS_IMAGE"
    cd -

    print_success "initramfs stored in: ${INITRAMFS_IMAGE}"
}

build_initramfs
