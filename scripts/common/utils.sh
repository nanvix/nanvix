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
