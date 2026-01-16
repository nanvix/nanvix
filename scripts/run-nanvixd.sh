#!/bin/bash

#
# A runner for Nanvix.
#
# Run './run-nanvixd.sh --help' for more information on how to use this utility.
#

#===================================================================================================
# Constants
#===================================================================================================

readonly DEFAULT_TENANT_ID="${USER}"
readonly DEFAULT_APP_NAME="default"
readonly DEFAULT_NANVIXD_HOST="127.0.0.1"
readonly DEFAULT_NANVIXD_PORT="8888"
readonly DEFAULT_NANVIXD_SOCKADDR="${DEFAULT_NANVIXD_HOST}:${DEFAULT_NANVIXD_PORT}"
readonly NANVIXD_BINARY_NAME="nanvixd.elf"
readonly MAX_TRIALS=100
readonly SLEEP_INTERVAL=0.1
readonly DEFAULT_TOOLCHAIN_BIN_DIR="${PWD}/toolchain/bin"
readonly DEFAULT_BIN_DIR="${PWD}/bin"
readonly DEFAULT_LOG_LEVEL="warn"
readonly OPTION_APP_NAME="--app-name"
readonly OPTION_BIN_DIR="--bin-dir"
readonly OPTION_HELP="--help"
readonly OPTION_LOG_LEVEL="--log-level"
readonly OPTION_NANVIXD_SOCKADDR="--nanvixd-sockaddr"
readonly OPTION_TENANT_ID="--tenant-id"
readonly OPTION_TOOLCHAIN_BIN_DIR="--toolchain-bin-dir"

#===================================================================================================
# Global Variables
#===================================================================================================

# CLI parameters.
TENANT_ID="${DEFAULT_TENANT_ID}"
APP_NAME="${DEFAULT_APP_NAME}"
NANVIXD_SOCKADDR="${DEFAULT_NANVIXD_SOCKADDR}"
TOOLCHAIN_BIN_DIR="${DEFAULT_TOOLCHAIN_BIN_DIR}"
BIN_DIR="${DEFAULT_BIN_DIR}"
LOG_LEVEL="${DEFAULT_LOG_LEVEL}"

# Derived nanvixd endpoint components.
NANVIXD_HOST="${DEFAULT_NANVIXD_HOST}"
NANVIXD_PORT="${DEFAULT_NANVIXD_PORT}"

# PID of the nanvixd process.
NANVIXD_PID=""

# Program to execute and its arguments.
PROGRAM_NAME=""
PROGRAM_ARGS=()

#===================================================================================================
# Helper Functions
#===================================================================================================

#
# Description
#
#   Prints the CLI usage instructions.
#
usage() {
    local fd="${1:-1}"

    # Quoting file descriptors is not supported on all POSIX shells.
    # shellcheck disable=SC2086
    cat <<EOF >&${fd}
Usage: $(basename "$0") [OPTIONS --] PROGRAM_NAME [ARG1 ARG2 ...]

Options:
    ${OPTION_APP_NAME} STR          Specify the application name (default: ${DEFAULT_APP_NAME})
    ${OPTION_BIN_DIR} STR           Specify the bin directory (default: ${DEFAULT_BIN_DIR})
    ${OPTION_HELP}                  Show this help message and exit.
    ${OPTION_LOG_LEVEL} STR         Specify the log level (default: ${DEFAULT_LOG_LEVEL})
    ${OPTION_NANVIXD_SOCKADDR} STR  Specify the nanvixd HTTP socket address (default: ${DEFAULT_NANVIXD_SOCKADDR})
    ${OPTION_TENANT_ID} STR         Specify the tenant ID (default: ${DEFAULT_TENANT_ID})
    ${OPTION_TOOLCHAIN_BIN_DIR} STR Specify the toolchain binary directory (default: ${DEFAULT_TOOLCHAIN_BIN_DIR})
EOF
}

#
# Description
#
#   Prints an error message and exits.
#
die() {
    echo "Error: $1" 1>&2
    echo 1>&2
    usage 2
    exit 1
}

#
# Description
#
#   Sets the nanvixd host and port from a socket address.
#
# Arguments
#
#   $1 - Socket address in HOST:PORT format.
#
set_nanvixd_endpoint() {
    local sockaddr="$1"

    if [[ "${sockaddr}" != *:* ]]; then
        die "Invalid nanvixd socket address '${sockaddr}'. Expected HOST:PORT."
    fi

    local host
    local port
    host=${sockaddr%%:*}
    port=${sockaddr##*:}

    if [ -z "${host}" ]; then
        die "Invalid nanvixd socket address '${sockaddr}'. Host cannot be empty."
    fi

    if [ -z "${port}" ] || ! [[ ${port} =~ ^[0-9]+$ ]]; then
        die "Invalid nanvixd socket address '${sockaddr}'. Port must be numeric."
    fi

    NANVIXD_HOST="${host}"
    NANVIXD_PORT="${port}"
}

#
# Description
#
#   Parses command-line arguments.
#
parse_arguments() {
    local positional=()
    local options_seen=0
    local separator_seen=0

    while [ $# -gt 0 ]; do
        case "$1" in
            "${OPTION_APP_NAME}")
                options_seen=1
                shift
                [ $# -eq 0 ] && die "Missing value for ${OPTION_APP_NAME}."
                [[ "$1" == -* ]] && die "Missing value for ${OPTION_APP_NAME}."
                APP_NAME="$1"
                shift
                continue
                ;;
            "${OPTION_BIN_DIR}")
                options_seen=1
                shift
                [ $# -eq 0 ] && die "Missing value for ${OPTION_BIN_DIR}."
                [[ "$1" == -* ]] && die "Missing value for ${OPTION_BIN_DIR}."
                BIN_DIR="$1"
                shift
                continue
                ;;
            "${OPTION_HELP}")
                usage 1
                exit 0
                ;;
            "${OPTION_LOG_LEVEL}")
                options_seen=1
                shift
                [ $# -eq 0 ] && die "Missing value for ${OPTION_LOG_LEVEL}."
                [[ "$1" == -* ]] && die "Missing value for ${OPTION_LOG_LEVEL}."
                LOG_LEVEL="$1"
                shift
                continue
                ;;
            "${OPTION_NANVIXD_SOCKADDR}")
                options_seen=1
                shift
                [ $# -eq 0 ] && die "Missing value for ${OPTION_NANVIXD_SOCKADDR}."
                [[ "$1" == -* ]] && die "Missing value for ${OPTION_NANVIXD_SOCKADDR}."
                NANVIXD_SOCKADDR="$1"
                shift
                continue
                ;;
            "${OPTION_TENANT_ID}")
                options_seen=1
                shift
                [ $# -eq 0 ] && die "Missing value for ${OPTION_TENANT_ID}."
                [[ "$1" == -* ]] && die "Missing value for ${OPTION_TENANT_ID}."
                TENANT_ID="$1"
                shift
                continue
                ;;
            "${OPTION_TOOLCHAIN_BIN_DIR}")
                options_seen=1
                shift
                [ $# -eq 0 ] && die "Missing value for ${OPTION_TOOLCHAIN_BIN_DIR}."
                [[ "$1" == -* ]] && die "Missing value for ${OPTION_TOOLCHAIN_BIN_DIR}."
                TOOLCHAIN_BIN_DIR="$1"
                shift
                continue
                ;;
            --)
                separator_seen=1
                shift
                while [ $# -gt 0 ]; do
                    positional+=("$1")
                    shift
                done
                break
                ;;
            -*)
                die "Unknown option: $1"
                ;;
            *)
                if [ ${options_seen} -eq 1 ] && [ ${separator_seen} -eq 0 ]; then
                    die "Expected '--' separator before program name '${1}'."
                fi
                positional+=("$1")
                shift
                continue
                ;;
        esac
        shift
    done

    if [ ${options_seen} -eq 1 ] && [ ${separator_seen} -eq 0 ]; then
        die "Expected '--' separator before program arguments."
    fi

    if [ ${#positional[@]} -eq 0 ]; then
        die "Missing program name."
    fi

    PROGRAM_NAME="${positional[0]}"

    if [ ${#positional[@]} -gt 1 ]; then
        PROGRAM_ARGS=("${positional[@]:1}")
    else
        PROGRAM_ARGS=()
    fi
}

#
# Description
#
#   Checks if the system meets the requirements to run nanvixd.
#
# Return Value
#
#   - On success, this function returns zero.
#   - On failure, this function returns non-zero.
#
check_system() {
    # Check if we are running on Linux.
    if [ "$(uname -s)" != "Linux" ]; then
        echo "Error: This operating system is not supported." 1>&2
        return 1
    fi

    # Check if user has access to kvm.
    if [ ! -r /dev/kvm ] || [ ! -w /dev/kvm ]; then
        echo "Error: User does not have read/write access to /dev/kvm." 1>&2
        return 1
    fi

    return 0
}

#
# Description
#
#   Checks if required tools are installed.
#
# Return Value
#
#   - On success, this function returns zero.
#   - On failure, this function returns non-zero.
#
check_tools() {
    # Check if 'nc' is not installed.
    if ! command -v nc &> "/dev/null"; then
        echo "Error: 'nc' is not installed. Please install GNU netcat." 1>&2
        return 1
    fi

    # Check if 'curl' is not installed.
    if ! command -v curl &> "/dev/null"; then
        echo "Error: 'curl' is not installed. Please install curl." 1>&2
        return 1
    fi

    # Check if 'jq' is not installed.
    if ! command -v jq &> "/dev/null"; then
        echo "Error: 'jq' is not installed. Please install jq." 1>&2
        return 1
    fi

    # Check if 'bc' is not installed.
    if ! command -v bc &> "/dev/null"; then
        echo "Error: 'bc' is not installed. Please install bc." 1>&2
        return 1
    fi

    return 0
}

#
# Description
#
#   Waits for a TCP socket to be ready.
#
# Arguments
#
#   $1 - The host address.
#   $2 - The port number.
#
# Return Value
#
#   - On success, this function returns zero.
#   - On failure, this function returns non-zero.
#
wait_for_tcp_socket() {
    local host=$1
    local port=$2

    for i in $(seq 1 $MAX_TRIALS); do
        if nc -z "${host}" "${port}" 2>/dev/null; then
            local elapsed_s
            elapsed_s=$(echo "scale=2; ${i} * ${SLEEP_INTERVAL}" | bc -l)
            if [ "$(echo "${elapsed_s} > 0" | bc -l)" -eq 1 ]; then
                echo "TCP socket ready after ${elapsed_s} s." 1>&2
            fi
            return 0
        fi

        echo "Waiting for TCP socket at $host:$port..." 1>&2
        sleep ${SLEEP_INTERVAL}
    done

    return 1
}

#
# Description
#
#   Wait for TCP connections in TIME_WAIT state to clear.
#
#   This function polls the system to check if there are lingering TCP connections
#   in TIME_WAIT state (typically from previous L2 runs) and waits until
#   they are cleared or the timeout is reached.
#
# Arguments
#
#   $1 - Maximum time to wait in seconds (default: 70).
#   $2 - Port number to check (default: uses NANVIXD_PORT).
#
# Return Value
#
#   - On success (connections cleared), this function returns zero.
#   - On timeout, this function returns non-zero.
#
wait_for_tcp_cleanup() {
    local max_wait_seconds="${1:-70}"
    local port="${2:-${NANVIXD_PORT}}"
    local poll_interval=2
    local start_time
    start_time=$(date +%s)

    echo "[TCP-CLEANUP] Starting TCP cleanup check (max_wait=${max_wait_seconds}s, port=${port})" 1>&2

    while true; do
        local current_time
        current_time=$(date +%s)
        local elapsed=$((current_time - start_time))

        if [ ${elapsed} -ge ${max_wait_seconds} ]; then
            echo "[TCP-CLEANUP] WARNING: Timeout reached after ${max_wait_seconds}s, some connections may still be in TIME_WAIT" 1>&2
            return 1
        fi

        # Count connections in TIME_WAIT state on the specified port.
        local time_wait_count=0
        if command -v ss &> /dev/null; then
            # Use ss if available (preferred).
            time_wait_count=$(ss -tan state time-wait sport "${port}" 2>/dev/null | tail -n +2 | wc -l)
        elif command -v netstat &> /dev/null; then
            # Fall back to netstat if ss is not available.
            time_wait_count=$(netstat -tan | grep ":${port}" | grep -c TIME_WAIT || echo 0)
        else
            echo "[TCP-CLEANUP] WARNING: Neither 'ss' nor 'netstat' available, skipping TCP cleanup check" 1>&2
            return 0
        fi
        if [ "${time_wait_count}" -eq 0 ]; then
            echo "[TCP-CLEANUP] All TCP connections cleared successfully" 1>&2
            return 0
        fi

        echo "[TCP-CLEANUP] Waiting for ${time_wait_count} TIME_WAIT connection(s) to clear... (${elapsed}s elapsed)" 1>&2
        sleep ${poll_interval}
    done
}

#
# Description
#
#   Creates a user VM and prints nanvixd response.
#
# Arguments
#
#   $1 - The tenant ID.
#   $2 - The application name.
#   $3 - The program name.
#   $4 - The program arguments.
#   $5 - The nanvixd HTTP address.
#
# Return Value
#
#   - On success, this function returns zero.
#   - On failure, this function returns non-zero.
#
create_user_vm() {
    local tenant_id="$1"
    local app_name="$2"
    local program_name="$3"
    local program_args="$4"
    local nanvix_http_addr="$5"

    local new_json
    new_json=$(jq -n \
        --arg tenant_id "${tenant_id}" \
        --arg app_name "${app_name}" \
        --arg program "${program_name}" \
        --arg program_args "${program_args}" \
        '{tenant_id: $tenant_id, app_name: $app_name, program: $program, program_args: $program_args}'
    )

    curl \
        --silent \
        --show-error \
        --fail-with-body \
        --header "Content-Type: application/json" \
        --header "X-NVX-Message-Type: NEW" \
        --request POST \
        --data "${new_json}" \
        "http://${nanvix_http_addr}"
}

#
# Description
#
#   Sends a KILL request to nanvixd and prints the exit code.
#
# Arguments
#   $1 - The user VM ID.
#   $2 - The nanvixd HTTP address.
#
# Return Value
#   - On success, this function returns zero.
#   - On failure, this function returns non-zero.
#
kill_user_vm() {
    local user_vm_id="$1"
    local nanvix_http_addr="$2"

    local kill_json
    kill_json=$(jq -n \
        --argjson user_vm_id "${user_vm_id}" \
        '{user_vm_id: $user_vm_id}'
    )

    curl \
        --silent \
        --show-error \
        --fail-with-body \
        --header "Content-Type: application/json" \
        --header "X-NVX-Message-Type: KILL" \
        --request POST \
        --data "${kill_json}" \
        "http://${nanvix_http_addr}" | jq -r '.exit_code'
}

#
# Description
#
#   Cleans up resources on script exit.
#
cleanup() {
    echo "[CLEANUP] Starting cleanup process..." 1>&2
    # Check if nanvixd is still running.
    if kill -0 -- "-${NANVIXD_PID}" 2>/dev/null; then
        echo "[CLEANUP] Killing nanvixd (pid=${NANVIXD_PID})..." 1>&2

        # First try graceful shutdown with SIGTERM to allow Drop handlers to run.
        echo "[CLEANUP] Sending SIGTERM for graceful shutdown..." 1>&2
        kill -s SIGTERM -- "-${NANVIXD_PID}" 2> /dev/null || true

        # Wait up to 5 seconds for graceful shutdown.
        echo "[CLEANUP] Waiting up to 5s for graceful shutdown..." 1>&2
        local count=0
        while kill -0 -- "-${NANVIXD_PID}" 2>/dev/null && [ $count -lt 50 ]; do
            sleep 0.1
            count=$((count + 1))
        done

        # If still running, force kill.
        if kill -0 -- "-${NANVIXD_PID}" 2>/dev/null; then
            echo "[CLEANUP] nanvixd did not exit gracefully, forcing kill..." 1>&2
            kill -s SIGKILL -- "-${NANVIXD_PID}" 2> /dev/null || true
        else
            echo "[CLEANUP] nanvixd exited gracefully" 1>&2
        fi

        wait "${NANVIXD_PID}" 2>/dev/null || true
    else
        echo "[CLEANUP] nanvixd process is not running" 1>&2
    fi

    # Clean up any stale Nanvix network namespaces.
    cleanup_stale_netns

    # If running in L2 mode, wait for TCP connections to clear for subsequent runs.
    if [ "${L2_VM}" = "yes" ]; then
        echo "[CLEANUP] Post-run: checking for lingering TCP connections..." 1>&2
        wait_for_tcp_cleanup 70 "${NANVIXD_PORT}"
    fi

    echo "[CLEANUP] Cleanup completed" 1>&2
}

#
# Description
#
#   Cleans up any stale Nanvix network namespaces left from previous runs.
#
cleanup_stale_netns() {
    echo "[NETNS-CLEANUP] Starting network namespace cleanup..." 1>&2
    # List all Nanvix network namespaces and delete them.
    local netns_list
    netns_list=$(sudo ip netns list 2>/dev/null | grep -o 'nvxns-[0-9]*' || true)

    if [ -n "${netns_list}" ]; then
        local count
        count=$(echo "${netns_list}" | wc -w)
        echo "[NETNS-CLEANUP] Found ${count} Nanvix namespace(s)" 1>&2
        for ns in ${netns_list}; do
            local ns_id="${ns#nvxns-}"

            # Delete veth pair first (host side).
            local veth_name="nvxgw-h-${ns_id}"
            if ! sudo ip link del "${veth_name}" 2>/dev/null; then
                echo "[NETNS-CLEANUP] WARNING: Failed to delete veth ${veth_name}" 1>&2
            fi

            # Delete the namespace.
            if ! sudo ip netns del "${ns}" 2>/dev/null; then
                echo "[NETNS-CLEANUP] WARNING: Failed to delete namespace ${ns}" 1>&2
            fi
        done
        echo "[NETNS-CLEANUP] Cleanup completed successfully" 1>&2
    else
        echo "[NETNS-CLEANUP] No stale namespaces found" 1>&2
    fi
}

#
# Description
#
#   Cleans up any stale socket files left from previous runs.
#
cleanup_stale_sockets() {
    rm -f /tmp/*.socket 2>/dev/null || true
}

#===================================================================================================
# Main Script
#===================================================================================================

main() {
    if [ $# -eq 0 ]; then
        usage 1
        return 0
    fi

    parse_arguments "$@"

    set_nanvixd_endpoint "${NANVIXD_SOCKADDR}"

    # Check if system meets requirements.
    check_system || return 1

    # Check if required tools are installed.
    check_tools || return 1

    # Clean up any stale socket files from previous runs.
    cleanup_stale_sockets

    # Before running in L2 mode, wait for TCP connections from previous runs to clear.
    # This is critical when L2 runs happen after non-L2 runs in sequence.
    if [ "${L2_VM}" = "yes" ]; then
        echo "[MAIN] This is an L2 run, checking for lingering TCP connections..." 1>&2
        wait_for_tcp_cleanup 70 "${NANVIXD_PORT}"
    fi

    # Clean up any stale network namespaces from previous runs.
    # This prevents resource conflicts from previous runs, especially when running
    # non-L2 runs after L2 runs in a sequence.
    echo "[MAIN] Cleaning up stale network namespaces..." 1>&2
    cleanup_stale_netns

    # Collect command line arguments.
    local program_name
    local program_args=""
    program_name="${PROGRAM_NAME}"

    if [ ${#PROGRAM_ARGS[@]} -gt 0 ]; then
        printf -v program_args '%s ' "${PROGRAM_ARGS[@]}"
        program_args=${program_args% }
    fi

    # Check if nanvixd binary exists.
    local nanvixd_binary_path
    nanvixd_binary_path="${BIN_DIR}/${NANVIXD_BINARY_NAME}"
    if [ ! -f "${nanvixd_binary_path}" ]; then
        # Search for nanvixd binary in current directory. Skip 'sysroot-*' directories, which may contain
        # stale versions of the binary when running this script from the source tree.
        echo "Warning: nanvixd binary not found at ${nanvixd_binary_path}. Searching in current directory..." 1>&2
        nanvixd_binary_path=$(find "." -type f -name "${NANVIXD_BINARY_NAME}" -not -path "./sysroot-*" -print -quit)
        if [ -z "${nanvixd_binary_path}" ]; then
            echo "Error: Unable to find nanvixd binary in current directory." 1>&2
            return 1
        fi
    fi

    local logs_dir
    logs_dir="logs/nanvixd-$(basename "${program_name}")"

    # Create logs directory.
    mkdir -p "${logs_dir}" || {
        echo "Error: Unable to create logs directory at ${logs_dir}." 1>&2
        return 1
    }

    # Run nanvixd in a new session.
    local console_file_name
    console_file_name="${logs_dir}/kernel_$(date "+%Y_%m_%d_%H_%M").log"
    echo "[MAIN] Starting nanvixd (log_level=${LOG_LEVEL}, l2_mode=${L2_VM:-no})" 1>&2
    echo "[MAIN] nanvixd binary: ${nanvixd_binary_path}" 1>&2
    echo "[MAIN] HTTP address: ${NANVIXD_SOCKADDR}" 1>&2
    echo "[MAIN] Logs directory: ${logs_dir}" 1>&2
    echo "[MAIN] Console file: ${console_file_name}" 1>&2
    RUST_LOG="${LOG_LEVEL},hyperlight_host=off" setsid "${nanvixd_binary_path}" \
        -http-addr "${NANVIXD_SOCKADDR}" \
        -toolchain-bin-dir "${TOOLCHAIN_BIN_DIR}" \
        -log-dir "${logs_dir}" \
        -netns-pool-size "0" \
        "$([ "$L2_VM" = "yes" ] && echo "-l2")" \
        -console-file "${console_file_name}" &
    NANVIXD_PID=$!
    echo "[MAIN] nanvixd started with PID: ${NANVIXD_PID}" 1>&2
    trap 'cleanup' EXIT

    # Wait for nanvixd to start by checking if the HTTP socket is listening.
    echo "[MAIN] Waiting for nanvixd HTTP socket to be ready..." 1>&2
    wait_for_tcp_socket "${NANVIXD_HOST}" "${NANVIXD_PORT}" || {
        echo "[MAIN] ERROR: nanvixd failed to start" 1>&2
        return 1
    }
    echo "[MAIN] nanvixd is ready" 1>&2

    # Create a VM.
    echo "[MAIN] Creating user VM (program=${program_name}, args='${program_args}')" 1>&2
    local new_response
    new_response=$(create_user_vm \
        "${TENANT_ID}" \
        "${APP_NAME}" \
        "${program_name}" \
        "${program_args}" \
        "${NANVIXD_SOCKADDR}") || {
        echo "[MAIN] ERROR: Failed to create VM" 1>&2
        return 1
    }
    echo "[MAIN] VM creation response received" 1>&2

    # Extract VM id from response.
    local vm_id
    vm_id=$(echo "${new_response}" | jq -r '.user_vm_id')
    if [ -z "${vm_id}" ] || [ "${vm_id}" = "null" ]; then
        echo "[MAIN] ERROR: nanvixd did not return a user_vm_id in its response: ${new_response}" 1>&2
        return 1
    fi
    echo "[MAIN] VM ID: ${vm_id}" 1>&2

    # Extract gateway socket address from response.
    local gateway_sockaddr
    gateway_sockaddr=$(echo "${new_response}" | jq -r '.gateway_sockaddr')
    if [ -z "${gateway_sockaddr}" ] || [ "${gateway_sockaddr}" = "null" ]; then
        echo "[MAIN] ERROR: nanvixd did not return a gateway_sockaddr in its response: ${new_response}" 1>&2
        return 1
    fi
    echo "[MAIN] Gateway socket address: ${gateway_sockaddr}" 1>&2

    # Connect to VM.
    if [[ "${L2_VM}" == "yes" ]]; then
        gateway_host=${gateway_sockaddr%:*}
        gateway_port=${gateway_sockaddr#*:}
        echo "[MAIN] Connecting to VM via TCP at ${gateway_host}:${gateway_port} (L2 mode)" 1>&2

        nc -v -q 0 "${gateway_host}" "${gateway_port}" || {
            echo "[MAIN] ERROR: Unable to connect VM at ${gateway_sockaddr} (L2)" 1>&2
            return 1
        }
    else
        echo "[MAIN] Connecting to VM via Unix socket at ${gateway_sockaddr}" 1>&2
        nc -v -U -q 0 "${gateway_sockaddr}" || {
            echo "[MAIN] ERROR: Unable to connect VM at ${gateway_sockaddr}" 1>&2
            return 1
        }
    fi
    echo "[MAIN] VM connection completed" 1>&2

    # Kill the user VM.
    echo "[MAIN] Requesting VM termination (vm_id=${vm_id})" 1>&2
    local kill_exit_code
    kill_exit_code=$(kill_user_vm "${vm_id}" "${NANVIXD_SOCKADDR}") || {
        echo "[MAIN] ERROR: Failed to stop VM" 1>&2
        return 1
    }
    echo "[MAIN] VM termination response received (exit_code=${kill_exit_code})" 1>&2

    if [ -n "${kill_exit_code}" ] && [ "${kill_exit_code}" != "0" ]; then
        echo "[MAIN] ERROR: VM exited with status code ${kill_exit_code}" 1>&2
        return 1
    fi

    echo "[MAIN] Script completed successfully" 1>&2
    return 0
}

# Call main function with all arguments
main "$@"
