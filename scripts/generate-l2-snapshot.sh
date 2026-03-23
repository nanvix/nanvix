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

#===================================================================================================
# Networking
#===================================================================================================

GUEST_MAC_ADDRESS="12:34:56:78:90:ab"
GUEST_BROADCAST_ADDRESS="192.168.249.1"
MASK="255.255.255.0"

LINUXD_CONFIG_TOML="${NANVIX_HOME}/build/linuxd_config.toml"
TAP_NAME=$(get_value_from_toml "${LINUXD_CONFIG_TOML}" "tap_name")
GUEST_TAP_IP_ADDRESS=$(get_value_from_toml "${LINUXD_CONFIG_TOML}" "guest_tap_ip_address")
HOST_TAP_IP_ADDRESS=$(get_value_from_toml "${LINUXD_CONFIG_TOML}" "host_tap_ip_address")
SNAPSHOT_MAGIC_STRING=$(get_value_from_toml "${LINUXD_CONFIG_TOML}" "snapshot_magic_string")

CLH_RUNTIME_DIR=$(mktemp -d /tmp/nanvix-clh-snapshot.XXXXXX)
CLH_API_SOCKET="${CLH_RUNTIME_DIR}/cloud-hypervisor.sock"
CLH_CONSOLE="${CLH_RUNTIME_DIR}/clh-console"
IVSHMEM_PATH="${NANVIX_L2_IVSHMEM_PATH:-}"
IVSHMEM_SIZE="${NANVIX_L2_IVSHMEM_SIZE:-}"
IVSHMEM_ARGS=()

trap 'rm -rf "${CLH_RUNTIME_DIR}"' EXIT

#===================================================================================================
# Generate snapshot
#===================================================================================================

SNAPSHOT_NAME=$(get_value_from_toml "${LINUXD_CONFIG_TOML}" "snapshot_name")
SNAPSHOT_PATH="${IMAGES_DIR}/${SNAPSHOT_NAME}"
L2_SYSVM_INITRAMFS="${IMAGES_DIR}/l2_sysvm_initramfs.img"

build_ivshmem_args() {
    if [[ -z "${IVSHMEM_PATH}" && -z "${IVSHMEM_SIZE}" ]]; then
        return
    fi

    if [[ -z "${IVSHMEM_PATH}" || -z "${IVSHMEM_SIZE}" ]]; then
        print_error "Both NANVIX_L2_IVSHMEM_PATH and NANVIX_L2_IVSHMEM_SIZE must be set."
        exit 1
    fi

    if ! [[ "${IVSHMEM_SIZE}" =~ ^[0-9]+$ ]] || [[ "${IVSHMEM_SIZE}" == "0" ]]; then
        print_error "NANVIX_L2_IVSHMEM_SIZE must be a positive integer."
        exit 1
    fi

    if ! "${CLOUD_HYPERVISOR_PATH}" --help 2>&1 | grep -q -- '--ivshmem'; then
        print_error "cloud-hypervisor at ${CLOUD_HYPERVISOR_PATH} does not support --ivshmem."
        exit 1
    fi

    mkdir -p -- "$(dirname -- "${IVSHMEM_PATH}")"
    truncate -s "${IVSHMEM_SIZE}" "${IVSHMEM_PATH}"

    IVSHMEM_ARGS=(
        --ivshmem
        "path=${IVSHMEM_PATH},size=${IVSHMEM_SIZE}"
    )
}

boot_clh_vm() {
    rm -f -- "${CLH_API_SOCKET}"
    rm -f -- "${CLH_CONSOLE}"
    build_ivshmem_args
    # FIXME(#1156): re-enable --seccomp true (default) when we cut a new Nanvix release that
    # includes an updated cloud-hypervisor.
    ${CLOUD_HYPERVISOR_PATH} \
        --seccomp false \
        --api-socket ${CLH_API_SOCKET} \
        --kernel "${L2_SYSVM_KERNEL}" \
        --initramfs "${L2_SYSVM_INITRAMFS}" \
        --console "file=${CLH_CONSOLE}" \
        --serial "off" \
        --cmdline "console=hvc0 rdinit=/init ip=${GUEST_TAP_IP_ADDRESS}::${GUEST_BROADCAST_ADDRESS}:${MASK}::eth0:off" \
        --cpus "boot=2" \
        --memory "size=512M,shared=on" \
        --rng "src=/dev/urandom" \
        "${IVSHMEM_ARGS[@]}" \
        --net "tap=${TAP_NAME},mac=${GUEST_MAC_ADDRESS},ip=${HOST_TAP_IP_ADDRESS},mask=${MASK},num_queues=2,queue_size=256" > /dev/null 2>&1 &

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
