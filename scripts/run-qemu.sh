#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#
# Runs Nanvix in QEMU.
#
# Usage:
#   run-qemu.sh <target> <machine> <image> [mode] [timeout] [--wait-for-string <string>]
#
# Modes:
#   --no-debug (default)  Run QEMU normally.
#   --debug               Run QEMU with GDB server attached.
#   --wait-for-string <s> Run QEMU in the background, monitor stdout for <s>,
#                         and terminate QEMU when found or on timeout.
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
export NANVIX_HOME="${NANVIX_HOME:-$(git rev-parse --show-toplevel)}"

# Kernel configuration file.
KERNEL_CONFIG="${SCRIPT_DIR}/../build/kernel_config.toml"

#==================================================================================================
# Argument Parsing
#==================================================================================================

# Positional arguments.
TARGET="${1:-}"
MACHINE="${2:-}"
IMAGE="${3:-}"
MODE="${4:---no-debug}"
TIMEOUT="${5:-}"

# Optional: --wait-for-string <string>.
WAIT_FOR_STRING=""
shift $(( $# > 5 ? 5 : $# ))
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

#==================================================================================================
# Target Configuration
#==================================================================================================

# Parse memory size from kernel configuration file.
MEMSIZE=$(get_value_from_toml "${KERNEL_CONFIG}" "memory_size")
if [[ -z "${MEMSIZE}" || ! "${MEMSIZE}" =~ ^[0-9]+$ ]]; then
	print_error "MEMSIZE is not set or is not a valid integer in ${KERNEL_CONFIG}."
	exit 1
fi

print_info "Memory Size: ${MEMSIZE}"

#===================================================================================================
# usage()
#===================================================================================================

#
# Description
#   Prints script usage and exits.
#
function usage
{
	echo "${SCRIPT_NAME} <target> <machine> <image> [mode] [timeout] [--wait-for-string <string>]"
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
	if [[ -z "${IMAGE}" ]]; then
		print_error "Missing image argument."
		usage
	fi
}

#===================================================================================================
# run_qemu()
#===================================================================================================

#
# Description
#   Runs a binary in QEMU.
#
# Arguments
#   $1 - Target architecture (e.g., i386).
#   $2 - Machine type (e.g., qemu-pc).
#   $3 - Image file (e.g., nanvix.iso).
#   $4 - Run mode (--no-debug, --debug, or --no-debug with WAIT_FOR_STRING set).
#   $5 - Timeout in seconds (optional).
#
function run_qemu
{
	local target=$1     # Target architecture.
	local machine=$2    # Machine.
	local image=$3      # Image.
	local mode=$4       # Spawn mode (run or debug).
	local timeout=$5    # Timeout for test mode.
	local GDB_PORT=1234 # GDB port used for debugging.
	local cmd=""

	# Check if the target is unsupported.
	if [[ "${target}" != "i386" ]]; then
		print_error "Unsupported target: ${target}"
		exit 1
	fi

	local qemu_machine=""
	local stdout=""
	local smp=""

	case "${machine}" in
		"qemu-baremetal")
			qemu_machine="-machine pc"
			stdout="-serial stdio"
			smp=""
			;;
		"qemu-baremetal-smp")
			qemu_machine="-machine pc"
			stdout="-serial stdio"
			smp="-smp 2"
			;;
		"qemu-pc")
			qemu_machine="-machine pc"
			stdout="-debugcon stdio"
			smp=""
			;;
		"qemu-pc-smp")
			qemu_machine="-machine pc"
			stdout="-debugcon stdio"
			smp="-smp 2"
			;;
		"qemu-isapc")
			qemu_machine="-machine isapc"
			stdout="-debugcon stdio"
			smp=""
			;;
		*)
			print_error "Unsupported machine: ${machine}"
			exit 1
			;;
	esac

	# Select QEMU from path, if available.
	local qemu_bin=""
	if command -v "qemu-system-${target}" >/dev/null 2>&1; then
		qemu_bin="qemu-system-${target}"
	else
		qemu_bin="${TOOLCHAIN_DIR}/qemu/bin/qemu-system-${target}"
	fi

	local qemu_cmd="${qemu_bin} \
		${qemu_machine} \
		${stdout} \
		${smp} \
		-display none \
		-cpu pentium3 \
		-m ${MEMSIZE}B \
		-mem-prealloc"

	cmd="${qemu_cmd} -cdrom ${image}"

	# Debug mode: attach GDB server and wait.
	if [[ "${mode}" = "--debug" ]]; then
		cmd="${cmd} -gdb tcp::${GDB_PORT} -S"
		# shellcheck disable=SC2086 # cmd is a composed command string; allow word splitting into separate args
		${cmd}
		return
	fi

	# Wait-for-string mode: run QEMU in the background, monitor stdout for a
	# specific string, and terminate QEMU when the string is found or on timeout.
	if [[ -n "${WAIT_FOR_STRING}" ]]; then
		run_qemu_wait_for_string "${cmd}" "${timeout}"
		return
	fi

	# Normal mode: run QEMU with optional timeout.
	if [[ -n "${timeout}" ]]; then
		cmd="timeout -s SIGINT --preserve-status --foreground ${timeout} ${cmd}"
	fi

	# shellcheck disable=SC2086 # cmd is a composed command string; allow word splitting into separate args
	${cmd} 2> stderr.log
}

#===================================================================================================
# run_qemu_wait_for_string()
#===================================================================================================

#
# Description
#   Runs QEMU in the background, monitors stdout for a specific string, and
#   terminates QEMU when the string is found or on timeout.
#
# Arguments
#   $1 - The full QEMU command to run.
#   $2 - Timeout in seconds.
#
function run_qemu_wait_for_string
{
	local cmd="$1"
	local timeout="${2:-600}"
	local log_file="run-stdout.log"

	rm -f "${log_file}"
	touch "${log_file}"

	print_info "Waiting for string '${WAIT_FOR_STRING}' (timeout: ${timeout}s)..."

	# Run QEMU in the background with stdout teed to a log file.
	# Use setsid to create a new process group for clean termination.
	# shellcheck disable=SC2086 # cmd is a composed command string; allow word splitting into separate args
	setsid ${cmd} > >(tee "${log_file}") 2>&1 &
	local qemu_pid=$!

	# Monitor stdout for the magic string.
	local elapsed=0
	while [[ "${elapsed}" -lt "${timeout}" ]]; do
		if grep -q "${WAIT_FOR_STRING}" "${log_file}" 2>/dev/null; then
			print_success "String '${WAIT_FOR_STRING}' found. Terminating QEMU."
			kill -INT -"${qemu_pid}" 2>/dev/null || true
			wait "${qemu_pid}" 2>/dev/null || true
			return 0
		fi

		# Check if QEMU has already exited.
		if ! kill -0 "${qemu_pid}" 2>/dev/null; then
			break
		fi

		sleep 1
		elapsed=$((elapsed + 1))
	done

	# Timeout or QEMU exited without the expected string.
	kill -INT -"${qemu_pid}" 2>/dev/null || true
	wait "${qemu_pid}" 2>/dev/null || true

	print_error "String '${WAIT_FOR_STRING}' not found within ${timeout}s."
	cat "${log_file}" || true
	return 1
}

#===================================================================================================
# Main
#===================================================================================================

# Verbose mode.
echo "====================================================================="
echo "TARGET           = ${TARGET}"
echo "MACHINE          = ${MACHINE}"
echo "SCRIPT_DIR       = ${SCRIPT_DIR}"
echo "SCRIPT_NAME      = ${SCRIPT_NAME}"
echo "IMAGE            = ${IMAGE}"
echo "MODE             = ${MODE}"
echo "TIMEOUT          = ${TIMEOUT}"
echo "WAIT_FOR_STRING  = ${WAIT_FOR_STRING}"
echo "====================================================================="

case "${TARGET}" in
	"x86")
		check_args
		case "${MACHINE}" in
			"qemu-baremetal" | "qemu-baremetal-smp" | "qemu-pc" | "qemu-pc-smp" | "qemu-isapc")
				run_qemu "i386" "${MACHINE}" "${IMAGE}" "${MODE}" "${TIMEOUT}"
				;;
			*)
				print_error "Unsupported machine: ${MACHINE}"
				exit 1
				;;
		esac
		;;
	*)
		print_error "Unsupported target: ${TARGET}"
		exit 1
		;;
esac
