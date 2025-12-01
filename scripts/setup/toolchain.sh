#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
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

readonly CONTRIB_DIR=${PREFIX}/src

# Binutils
readonly BINUTILS_REPOSITORY="https://github.com/nanvix/binutils"
readonly BINUTILS_COMMIT="cce4ffcd98cfd5e715f2b323a6a585907f102a8a"
readonly BINUTILS_HOME="${CONTRIB_DIR}/binutils"

# GCC
readonly GCC_REPOSITORY="https://github.com/nanvix/gcc"
readonly GCC_COMMIT="9af215ccf6fae9ba273ec45283e0ed3bcabe2429"
readonly GCC_HOME="${CONTRIB_DIR}/gcc"

# NewLib
readonly NEWLIB_REPOSITORY="https://github.com/nanvix/newlib"
readonly NEWLIB_COMMIT="e12d84a6789c07f938db4f6440ea0b427914c735"
readonly NEWLIB_HOME="${CONTRIB_DIR}/newlib"

# LLVM
readonly LLVM_REPOSITORY="https://github.com/nanvix/llvm-project"
readonly LLVM_COMMIT="cc6cdbfb0294fcddf19e0cdcf1550898783c82ba"
readonly LLVM_HOME="${CONTRIB_DIR}/llvm-project"

# Rust
readonly RUST_REPOSITORY="https://github.com/nanvix/rust"
readonly RUST_COMMIT="8a5539dd2738dfc5fd42264b04bb09d1f742e4e0"
readonly RUST_HOME="${CONTRIB_DIR}/rust"

#===================================================================================================
# Utility Functions
#===================================================================================================

#
# Description
#
#   Builds a tool from its source directory.
#
#
# Parameters
#
#   $1 - Tool source path.
#   $2 - Tool install path.
#   $3 - Build stage (e.g., stage0, stage1).
#   $4 - Sysroot path (optional).
#
# Return Value
#
#   - On success, this function returns zero.
#   - On error, this function returns non-zero.
#
# Usage Example
#
#   build_tool "/path/to/tool/source" "/path/to/install" "stage0" "/path/to/sysroot"
#
build_tool() {
    local tool_path=$1
    local install_path=$2
    local stage=$3
    local sysroot_path=$4

    # Check if tool path is empty.
    if [[ -z "${tool_path}" ]]; then
        print_error "Tool path is empty."
        return 1
    fi

    # Check if tool path exists.
    if [[ ! -d "${tool_path}" ]]; then
        print_error "Tool path '${tool_path}' does not exist."
        return 1
    fi

    pushd "${tool_path}" || {
        print_error "Failed to change directory to '${tool_path}'."
        return 1
    }

    # Check whether to pass sysroot path or not.
    if [[ -z "${sysroot_path}" ]]; then
        ./z configure --install-location="${install_path}" --stage="${stage}" || {
            print_error "Failed to configure tool in '${tool_path}'."
            popd || {
                print_error "Failed to change directory back from '${tool_path}'."
            }
            return 1
        }
    else
        ./z configure --install-location="${install_path}" --sysroot-location="${sysroot_path}" --stage="${stage}" || {
            print_error "Failed to configure tool in '${tool_path}'."
            popd || {
                print_error "Failed to change directory back from '${tool_path}'."
            }
            return 1
        }
    fi

    ./z build || {
        print_error "Failed to build tool in '${tool_path}'."
        popd || {
            print_error "Failed to change directory back from '${tool_path}'."
        }
        return 1
    }

    ./z install || {
        print_error "Failed to install tool in '${tool_path}'."
        popd || {
            print_error "Failed to change directory back from '${tool_path}'."
        }
        return 1
    }

    popd || {
        print_error "Failed to change directory back from '${tool_path}'."
        return 1
    }

    return 0
}

#===================================================================================================
# Main Script
#===================================================================================================

# Clone Binutils repository.
clone_repo "${BINUTILS_REPOSITORY}" "${CONTRIB_DIR}" "${BINUTILS_COMMIT}" || {
    print_error "Failed to clone Binutils repository."
    exit 1
}

# Clone GCC repository.
clone_repo "${GCC_REPOSITORY}" "${CONTRIB_DIR}" "${GCC_COMMIT}" || {
    print_error "Failed to clone GCC repository."
    exit 1
}

# Clone NewLib repository.
clone_repo "${NEWLIB_REPOSITORY}" "${CONTRIB_DIR}" "${NEWLIB_COMMIT}" || {
    print_error "Failed to clone Newlib repository."
    exit 1
}

# Clone LLVM repository.
clone_repo "${LLVM_REPOSITORY}" "${CONTRIB_DIR}" "${LLVM_COMMIT}" || {
    print_error "Failed to clone Newlib repository."
    exit 1
}

# Clone Rust repository.
clone_repo "${RUST_REPOSITORY}" "${CONTRIB_DIR}" "${RUST_COMMIT}" || {
    print_error "Failed to clone Rust repository."
    exit 1
}

# Build Binutils and cleanup.
build_tool "${BINUTILS_HOME}" "${PREFIX}" "0" "${PREFIX}" || {
    print_error "Failed to build Binutils."
    exit 1
}
rm -rf "${BINUTILS_HOME}" || {
    print_warning "Failed to remove Binutils source directory '${BINUTILS_HOME}'."
}

# Build GCC stage0. We keep artifacts for stage1.
build_tool "${GCC_HOME}" "${PREFIX}" "0" "${PREFIX}" || {
    print_error "Failed to build GCC stage0."
    exit 1
}

# Build NewLib and cleanup.
build_tool "${NEWLIB_HOME}" "${PREFIX}" "0" "${PREFIX}" || {
    print_error "Failed to build Newlib."
    exit 1
}
rm -rf "${NEWLIB_HOME}" || {
    print_warning "Failed to remove Newlib source directory '${NEWLIB_HOME}'."
}

# Build GCC stage1 and cleanup.
build_tool "${GCC_HOME}" "${PREFIX}" "1" "${PREFIX}" || {
    print_error "Failed to build GCC stage1."
    exit 1
}
rm -rf "${GCC_HOME}" || {
    print_warning "Failed to remove GCC source directory '${GCC_HOME}'."
}

# Build LLVM and cleanup.
build_tool "${LLVM_HOME}" "${PREFIX}" "0" "${PREFIX}" || {
    print_error "Failed to build LLVM."
    exit 1
}
rm -rf "${LLVM_HOME}" || {
    print_warning "Failed to remove LLVM source directory '${LLVM_HOME}'."
}

# Clone and build Rust. We don't cleanup as artifacts cannot be removed.
build_tool "${RUST_HOME}" "${PREFIX}" "0" "${PREFIX}" || {
    print_error "Failed to build Rust."
    exit 1
}

# Clone, build and cleanup Python.
"${SCRIPTS_DIR}/python.sh" "${PREFIX}" || {
    print_error "Failed to build Python."
    exit 1
}
rm -rf "${CONTRIB_DIR}/cpython" || {
    print_warning "Failed to remove Python source directory '${CONTRIB_DIR}/cpython'."
}

# Clone, build and cleanup Cloud Hypervisor.
"${SCRIPTS_DIR}/cloud-hypervisor.sh" "${PREFIX}" || {
    print_error "Failed to build Cloud Hypervisor."
    exit 1
}
rm -rf "${CONTRIB_DIR}/cloud-hypervisor*" || {
    print_warning "Failed to remove Cloud Hypervisor source directory '${CONTRIB_DIR}/cloud-hypervisor'."
}

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
