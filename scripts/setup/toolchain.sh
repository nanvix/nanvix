#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Script Arguments
#===================================================================================================

PREFIX=${1:-$PWD/toolchain}

#===================================================================================================
# Environment Variables
#===================================================================================================

SCRIPTS_DIR=$(dirname "$(readlink -f "$0")")
REPO_ROOT_DIR=$(git rev-parse --show-toplevel 2>/dev/null) || {
    echo "ERROR: Not in a git repository" >&2
    exit 1
}
CARGO_TOML_FILE_PATH="${REPO_ROOT_DIR}/Cargo.toml"


#===================================================================================================
# Helper Functions
#===================================================================================================

#
# Description
#
#   Extracts the current version from the Cargo.toml file and formats it for toolchain versioning.
#
# Arguments
#
#   $1 - The path to the Cargo.toml file.
#
# Return Value
#
#   The toolchain version as a string in the format A.B.x.
#
# Usage Example
#
#   toolchain_version=$(extract_toolchain_version "path/to/Cargo.toml")
#
extract_toolchain_version() {
    local cargo_toml="$1"

    # Check if the Cargo.toml file does not exist.
    if [[ ! -f "$cargo_toml" ]]; then
        echo "ERROR: Cargo.toml not found at $cargo_toml" >&2
        exit 1
    fi

    local current_version
    current_version=$(sed -n 's/^[[:space:]]*version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$cargo_toml" | head -n1)

    # Check if version was not extracted successfully.
    if [[ -z "$current_version" ]]; then
        echo "ERROR: Could not extract version from Cargo.toml" >&2
        exit 1
    fi

    # Extract major and minor version numbers.
    local major minor
    major=$(echo "$current_version" | cut -d. -f1)
    minor=$(echo "$current_version" | cut -d. -f2)

    # Validate version format.
    if [[ ! "$major" =~ ^[0-9]+$ ]] || [[ ! "$minor" =~ ^[0-9]+$ ]]; then
        echo "ERROR: Invalid version format '$current_version'" >&2
        exit 1
    fi

    echo "${major}.${minor}.x"
}

#===================================================================================================
# Main Script
#===================================================================================================

"${SCRIPTS_DIR}/binutils.sh" "${PREFIX}"
"${SCRIPTS_DIR}/gcc.sh" stage0 "${PREFIX}"
"${SCRIPTS_DIR}/newlib.sh" "${PREFIX}"
"${SCRIPTS_DIR}/gcc.sh" stage1 "${PREFIX}"
"${SCRIPTS_DIR}/rust.sh" "${PREFIX}"
"${SCRIPTS_DIR}/python.sh" "${PREFIX}"
"${SCRIPTS_DIR}/cloud-hypervisor.sh" "${PREFIX}"

# Create version file as the final step.
toolchain_version=$(extract_toolchain_version "$CARGO_TOML_FILE_PATH")
echo "nanvix-toolchain-v${toolchain_version}" > "${PREFIX}/version" || {
    echo "ERROR: Failed to create version file at ${PREFIX}/version" >&2
    exit 1
}
echo "Created toolchain version file: ${PREFIX}/version with version: $toolchain_version"
