#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================

# Fast fail on errors, unset variables, and pipe failures.
set -euo pipefail

#===================================================================================================
# Imports
#===================================================================================================

# Get the repository root directory.
REPO_ROOT_DIR=$(git rev-parse --show-toplevel)

# Directory where to find scripts to import.
IMPORT_DIR="${REPO_ROOT_DIR}/scripts/common"

source "${IMPORT_DIR}/logging.sh"

#===================================================================================================
# Global Variables
#===================================================================================================

# Configuration matrix for testing all supported machines.
declare -a MACHINE_TYPES=("qemu-isapc" "qemu-pc" "qemu-baremetal" "microvm" "hyperlight")
declare -a BUILD_TYPES=("debug" "release")
declare -a STEP_TYPES=("lint" "build" "test")

# Padding to force aligned formatting.
PADDING=40

# Counters for passed and failed steps.
passed_count=0
failed_count=0

# Total elapsed time.
total_elapsed_time=0

#===================================================================================================
# Functions
#===================================================================================================

#
# Description
#   Converts build type to RELEASE flag.
#
# Arguments
#   $1 - Build type.
#
# Returns
#   String with RELEASE value.
#
# Usage Example
#   flag=$(get_release_flag "release")
#
get_release_flag() {
    local build_type="${1}"

    case "${build_type}" in
        debug)
            echo "RELEASE=no"
            ;;
        release)
            echo "RELEASE=yes"
            ;;
        *)
            print_error "(pipeline) Invalid build type: ${build_type}"
            exit 1
            ;;
    esac
}

#
# Description
#   Converts step type to make target.
#
# Arguments
#   $1 - Step type.
#
# Returns
#   String with make target.
#
# Usage Example
#   target=$(get_make_target "lint")
#
get_make_target() {
    local step="${1}"

    case "${step}" in
        lint)
            echo "lint-check"
            ;;
        build)
            echo "all"
            ;;
        test)
            echo "run-unit-tests run-nanvixd-tests"
            ;;
        *)
            print_error "(pipeline) Invalid step type: ${step}"
            exit 1
            ;;
    esac
}

#===================================================================================================
# Main Script
#===================================================================================================

main() {
    # Check if we are running inside a Git repository.
    if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        print_error "(pipeline) This script must be run inside a Git repository."
        exit 1
    fi

    # Create a temporary file to capture output.
    tmpfile="$(mktemp)"
    trap 'rm -f "$tmpfile"' EXIT

    print_info "(pipeline) Running CI pipeline for all configurations..."

    # Iterate through all build types, steps, and machines.
    for build_type in "${BUILD_TYPES[@]}"; do
        for step in "${STEP_TYPES[@]}"; do
            for machine in "${MACHINE_TYPES[@]}"; do
                # Get configuration flags.
                local release_flag
                release_flag=$(get_release_flag "${build_type}")

                # Get make target for this step.
                local make_target
                make_target=$(get_make_target "${step}")

                # Format message for display.
                local msg
                msg="${build_type} ${step} ${machine}"
                printf "%-${PADDING}s" "${msg}"

                # Start timing.
                local start_time
                start_time=$(date +%s%3N)

                # Build command with all parameters.
                local build_command
                build_command="./z build -- MACHINE=${machine} LOG_LEVEL=trace ${release_flag} ${make_target}"

                # Run the step and capture return code.
                local return_code
                if eval "${build_command}" > "${tmpfile}" 2>&1; then
                    return_code=0
                else
                    return_code=$?
                fi

                # End timing.
                local end_time
                end_time=$(date +%s%3N)
                local elapsed_time
                elapsed_time=$((end_time - start_time))
                total_elapsed_time=$((total_elapsed_time + elapsed_time))
                local elapsed_seconds
                elapsed_seconds=$((elapsed_time / 1000))
                local elapsed_milliseconds
                elapsed_milliseconds=$((elapsed_time % 1000))

                # Report result.
                if [[ ${return_code} -ne 0 ]]; then
                    cat "${tmpfile}"
                    print_error "${elapsed_seconds}.${elapsed_milliseconds}s"
                    failed_count=$((failed_count + 1))
                else
                    print_success "${elapsed_seconds}.${elapsed_milliseconds}s"
                    passed_count=$((passed_count + 1))
                fi
            done
        done
    done

    # Print summary.
    local total_seconds
    total_seconds=$((total_elapsed_time / 1000))
    local total_milliseconds
    total_milliseconds=$((total_elapsed_time % 1000))
    print_info "(pipeline) Total Passed: ${passed_count}"
    print_info "(pipeline) Total Failed: ${failed_count}"
    print_info "(pipeline) Total Time: ${total_seconds}.${total_milliseconds}s"

    # Exit with error if there are failed steps.
    if [[ ${failed_count} -ne 0 ]]; then
        print_error "(pipeline) Pipeline failed with ${failed_count} failed step(s)."
        exit 1
    fi

    print_success "(pipeline) All pipeline steps passed successfully."
}

# Run the main function.
main "${@}"
