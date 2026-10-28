#!/bin/bash
# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# shellcheck source=/dev/null

#
# Unified cleanup script for Nanvix resources.
#
# This script consolidates all cleanup logic used across:
# - .github/actions/cleanup/action.yml
#
# Usage:
#   cleanup-nanvixd.sh [OPTIONS]
#
# Options:
#   --kill-processes          Kill dangling Nanvix processes (nanvixd and uservm).
#   --kill-process-group      Kill a specific process group gracefully (SIGTERM then SIGKILL).
#   --process-group-pid PID   PID of the process group to kill (requires --kill-process-group).
#   --graceful-timeout SEC    Timeout for graceful shutdown in seconds (default: 10).
#   --sockets                 Clean up leaked gateway sockets (/tmp/nvx-standalone-gw-*.sock).
#   --list-sockets            List leaked gateway sockets, one path per line on stdout.
#   --temp-dirs               Clean up temporary directories (nvx:*, nanvix-test-*, etc.).
#   --wait-port PORT          Wait for a TCP port to become available.
#   --wait-port-timeout SEC   Maximum time to wait for port (default: 70).
#   --all                     Perform all cleanup actions (except process group kill and port wait).
#   --verbose                 Enable verbose output.
#   --help                    Show this help message.
#

set -euo pipefail

#===================================================================================================
# Constants
#===================================================================================================

readonly SCRIPT_NAME="cleanup-nanvixd.sh"
readonly DEFAULT_GRACEFUL_TIMEOUT=10
readonly DEFAULT_PORT_WAIT_TIMEOUT=70
readonly POLL_INTERVAL_SECONDS=2
readonly POLL_INTERVAL_DECISECONDS=1
# Directory of the gateway sockets that nanvixd binds by default, and regular expression that
# matches their names (nvx-standalone-gw-<pid>.sock). Matching nothing but digits for the process ID
# ensures that listed paths never contain a newline, so they can be safely processed one per line.
readonly GATEWAY_SOCKET_DIR="/tmp"
readonly GATEWAY_SOCKET_NAME_REGEX="nvx-standalone-gw-[0-9]+\.sock"

#===================================================================================================
# Global Variables
#===================================================================================================

VERBOSE=false
DO_KILL_PROCESSES=false
DO_KILL_PROCESS_GROUP=false
PROCESS_GROUP_PID=""
GRACEFUL_TIMEOUT="${DEFAULT_GRACEFUL_TIMEOUT}"
DO_SOCKETS=false
DO_LIST_SOCKETS=false
DO_TEMP_DIRS=false
WAIT_PORT=""
PORT_WAIT_TIMEOUT="${DEFAULT_PORT_WAIT_TIMEOUT}"

#===================================================================================================
# Helper Functions
#===================================================================================================

log_info() {
    echo "[CLEANUP] $1" 1>&2
}

log_verbose() {
    if [ "${VERBOSE}" = true ]; then
        echo "[CLEANUP] $1" 1>&2
    fi
}

log_warning() {
    echo "[CLEANUP] WARNING: $1" 1>&2
}

log_error() {
    echo "[CLEANUP] ERROR: $1" 1>&2
}

#
# Description
#
#   Validates that a value is a positive integer.
#
# Arguments
#
#   $1 - Value to validate.
#   $2 - Option name (for error message).
#
# Return Value
#
#   Returns 0 if valid, exits with error otherwise.
#
validate_positive_integer() {
    local value="$1"
    local option="$2"
    if ! [[ "${value}" =~ ^[0-9]+$ ]] || [ "${value}" -le 0 ]; then
        log_error "Invalid value for ${option}: '${value}' (must be a positive integer)."
        exit 1
    fi
}

usage() {
    cat <<EOF
Usage: ${SCRIPT_NAME} [OPTIONS]

Options:
  --kill-processes        Kill dangling Nanvix processes
  --kill-process-group    Kill a specific process group gracefully
  --process-group-pid PID PID of the process group to kill
  --graceful-timeout SEC  Timeout for graceful shutdown (default: ${DEFAULT_GRACEFUL_TIMEOUT})
  --sockets               Clean up leaked nanvixd gateway sockets
  --list-sockets          List leaked nanvixd gateway sockets
  --temp-dirs             Clean up temporary directories
  --wait-port PORT        Wait for a TCP port to become available
  --wait-port-timeout SEC Maximum time to wait for port (default: ${DEFAULT_PORT_WAIT_TIMEOUT})
  --all                   Perform all cleanup actions
  --verbose               Enable verbose output
  --help                  Show this help message

Examples:
  # Full cleanup (CI pre/post)
  ${SCRIPT_NAME} --all --verbose

  # Clean up after a test run
  ${SCRIPT_NAME} --sockets --temp-dirs

  # Check for gateway sockets leaked by nanvixd
  ${SCRIPT_NAME} --list-sockets

  # Kill dangling processes and wait for port
  ${SCRIPT_NAME} --kill-processes --wait-port 9999

  # Graceful process group shutdown
  ${SCRIPT_NAME} --kill-process-group --process-group-pid 12345
EOF
}

#===================================================================================================
# Cleanup Functions
#===================================================================================================

#
# Description
#
#   Kills dangling Nanvix processes from previous runs.
#
cleanup_kill_processes() {
    log_info "Killing dangling Nanvix processes..."
    local killed=0

    # Use -f to match against full command line instead of -x (exact comm match).
    # The comm field in /proc/[pid]/comm may differ from the executable name.
    for proc in nanvixd.elf uservm.elf; do
        if pgrep -f "${proc}" > /dev/null 2>&1; then
            log_verbose "Killing ${proc} processes..."
            sudo pkill -9 -f "${proc}" 2>/dev/null || true
            killed=$((killed + 1))
        fi
    done

    if [ "${killed}" -eq 0 ]; then
        log_verbose "No dangling processes found."
    else
        log_info "Killed ${killed} process type(s)."
    fi
}

#
# Description
#
#   Kills a process group gracefully (SIGTERM then SIGKILL).
#
# Arguments
#
#   $1 - PID of the process group leader.
#   $2 - Graceful timeout in seconds.
#
cleanup_kill_process_group() {
    local pid="$1"
    local timeout="$2"

    if [ -z "${pid}" ]; then
        log_error "No process group PID specified."
        return 1
    fi

    # Check if process group exists.
    if ! kill -0 -- "-${pid}" 2>/dev/null; then
        log_verbose "Process group ${pid} is not running."
        return 0
    fi

    log_info "Killing process group ${pid}..."

    # First try graceful shutdown with SIGTERM to allow Drop handlers to run.
    log_verbose "Sending SIGTERM for graceful shutdown..."
    kill -s SIGTERM -- "-${pid}" 2>/dev/null || true

    # Wait for graceful shutdown.
    local max_wait_count="$(( (timeout * 10) / POLL_INTERVAL_DECISECONDS ))"
    log_verbose "Waiting up to ${timeout}s for graceful shutdown..."
    local count=0
    while kill -0 -- "-${pid}" 2>/dev/null && [ "${count}" -lt "${max_wait_count}" ]; do
        sleep "0.${POLL_INTERVAL_DECISECONDS}"
        count=$((count + 1))
    done

    # If still running, force kill.
    if kill -0 -- "-${pid}" 2>/dev/null; then
        log_warning "Process group ${pid} did not exit gracefully, forcing kill..."
        kill -s SIGKILL -- "-${pid}" 2>/dev/null || true
    else
        log_verbose "Process group ${pid} exited gracefully."
    fi

    wait "${pid}" 2>/dev/null || true
    log_info "Process group ${pid} cleanup completed."
}

#
# Description
#
#   Lists the gateway sockets that nanvixd left behind, one path per line on stdout.
#
#   nanvixd binds a gateway socket for each VM, and it removes the socket when it tears the VM
#   down, including when it shuts down gracefully on SIGINT, SIGTERM, SIGHUP, or SIGQUIT. Once
#   every nanvixd process is gone, any remaining gateway socket was leaked, which happens when
#   nanvixd is killed with SIGKILL or crashes.
#
list_sockets() {
    find "${GATEWAY_SOCKET_DIR}" -maxdepth 1 -type s -regextype posix-extended \
        -regex "${GATEWAY_SOCKET_DIR}/${GATEWAY_SOCKET_NAME_REGEX}" 2>/dev/null | sort || true
}

#
# Description
#
#   Cleans up the gateway sockets that nanvixd left behind.
#
cleanup_sockets() {
    log_info "Cleaning up gateway sockets..."
    local socket
    while IFS= read -r socket; do
        log_verbose "Removing ${socket}..."
        if ! rm -f -- "${socket}" 2>/dev/null; then
            log_warning "Failed to remove ${socket}."
        fi
    done < <(list_sockets)
    log_verbose "Socket cleanup completed."
}

#
# Description
#
#   Cleans up temporary directories created by Nanvix.
#
cleanup_temp_dirs() {
    log_info "Cleaning up temporary directories..."

    # nvx:* directories (created by TemporaryDirectory in nanvixd).
    sudo find /tmp -maxdepth 1 -name 'nvx:*' -exec rm -rf {} + 2>/dev/null || true
    log_verbose "Removed nvx:* directories."

    # nanvix-test-* directories.
    rm -rf /tmp/nanvix-test-* 2>/dev/null || true
    log_verbose "Removed nanvix-test-* directories."

    log_info "Temporary directory cleanup completed."
}

#
# Description
#
#   Checks if a TCP port is available.
#
# Arguments
#
#   $1 - Port number.
#
# Return Value
#
#   Returns 0 if port is available, 1 otherwise.
#
is_port_available() {
    local port="$1"

    # Check if port is in LISTEN state.
    if command -v ss &> /dev/null; then
        if ss -tln "sport = :${port}" 2>/dev/null | tail -n +2 | grep -q .; then
            return 1
        fi
        # Check if port is in TIME_WAIT state.
        if ss -tan state time-wait "sport = :${port}" 2>/dev/null | tail -n +2 | grep -q .; then
            return 1
        fi
    elif command -v netstat &> /dev/null; then
        if netstat -tln 2>/dev/null | grep -Eq ":${port}[[:space:]]+.*LISTEN"; then
            return 1
        fi
        if netstat -tan 2>/dev/null | grep -E ":${port}[[:space:]]+" | grep -q TIME_WAIT; then
            return 1
        fi
    fi

    return 0
}

#
# Description
#
#   Waits for a TCP port to become available.
#
# Arguments
#
#   $1 - Port number.
#   $2 - Maximum wait time in seconds.
#
# Return Value
#
#   Returns 0 if port becomes available, 1 on timeout.
#
wait_for_port() {
    local port="$1"
    local max_wait="$2"
    local start_time
    start_time=$(date +%s)

    log_info "Waiting for port ${port} to become available (max ${max_wait}s)..."

    while true; do
        local current_time
        current_time=$(date +%s)
        local elapsed=$((current_time - start_time))

        if is_port_available "${port}"; then
            if [ "${elapsed}" -gt 0 ]; then
                log_info "Port ${port} is available (waited ${elapsed}s)."
            else
                log_info "Port ${port} is available."
            fi
            return 0
        fi

        if [ "${elapsed}" -ge "${max_wait}" ]; then
            log_error "Port ${port} not available after ${max_wait}s."
            ss -tan "sport = :${port}" 2>/dev/null || true
            return 1
        fi

        log_verbose "Port ${port} in use, waiting... (${elapsed}s elapsed)"
        sleep "${POLL_INTERVAL_SECONDS}"
    done
}

#===================================================================================================
# Argument Parsing
#===================================================================================================

parse_arguments() {
    while [ $# -gt 0 ]; do
        case "$1" in
            --kill-processes)
                DO_KILL_PROCESSES=true
                shift
                ;;
            --kill-process-group)
                DO_KILL_PROCESS_GROUP=true
                shift
                ;;
            --process-group-pid)
                shift
                [ $# -eq 0 ] && { log_error "Missing value for --process-group-pid."; exit 1; }
                validate_positive_integer "$1" "--process-group-pid"
                PROCESS_GROUP_PID="$1"
                shift
                ;;
            --graceful-timeout)
                shift
                [ $# -eq 0 ] && { log_error "Missing value for --graceful-timeout."; exit 1; }
                validate_positive_integer "$1" "--graceful-timeout"
                GRACEFUL_TIMEOUT="$1"
                shift
                ;;
            --sockets)
                DO_SOCKETS=true
                shift
                ;;
            --list-sockets)
                DO_LIST_SOCKETS=true
                shift
                ;;
            --temp-dirs)
                DO_TEMP_DIRS=true
                shift
                ;;
            --wait-port)
                shift
                [ $# -eq 0 ] && { log_error "Missing value for --wait-port."; exit 1; }
                validate_positive_integer "$1" "--wait-port"
                WAIT_PORT="$1"
                shift
                ;;
            --wait-port-timeout)
                shift
                [ $# -eq 0 ] && { log_error "Missing value for --wait-port-timeout."; exit 1; }
                validate_positive_integer "$1" "--wait-port-timeout"
                PORT_WAIT_TIMEOUT="$1"
                shift
                ;;
            --all)
                DO_KILL_PROCESSES=true
                DO_SOCKETS=true
                DO_TEMP_DIRS=true
                shift
                ;;
            --verbose)
                VERBOSE=true
                shift
                ;;
            --help)
                usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1."
                usage
                exit 1
                ;;
        esac
    done
}

#===================================================================================================
# Main Script
#===================================================================================================

main() {
    parse_arguments "$@"

    # Validate --kill-process-group and --process-group-pid relationship.
    if [ "${DO_KILL_PROCESS_GROUP}" = true ] && [ -z "${PROCESS_GROUP_PID}" ]; then
        log_error "--kill-process-group requires --process-group-pid."
        exit 1
    fi
    if [ -n "${PROCESS_GROUP_PID}" ] && [ "${DO_KILL_PROCESS_GROUP}" = false ]; then
        log_error "--process-group-pid requires --kill-process-group."
        exit 1
    fi

    local any_action=false

    # List leaked sockets before any cleanup action gets a chance to remove them.
    if [ "${DO_LIST_SOCKETS}" = true ]; then
        list_sockets
        any_action=true
    fi

    if [ "${DO_KILL_PROCESSES}" = true ]; then
        cleanup_kill_processes
        any_action=true
    fi

    if [ "${DO_KILL_PROCESS_GROUP}" = true ]; then
        cleanup_kill_process_group "${PROCESS_GROUP_PID}" "${GRACEFUL_TIMEOUT}"
        any_action=true
    fi

    if [ "${DO_SOCKETS}" = true ]; then
        cleanup_sockets
        any_action=true
    fi

    if [ "${DO_TEMP_DIRS}" = true ]; then
        cleanup_temp_dirs
        any_action=true
    fi

    if [ -n "${WAIT_PORT}" ]; then
        if ! wait_for_port "${WAIT_PORT}" "${PORT_WAIT_TIMEOUT}"; then
            exit 1
        fi
        any_action=true
    fi

    if [ "${any_action}" = false ]; then
        log_warning "No cleanup action specified. Use --help for usage."
        exit 1
    fi

    log_info "All cleanup actions completed."
}

main "$@"
