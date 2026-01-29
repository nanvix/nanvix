#!/bin/bash

#
# Test script for running Nanvix programs via nanvixd in HTTP or terminal mode.
#
# Usage:
#   test-nanvixd.sh <MODE> <NANVIXD_SOCKADDR> <PROGRAM_NAME> <PROGRAM_ARGS> <PROGRAM_INPUT> <PROGRAM_EXPECTED_OUTPUT> [TIMEOUT]
#
# Arguments:
#   MODE                       - Mode of operation: 'http' or 'terminal'
#   NANVIXD_SOCKADDR           - Socket address for nanvixd (e.g., 127.0.0.1:8181). Empty string for terminal mode.
#   PROGRAM_NAME               - Path to the program to execute
#   PROGRAM_ARGS               - Arguments to pass to the program (use '' for none)
#                                  HTTP mode: passed as JSON array (e.g., '["arg1", "arg2"]')
#                                  Terminal mode: not supported (must be empty)
#   PROGRAM_INPUT              - Input to feed to the program (use '' for none)
#                                  HTTP mode: passed as JSON string
#                                  Terminal mode: passed directly via stdin
#   PROGRAM_EXPECTED_OUTPUT    - Expected output string to match
#   TIMEOUT                    - Optional timeout in seconds (default: 90)
#

set -euo pipefail

#===================================================================================================
# Constants
#===================================================================================================

# Number of ports to search when finding an available alternative port.
readonly PORT_RANGE_SIZE=1000

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

# Mode of operation: 'http' or 'terminal'.
MODE=${1:-http}
NANVIXD_SOCKADDR=$2
PROGRAM_NAME=$3
PROGRAM_ARGS=$4
PROGRAM_INPUT=$5
PROGRAM_EXPECTED_OUTPUT=$6
TIMEOUT=${7:-90}

# Validate mode.
if [ "${MODE}" != "http" ] && [ "${MODE}" != "terminal" ]; then
    print_error "Invalid mode '${MODE}'. Expected 'http' or 'terminal'."
    exit 1
fi

# Check if expected program output is empty.
if [ -z "${PROGRAM_EXPECTED_OUTPUT}" ]; then
    print_error "Expected program output cannot be empty."
    exit 1
fi

# For HTTP mode, validate socket address format.
if [ "${MODE}" = "http" ] && ! validate_sockaddr "${NANVIXD_SOCKADDR}"; then
    print_error "Invalid socket address format: '${NANVIXD_SOCKADDR}'. Expected format: <HOST>:<PORT>."
    exit 1
fi

LOGS_DIR=${NANVIX_HOME}/logs/nanvixd-$(basename "${PROGRAM_NAME}")

#===================================================================================================
# Test execution
#===================================================================================================

# Temporary Directory.
mkdir -p "${LOGS_DIR}"

# HTTP mode: Test programs via nanvixd's HTTP API.
# In this mode, program arguments and input are sent as JSON payloads to the HTTP endpoint.
if [ "${MODE}" = "http" ]; then
    # Parse socket address into host and port components.
    # Note: validation already performed above, so parse_sockaddr should not fail.
    parse_sockaddr "${NANVIXD_SOCKADDR}" NANVIXD_HOST NANVIXD_PORT

    # Force base-10 parsing to avoid octal interpretation for leading zeros.
    NANVIXD_PORT_INT=$((10#${NANVIXD_PORT}))

    if ! is_port_available "${NANVIXD_HOST}" "${NANVIXD_PORT_INT}"; then
        print_info "Port ${NANVIXD_PORT} is in use, searching for available port..."

        # Define port range based on original port.
        # Note: There is a TOCTOU (time-of-check-time-of-use) race condition between finding
        # an available port and actually binding to it. Another process could bind to the port
        # in between. This is acceptable as the probability is low and retries will handle it.
        PORT_RANGE_START="${NANVIXD_PORT_INT}"
        PORT_RANGE_END=$((NANVIXD_PORT_INT + PORT_RANGE_SIZE))

        # Clamp port range end to maximum valid port number.
        if [ "${PORT_RANGE_END}" -gt 65535 ]; then
            PORT_RANGE_END=65535
        fi

        if ! AVAILABLE_PORT=$(find_available_port "${NANVIXD_HOST}" "${PORT_RANGE_START}" "${PORT_RANGE_END}"); then
            print_error "No available port found in range ${PORT_RANGE_START}-${PORT_RANGE_END}"
            exit 1
        fi

        print_info "Using alternative port: ${AVAILABLE_PORT}"
        NANVIXD_SOCKADDR="${NANVIXD_HOST}:${AVAILABLE_PORT}"
    fi

    # Parameters for the requests to nanvixd.
    TENANT_ID="foo"
    APP_NAME="bar"

    RUN_COMMAND=("${NANVIX_HOME}/scripts/run-nanvixd.sh"
        "--tenant-id" "${TENANT_ID}"
        "--app-name" "${APP_NAME}"
        "--nanvixd-sockaddr" "${NANVIXD_SOCKADDR}"
        "--toolchain-bin-dir" "${TOOLCHAIN_DIR}/bin"
        "--bin-dir" "${NANVIX_HOME}/bin"
        "--log-level" "${LOG_LEVEL}"
        "--"
        "${PROGRAM_NAME}"
    )

    if [ -n "${PROGRAM_ARGS}" ]; then
        RUN_COMMAND+=("${PROGRAM_ARGS}")
    fi

    RUN_STDERR_LOG="${LOGS_DIR}/runner_$(date "+%Y_%m_%d_%H_%M").log"

    set +e
    if [ -n "${PROGRAM_INPUT}" ]; then
        PROGRAM_ACTUAL_OUTPUT=$( \
            cd "${NANVIX_HOME}" && \
            printf "%s\n" "${PROGRAM_INPUT}" | \
            timeout -s SIGINT --preserve-status --foreground "${TIMEOUT}" \
                "${RUN_COMMAND[@]}" 2> "${RUN_STDERR_LOG}" | tr -d '\0'
        )
    else
        PROGRAM_ACTUAL_OUTPUT=$( \
            cd "${NANVIX_HOME}" && \
            timeout -s SIGINT --preserve-status --foreground "${TIMEOUT}" \
                "${RUN_COMMAND[@]}" < /dev/null 2> "${RUN_STDERR_LOG}" | tr -d '\0'
        )
    fi
    RUN_STATUS=$?
    set -e

    if [ "${RUN_STATUS}" -ne 0 ]; then
        print_error "Test failed: run-nanvixd.sh exited with status ${RUN_STATUS}. See ${RUN_STDERR_LOG}."
        exit 1
    fi
# Terminal mode: Test programs via nanvixd's terminal interface.
# In this mode, the program is invoked directly and input is provided via stdin (not JSON).
else
    # Terminal mode: directly invoke nanvixd.elf with -- separator.
    RUN_COMMAND=("${NANVIX_HOME}/bin/nanvixd.elf" "--" "${PROGRAM_NAME}")

    if [ -n "${PROGRAM_ARGS}" ]; then
        RUN_COMMAND+=("${PROGRAM_ARGS}")
    fi

    RUN_STDERR_LOG="${LOGS_DIR}/runner_$(date "+%Y_%m_%d_%H_%M").log"

    set +e
    if [ -n "${PROGRAM_INPUT}" ]; then
        PROGRAM_ACTUAL_OUTPUT=$( \
            cd "${NANVIX_HOME}" && \
            printf "%s\n" "${PROGRAM_INPUT}" | \
            timeout -s SIGINT --preserve-status --foreground "${TIMEOUT}" \
                "${RUN_COMMAND[@]}" 2> "${RUN_STDERR_LOG}" | tr -d '\0'
        )
    else
        PROGRAM_ACTUAL_OUTPUT=$( \
            cd "${NANVIX_HOME}" && \
            timeout -s SIGINT --preserve-status --foreground "${TIMEOUT}" \
                "${RUN_COMMAND[@]}" < /dev/null 2> "${RUN_STDERR_LOG}" | tr -d '\0'
        )
    fi
    RUN_STATUS=$?
    set -e

    if [ "${RUN_STATUS}" -ne 0 ]; then
        print_error "Test failed: nanvixd.elf exited with status ${RUN_STATUS}. See ${RUN_STDERR_LOG}."
        exit 1
    fi
fi

# Save program output to a log file.
file_name=$(basename -- "${PROGRAM_NAME}")
file_name_no_ext="${file_name%.*}"
log_file="${LOGS_DIR}/${file_name_no_ext}_$(date "+%Y_%m_%d_%H_%M").log"
printf "%s" "${PROGRAM_ACTUAL_OUTPUT}" > "${log_file}"

# Check if curl.log contains the expected output.
if grep -F -q -- "${PROGRAM_EXPECTED_OUTPUT}" <<< "${PROGRAM_ACTUAL_OUTPUT}"; then
    print_success "Test passed."
    exit 0
else
    print_error "Test failed: expected output '${PROGRAM_EXPECTED_OUTPUT}' but got '${PROGRAM_ACTUAL_OUTPUT}'"
    exit 1
fi
