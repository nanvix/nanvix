#!/bin/bash

SOCKADDR=$1
PROGRAM_NAME=$2
PROGRAM_ARGS=$3
PROGRAM_ENV=$4
PROGRAM_EXPECTED_OUTPUT=$5
PROGRAM_EXPECTED_EXIT_CODE=$6
TIMEOUT=${7:-90}

NANVIX_HOME=`git rev-parse --show-toplevel`
LOGS_DIR=${NANVIX_HOME}/logs/linuxd-$(basename "${PROGRAM_NAME}")

mkdir -p ${LOGS_DIR}

# Run linuxd.
LINUXD_STDOUT_FILE_NAME="${LOGS_DIR}/linuxd-stdout_$(date "+%Y_%m_%d_%H_%M").log"
LINUXD_STDERR_FILE_NAME="${LOGS_DIR}/linuxd-stderr_$(date "+%Y_%m_%d_%H_%M").log"
RUST_LOG=trace ./bin/linuxd.elf -user-vm-bind-addr ${SOCKADDR} \
    -log-to-file \
    1> ${LINUXD_STDOUT_FILE_NAME} \
    2> ${LINUXD_STDERR_FILE_NAME} &
LINUXD_PID=$!

# Wait for linuxd to start.
sleep 0.1

# Run microvm.
MICROVM_STDOUT_FILE_NAME="${LOGS_DIR}/microvm-stdout_$(date "+%Y_%m_%d_%H_%M").log"
MICROVM_STDERR_FILE_NAME="${LOGS_DIR}/microvm-stderr_$(date "+%Y_%m_%d_%H_%M").log"
RUST_LOG=trace timeout -s SIGINT --preserve-status --foreground ${TIMEOUT} \
    ./bin/microvm.elf \
        -kernel ./bin/kernel.elf \
        -initrd ${PROGRAM_NAME} \
        -gateway ${SOCKADDR} \
        -log-to-file \
        -initrd_args "${PROGRAM_ARGS};${PROGRAM_ENV}" \
        1> ${MICROVM_STDOUT_FILE_NAME} \
        2> ${MICROVM_STDERR_FILE_NAME}
MICROVM_EXIT_CODE=$?

# Kill linuxd and remove socket.
sudo /usr/bin/kill -s SIGINT $LINUXD_PID
sudo -E rm -f ${SOCKADDR}

# Move all Rust logs to the logs directory.
# FIXME: https://github.com/nanvix/nanvix/issues/543
mv *.log ${LOGS_DIR}/

# Check microvm status to see if it exited successfully.
if [ ${MICROVM_EXIT_CODE} -eq ${PROGRAM_EXPECTED_EXIT_CODE} ]; then
    # Check if LINUXD_STDOUT_FILE_NAME contains the expected output.
    grep -q "${PROGRAM_EXPECTED_OUTPUT}" ${LINUXD_STDOUT_FILE_NAME}
    GREP_EXIT_CODE=$?

    if [ ${GREP_EXIT_CODE} -eq 0 ]; then
        echo "Test passed."
        exit 0
    fi

    echo "Test failed: expected output '${PROGRAM_EXPECTED_OUTPUT}' not in program output"
    exit 1
else
    echo "Test failed: microVM exitted with code: ${MICROVM_EXIT_CODE}"
    exit ${MICROVM_EXIT_CODE}
fi
