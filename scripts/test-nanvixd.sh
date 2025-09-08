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
source "${NANVIX_HOME}/scripts/common/cloud_hypervisor_vars.sh"

#===================================================================================================
# Command line arguments
#===================================================================================================

NANVIXD_SOCKADDR=$1
PROGRAM_NAME=$2
PROGRAM_ARGS=$3
PROGRAM_INPUT=$4
PROGRAM_EXPECTED_OUTPUT=$5
TIMEOUT=${6:-90}
NOEOF=${7:-'false'}

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

wait_for_unix_socket() {
    local path="$1"

    print_info "Waiting for UNIX socket at ${path}..."
    for i in $(seq 1 $MAX_TRIALS); do
        if [ -S "${path}" ]; then
            print_info "UNIX socket available after $(echo "${i} * ${SLEEP_INTERVAL}" | bc -l) ms."
            return
        fi

        sleep ${SLEEP_INTERVAL}
    done

    print_error "Timed-out waiting for UNIX socket at ${path}"
}

#
# Description
#
# This method tries to write to a netcat connection over TCP and retries if is
# not ready.
#
# Probing the connection using nc is tricky because it has the side-effect that
# we establish the connection once. This is OK for the main nanvixd HTTP server,
# but not for the gateway connection which only accepts one connection, and
# moves on.
#
# To overcome situations where the connection is not ready, we need to set the
# -w flag to not hang, and retry to connect. Annoyingly, this same flag also
# works as an idle timeout, so we need to set it long enough for long tests
# to pass. Netcat is only awaken from the -w wait with an EOF. For tests that
# do not pass an EOF, we break after the first line has been read.
#
# All of the aforementioned complexities are not an issue with Unix sockets
# because we can wait on the socket to be ready by waiting on the file to be
# available, without attempting to connect.
#
# Arguments
#
# - gateway_host: IP of the host.
# - gateway_port: port of the host.
# - program_input: what to write to the nc connection.
#
# Returns
#
# The output returned by netcat.
#
write_to_nc_retry_if_failed() {
    local gateway_host=$1
    local gateway_port=$2
    local program_input=$3

    local program_actual_output=""
    for i in $(seq 1 $MAX_TRIALS); do
        if [[ "${NOEOF}" == "true" ]]; then
            if out=$(echo "$program_input" \
                | nc -w ${TIMEOUT} -q 0 "$gateway_host" "$gateway_port" 2>/dev/null \
                | awk 'NR==1 {print; exit}'); then
                program_actual_output="$(printf %s "$out" | tr -d '\0')"
                break
            fi
        else
            if out=$(echo "$program_input" \
                | nc -w ${TIMEOUT} -q 0 "$gateway_host" "$gateway_port" 2>/dev/null); then
                program_actual_output="$(printf %s "$out" | tr -d '\0')"
                break
            fi
        fi

        sleep "$SLEEP_INTERVAL"
    done

    if [[ -z "$program_actual_output" ]]; then
        print_error "Failed to connect to ${gateway_host}:${gateway_port} after ${MAX_TRIALS} attempts."
        exit 1
    fi

    # Return program output.
    echo ${program_actual_output}
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
RUST_LOG=trace setsid timeout -s SIGINT --preserve-status --foreground "${TIMEOUT}" \
    ./bin/nanvixd.elf \
        -http-addr "${NANVIXD_SOCKADDR}" \
        -toolchain-bin-dir "${TOOLCHAIN_DIR}/bin" \
        -tmp-dir "${TMP_DIR}" \
        "$([ "$L2_VM" = "yes" ] && echo "-l2")" \
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
if [[ "${L2_VM}" == "yes" ]]; then
    # In an L2 VM, the gateway socket address corresponds to a TCP socket, so
    # we need to split host:port into host port.
    gateway_host=${GATEWAY_SOCKADDR%:*}
    gateway_port=${GATEWAY_SOCKADDR#*:}

    # In the gateway we cannot wait for a TCP socket by probing using `nc`
    # because linuxd accepts only one connection, and the probing would be
    # mistook by a genuine connection.
    PROGRAM_ACTUAL_OUTPUT=$(write_to_nc_retry_if_failed "${gateway_host}" "${gateway_port}" "${PROGRAM_INPUT}")
else
    # In an L2 VM, the gateway socket address corresponds to a UNIX socket.
    wait_for_unix_socket "${GATEWAY_SOCKADDR}"
    PROGRAM_ACTUAL_OUTPUT=$(
        echo "${PROGRAM_INPUT}" | nc -U -q 0 "${GATEWAY_SOCKADDR}" | tr -d '\0'
    )
fi

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

# Move L2 VM logs to log dir.
if [[ "${L2_VM}" == "yes" ]]; then
    linuxd_log_file="${LOGS_DIR}/linuxd_l2_$(date "+%Y_%m_%d_%H_%M").log"
    cp ${CLH_CONSOLE} ${linuxd_log_file}
fi

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
