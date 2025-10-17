#!/bin/bash

set -euo pipefail

#===================================================================================================
# Global Variables
#===================================================================================================

NANVIX_HOME=$(git rev-parse --show-toplevel)

#===================================================================================================
# Utilities
#===================================================================================================

source "${NANVIX_HOME}/scripts/common/logging.sh"
source "${NANVIX_HOME}/scripts/common/utils.sh"

#===================================================================================================
# Command line arguments
#===================================================================================================

NANVIXD_SOCKADDR=$1
PROGRAM_NAME=$2
PROGRAM_ARGS=$3
PROGRAM_INPUT=$4
PROGRAM_EXPECTED_OUTPUT=$5
TIMEOUT=${6:-90}

# Check if expected program output is empty.
if [ -z "${PROGRAM_EXPECTED_OUTPUT}" ]; then
    print_error "expected program output is empty and it cannot."
    exit 1
fi

LOGS_DIR=${NANVIX_HOME}/logs/nanvixd-$(basename "${PROGRAM_NAME}")

#===================================================================================================
# Helper functions
#===================================================================================================

MAX_TRIALS=100
SLEEP_INTERVAL=0.1

wait_for_tcp_socket() {
    local host=$1
    local port=$2

    for i in $(seq 1 $MAX_TRIALS); do
        print_info "Waiting for TCP socket at $host:$port..."
        sleep ${SLEEP_INTERVAL}

        if nc -z "${host}" "${port}" 2>/dev/null; then
            print_info "TCP socket ready after $(echo "${i} * ${SLEEP_INTERVAL}" | bc -l) ms."
            return
        fi
    done

    print_error "Timed-out waiting for TCP socket to be ready at $host:$port"
}

#===================================================================================================
# Test execution
#===================================================================================================

# Parameters for the requests to nanvixd.
TENANT_ID="foo"
APP_NAME="bar"

# Temporary Directory
TMP_DIR_PATH="/tmp/nanvixd"
TMP_DIR=$(mktemp -d ${TMP_DIR_PATH}-XXXXXX)

mkdir -p "${LOGS_DIR}"

# Run nanvixd.
CONSOLE_FILE_NAME="${LOGS_DIR}/kernel_$(date "+%Y_%m_%d_%H_%M").log"
RUST_LOG=trace,hyperlight_host=none setsid timeout -s SIGINT --preserve-status --foreground "${TIMEOUT}" \
    ./bin/nanvixd.elf \
        -http-addr "${NANVIXD_SOCKADDR}" \
        -toolchain-bin-dir "${TOOLCHAIN_DIR}/bin" \
        -tmp-dir "${TMP_DIR}" \
        -console-file "${CONSOLE_FILE_NAME}" &
NANVIXD_PID=$!

cleanup() {
    # Sometimes error messages in this script are hard to parse because they
    # come from nested bash calls and we cannot color them accordingly.
    # During clean-up, print a clear error message in red if the test has
    # failed.
    if kill -0 -- "-${NANVIXD_PID}" 2>/dev/null; then
        print_error "Test failed: cleaning-up and nanvixd.elf is still alive"

        # Make sure we force-clean any zombie processes, and ignore errors.
        kill -s SIGKILL -- "-${NANVIXD_PID}" 2> /dev/null || true

        wait "${NANVIXD_PID}" 2>/dev/null || true
    fi

    rm -rf "${TMP_DIR}"
}
trap 'cleanup' EXIT

# Extract port number from nanvixd.
NANVIXD_PORT_NUMBER=$(echo "${NANVIXD_SOCKADDR}" | cut -d: -f2)

# Wait for nanvixd to start by checking if the HTTP socket is listening.
print_info "Waiting for nanvixd to be ready"
wait_for_tcp_socket "127.0.0.1" "$NANVIXD_PORT_NUMBER"

# Run a client.
NEW_JSON=$(jq -n \
    --arg tenant_id "${TENANT_ID}" \
    --arg app_name "${APP_NAME}" \
    --arg program "${PROGRAM_NAME}" \
    --arg program_args "${PROGRAM_ARGS}" \
    '{tenant_id: $tenant_id, app_name: $app_name, program: $program, program_args: $program_args}'
)
NEW_RESPONSE=$(curl \
    --silent \
    --header "Content-Type: application/json" \
    --header "X-NVX-Message-Type: NEW" \
    --request POST \
    --data "${NEW_JSON}" \
    http://localhost:"${NANVIXD_PORT_NUMBER}")
VM_ID=$(echo "${NEW_RESPONSE}" | jq -r '.user_vm_id')
GATEWAY_SOCKADDR=$(echo "${NEW_RESPONSE}" | jq -r '.gateway_sockaddr')

print_info "VM ID: ${VM_ID}"
print_info "Gateway Socket Address: ${GATEWAY_SOCKADDR}"

# Get output by writing to the gateway socket address.
PROGRAM_ACTUAL_OUTPUT=$(echo "${PROGRAM_INPUT}" | nc -U -q 0 "${GATEWAY_SOCKADDR}" | tr -d '\0')

# Save program output to a log file.
file_name=$(basename -- "${PROGRAM_NAME}")
file_name_no_ext="${file_name%.*}"
log_file="${LOGS_DIR}/${file_name_no_ext}_$(date "+%Y_%m_%d_%H_%M").log"
echo "${PROGRAM_ACTUAL_OUTPUT}" > "${log_file}"

# Kill the user VM.
KILL_JSON=$(jq -n \
    --argjson user_vm_id "${VM_ID}" \
    '{user_vm_id: $user_vm_id}'
)
KILL_EXIT_CODE=$(curl \
    --silent \
    --header "Content-Type: application/json" \
    --header "X-NVX-Message-Type: KILL" \
    --request POST \
    --data "${KILL_JSON}" \
    http://localhost:"${NANVIXD_PORT_NUMBER}" | jq -r '.exit_code')

if [ "${KILL_EXIT_CODE}" != "0" ]; then
    print_error "Test failed: error killing user VM (code=${KILL_EXIT_CODE})"
    exit 1
fi

# Move all Rust logs to the logs directory.
# FIXME: https://github.com/nanvix/nanvix/issues/543
find . -maxdepth 1 -name '*.log' -exec mv {} "${LOGS_DIR}"/ \; 2>/dev/null || true

kill -s SIGINT "${NANVIXD_PID}" || true
wait "${NANVIXD_PID}" 2>/dev/null || true

# Check if curl.log contains the expected output.
if grep -F -q -- "${PROGRAM_EXPECTED_OUTPUT}" <<< "${PROGRAM_ACTUAL_OUTPUT}"; then
    print_success "Test passed."
    exit 0
else
    print_error "Test failed: expected output '${PROGRAM_EXPECTED_OUTPUT}' but got '${PROGRAM_ACTUAL_OUTPUT}'"
    exit 1
fi
