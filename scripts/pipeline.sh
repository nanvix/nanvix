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

# Path to the z helper script.
Z_SCRIPT="${REPO_ROOT_DIR}/z"

# Directory where to find scripts to import.
IMPORT_DIR="${REPO_ROOT_DIR}/scripts/common"

source "${IMPORT_DIR}/logging.sh"

#===================================================================================================
# Global Variables
#===================================================================================================

# Configuration matrix for testing all supported machines.
declare -a MACHINE_TYPES=("microvm")
declare -a BUILD_TYPES=("debug" "release")
declare -a DEPLOYMENT_TYPES=("standalone" "single-process")
declare -a STEP_TYPES=("spellcheck" "format" "lint" "build" "test")
declare -a MACHINE_INDEPENDENT_STEPS=("spellcheck" "format")

# Padding to force aligned formatting.
PADDING=60

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
#   Converts deployment type to build flags.
#
# Arguments
#   $1 - Deployment type.
#
# Returns
#   String with DEPLOYMENT_MODE value.
#
# Usage Example
#   flags=$(get_deployment_flags "single-process")
#
get_deployment_flags() {
    local deployment_type="${1}"

    case "${deployment_type}" in
        standalone|single-process)
            echo "DEPLOYMENT_MODE=${deployment_type}"
            ;;
        *)
            print_error "(pipeline) Invalid deployment type: ${deployment_type}"
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
        format)
            echo "format-check"
            ;;
        spellcheck)
            echo "spellcheck"
            ;;
        lint)
            echo "lint-check"
            ;;
        build)
            echo "all"
            ;;
        test)
            echo "test"
            ;;
        *)
            print_error "(pipeline) Invalid step type: ${step}"
            exit 1
            ;;
    esac
}

#
# Description
#   Checks if a step is machine-independent.
#
# Arguments
#   $1 - Step type.
#
# Returns
#   0 if machine-independent, 1 otherwise.
#
# Usage Example
#   if is_machine_independent "lint"; then
#
is_machine_independent() {
    local step="${1}"
    local machine_independent_step

    for machine_independent_step in "${MACHINE_INDEPENDENT_STEPS[@]}"; do
        if [[ "${step}" == "${machine_independent_step}" ]]; then
            return 0
        fi
    done
    return 1
}

#
# Description
#   Runs a single pipeline step and reports the result.
#
# Arguments
#   $1 - Build type (debug or release).
#   $2 - Step type (format, spellcheck, lint, build, test).
#   $3 - Temporary file path for capturing output.
#   $4 - Machine type (optional, empty for machine-independent steps).
#   $5 - Deployment type (optional, empty for machine-independent steps).
#
# Returns
#   0 on success, non-zero on failure.
#
# Usage Example
#   run_step "debug" "lint" "/tmp/output" "" ""
#   run_step "release" "build" "/tmp/output" "microvm" "single-process"
#
run_step() {
    local build_type="${1}"
    local step="${2}"
    local tmpfile="${3}"
    local machine="${4-}"
    local deployment="${5-}"
    local release_flag
    local deployment_flags
    local make_target
    local msg
    local start_time
    local end_time
    local elapsed_time
    local elapsed_seconds
    local elapsed_milliseconds
    local return_code

    release_flag=$(get_release_flag "${build_type}")
    make_target=$(get_make_target "${step}")

    if [[ -z "${machine}" ]]; then
        msg="${build_type} ${step}"
    else
        deployment_flags=$(get_deployment_flags "${deployment}")
        msg="${build_type} ${step} ${machine} ${deployment}"
    fi
    printf "%-${PADDING}s" "${msg}"

    start_time=$(date +%s%3N)

    # Run the step and capture return code.
    if [[ -z "${machine}" ]]; then
        if "${Z_SCRIPT}" build -- LOG_LEVEL=trace "${release_flag}" "${make_target}" > "${tmpfile}" 2>&1; then
            return_code=0
        else
            return_code=$?
        fi
    else
        # shellcheck disable=SC2086 # deployment_flags is a list of VAR=VAL words; allow word splitting into separate args
        if "${Z_SCRIPT}" build -- MACHINE="${machine}" LOG_LEVEL=trace "${release_flag}" ${deployment_flags} "${make_target}" > "${tmpfile}" 2>&1; then
            return_code=0
        else
            return_code=$?
        fi
    fi

    end_time=$(date +%s%3N)
    elapsed_time=$((end_time - start_time))
    total_elapsed_time=$((total_elapsed_time + elapsed_time))
    elapsed_seconds=$((elapsed_time / 1000))
    elapsed_milliseconds=$((elapsed_time % 1000))

    # Report result.
    if [[ ${return_code} -ne 0 ]]; then
        cat "${tmpfile}"
        print_error "${elapsed_seconds}.${elapsed_milliseconds}s"
        failed_count=$((failed_count + 1))
        return 1
    else
        print_success "${elapsed_seconds}.${elapsed_milliseconds}s"
        passed_count=$((passed_count + 1))
        return 0
    fi
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

    # Ensure helper script exists even when running outside the repository root.
    if [[ ! -x "${Z_SCRIPT}" ]]; then
        print_error "(pipeline) Cannot find executable z helper at ${Z_SCRIPT}."
        exit 1
    fi

    # Create a temporary file to capture output.
    tmpfile="$(mktemp)"
    trap 'rm -f "$tmpfile"' EXIT

    print_info "(pipeline) Running CI pipeline for all configurations..."
    print_info "(pipeline) Matrix: ${#BUILD_TYPES[@]} build types x ${#MACHINE_TYPES[@]} machines x ${#DEPLOYMENT_TYPES[@]} deployments x ${#STEP_TYPES[@]} steps"

    # Track which machine-independent steps have been run for each build type.
    declare -A run_machine_independent_steps

    # Iterate through all build types, machines, deployments, and steps.
    for build_type in "${BUILD_TYPES[@]}"; do
        for machine in "${MACHINE_TYPES[@]}"; do
            for deployment in "${DEPLOYMENT_TYPES[@]}"; do
                for step in "${STEP_TYPES[@]}"; do
                    # Check if this step is machine-independent.
                    if is_machine_independent "${step}"; then
                        # Run machine-independent steps only once per build type.
                        local key="${build_type}_${step}"
                        if [[ -z "${run_machine_independent_steps[${key}]+isset}" ]]; then
                            run_step "${build_type}" "${step}" "${tmpfile}" "" ""
                            run_machine_independent_steps["${key}"]=1
                        fi
                    else
                        # Run machine-dependent steps for each machine/deployment combination.
                        run_step "${build_type}" "${step}" "${tmpfile}" "${machine}" "${deployment}"
                    fi
                done
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
