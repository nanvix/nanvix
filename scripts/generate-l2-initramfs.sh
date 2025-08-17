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
IMAGES_DIR="${NANVIX_HOME}/build/images"
INITRAMFS_IMAGE="${IMAGES_DIR}/l2_sysvm_initramfs.img"
INITRAMFS_DIR="${IMAGES_DIR}/l2-sysvm-rootfs"
LINUXD_ELF="${NANVIX_HOME}/bin/linuxd.elf"

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
        echo "WARN: removing initramfs from ${INITRAMFS_DIR}"
        rm -rf ${INITRAMFS_DIR}
    fi
fi

#===================================================================================================
# Socket address parsing
#===================================================================================================

# FIXME (#839): these values are currently hard-coded here and in src/utils/nanvixd/src/config.rs
CONTROL_PLANE_SOCKADDR="192.168.249.3:9000"
USER_VM_SOCKADDR="192.168.249.2:9001"
GATEWAY_SOCKADDR="192.168.249.2:9002"

#===================================================================================================
# Build initramfs
#===================================================================================================

UBUNTU_VERSION="jammy"
UBUNTU_MIRROR=$(awk '/^URIs:/ { print $2; exit }' /etc/apt/sources.list.d/ubuntu.sources)

build_initramfs() {
    mkdir -p ${IMAGES_DIR}

    if [ ! -d "$INITRAMFS_DIR" ]; then
        echo "Creating minimal ubuntu (${UBUNTU_VERSION}, mirror: ${UBUNTU_MIRROR}) initramfs..."
        sudo debootstrap \
            --variant=minbase \
            --include=busybox-static,iputils-arping,libc6,libgcc-s1,libstdc++6,iproute2,jq \
            "${UBUNTU_VERSION}" "${INITRAMFS_DIR}" "${UBUNTU_MIRROR}"
        sudo chown -R "$(id -u):$(id -g)" "${INITRAMFS_DIR}"

        echo "Creating /dev and /proc mount points..."
        mkdir -p "${INITRAMFS_DIR}"/{proc,bin,sbin,sys,dev,etc}
    fi

    echo "Adding linuxd.elf to initramfs..."
    cp "$LINUXD_ELF" "$INITRAMFS_DIR/bin/linuxd.elf"
    chmod +x "$INITRAMFS_DIR/bin/linuxd.elf"
cat >/tmp/init <<EOF
#!/bin/sh

echo "[init] Nanvix L2 System VM init wrapper started!"

# Set-up any resources that linuxd needs when running in the L2-VM.

# We must bind to the same IP, as it is the only one available in the guest.
echo "[init] Nanvix L2 System VM passed init gate. Starting linuxd..."
/usr/bin/linuxd.elf \
    -control-plane-addr ${CONTROL_PLANE_SOCKADDR} \
    -control-plane-socket-type tcp \
    -user-vm-bind-addr ${USER_VM_SOCKADDR} \
    -user-vm-bind-socket-type tcp \
    -gateway-bind-addr ${GATEWAY_SOCKADDR} \
    -gateway-bind-socket-type tcp

echo "[init] Nanvix L2 System VM shutting down!"
busybox poweroff -f
EOF
    cp /tmp/init "$INITRAMFS_DIR/init"
    chmod +x "$INITRAMFS_DIR/init"

    echo "Creating initramfs..."
    cd "${INITRAMFS_DIR}"
    find . | sudo cpio -H newc -o | gzip -9 > "$INITRAMFS_IMAGE"
    cd -

    echo "Done! initramfs stored in: ${INITRAMFS_IMAGE}"
}

build_initramfs
