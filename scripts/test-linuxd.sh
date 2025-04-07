#!/bin/bash

SOCKADDR=$1
PROGRAM_NAME=$2
PROGRAM_ARGS=$3
PROGRAM_ENV=$4
PROGRAM_EXPECTED_OUTPUT=$5
TIMEOUT=${6:-90}

# Run linuxd.
./bin/linuxd.elf -bind-addr ${SOCKADDR}  2>linuxd.stdout 1> linuxd.stderr &
LINUXD_PID=$!

# Wait for linuxd to start.
sleep 0.1

# Run microvm.
timeout -s SIGINT --preserve-status --foreground ${TIMEOUT} \
    ./bin/microvm.elf \
        -kernel ./bin/kernel.elf \
        -initrd ${PROGRAM_NAME} \
        -gateway ${SOCKADDR} \
        -initrd_args "${PROGRAM_ARGS};${PROGRAM_ENV}" \
    2>&1 2> microvm.log
MICROVM_EXIT_CODE=$?

# Kill linuxd and remove socket.
sudo /usr/bin/pkill -e -INT -P $LINUXD_PID
sudo -E rm -f ${SOCKADDR}


# Check microvm status to see if it exited successfully.
if [ ${MICROVM_EXIT_CODE} -eq 0 ]; then
        echo "Test passed."
        exit 0
    # Check if linuxd.stdout contains the expected output.
    if grep -q "${PROGRAM_EXPECTED_OUTPUT}" linuxd.stdout; then
        echo "Test passed."
        exit 0
    fi

    echo "Test failed."
    exit 1
else
    echo "Test failed."
    exit ${MICROVM_EXIT_CODE}
fi
