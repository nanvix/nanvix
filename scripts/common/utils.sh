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
