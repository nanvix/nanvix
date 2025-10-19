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
# Test execution
#===================================================================================================

# Parameters for the requests to nanvixd.
TENANT_ID="foo"
APP_NAME="bar"

# Temporary Directory
mkdir -p "${LOGS_DIR}"

RUN_COMMAND=("${NANVIX_HOME}/scripts/run-nanvixd.sh"
    "--tenant-id" "${TENANT_ID}"
    "--app-name" "${APP_NAME}"
    "--nanvixd-sockaddr" "${NANVIXD_SOCKADDR}"
    "--toolchain-bin-dir" "${TOOLCHAIN_DIR}/bin"
    "--"
    "${PROGRAM_NAME}"
)

if [ -n "${PROGRAM_ARGS}" ]; then
    RUN_COMMAND+=("${PROGRAM_ARGS}")
fi

RUN_STDERR_LOG="${LOGS_DIR}/runner_$(date "+%Y_%m_%d_%H_%M").log"

set +e
PROGRAM_ACTUAL_OUTPUT=$( \
    cd "${NANVIX_HOME}" && \
    { printf "%s" "${PROGRAM_INPUT}"; printf '\n'; } | \
    timeout -s SIGINT --preserve-status --foreground "${TIMEOUT}" \
        "${RUN_COMMAND[@]}" 2> "${RUN_STDERR_LOG}" | tr -d '\0'
)
RUN_STATUS=$?
set -e

if [ "${RUN_STATUS}" -ne 0 ]; then
    print_error "Test failed: run-nanvixd.sh exited with status ${RUN_STATUS}. See ${RUN_STDERR_LOG}."
    exit 1
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
