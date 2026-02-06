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
# Timeout for waiting for port availability (in seconds).
readonly PORT_AVAILABILITY_TIMEOUT=120
# Poll interval for port availability checks (in seconds).
readonly PORT_POLL_INTERVAL=2
# Timeout for cleanup operations (in seconds).
readonly CLEANUP_GRACEFUL_TIMEOUT=10
# Timeout for TCP connections cleanup (in seconds).
readonly TCP_CLEANUP_TIMEOUT=120
readonly OPTION_APP_NAME="--app-name"
readonly OPTION_BIN_DIR="--bin-dir"
readonly OPTION_HELP="--help"
readonly OPTION_LOG_LEVEL="--log-level"
readonly OPTION_NANVIXD_SOCKADDR="--nanvixd-sockaddr"
readonly OPTION_TENANT_ID="--tenant-id"
readonly OPTION_TOOLCHAIN_BIN_DIR="--toolchain-bin-dir"

#===================================================================================================
# Imports
#===================================================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/common/utils.sh"

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

    if ! parse_sockaddr "${sockaddr}" NANVIXD_HOST NANVIXD_PORT; then
        die "Invalid nanvixd socket address '${sockaddr}'. Expected HOST:PORT."
    fi
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

    # Kill nanvixd process group gracefully using cleanup script.
    if [ -n "${NANVIXD_PID}" ]; then
        "${SCRIPT_DIR}/cleanup-nanvixd.sh" \
            --kill-process-group \
            --process-group-pid "${NANVIXD_PID}" \
            --graceful-timeout "${CLEANUP_GRACEFUL_TIMEOUT}" \
            --verbose 1>&2 || true
    fi

    # Clean up network namespaces and sockets.
    "${SCRIPT_DIR}/cleanup-nanvixd.sh" --netns --sockets --verbose 1>&2 || true

    # Wait for TCP connections to clear for subsequent runs.
    echo "[CLEANUP] Post-run: checking for lingering TCP connections..." 1>&2
    "${SCRIPT_DIR}/cleanup-nanvixd.sh" \
        --wait-tcp-cleanup "${NANVIXD_PORT}" \
        --tcp-cleanup-timeout "${TCP_CLEANUP_TIMEOUT}" \
        --verbose 1>&2 || true

    echo "[CLEANUP] Cleanup completed" 1>&2
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

    # Clean up stale resources from previous runs using cleanup script.
    echo "[MAIN] Pre-run cleanup: sockets" 1>&2
    "${SCRIPT_DIR}/cleanup-nanvixd.sh" --sockets --verbose 2>&1 || true

    # Before running in L2 mode, wait for TCP connections from previous runs to clear.
    # This is critical when L2 runs happen after non-L2 runs in sequence.
    if [ "${L2_VM}" = "yes" ]; then
        echo "[MAIN] This is an L2 run, checking for lingering TCP connections..." 1>&2
        "${SCRIPT_DIR}/cleanup-nanvixd.sh" \
            --wait-tcp-cleanup "${NANVIXD_PORT}" \
            --tcp-cleanup-timeout 70 \
            --verbose 2>&1 || true
    fi

    # Clean up any stale network namespaces from previous runs.
    # This prevents resource conflicts from previous runs, especially when running
    # non-L2 runs after L2 runs in a sequence.
    echo "[MAIN] Cleaning up stale network namespaces..." 1>&2
    "${SCRIPT_DIR}/cleanup-nanvixd.sh" --netns --verbose 2>&1 || true

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

    # Check if port is available before starting nanvixd.
    # This prevents "Address already in use" errors from previous runs.
    echo "[MAIN] Checking if port ${NANVIXD_PORT} is available..." 1>&2
    if ! wait_for_port_available "${NANVIXD_HOST}" "${NANVIXD_PORT}" "${PORT_AVAILABILITY_TIMEOUT}" "${PORT_POLL_INTERVAL}" 1>&2; then
        echo "[MAIN] ERROR: Port ${NANVIXD_PORT} is not available after waiting ${PORT_AVAILABILITY_TIMEOUT}s" 1>&2
        echo "[MAIN] This may be caused by a previous test run that did not clean up properly." 1>&2
        echo "[MAIN] Try running: ss -tlnp sport = :${NANVIXD_PORT}" 1>&2
        return 1
    fi

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

    # Report non-zero exit codes but don't treat them as script failures.
    # The caller (test framework) is responsible for validating the exit code.
    if [ -n "${kill_exit_code}" ] && [ "${kill_exit_code}" != "0" ]; then
        echo "[MAIN] VM exited with non-zero status code ${kill_exit_code}" 1>&2
    fi

    echo "[MAIN] Script completed successfully" 1>&2
    return 0
}

# Call main function with all arguments
main "$@"
