#!/bin/bash

NANVIXD_SOCKADDR=$1
LINUXD_SOCKADDR=$2
SANDBOX_SOCKADDR=$3
PROGRAM_NAME=$4
PROGRAM_ARGS=$5
PROGRAM_EXPECTED_OUTPUT=$6
TIMEOUT=${7:-90}

NANVIX_HOME=`git rev-parse --show-toplevel`
LOGS_DIR=${NANVIX_HOME}/logs

kill_children() {
    local parent_pid=$1

    if [ -z "$parent_pid" ]; then
           return
    fi

    # Find all child processes
    local children=$(pgrep -P $parent_pid)
    for child in $children; do
        # Recursively call the function for each child process
        kill_children $child
    done

    # Kill the parent process
    sudo /usr/bin/pkill -e -INT -P $parent_pid
}

# Run nanvixd.
NANVIXD_STDOUT_FILE_NAME="${LOGS_DIR}/nanvixd-stdout_$(date "+%Y_%m_%d_%H_%M").log"
NANVIXD_STDERR_FILE_NAME="${LOGS_DIR}/nanvixd-stderr_$(date "+%Y_%m_%d_%H_%M").log"
CONSOLE_FILE_NAME="${LOGS_DIR}/kernel_$(date "+%Y_%m_%d_%H_%M").log"
RUST_LOG=trace timeout -s SIGINT --preserve-status --foreground ${TIMEOUT} \
    ./bin/nanvixd.elf \
        -http-addr ${NANVIXD_SOCKADDR} \
        -linuxd-addr ${LINUXD_SOCKADDR} \
        -sandbox-addr ${SANDBOX_SOCKADDR} \
        -console-file ${CONSOLE_FILE_NAME} \
        -keep-alive 0 \
        1> ${NANVIXD_STDOUT_FILE_NAME} \
        2> ${NANVIXD_STDERR_FILE_NAME} &
NANVIXD_PID=$!

# Extract port number from nanvixd.
NANVIXD_PORT_NUMBER=$(echo ${NANVIXD_SOCKADDR} | cut -d: -f2)

# Wait for nanvixd to start.
sleep 0.1

# Run a client.
CURL_STDOUT_FILE_NAME="${LOGS_DIR}/curl-stdout_$(date "+%Y_%m_%d_%H_%M").log"
curl \
    --silent \
    --header \
    "Content-Type: application/json" \
    --request POST \
    --data "{\"clientid\":1, \"program\": \"${PROGRAM_NAME}\", \"args\":${PROGRAM_ARGS}}" \
    http://localhost:${NANVIXD_PORT_NUMBER} \
    > ${CURL_STDOUT_FILE_NAME}

# Move all Rust logs to the logs directory.
# FIXME: https://github.com/nanvix/nanvix/issues/543
mv *.log ${LOGS_DIR}/

# Check if curl.log contains the expected output.
grep -q "${PROGRAM_EXPECTED_OUTPUT}" ${CURL_STDOUT_FILE_NAME}
GREP_EXIT_CODE=$?
if [ ${GREP_EXIT_CODE} -eq 0 ]; then
    echo "Test passed."
    kill_children $NANVIXD_PID
    sudo -E rm -f /tmp/${NANVIXD_SOCKADDR}*.socket
    sudo -E rm -f /tmp/${LINUXD_SOCKADDR}*.socket
    sudo -E rm -f /tmp/${SANDBOX_SOCKADDR}*.socket
    exit 0
else
    echo "Test failed: expected output '${PROGRAM_EXPECTED_OUTPUT}' not in program output"
    kill_children $NANVIXD_PID
    sudo -E rm -f /tmp/${NANVIXD_SOCKADDR}*.socket
    sudo -E rm -f /tmp/${LINUXD_SOCKADDR}*.socket
    sudo -E rm -f /tmp/${SANDBOX_SOCKADDR}*.socket
    exit 1
fi
