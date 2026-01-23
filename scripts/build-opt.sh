#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Unified Optional Dependency Builder
#===================================================================================================
# Usage:
#   build-opt.sh <rule> <toolchain_dir> <sysroot_dir> <repository_url> <commit_id> <dirname>
#
#   rule:           init | build | clean
#   toolchain_dir:  Path to Nanvix cross toolchain (required)
#   sysroot_dir:    Target sysroot directory (required)
#   repository_url: Git repository URL (required)
#   commit_id:      Commit to checkout (required)
#   dirname:        Local directory name under ${sysroot_dir}/src (required)
#
#===================================================================================================

set -euo pipefail

#===================================================================================================
# Script Arguments
#===================================================================================================

if [[ $# -ne 6 ]]; then
    echo "Usage: $0 <rule> <toolchain_dir> <sysroot_dir> <repository_url> <commit_id> <dirname>" >&2
    exit 1
fi

RULE=$1
TOOLCHAIN_DIR=$2
SYSROOT_DIR=$3
REPOSITORY=$4
COMMIT=$5
DIRNAME=$6

#===================================================================================================
# Global Variables
#===================================================================================================

SCRIPT_DIR="$(dirname "$(realpath "${BASH_SOURCE[0]}")")"
NANVIX_HOME="$(realpath "${SCRIPT_DIR}/..")"
# Use CONTRIB_SRC_DIR if set (e.g., in Docker builds), otherwise fall back to sysroot/src.
CONTRIB_DIR="${CONTRIB_SRC_DIR:-${SYSROOT_DIR}/src}"
REPOSITORY_HOME="${CONTRIB_DIR}/${DIRNAME}"

#===================================================================================================
# Imports
#===================================================================================================

source "${NANVIX_HOME}/scripts/common/logging.sh"
source "${NANVIX_HOME}/scripts/common/utils.sh"

#===================================================================================================
# Functions
#===================================================================================================

init_repo() {
    mkdir -p "${CONTRIB_DIR}" || exit 1
    if [[ ! -d "${REPOSITORY_HOME}/.git" ]]; then
        git clone "${REPOSITORY}" "${REPOSITORY_HOME}" || exit 1
    else
        git -C "${REPOSITORY_HOME}" fetch origin || exit 1
        git -C "${REPOSITORY_HOME}" reset --hard || exit 1
    fi
    git -C "${REPOSITORY_HOME}" checkout "${COMMIT}" || exit 1
}

do_build() {
    cd "${REPOSITORY_HOME}" || exit 1
    ./z configure --toolchain-path="${TOOLCHAIN_DIR}" --sysroot-path="${SYSROOT_DIR}"
    ./z build
    ./z install
}

do_clean() {
    if [[ -d "${REPOSITORY_HOME}" ]]; then
        cd "${REPOSITORY_HOME}" || exit 1
        ./z clean || {
            print_warning "Failed to clean optional dependency in ${REPOSITORY_HOME}"
        }
    fi
}

#===================================================================================================
# Main Script
#===================================================================================================

# Save environment variables.
OLD_PATH=${PATH}
OLD_CC=${CC:-}
OLD_CXX=${CXX:-}
OLD_CFLAGS=${CFLAGS:-}
OLD_CXXFLAGS=${CXXFLAGS:-}
OLD_LDFLAGS=${LDFLAGS:-}

# Add sccache utility to PATH if available.
if [[ -n "${SCCACHE:-}" ]]; then
    sccache_dir="$(dirname "${SCCACHE}")"
    if [[ -d "${sccache_dir}" ]]; then
        export PATH="${sccache_dir}:${PATH}"
    fi
fi

# Hide tool overrides that might interfere with the build process.
unset CC CXX CPP LD CFLAGS CXXFLAGS LDFLAGS

case "${RULE}" in
    init)
        init_repo
        ;;
    build)
        init_repo
        do_build
        ;;
    clean)
        do_clean
        ;;
    *)
        echo "Unknown rule: ${RULE}" >&2
        exit 1
        ;;
esac

# Restore environment variables.
export PATH=${OLD_PATH}
if [[ -n "${OLD_CC}" ]]; then export CC="${OLD_CC}"; else unset CC; fi
if [[ -n "${OLD_CXX}" ]]; then export CXX="${OLD_CXX}"; else unset CXX; fi
if [[ -n "${OLD_CFLAGS}" ]]; then export CFLAGS="${OLD_CFLAGS}"; else unset CFLAGS; fi
if [[ -n "${OLD_CXXFLAGS}" ]]; then export CXXFLAGS="${OLD_CXXFLAGS}"; else unset CXXFLAGS; fi
if [[ -n "${OLD_LDFLAGS}" ]]; then export LDFLAGS="${OLD_LDFLAGS}"; else unset LDFLAGS; fi

exit 0
