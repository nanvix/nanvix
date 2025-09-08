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
# Follow sym-links to avoid capabilities in the cloud-hypervisor binary not being propagated.
BIN_DIR=$(readlink -f -- "${PREFIX}/bin")
SHARE_DIR="${PREFIX}/share/cloud-hypervisor"
IMAGES_DIR="${NANVIX_HOME}/images"
L2_SYSVM_KERNEL="${SHARE_DIR}/l2_sysvm_vmlinux.bin"

# Cloud-hypervisor variables.
CLOUD_HYPERVISOR_PATH="${BIN_DIR}/cloud-hypervisor"
CLOUD_HYPERVISOR_REMOTE_PATH="${BIN_DIR}/ch-remote"

#===================================================================================================
# Utilities
#===================================================================================================

source "${NANVIX_HOME}/scripts/common/logging.sh"
source "${NANVIX_HOME}/scripts/common/utils.sh"
source "${NANVIX_HOME}/scripts/common/cloud_hypervisor_vars.sh"

#===================================================================================================
# Networking
#===================================================================================================

GUEST_MAC_ADDRESS="12:34:56:78:90:ab"
GUEST_BROADCAST_ADDRESS="192.168.249.1"
MASK="255.255.255.0"

LINUXD_CONFIG_TOML="${NANVIX_HOME}/build/linuxd_config.toml"
GUEST_TAP_IP_ADDRESS=$(get_value_from_toml "${LINUXD_CONFIG_TOML}" "guest_tap_ip_address")
HOST_TAP_IP_ADDRESS=$(get_value_from_toml "${LINUXD_CONFIG_TOML}" "host_tap_ip_address")
SNAPSHOT_MAGIC_STRING=$(get_value_from_toml "${LINUXD_CONFIG_TOML}" "snapshot_magic_string")


trap 'rm -rf "${CLH_API_SOCKET}" "${CLH_CONSOLE}"' EXIT

#===================================================================================================
# Generate snapshot
#===================================================================================================

SNAPSHOT_NAME=$(get_value_from_toml "${LINUXD_CONFIG_TOML}" "snapshot_name")
SNAPSHOT_PATH="${IMAGES_DIR}/${SNAPSHOT_NAME}"
L2_SYSVM_INITRAMFS="${IMAGES_DIR}/l2_sysvm_initramfs.img"

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
        --memory "size=1G" \
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
print_info "Booting CLH VM to be snapshotted..."
vm_pid=$(boot_clh_vm)

# Insert trap to kill the VM on exit in case of failure, but ignore failures if the VM
# has already been properly killed.
trap 'kill -s SIGKILL ${vm_pid} || true' EXIT

( tail -F ${CLH_CONSOLE} > $fifo 2> /dev/null ) &

print_info "Waiting for CLH VM to boot..."
while IFS= read -r line; do
    if [[ "$line" == *"${SNAPSHOT_MAGIC_STRING}"* ]]; then
        print_success "... CLH VM done booting!"
        sleep 1
        break
    fi
done < "$fifo"

print_info "Snapshotting CLH VM..."
snapshot_clh_vm
print_success "Snapshot done!"

# Clean-up.
rm "$fifo"
rm -f ${CLH_CONSOLE}

print_info "Killing CLH VM..."
kill -s SIGTERM ${vm_pid}
rm -f ${CLH_API_SOCKET}

print_success "Snapshot is available at: ${SNAPSHOT_PATH}"
