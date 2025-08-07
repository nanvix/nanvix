#!/bin/bash

set -euo pipefail

NANVIXD_SOCKADDR=$1
PROGRAM_NAME=$2
PROGRAM_ARGS=$3
PROGRAM_INPUT=$4
PROGRAM_EXPECTED_OUTPUT=$5
TIMEOUT=${6:-90}

NANVIX_HOME=$(git rev-parse --show-toplevel)
LOGS_DIR=${NANVIX_HOME}/logs/nanvixd-$(basename "${PROGRAM_NAME}")

# Parameters for the requests to nanvixd.
TENANT_ID="foo"
APP_NAME="bar"

# Temporary Directory
TMP_DIR_PATH="/tmp/nanvixd"
TMP_DIR=$(mktemp -d ${TMP_DIR_PATH}-XXXXXX)
trap 'rm -rf "${TMP_DIR}"' EXIT

mkdir -p "${LOGS_DIR}"

# Run nanvixd.
CONSOLE_FILE_NAME="${LOGS_DIR}/kernel_$(date "+%Y_%m_%d_%H_%M").log"
RUST_LOG=trace timeout -s SIGINT --preserve-status --foreground "${TIMEOUT}" \
    ./bin/nanvixd.elf \
        -http-addr "${NANVIXD_SOCKADDR}" \
        -tmp-dir "${TMP_DIR}" \
        -console-file "${CONSOLE_FILE_NAME}" &
NANVIXD_PID=$!

# Extract port number from nanvixd.
NANVIXD_PORT_NUMBER=$(echo "${NANVIXD_SOCKADDR}" | cut -d: -f2)

# Wait for nanvixd to start by checking if the HTTP socket is listening.
MAX_TRIALS=100
SLEEP_INTERVAL=0.1
for i in $(seq 1 $MAX_TRIALS); do
    echo "Waiting for nanvixd to start ... ($(echo "$i * $SLEEP_INTERVAL" | bc) s elapsed)"
    sleep ${SLEEP_INTERVAL}

    if ss -tln | grep -q ":${NANVIXD_PORT_NUMBER} "; then
        echo "nanvixd started after ${i} ms."
        break
    fi
done

# Check again after waiting.
if ! ss -tln | grep -q ":${NANVIXD_PORT_NUMBER} "; then
    echo "nanvixd failed to start"
    exit 2 # Error Code 2: No such file or directory (ENOENT)
fi

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

echo "VM ID: ${VM_ID}"
echo "Gateway Socket Address: ${GATEWAY_SOCKADDR}"

# Get output by writing to the gateway socket address.
PROGRAM_ACTUAL_OUTPUT=$(echo "${PROGRAM_INPUT}" | nc -U -q 0 "${GATEWAY_SOCKADDR}" | tr -d '\0')

# Kill the user VM.
KILL_JSON=$(jq -n \
    --arg user_vm_id "${VM_ID}" \
    '{user_vm_id: $user_vm_id}'
)
KILL_EXIT_CODE=$(curl \
    --silent \
    --header "Content-Type: application/json" \
    --header "X-NVX-Message-Type: KILL" \
    --request POST \
    --data "${KILL_JSON}" \
    http://localhost:"${NANVIXD_PORT_NUMBER}" | jq -r '.exit_code')

if [ "${KILL_EXIT_CODE}" -ne 0 ]; then
    echo "Test failed: error killing user VM (code=${KILL_EXIT_CODE})"
    exit 1
fi

# Move all Rust logs to the logs directory.
# FIXME: https://github.com/nanvix/nanvix/issues/543
find . -maxdepth 1 -name '*.log' -exec mv {} "${LOGS_DIR}"/ \; 2>/dev/null || true

kill -s SIGINT "${NANVIXD_PID}" || true

# Check if curl.log contains the expected output.
echo "${PROGRAM_ACTUAL_OUTPUT}" | grep -q "${PROGRAM_EXPECTED_OUTPUT}"
GREP_EXIT_CODE=$?
if [ "${GREP_EXIT_CODE}" -eq 0 ]; then
    echo "Test passed."
    exit 0
else
    echo "Test failed: expected output '${PROGRAM_EXPECTED_OUTPUT}' but got '${PROGRAM_ACTUAL_OUTPUT}'"
    exit 1
fi
