#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

set -euo pipefail

#===================================================================================================
# Description
#===================================================================================================

# Builds a minimal Docker image for the Nanvix toolchain by extracting from the existing full image.
# The resulting image keeps only runtime assets, reducing size while preserving build capability.

#===================================================================================================
# Imports
#===================================================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMMON_DIR="${SCRIPT_DIR%/setup}/common"

# shellcheck source=../common/utils.sh disable=SC1091
source "${COMMON_DIR}/utils.sh"

#===================================================================================================
# Script Arguments
#===================================================================================================

REPO_ROOT_DIR=$(get_repo_root 2>/dev/null) || {
    echo "ERROR: Not in a git repository." >&2
    exit 1
}

CARGO_VERSION=$(get_cargo_toml_version "${REPO_ROOT_DIR}/Cargo.toml")
TOOLCHAIN_TAG="v${CARGO_VERSION%.*}.x"

# Base image that provides the full toolchain; matches the Cargo version (same logic as z).
TOOLCHAIN_IMAGE="nanvix/toolchain:${TOOLCHAIN_TAG}"

# Default output tag suffixed with -minimal as requested.
DEFAULT_TAG="nanvix/toolchain:${TOOLCHAIN_TAG}-minimal"

TAG=${1:-"${DEFAULT_TAG}"}

#===================================================================================================
# Build
#===================================================================================================

RUST_VERSION=$(grep "^channel" "${REPO_ROOT_DIR}/rust-toolchain" | cut -d'"' -f2) || {
    echo "ERROR: Failed to extract Rust version from rust-toolchain file: ${REPO_ROOT_DIR}/rust-toolchain" >&2
    exit 1
}

echo "Building minimal Docker image..."
echo "  Base image : ${TOOLCHAIN_IMAGE}"
echo "  Output tag : ${TAG}"
echo "  Rust toolchain: ${RUST_VERSION}"
echo ""

docker build \
    --file "${SCRIPT_DIR}/Dockerfile.optimized" \
    --build-arg RUST_VERSION="${RUST_VERSION}" \
    --build-arg TOOLCHAIN_IMAGE="${TOOLCHAIN_IMAGE}" \
    --tag "${TAG}" \
    --progress=plain \
    "${SCRIPT_DIR}"

echo ""
echo "✅ Successfully built: ${TAG}"
echo ""
docker images | grep -E "(REPOSITORY|nanvix/toolchain)"
echo ""
echo "Usage:"
echo "  docker run --rm -v \\\$(pwd):/workspace ${TAG} \\
    bash -c 'ln -sf /opt/nanvix /workspace/toolchain && cd /workspace && ./z build -- all'"
