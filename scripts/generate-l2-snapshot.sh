#!/usr/bin/env bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# This script generates a snapshot for the L2 System VM. It can also be used as
# a reference to deploy standalone L2 VMs, by copying the command line arguments
# in boot_clh_vm.

set -euo pipefail

#===================================================================================================
# Script Arguments
#===================================================================================================

PREFIX=${1:-$PWD/toolchain}

#===================================================================================================
# Global Variables
#===================================================================================================

NANVIX_HOME=$(git rev-parse --show-toplevel)
BIN_DIR="${PREFIX}/bin"
SHARE_DIR="${PREFIX}/share/cloud-hypervisor"
IMAGES_DIR="${NANVIX_HOME}/build/images"
L2_SYSVM_KERNEL="${SHARE_DIR}/l2_sysvm_vmlinux.bin"

# Snapshot variables.
SNAPSHOT_PATH="${IMAGES_DIR}/l2-sysvm-snapshot"
L2_SYSVM_INITRAMFS="${IMAGES_DIR}/l2_sysvm_initramfs.img"

# Cloud-hypervisor variables.
CLOUD_HYPERVISOR_PATH="${BIN_DIR}/cloud-hypervisor"
CLOUD_HYPERVISOR_REMOTE_PATH="${BIN_DIR}/ch-remote"

#===================================================================================================
# Networking
#===================================================================================================

# FIXME (#838): these values are currently hard-coded here and in src/utils/nanvixd/src/config.rs
GUEST_MAC_ADDRESS="12:34:56:78:90:ab"
GUEST_BROADCAST_ADDRESS="192.168.249.1"
MASK="255.255.255.0"
GUEST_TAP_IP_ADDRESS="192.168.249.2"
HOST_TAP_IP_ADDRESS="192.168.249.3"

CLH_API_SOCKET="/tmp/cloud-hypervisor.sock"
CLH_CONSOLE="/tmp/clh-console"

#===================================================================================================
# Generate snapshot
#===================================================================================================

boot_clh_vm() {
    rm -f ${CLH_API_SOCKET}
    rm -f ${CLH_CONSOLE}
    ${CLOUD_HYPERVISOR_PATH} \
        --api-socket ${CLH_API_SOCKET} \
        --kernel "${L2_SYSVM_KERNEL}" \
        --initramfs "${L2_SYSVM_INITRAMFS}" \
        --console "file=${CLH_CONSOLE}" \
        --serial "off" \
        --cmdline "console=hvc0 rdinit=/init ip=${GUEST_TAP_IP_ADDRESS}::${GUEST_BROADCAST_ADDRESS}:${MASK}::eth0:off" \
        --cpus "boot=2" \
        --memory "size=512M" \
        --rng "src=/dev/urandom" \
        --net "tap=vmtap0,mac=${GUEST_MAC_ADDRESS},ip=${HOST_TAP_IP_ADDRESS},mask=${MASK},num_queues=2,queue_size=256" > /dev/null 2>&1 &

    # Return the PID
    echo $!
}

snapshot_clh_vm() {
    rm -rf ${SNAPSHOT_PATH}
    mkdir -p ${SNAPSHOT_PATH}
    ${CLOUD_HYPERVISOR_REMOTE_PATH} --api-socket=${CLH_API_SOCKET} pause
    sleep 1
    ${CLOUD_HYPERVISOR_REMOTE_PATH} --api-socket=${CLH_API_SOCKET} snapshot file://${SNAPSHOT_PATH}
}

# Create a named pipe (FIFO) for capturing the VM's stdout, and know when it has finished booting.
fifo=$(mktemp -u)
mkfifo "$fifo"

# Boot the VM and wait until it has finished booting to take a snapshot.
vm_pid=$(boot_clh_vm)

( tail -F ${CLH_CONSOLE} > $fifo ) &
tail_pid=$!

echo -n "Waiting for CLH VM to boot..."
while IFS= read -r line; do
    if [[ "$line" == *"Nanvix L2 System VM init wrapper started"* ]]; then
        echo "... CLH VM done booting!"
        sleep 1
        break
    fi
done < "$fifo"

echo -n "Snapshotting CLH VM..."
snapshot_clh_vm
echo "...snapshot done!"

# Clean-up.
rm "$fifo"
rm -f ${CLH_CONSOLE}

echo -n "Killing CLH VM..."
kill -s SIGTERM ${vm_pid}
kill -s SIGTERM ${tail_pid}
rm -f ${CLH_API_SOCKET}
echo "...done!"

echo "Snapshot is available at: ${SNAPSHOT_PATH}"
