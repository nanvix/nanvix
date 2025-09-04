#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Script Arguments
#===================================================================================================

PREFIX=${1:-$PWD/toolchain}

#==================================================================================================
# Imports
#==================================================================================================

# Directory where to find scripts to import.
IMPORT_DIR="$(cd "$(dirname "$0")" && pwd)/../common"

source "${IMPORT_DIR}/logging.sh"
source "${IMPORT_DIR}/utils.sh"

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
current_version=$(get_cargo_toml_version "$CARGO_TOML_FILE_PATH")
major=$(echo "$current_version" | cut -d. -f1)
minor=$(echo "$current_version" | cut -d. -f2)
toolchain_version="${major}.${minor}.x"
echo "nanvix-toolchain-v${toolchain_version}" > "${PREFIX}/version" || {
    print_error "Failed to create version file at ${PREFIX}/version."
    exit 1
}
print_success "Created toolchain version file: ${PREFIX}/version with version: ${toolchain_version}"
