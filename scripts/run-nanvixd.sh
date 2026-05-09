#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#
# Runs Nanvix via nanvixd (microvm).
#
# Usage:
#   run-nanvixd.sh <machine> <image> [timeout] [--wait-for-string <string>]
#
# Arguments:
#   machine  Machine type (microvm).
#   image    Path to the multibin system image.
#   timeout  Timeout in seconds (default: 120).
#
# Options:
#   --wait-for-string <s>  Monitor console output for <s> and terminate nanvixd
#                          when found or on timeout.
#
# Environment:
#   NANVIXD            Path to nanvixd binary  (default: ./bin/nanvixd.elf).
#   CLH_BIN_PATH       CLH binary directory     (default: $HOME/toolchain/bin).
#   LOG_DIR            Log output directory     (default: ./logs).
#   RUST_LOG           Rust log level           (default: info).
#

# Fast fail on errors and unset variables.
set -euo pipefail

#==================================================================================================
# Imports
#==================================================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
source "${SCRIPT_DIR}/common/utils.sh"

#==================================================================================================
# Global Variables
#==================================================================================================

export SCRIPT_NAME="$0"
export SCRIPT_DIR

# Defaults for nanvixd configuration.
NANVIXD="${NANVIXD:-./bin/nanvixd.elf}"
CLH_BIN_PATH="${CLH_BIN_PATH:-${HOME}/toolchain/bin}"
LOG_DIR="${LOG_DIR:-./logs}"
export RUST_LOG="${RUST_LOG:-info}"

#==================================================================================================
# Argument Parsing
#==================================================================================================

# Positional arguments.
MACHINE="${1:-}"
IMAGE="${2:-}"
TIMEOUT="${3:-120}"

# Optional: --wait-for-string <string>.
WAIT_FOR_STRING=""
shift $(( $# > 3 ? 3 : $# ))
while [[ $# -gt 0 ]]; do
	case "$1" in
		--wait-for-string)
			if [[ $# -lt 2 ]]; then
				print_error "Missing argument for --wait-for-string."
				exit 1
			fi
			WAIT_FOR_STRING="$2"
			shift 2
			;;
		*)
			print_error "Unknown option: $1"
			exit 1
			;;
	esac
done

#===================================================================================================
# usage()
#===================================================================================================

#
# Description
#   Prints script usage and exits.
#
function usage
{
	echo "${SCRIPT_NAME} <machine> <image> [timeout] [--wait-for-string <string>]"
	exit 1
}

#===================================================================================================
# check_args()
#===================================================================================================

#
# Description
#   Validates required script arguments.
#
function check_args
{
	# Validate machine type.
	case "${MACHINE}" in
		microvm) ;;
		*)
			print_error "Unsupported machine type: '${MACHINE}'. Expected 'microvm'."
			usage
			;;
	esac

	# Validate image path.
	if [[ -z "${IMAGE}" ]]; then
		print_error "Missing image argument."
		usage
	fi
	if [[ ! -f "${IMAGE}" ]]; then
		print_error "Image file not found: ${IMAGE}"
		exit 1
	fi

	# Validate nanvixd binary.
	if [[ ! -x "${NANVIXD}" ]]; then
		print_error "nanvixd binary not found or not executable: ${NANVIXD}"
		exit 1
	fi

	# Validate timeout is a positive integer.
	if [[ ! "${TIMEOUT}" =~ ^[0-9]+$ ]] || [[ "${TIMEOUT}" -eq 0 ]]; then
		print_error "Timeout must be a positive integer, got: '${TIMEOUT}'"
		exit 1
	fi
}

#===================================================================================================
# run_nanvixd()
#===================================================================================================

#
# Description
#   Runs nanvixd with optional timeout.
#
function run_nanvixd
{
	local cmd="${NANVIXD} -console-file /dev/stdout -clh-bin-path ${CLH_BIN_PATH} -log-dir ${LOG_DIR} -- ${IMAGE}"

	# Wait-for-string mode: run nanvixd in the background, monitor console output
	# for a specific string, and terminate nanvixd when the string is found or on
	# timeout.
	if [[ -n "${WAIT_FOR_STRING}" ]]; then
		run_nanvixd_wait_for_string "${cmd}" "${TIMEOUT}"
		return
	fi

	# Normal mode: run nanvixd with timeout.
	cmd="timeout -s SIGTERM --preserve-status --foreground ${TIMEOUT} ${cmd}"

	# shellcheck disable=SC2086 # cmd is a composed command string; allow word splitting into separate args
	${cmd} 2> stderr.log
}

#===================================================================================================
# run_nanvixd_wait_for_string()
#===================================================================================================

#
# Description
#   Runs nanvixd in the background, monitors console output for a specific
#   string, and terminates nanvixd when the string is found or on timeout.
#
# Arguments
#   $1 - The full nanvixd command to run.
#   $2 - Timeout in seconds.
#
function run_nanvixd_wait_for_string
{
	local cmd="$1"
	local timeout="${2:-120}"
	local log_file="run-stdout.log"

	rm -f "${log_file}"
	touch "${log_file}"

	print_info "Waiting for string '${WAIT_FOR_STRING}' (timeout: ${timeout}s)..."

	# Run nanvixd in the background with stdout teed to a log file.
	# Use setsid to create a new process group for clean termination.
	# shellcheck disable=SC2086 # cmd is a composed command string; allow word splitting into separate args
	setsid ${cmd} > >(tee "${log_file}") 2>&1 &
	local nanvixd_pid=$!

	# Monitor console output for the magic string.
	local elapsed=0
	while [[ "${elapsed}" -lt "${timeout}" ]]; do
		if grep -q "${WAIT_FOR_STRING}" "${log_file}" 2>/dev/null; then
			print_success "String '${WAIT_FOR_STRING}' found. Terminating nanvixd."
			kill -TERM -"${nanvixd_pid}" 2>/dev/null || true
			wait "${nanvixd_pid}" 2>/dev/null || true
			return 0
		fi

		# Check if nanvixd has already exited.
		if ! kill -0 "${nanvixd_pid}" 2>/dev/null; then
			# Process exited; check if the string appeared in the final output.
			if grep -q "${WAIT_FOR_STRING}" "${log_file}" 2>/dev/null; then
				print_success "String '${WAIT_FOR_STRING}' found (nanvixd exited)."
				return 0
			fi
			break
		fi

		sleep 1
		elapsed=$((elapsed + 1))
	done

	# Timeout or nanvixd exited without the expected string.
	kill -TERM -"${nanvixd_pid}" 2>/dev/null || true
	wait "${nanvixd_pid}" 2>/dev/null || true

	print_error "String '${WAIT_FOR_STRING}' not found within ${timeout}s."
	cat "${log_file}" || true
	return 1
}

#===================================================================================================
# Main
#===================================================================================================

# Verbose mode.
echo "====================================================================="
echo "MACHINE          = ${MACHINE}"
echo "IMAGE            = ${IMAGE}"
echo "NANVIXD          = ${NANVIXD}"
echo "CLH_BIN_PATH     = ${CLH_BIN_PATH}"
echo "LOG_DIR          = ${LOG_DIR}"
echo "RUST_LOG         = ${RUST_LOG}"
echo "TIMEOUT          = ${TIMEOUT}"
echo "WAIT_FOR_STRING  = ${WAIT_FOR_STRING}"
echo "====================================================================="

check_args

mkdir -p "${LOG_DIR}"

run_nanvixd
