#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#
# Ensures the correct version of Verus is installed.
#
# Reads the expected version from build/verus-version, checks whether Verus is
# already present at that version in the target directory, and downloads/installs
# it when missing or outdated. Works for both local and Docker-based toolchains.
#
# Usage: ./scripts/setup/verus.sh [--download-only] <install-dir>
#
# Options:
#   --download-only  Download the Verus archive to the local cache and exit
#                    without installing. Useful for CI pre-warming.
#
# Environment Variables:
#   GITHUB_TOKEN        GitHub token for authenticated downloads (optional).
#   GH_TOKEN            Alternative to GITHUB_TOKEN (GitHub CLI convention).
#   VERUS_ZIP_CACHE_DIR Override the default cache directory (.verus-cache).
#

# Fast fail on errors, unset variables, and pipe failures.
set -euo pipefail

#===================================================================================================
# Imports
#===================================================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMMON_DIR="${SCRIPT_DIR%/setup}/common"

# shellcheck source=../common/logging.sh disable=SC1091
source "${COMMON_DIR}/logging.sh"

#===================================================================================================
# Constants
#===================================================================================================

# Repository root (fallback to relative path when not inside a git worktree).
REPO_ROOT_DIR=$(git rev-parse --show-toplevel 2>/dev/null || echo "$(cd "${SCRIPT_DIR}/../.." && pwd)")

# Version file that pins the Verus release to use.
VERUS_VERSION_FILE="${REPO_ROOT_DIR}/build/verus-version"

# GitHub release URL template.
VERUS_RELEASE_URL="https://github.com/verus-lang/verus/releases/download/release"

# Subdirectory name inside the release zip.
VERUS_ZIP_SUBDIR="verus-x86-linux"

# Directory for caching downloaded Verus zip archives. Keyed by version so that
# only the first download hits the network; subsequent runs (and CI cache
# restores) reuse the local copy. Override via VERUS_ZIP_CACHE_DIR env var.
VERUS_ZIP_CACHE_DIR="${VERUS_ZIP_CACHE_DIR:-${REPO_ROOT_DIR}/.verus-cache}"

# curl resilience settings for transient network / server errors.
CURL_RETRY_COUNT=5          # Number of retries on transient errors.
CURL_RETRY_DELAY=10         # Fixed delay (seconds) between retries (overrides curl's default backoff).
CURL_CONNECT_TIMEOUT=30     # Max seconds to wait for a TCP connection.
CURL_MAX_TIME=300           # Overall request deadline (5 minutes).

#===================================================================================================
# Functions
#===================================================================================================

#
# DESCRIPTION
#   Prints curl authorization arguments to stdout (one per line) when a
#   GitHub token is available. Checks GITHUB_TOKEN first, then falls
#   back to GH_TOKEN (GitHub CLI convention). The caller captures the
#   output via mapfile to build an array, which avoids Bash 4.3+ nameref
#   requirements and keeps the script portable.
#
# USAGE
#   local auth_args=()
#   mapfile -t auth_args < <(get_curl_auth_args)
#   curl "${auth_args[@]}" ...
#
get_curl_auth_args() {
    local token="${GITHUB_TOKEN:-${GH_TOKEN:-}}"
    if [[ -n "${token}" ]]; then
        printf '%s\n' "-H" "Authorization: Bearer ${token}"
    fi
}

#
# DESCRIPTION
#   Downloads the Verus release archive to a temporary directory, using
#   the local zip cache when available. On a fresh download the archive
#   is also persisted to the cache for future runs.
#
# ARGUMENTS
#   $1 - Destination file path for the downloaded zip.
#   $2 - The Verus version to download.
#
download_verus_archive() {
    local dest_path="$1"
    local version="$2"

    local zip_name="verus-${version}-x86-linux.zip"
    local download_url="${VERUS_RELEASE_URL}/${version}/${zip_name}"

    # Try the local zip cache first to avoid a network round-trip.
    # Discard cached files that are empty or obviously corrupt (size zero),
    # and validate non-empty cached zips before using them.
    if [[ -n "${VERUS_ZIP_CACHE_DIR}" ]]; then
        local cached_zip="${VERUS_ZIP_CACHE_DIR}/${zip_name}"
        if [[ -f "${cached_zip}" && -s "${cached_zip}" ]]; then
            # Validate the cached archive; if invalid, delete and fall back to downloading.
            if unzip -tq "${cached_zip}" >/dev/null 2>&1; then
                print_info "Using cached Verus archive from ${cached_zip}"
                cp "${cached_zip}" "${dest_path}"
                return 0
            else
                print_warning "Cached Verus archive at ${cached_zip} is corrupted; deleting and re-downloading."
                rm -f "${cached_zip}"
            fi
        fi
    fi

    print_info "Downloading Verus ${version}..."
    local auth_args=()
    mapfile -t auth_args < <(get_curl_auth_args)
    if ! curl -fsSL \
            --retry "${CURL_RETRY_COUNT}" \
            --retry-delay "${CURL_RETRY_DELAY}" \
            --retry-all-errors \
            --connect-timeout "${CURL_CONNECT_TIMEOUT}" \
            --max-time "${CURL_MAX_TIME}" \
            "${auth_args[@]}" \
            -o "${dest_path}" "${download_url}"; then
        print_error "Failed to download Verus from ${download_url}"
        exit 1
    fi

    # Persist the downloaded archive for future runs.
    if [[ -n "${VERUS_ZIP_CACHE_DIR}" ]]; then
        mkdir -p "${VERUS_ZIP_CACHE_DIR}"
        cp "${dest_path}" "${VERUS_ZIP_CACHE_DIR}/${zip_name}"
    fi
}

#
# DESCRIPTION
#   Reads and returns the expected Verus version from the version file.
#
# RETURNS
#   The version string (e.g., "0.2026.02.06.4a2b93e").
#
get_expected_version() {
    if [[ ! -f "${VERUS_VERSION_FILE}" ]]; then
        print_error "Verus version file not found: ${VERUS_VERSION_FILE}"
        exit 1
    fi

    local version
    version=$(tr -d '[:space:]' < "${VERUS_VERSION_FILE}")

    if [[ -z "${version}" ]]; then
        print_error "Verus version file is empty: ${VERUS_VERSION_FILE}"
        exit 1
    fi

    echo "${version}"
}

#
# DESCRIPTION
#   Reads the currently installed Verus version from the install directory.
#
# ARGUMENTS
#   $1 - The installation directory.
#
# RETURNS
#   The installed version string, or empty if not installed.
#
get_installed_version() {
    local install_dir="$1"

    if [[ -f "${install_dir}/version.txt" ]]; then
        tr -d '[:space:]' < "${install_dir}/version.txt"
    fi
}

#
# DESCRIPTION
#   Downloads and installs Verus into the target directory.
#
# ARGUMENTS
#   $1 - The target installation directory.
#   $2 - The Verus version to install.
#
install_verus() {
    local install_dir="$1"
    local version="$2"

    local zip_name="verus-${version}-x86-linux.zip"

    # Validate that the install directory looks reasonable.
    if [[ "${install_dir}" != *verus* ]]; then
        print_error "Install directory '${install_dir}' does not contain 'verus'. Aborting for safety."
        exit 1
    fi

    # Create a temporary directory for the download.
    local tmp_dir
    tmp_dir=$(mktemp -d)

    # Ensure the temporary directory is always cleaned up.
    trap 'rm -rf "${tmp_dir}"' EXIT

    download_verus_archive "${tmp_dir}/${zip_name}" "${version}"

    print_info "Extracting Verus to ${install_dir}..."
    local extract_ok=true
    if command -v unzip &>/dev/null; then
        unzip -o -q "${tmp_dir}/${zip_name}" -d "${tmp_dir}" || extract_ok=false
    elif command -v python3 &>/dev/null; then
        # Use Python's zipfile with permission restoration (unzip preserves these
        # automatically, but zipfile.extractall does not).
        python3 -c "
import zipfile, sys, os
with zipfile.ZipFile(sys.argv[1]) as z:
    z.extractall(sys.argv[2])
    for info in z.infolist():
        if info.external_attr:
            perm = info.external_attr >> 16
            if perm:
                os.chmod(os.path.join(sys.argv[2], info.filename), perm)
" "${tmp_dir}/${zip_name}" "${tmp_dir}" || extract_ok=false
    else
        print_error "Either unzip or python3 is required to extract the Verus archive."
        exit 1
    fi

    if ! ${extract_ok}; then
        print_error "Failed to extract Verus archive."
        exit 1
    fi

    # Verify the expected subdirectory exists in the archive.
    if [[ ! -d "${tmp_dir}/${VERUS_ZIP_SUBDIR}" ]]; then
        print_error "Expected directory '${VERUS_ZIP_SUBDIR}' not found in archive."
        exit 1
    fi

    # Replace the install directory contents atomically.
    mkdir -p "${install_dir}"
    find "${install_dir:?}" -mindepth 1 -maxdepth 1 -exec rm -rf {} +
    cp -a "${tmp_dir}/${VERUS_ZIP_SUBDIR}"/. "${install_dir}/"

    # Cleanup temporary directory.
    rm -rf "${tmp_dir}"
    trap - EXIT

    print_success "Verus ${version} installed to ${install_dir}."
}

#
# DESCRIPTION
#   Ensures the Rust toolchain required by Verus is installed.
#   Reads the required toolchain from version.json and installs it if missing.
#
# ARGUMENTS
#   $1 - The Verus installation directory.
#
ensure_verus_toolchain() {
    local install_dir="$1"

    if [[ -f "${install_dir}/version.json" ]] && command -v rustup &>/dev/null; then
        local required_toolchain
        required_toolchain=$(python3 -c "
import json, sys
data = json.load(open(sys.argv[1]))
print(data.get('verus', {}).get('toolchain', ''))
" "${install_dir}/version.json" 2>/dev/null) || {
            print_warning "Failed to parse ${install_dir}/version.json; skipping toolchain check."
            return 0
        }

        if [[ -n "${required_toolchain}" ]]; then
            if ! rustup run "${required_toolchain}" rustc --version &>/dev/null; then
                print_info "Installing Rust toolchain '${required_toolchain}' required by Verus..."
                rustup toolchain install "${required_toolchain}" --profile minimal --component rust-src
            fi
        fi
    fi
}

#===================================================================================================
# Main
#===================================================================================================

main() {
    local download_only=false
    if [[ "${1:-}" == "--download-only" ]]; then
        download_only=true
        shift
    fi

    local install_dir="${1:-${HOME}/verus}"

    local expected_version
    expected_version=$(get_expected_version)

    # --download-only: populate the cache and exit without installing.
    if ${download_only}; then
        if ! command -v curl &>/dev/null; then
            print_error "curl is required but not installed."
            exit 1
        fi
        local zip_name="verus-${expected_version}-x86-linux.zip"
        local tmp_dir
        tmp_dir=$(mktemp -d)
        trap 'rm -rf "${tmp_dir}"' EXIT
        download_verus_archive "${tmp_dir}/${zip_name}" "${expected_version}"
        rm -rf "${tmp_dir}"
        trap - EXIT
        print_success "Verus ${expected_version} archive cached in ${VERUS_ZIP_CACHE_DIR}."
        return 0
    fi

    local installed_version
    installed_version=$(get_installed_version "${install_dir}")

    if [[ "${installed_version}" == "${expected_version}" ]]; then
        print_info "Verus ${expected_version} is already installed in ${install_dir}."
        ensure_verus_toolchain "${install_dir}"
        return 0
    fi

    if [[ -n "${installed_version}" ]]; then
        print_info "Verus version mismatch (found '${installed_version}', expected '${expected_version}'). Updating..."
    else
        print_info "Verus not found in ${install_dir}. Installing..."
    fi

    # Check dependencies.
    if ! command -v curl &>/dev/null; then
        print_error "curl is required but not installed."
        exit 1
    fi
    if ! command -v unzip &>/dev/null && ! command -v python3 &>/dev/null; then
        print_error "Either unzip or python3 is required but neither is installed."
        exit 1
    fi

    install_verus "${install_dir}" "${expected_version}"
    ensure_verus_toolchain "${install_dir}"
}

main "$@"
