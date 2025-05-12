#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Ask for sudo password in advance
sudo -v

# Directory in which the Nanvix toolchain is installed.
TOOLCHAIN_DIR=${1:-$(pwd)/toolchain}

# TTY device on which output should be redirected.
TTY=${2:-/dev/null}

# Timeout for each step.
TIMEOUT=300

# Padding to force aligned formatting.
PADDING=40

# Colors for output messages.
GREEN_COLOR='\033[0;32m'
RED_COLOR='\033[0;31m'
NO_COLOR='\033[0m'

# Counters for passed and failed steps
passed_count=0
failed_count=0

# Total elapsed time
total_elapsed_time=0

for mode in "debug" "release";
do
    for step in "lint" "build" "test";
    do
        for target in "qemu-isapc" "qemu-pc" "qemu-baremetal" "microvm" "hyperlight";
        do
            msg="$mode $step $target"
            printf "%-${PADDING}s" "$msg"
            start_time=$(date +%s%3N)
            python3 scripts/ci.py --${mode} --toolchain-dir=$TOOLCHAIN_DIR --target-arch=x86 --log-level=trace --target-machine=$target --${step} --timeout $TIMEOUT > $TTY
            return_code=$?
            end_time=$(date +%s%3N)
            elapsed_time=$((end_time - start_time))
            total_elapsed_time=$((total_elapsed_time + elapsed_time))
            elapsed_seconds=$((elapsed_time / 1000))
            elapsed_milliseconds=$((elapsed_time % 1000))
            if [ $return_code -ne 0 ]; then
                echo -e " [${RED_COLOR}FAILED${NO_COLOR}] ${elapsed_seconds}.${elapsed_milliseconds}s"
                failed_count=$((failed_count + 1))
            else
                echo -e " [${GREEN_COLOR}PASSED${NO_COLOR}] ${elapsed_seconds}.${elapsed_milliseconds}s"
                passed_count=$((passed_count + 1))
            fi
        done
    done
done

# Print summary
total_seconds=$((total_elapsed_time / 1000))
total_milliseconds=$((total_elapsed_time % 1000))
echo "Total Passed: $passed_count"
echo "Total Failed: $failed_count"
echo "Total Time: ${total_seconds}.${total_milliseconds}s"

# Exit with error if there are failed steps
if [ $failed_count -ne 0 ]; then
    exit 1
fi
