#!/bin/bash

NANVIXD_SOCKADDR=$1
LINUXD_SOCKADDR=$2
SANDBOX_SOCKADDR=$3
PROGRAM_NAME=$4
PROGRAM_ARGS=$5
PROGRAM_EXPECTED_OUTPUT=$6
TIMEOUT=${7:-90}

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
timeout -s SIGINT --preserve-status --foreground ${TIMEOUT} \
    ./bin/nanvixd.elf \
        -http-addr ${NANVIXD_SOCKADDR} \
        -linuxd-addr ${LINUXD_SOCKADDR} \
        -sandbox-addr ${SANDBOX_SOCKADDR} \
        -keep-alive 0 \
    2>&1 2> nanvixd.log &
NANVIXD_PID=$!

# Extract port number from nanvixd.
NANVIXD_PORT_NUMBER=$(echo ${NANVIXD_SOCKADDR} | cut -d: -f2)

# Wait for nanvixd to start.
sleep 0.1

# Run a client.
curl \
    --silent \
    --header \
    "Content-Type: application/json" \
    --request POST \
    --data "{\"clientid\":1, \"program\": \"${PROGRAM_NAME}\", \"args\":${PROGRAM_ARGS}}" \
    http://localhost:${NANVIXD_PORT_NUMBER} \
    > curl.log

# Check if curl.log contains the expected output.
if grep -q "${PROGRAM_EXPECTED_OUTPUT}" curl.log; then
    echo "Test passed."
    kill_children $NANVIXD_PID
    sudo -E rm -f /tmp/${NANVIXD_SOCKADDR}*.socket
    sudo -E rm -f /tmp/${LINUXD_SOCKADDR}*.socket
    sudo -E rm -f /tmp/${SANDBOX_SOCKADDR}*.socket
    exit 0
else
    echo "Test failed."
    kill_children $NANVIXD_PID
    sudo -E rm -f /tmp/${NANVIXD_SOCKADDR}*.socket
    sudo -E rm -f /tmp/${LINUXD_SOCKADDR}*.socket
    sudo -E rm -f /tmp/${SANDBOX_SOCKADDR}*.socket
    exit 1
fi
