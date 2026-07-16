#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#
# Generates a JSON manifest file with build metadata and git info.
#
# Run './generate-manifest.sh --help' for usage information.
#
# Environment Variables:
#   MEMORY_SIZE_BYTES - Memory size in bytes (required, exported by the Makefile).
#
# Arguments:
#   $1 - output-file      Path to the manifest JSON file to generate.
#   $2 - version          Release version string.
#   $3 - machine          Target machine type.
#   $4 - target           Target architecture.
#   $5 - build_mode       Build mode (debug or release).
#   $6 - log_level        Log level.
#

#===================================================================================================

# Fast fail on errors, unset variables, and pipe failures
set -euo pipefail

#==================================================================================================
# Imports
#==================================================================================================

# Directory where to find scripts to import.
IMPORT_DIR="$(cd "$(dirname "$0")" && pwd)/common"

source "${IMPORT_DIR}/logging.sh"
source "${IMPORT_DIR}/utils.sh"

#===================================================================================================
# Functions
#===================================================================================================

#
# Description
#
#   Prints usage information for this script.
#
# Usage Example
#
#   print_help
#
print_help() {
    cat << EOF
Generates a JSON manifest file with build metadata and git information.

Usage: $0 <output-file> <version> <machine> <target> <build_mode> <log_level>

Environment Variables:
  MEMORY_SIZE_BYTES  Memory size in bytes (required, exported by the Makefile).

Arguments:
  output-file      Path to the manifest JSON file to generate.
  version          Release version string.
  machine          Target machine type.
  target           Target architecture.
  build_mode       Build mode (debug or release).
  log_level        Log level.
EOF
}

#===================================================================================================
# Main
#===================================================================================================

# Check that all required external utilities are installed.
for cmd in jq date; do
    if ! command -v "${cmd}" >/dev/null 2>&1; then
        print_error "'${cmd}' is required but not installed."
        exit 1
    fi
done

if [[ "${1:-}" == "--help" ]]; then
    print_help
    exit 0
fi

if [[ $# -ne 6 ]]; then
  print_error "Expected 6 arguments, got $#."
    print_help >&2
    exit 1
fi

if [[ -z "${MEMORY_SIZE_BYTES:-}" ]]; then
    print_error "MEMORY_SIZE_BYTES environment variable is required but not set."
    exit 1
fi

OUTPUT_FILE="$1"
VERSION="$2"
MACHINE="$3"
TARGET="$4"
BUILD_MODE="$5"
LOG_LEVEL="$6"

TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
# Get git commit hash, or "unknown" if we're not in a git repository.
if command -v git >/dev/null 2>&1 && git rev-parse --git-dir >/dev/null 2>&1; then
    GIT_COMMIT="$(git rev-parse HEAD)"
else
    GIT_COMMIT="unknown"
fi

# Ensure the output directory exists.
mkdir -p "$(dirname "${OUTPUT_FILE}")"

# Use MEMORY_SIZE_BYTES from the environment.
MEMORY_SIZE="${MEMORY_SIZE_BYTES}"

# Generate the manifest using jq to safely escape all values.
# Write to a temporary file and move into place for an atomic update. If jq fails unexpectedly, we
# don't leave behind a corrupted manifest.
TMP_FILE="$(mktemp)"
trap 'rm -f "${TMP_FILE}"' EXIT
jq -n \
  --arg version         "${VERSION}" \
  --arg machine         "${MACHINE}" \
  --arg target          "${TARGET}" \
  --arg build_mode      "${BUILD_MODE}" \
  --arg log_level       "${LOG_LEVEL}" \
  --arg timestamp       "${TIMESTAMP}" \
  --arg git_commit      "${GIT_COMMIT}" \
  --argjson memory_size "${MEMORY_SIZE}" \
  '{
    version: $version,
    machine: $machine,
    target: $target,
    build_mode: $build_mode,
    log_level: $log_level,
    timestamp: $timestamp,
    git_commit: $git_commit,
    memory_size: $memory_size
  }' > "${TMP_FILE}"
mv "${TMP_FILE}" "${OUTPUT_FILE}"
