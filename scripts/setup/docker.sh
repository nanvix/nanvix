#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#
# Utility for building a Docker image with the Nanvix toolchain.
#
# Run './scripts/setup/docker.sh' to build the Docker image.
#

#==================================================================================================

# Fast fail on errors, unset variables, and pipe failures.
set -euo pipefail

#==================================================================================================
# Imports
#==================================================================================================

# Directory where to find scripts to import.
IMPORT_DIR="$(cd "$(dirname "$0")" && pwd)/../common"

source "${IMPORT_DIR}/logging.sh"

#==================================================================================================
# Global Constants
#==================================================================================================

# Directories
REPO_ROOT_DIR=$(git rev-parse --show-toplevel)   # /
REPO_SCRIPTS_DIR="${REPO_ROOT_DIR}/scripts"      # /scripts
REPO_LOGS_DIR="${REPO_ROOT_DIR}/logs"            # /logs

# Default branch name.
DEFAULT_BRANCH_NAME=$(git -C "${REPO_ROOT_DIR}" remote show origin | awk '/HEAD branch/ {print $NF}')

# Latest commit hash made to the default branch.
NANVIX_VERSION=$(git -C "${REPO_ROOT_DIR}" rev-parse origin/${DEFAULT_BRANCH_NAME})

# Dockerfile path.
DOCKERFILE_PATH="${REPO_SCRIPTS_DIR}/setup/Dockerfile"

# Default name for the Docker image.
DOCKER_IMAGE_NAME="nanvix/toolchain"

# Default tag for the Docker image.
DOCKER_IMAGE_TAG="latest"

#==================================================================================================
# Functions
#==================================================================================================

#
# DESCRIPTION
#   Gets the rust version used in the project.
#
# RETURNS
#   - On success, a string containing the Rust version (e.g., "nightly-2025-03-28").
#   - On failure, an empty string.
#
# USAGE EXAMPLE
#   RUST_VERSION=$(get_rust_version)
#
get_rust_version() {
    local rust_toolchain_file
    rust_toolchain_file=$(mktemp)

    # Get rust-toolchain file from the default branch.
    git -C "${REPO_ROOT_DIR}" show origin/${DEFAULT_BRANCH_NAME}:rust-toolchain > "${rust_toolchain_file}" 2>/dev/null || {
        rm -f "${rust_toolchain_file}"
        print_warning "Could not find 'rust-toolchain' file in the repository."
        echo ""
        return
    }

    # Extract the line containing the Rust channel.
    local channel_line
    channel_line=$(grep -E '^[[:space:]]*channel[[:space:]]*=' "${rust_toolchain_file}" | head -n 1)


    # If the channel line is not empty, assume 'rust-toolchain' is in TOML format.
    # Otherwise, assume it is in plain text format (single line with version).
    local version=""
    if [ -n "${channel_line}" ]; then
        # Try to extract the channel from TOML format.
        # Get the value after '=' and remove spaces and quotes.
        version=$(echo "${channel_line}" | cut -d'=' -f2 | tr -d '[:space:]' | tr -d '"')
    else
        # Fallback: assume plain text format (single line with version).
        version=$(head -n 1 "${rust_toolchain_file}" | tr -d '[:space:]')
    fi

    rm -f "${rust_toolchain_file}"

    # Check if the version string is empty.
    if [ -z "${version}" ]; then
        print_error "Failed to extract Rust version from 'rust-toolchain' file."
        echo ""
        return
    fi

    echo "${version}"
}

#==================================================================================================
# Main Script
#==================================================================================================

#
# DESCRIPTION
#   Builds the Docker image for the Nanvix toolchain.
#
main() {
    print_message "Building Docker image for Nanvix toolchain..."

    # Sanity check that we are running inside a git repository.
    if ! git rev-parse --is-inside-work-tree &> /dev/null; then
        print_error "This script must be run inside a Git repository."
        exit 1
    fi

    # Sanity check that branch name was set correctly.
    if [ -z "${DEFAULT_BRANCH_NAME}" ]; then
        print_error "Could not determine the default branch name."
        exit 1
    fi

    # Sanity check that repository directory was set correctly.
    if [ ! -d "${REPO_ROOT_DIR}" ]; then
        print_error "Could not determine the repository root directory."
        exit 1
    fi

    # Sanity check that docker is installed.
    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed."
        exit 1
    fi

    # Check if current user is not in the 'docker' group.
    if ! id -nG | grep -qw docker; then
        print_error "Current user is not in the 'docker' group."
        exit 1
    fi

    # Sanity check that the Dockerfile exists.
    if [ ! -f "${DOCKERFILE_PATH}" ]; then
        print_error "Dockerfile not found at ${DOCKERFILE_PATH}"
        exit 1
    fi
    print_message "Using Dockerfile at '${DOCKERFILE_PATH}'"


    local rust_version
    rust_version=$(get_rust_version)
    print_message "Using Rust version: ${rust_version}"

    # Sanity check if rust version was correctly set.
    if [ -z "${rust_version}" ]; then
        print_error "Could not determine Rust version from rust-toolchain file."
        exit 1
    fi

    # Create logs directory if it doesn't exist.
    mkdir -p "${REPO_LOGS_DIR}"

    docker build --no-cache \
        --progress=plain \
        --build-arg RUST_VERSION="${rust_version}" \
        --build-arg NANVIX_VERSION="${NANVIX_VERSION}" \
        -t "${DOCKER_IMAGE_NAME}:${DOCKER_IMAGE_TAG}" "${REPO_SCRIPTS_DIR}/setup/" \
    2>&1 | tee "${REPO_LOGS_DIR}/docker-build.log"

    print_message "Logs saved to ${REPO_LOGS_DIR}/docker-build.log"

    print_success "Docker image built successfully."
}

main "$@"
