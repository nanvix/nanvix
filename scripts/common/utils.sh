#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#
# Utility functions.
#

#===================================================================================================
# Include Guard
#===================================================================================================

# Skip this file if already included.
if [[ -n "${__UTILS_SH_INCLUDED:-}" ]]; then
    return
fi
readonly __UTILS_SH_INCLUDED=1

#==================================================================================================
# Imports
#==================================================================================================

source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/logging.sh"

#==================================================================================================
# Functions
#==================================================================================================

#
# Description
#
#   Resolves a path to an absolute path without assuming GNU realpath -m is available.
#   If the path exists, it is resolved via cd + pwd -P. Otherwise, an absolute path
#   is constructed without normalization.
#
# Arguments
#
#   $1 - The path to resolve.
#
# Return Value
#
#   - On success, prints the resolved absolute path.
#
# Usage Example
#
#   resolved=$(resolve_path "$TARGET_DIR")
#
resolve_path() {
    local path="$1"
    local resolved=""

    if cd "${path}" >/dev/null 2>&1; then
        resolved="$(pwd -P)"
        cd - >/dev/null 2>&1 || true
    else
        # For non-existent paths, construct an absolute path without normalization.
        if [[ "${path}" == /* ]]; then
            resolved="${path}"
        else
            resolved="$(pwd -P)/${path}"
        fi
    fi

    printf '%s\n' "${resolved}"
}

#
# Description
#
#   Gets the current version from a Cargo.toml file.
#
# Arguments
#
#   $1 - The path to the Cargo.toml file.
#
# Return Value
#
#   - On success, a string containing the current version in the format MAJOR.MINOR.PATCH.
#   - On failure, exits with a non-zero status.
#
# Usage Example
#
#   cargo_toml_version=$(get_cargo_toml_version "path/to/Cargo.toml")
#
get_cargo_toml_version() {
    local cargo_toml="$1"

    # Check if the target file does not exist.
    if [[ ! -f "$cargo_toml" ]]; then
        print_error "$cargo_toml does not exist."
        exit 1
    fi

    # Check if target file is not a toml file.
    if [[ "${cargo_toml##*.}" != "toml" ]]; then
        print_error "$cargo_toml is not a toml file."
        exit 1
    fi

    local cargo_toml_version
    cargo_toml_version=$(sed -n 's/^[[:space:]]*version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$cargo_toml" | head -n1)

    # Check if version was not extracted successfully.
    if [[ -z "$cargo_toml_version" ]]; then
        print_error "Could not extract version from ${cargo_toml}."
        exit 1
    fi

    echo "$cargo_toml_version"
}

#
# Description
#
#   Get the repository root directory.
#
# Return Value
#
#   The absolute path to the repository root directory.
#
# Usage Example
#
#   repo_root=$(get_repo_root)
#
get_repo_root() {
    git rev-parse --show-toplevel
}

#
# Description
#
#   Reads a value from a simple, single-level TOML file with key = value pairs.
#
# Arguments
#
#   $1 - The path to the TOML file.
#   $2 - The key to get the value for.
#
# Return Value
#
#   - On success, a string containing the value for the given key.
#   - On failure, exits with a non-zero status.
#
# Usage Example
#
#   kstack_size=$(get_value_from_toml "./build/kernel_config.toml" "kstack_size")
#
get_value_from_toml() {
    local toml_path=$1
    local toml_key=$2
    local val
    val="$(
    sed -nE "s/^[[:space:]]*${toml_key}[[:space:]]*=[[:space:]]*(\"([^\"]*)\"|\'([^\']*)\'|([^[:space:]]+)).*/\2\3\4/p" "$toml_path" \
    | head -n1
    )"
    [[ -n "$val" ]] && printf '%s' "$val" || exit 1
}

#
# Description
#
#   Clones a repository.
#
# Parameters
#
#   $1 - Repository URL.
#   $2 - Repository base path.
#   $3 - Commit to checkout.
#
# Return Value
#
#   - On success, this function returns zero.
#   - On error, this function returns non-zero.
#
# Usage Example
#
#   clone_repo "https://github.com/nanvix/gcc" "/path/to/dir" "commit_id"
#
clone_repo() {
    local repository_url=$1
    local repository_basepath=$2
    local commit=$3

    # Check if repository URL is empty.
    if [[ -z "${repository_url}" ]]; then
        print_error "Repository URL is empty."
        return 1
    fi

    # Check if repository base path is empty.
    if [[ -z "${repository_basepath}" ]]; then
        print_error "Repository path is empty."
        return 1
    fi

    # Create repository base path if it does not exist.
    mkdir -p "${repository_basepath}" || {
        print_error "Failed to create directory '${repository_basepath}'."
        return 1
    }

    # Infer repository name from repository url.
    local repository_name
    repository_name=$(basename -s .git "${repository_url}")

    local repository_path="${repository_basepath}/${repository_name}"

    # Clone repository if it does not exist, else fetch latest changes.
    if [[ ! -d "${repository_path}/.git" ]]; then
        git clone "${repository_url}" "${repository_path}" || {
            print_error "Failed to clone repository '${repository_url}' to '${repository_path}'."
            return 1
        }
    else
        git -C "${repository_path}" fetch origin || {
            print_error "Failed to fetch latest changes for repository '${repository_url}'."
            return 1
        }
        git -C "${repository_path}" reset --hard || {
            print_error "Failed to reset repository '${repository_url}'."
            return 1
        }
    fi

    # Checkout to the specified commit.
    git -C "${repository_path}" checkout "${commit}" || {
        print_error "Failed to checkout to commit '${commit}' in repository '${repository_url}'."
        return 1
    }

    return 0
}

#===================================================================================================
# Socket Address and Port Utilities
#===================================================================================================

#
# Description
#
#   Validates that a string is a valid socket address in HOST:PORT format.
#
# Arguments
#
#   $1 - The socket address to validate.
#
# Return Value
#
#   - Returns zero if the socket address is valid.
#   - Returns non-zero if the socket address is invalid or empty.
#
# Usage Example
#
#   if validate_sockaddr "127.0.0.1:8181"; then
#       echo "Valid socket address"
#   fi
#
validate_sockaddr() {
    local sockaddr=$1

    # Check if socket address is empty.
    if [ -z "${sockaddr}" ]; then
        print_error "validate_sockaddr: socket address is empty."
        return 1
    fi

    # Check if socket address contains a colon separator.
    if [[ "${sockaddr}" != *:* ]]; then
        print_error "validate_sockaddr: missing colon separator in '${sockaddr}'."
        return 1
    fi

    # Ensure there is exactly one colon separator (HOST:PORT format).
    local colon_only
    colon_only=${sockaddr//[^:]/}
    if [[ "${#colon_only}" -ne 1 ]]; then
        print_error "validate_sockaddr: multiple colons in '${sockaddr}'."
        return 1
    fi

    local host
    local port
    host=${sockaddr%%:*}
    port=${sockaddr##*:}

    # Check if host is empty.
    if [ -z "${host}" ]; then
        print_error "validate_sockaddr: host is empty in '${sockaddr}'."
        return 1
    fi

    # Validate host format: must contain only alphanumeric characters, dots, hyphens, or be an IP address.
    # This catches obviously invalid hostnames like "invalid!!host" while allowing:
    # - IPv4 addresses (e.g., 127.0.0.1)
    # - Hostnames (e.g., localhost, my-server, server.example.com)
    if ! [[ "${host}" =~ ^[a-zA-Z0-9]([a-zA-Z0-9.-]*[a-zA-Z0-9])?$ ]]; then
        print_error "validate_sockaddr: invalid host format in '${sockaddr}'."
        return 1
    fi

    # Check if port is empty or not numeric.
    if [ -z "${port}" ] || ! [[ ${port} =~ ^[0-9]+$ ]]; then
        print_error "validate_sockaddr: port is empty or not numeric in '${sockaddr}'."
        return 1
    fi

    # Check if port is in valid range (1-65535).
    if [ "${port}" -lt 1 ] || [ "${port}" -gt 65535 ]; then
        print_error "validate_sockaddr: port out of range in '${sockaddr}'."
        return 1
    fi

    return 0
}

#
# Description
#
#   Parses a socket address and outputs its host and port components.
#   This function assumes the socket address has already been validated.
#
# Arguments
#
#   $1 - The socket address to parse.
#   $2 - Variable name to store the host (optional, uses PARSED_HOST if not provided).
#   $3 - Variable name to store the port (optional, uses PARSED_PORT if not provided).
#
# Return Value
#
#   - Returns zero on success and sets the specified variables.
#   - Returns non-zero if the socket address is invalid.
#
# Usage Example
#
#   parse_sockaddr "127.0.0.1:8181" my_host my_port
#   echo "Host: ${my_host}, Port: ${my_port}"
#
parse_sockaddr() {
    local sockaddr=$1
    local host_var=${2:-PARSED_HOST}
    local port_var=${3:-PARSED_PORT}

    if ! validate_sockaddr "${sockaddr}"; then
        print_error "parse_sockaddr: invalid socket address '${sockaddr}'."
        return 1
    fi

    # Use printf -v to set the variables by name.
    printf -v "${host_var}" '%s' "${sockaddr%%:*}"
    printf -v "${port_var}" '%s' "${sockaddr##*:}"

    return 0
}

#
# Description
#
#   Checks if a TCP port is available (not in use).
#
# Arguments
#
#   $1 - The host address.
#   $2 - The port number.
#
# Return Value
#
#   - Returns zero if the port is available.
#   - Returns non-zero if the port is in use.
#
# Usage Example
#
#   if is_port_available "127.0.0.1" "8181"; then
#       echo "Port is available"
#   fi
#
is_port_available() {
    local host=$1
    local port=$2

    # Validate that port is numeric.
    if ! [[ "${port}" =~ ^[0-9]+$ ]]; then
        print_error "is_port_available: port '${port}' is not numeric."
        return 1
    fi

    # Check if the port is currently in LISTEN state.
    # Note: ss and netstat check all interfaces, not the specific host.
    if command -v ss &> /dev/null; then
        # The -l flag already filters for listening sockets, so we just check if any output exists.
        if ss -tln "sport = :${port}" 2>/dev/null | tail -n +2 | grep -q .; then
            print_error "is_port_available: port ${port} is in LISTEN state."
            return 1
        fi
    elif command -v netstat &> /dev/null; then
        if netstat -tln 2>/dev/null | grep -Eq ":${port}[[:space:]]+.*LISTEN"; then
            print_error "is_port_available: port ${port} is in LISTEN state."
            return 1
        fi
    fi

    # Check if the port is in TIME_WAIT state.
    if command -v ss &> /dev/null; then
        if ss -tan state time-wait "sport = :${port}" 2>/dev/null | tail -n +2 | grep -q .; then
            print_error "is_port_available: port ${port} is in TIME_WAIT state."
            return 1
        fi
    elif command -v netstat &> /dev/null; then
        if netstat -tan 2>/dev/null | grep -E ":${port}[[:space:]]+" | grep -q TIME_WAIT; then
            print_error "is_port_available: port ${port} is in TIME_WAIT state."
            return 1
        fi
    fi

    # As an additional check, try to connect to the specific host:port.
    # This provides a targeted check for the specific interface, complementing the
    # ss/netstat checks above which only verify that no process is listening on that
    # port on any interface. A successful connection here means the port is in use.
    # Note: The -z option behavior varies between netcat implementations (GNU vs OpenBSD).
    # We use a short timeout (-w 1) to avoid hanging on unresponsive hosts.
    if command -v nc &> /dev/null; then
        if nc -z -w 1 "${host}" "${port}" 2>/dev/null; then
            print_error "is_port_available: connection to ${host}:${port} succeeded (port in use)."
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
#   $1 - The host address.
#   $2 - The port number.
#   $3 - Maximum wait time in seconds (default: 120).
#   $4 - Poll interval in seconds (default: 1).
#
# Return Value
#
#   - Returns zero if the port becomes available.
#   - Returns non-zero if timeout is reached.
#
# Note
#
#   There is a TOCTOU (time-of-check-time-of-use) race condition between this
#   function returning and the caller binding to the port. Another process could
#   bind to the port in between. This is acceptable as the probability is low.
#
# Usage Example
#
#   wait_for_port_available "127.0.0.1" "8181" 60 1
#
wait_for_port_available() {
    local host=$1
    local port=$2
    local max_wait=${3:-120}
    local poll_interval=${4:-1}
    local start_time
    start_time=$(date +%s)

    print_info "[PORT-CHECK] Checking if port ${port} is available (max_wait=${max_wait}s)..."

    while true; do
        local current_time
        current_time=$(date +%s)
        local elapsed=$((current_time - start_time))

        if [ "${elapsed}" -ge "${max_wait}" ]; then
            print_error "[PORT-CHECK] Timeout: port ${port} not available after ${max_wait}s"
            return 1
        fi

        if is_port_available "${host}" "${port}"; then
            if [ "${elapsed}" -gt 0 ]; then
                print_info "[PORT-CHECK] Port ${port} is available (waited ${elapsed}s)"
            else
                print_info "[PORT-CHECK] Port ${port} is available"
            fi
            return 0
        fi

        print_debug "[PORT-CHECK] Port ${port} is in use, waiting... (${elapsed}s elapsed)"
        sleep "${poll_interval}"
    done
}

#
# Description
#
#   Finds an available port within a range.
#
# Arguments
#
#   $1 - The host address.
#   $2 - Starting port number.
#   $3 - Ending port number (inclusive).
#
# Return Value
#
#   - On success, prints an available port number and returns zero.
#   - On failure, returns non-zero.
#
# Usage Example
#
#   available_port=$(find_available_port "127.0.0.1" 8181 8281)
#
find_available_port() {
    local host=$1
    local start_port_str=$2
    local end_port_str=$3
    local start_port
    local port
    local end_port
    local min_port=1
    local max_port=65535

    # Validate that host is not empty.
    if [[ -z "${host}" ]]; then
        print_error "find_available_port: host cannot be empty."
        return 1
    fi

    # Validate that start_port and end_port are numeric.
    if [[ -z "${start_port_str}" || -z "${end_port_str}" ]]; then
        print_error "find_available_port: start_port and end_port must be provided."
        return 1
    fi

    if [[ ! "${start_port_str}" =~ ^[0-9]+$ || ! "${end_port_str}" =~ ^[0-9]+$ ]]; then
        print_error "find_available_port: start_port and end_port must be numeric."
        return 1
    fi

    # Force base-10 parsing to avoid octal interpretation for leading zeros.
    start_port=$((10#${start_port_str}))
    end_port=$((10#${end_port_str}))

    # Clamp ports to valid range [1, 65535].
    if (( start_port < min_port )); then
        start_port=${min_port}
    fi

    if (( end_port > max_port )); then
        end_port=${max_port}
    fi

    # If the range is invalid after clamping, fail.
    if (( start_port > end_port )); then
        print_error "find_available_port: invalid port range (${start_port} > ${end_port})."
        return 1
    fi

    for ((port=start_port; port<=end_port; port++)); do
        if is_port_available "${host}" "${port}"; then
            echo "${port}"
            return 0
        fi
    done

    print_error "find_available_port: no available port found in range ${start_port}-${end_port}."
    return 1
}
