#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#
# Updates Verus and vstd to the latest release.
#
# This script resolves the latest Verus release tag from the verus-lang/verus
# repository, determines the matching vstd version, and updates:
#   - build/verus-version  (release version string)
#   - Cargo.toml           (vstd exact version pin)
#   - Cargo.lock           (regenerated via cargo update)
#
# The script prints key=value pairs to stdout for CI consumption.
# All log messages are sent to stderr so stdout contains only machine-readable output.
# When no update is needed, it prints UPDATED=false and exits 0.
#
# Environment Variables:
#   GITHUB_TOKEN / GH_TOKEN  Optional GitHub token for authenticated API requests.
#
# Run './scripts/update-verus.sh --help' for usage information.
#

#===================================================================================================

# Fast fail on errors, unset variables, and pipe failures.
set -euo pipefail

#==================================================================================================
# Imports
#==================================================================================================

IMPORT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common"

source "${IMPORT_DIR}/logging.sh"
source "${IMPORT_DIR}/utils.sh"

#===================================================================================================
# Constants
#===================================================================================================

readonly VERUS_REPO="verus-lang/verus"
readonly GITHUB_API="https://api.github.com"
readonly CRATES_IO_API="https://crates.io/api/v1"
readonly CRATES_IO_USER_AGENT="nanvix-verus-updater (https://github.com/nanvix/nanvix)"

#===================================================================================================
# Global Variables
#===================================================================================================

REPO_ROOT_DIR=$(get_repo_root)
readonly REPO_ROOT_DIR

readonly VERUS_VERSION_FILE="${REPO_ROOT_DIR}/build/verus-version"
readonly CARGO_TOML_FILE="${REPO_ROOT_DIR}/Cargo.toml"

#===================================================================================================
# Functions
#===================================================================================================

print_help() {
    cat << EOF
Updates Verus and vstd to the latest release.

Usage: $0 [OPTIONS]

Options:
  --help    Print this help information and exit.

Environment Variables:
  GITHUB_TOKEN    GitHub token for authenticated API requests.
  GH_TOKEN        Alternative to GITHUB_TOKEN (GitHub CLI convention).
EOF
}

#
# Description
#
#   Builds curl arguments for GitHub API authentication.
#   Checks GITHUB_TOKEN first, then falls back to GH_TOKEN.
#
# Returns
#
#   Prints authorization arguments for curl, or nothing if no token is set.
#
get_curl_auth_args() {
    local token=""

    if [[ -n "${GITHUB_TOKEN:-}" ]]; then
        token="${GITHUB_TOKEN}"
    elif [[ -n "${GH_TOKEN:-}" ]]; then
        token="${GH_TOKEN}"
    fi

    if [[ -n "${token}" ]]; then
        echo "-H"
        echo "Authorization: Bearer ${token}"
    fi
}

#
# Description
#
#   Performs a GET request to the GitHub API and returns the JSON response.
#
# Arguments
#
#   $1 - API path (e.g., "repos/verus-lang/verus/tags?per_page=100").
#
# Returns
#
#   The JSON response body on stdout.
#   Exits with non-zero on failure.
#
github_api_get() {
    local api_path="$1"
    local url="${GITHUB_API}/${api_path}"

    local curl_args=(curl -sL --fail --connect-timeout 30 --max-time 120)

    # Add auth header if a token is available.
    local auth_args
    auth_args=$(get_curl_auth_args)
    if [[ -n "${auth_args}" ]]; then
        # Read the two arguments (flag + value) produced by get_curl_auth_args.
        while IFS= read -r arg; do
            curl_args+=("${arg}")
        done <<< "${auth_args}"
    fi

    curl_args+=("${url}")
    "${curl_args[@]}"
}

#
# Description
#
#   Performs a GET request to the crates.io API and returns the JSON response.
#
# Arguments
#
#   $1 - API path (e.g., "crates/vstd/versions").
#
# Returns
#
#   The JSON response body on stdout.
#   Exits with non-zero on failure.
#
crates_io_api_get() {
    local api_path="$1"
    local url="${CRATES_IO_API}/${api_path}"

    curl -sL --fail --connect-timeout 30 --max-time 120 \
        --user-agent "${CRATES_IO_USER_AGENT}" "${url}"
}

#
# Description
#
#   Resolves the latest non-rolling Verus release tag, commit SHA, and publication time.
#   Sets LATEST_VERSION, LATEST_COMMIT, and LATEST_RELEASED_AT global variables.
#
resolve_latest_release() {
    print_info "Fetching Verus release tags..." >&2

    local tags_json
    tags_json=$(github_api_get "repos/${VERUS_REPO}/tags?per_page=100")

    # Filter for release/ tags, exclude rolling, sort by version descending, pick first.
    # Tags follow the pattern "release/<version>" (single slash), so field 2 is the version.
    local latest_tag
    latest_tag=$(echo "${tags_json}" \
        | jq -r '.[].name' \
        | { grep '^release/' || true; } \
        | { grep -v '/rolling/' || true; } \
        | sort -t/ -k2 -rV \
        | head -1)

    if [[ -z "${latest_tag}" ]]; then
        print_error "Could not determine the latest Verus release tag."
        exit 1
    fi

    # Extract version string (e.g., "0.2026.02.06.4a2b93e").
    LATEST_VERSION="${latest_tag#release/}"

    # Read the release publication time. Verus publishes crates before tagging a release, then
    # updates the source manifests after the tag, so the tagged manifest may contain stale versions.
    local encoded_tag="${latest_tag//\//%2F}"
    local release_json
    release_json=$(github_api_get "repos/${VERUS_REPO}/releases/tags/${encoded_tag}")

    LATEST_RELEASED_AT=$(echo "${release_json}" | jq -r '.published_at')

    if [[ -z "${LATEST_RELEASED_AT}" || "${LATEST_RELEASED_AT}" == "null" ]]; then
        print_error "Could not determine the publication time for tag ${latest_tag}."
        exit 1
    fi

    # Resolve the full commit SHA for that tag.
    local commit_json
    commit_json=$(github_api_get "repos/${VERUS_REPO}/commits/${latest_tag}")

    LATEST_COMMIT=$(echo "${commit_json}" | jq -r '.sha')

    if [[ -z "${LATEST_COMMIT}" || "${LATEST_COMMIT}" == "null" ]]; then
        print_error "Could not resolve commit for tag ${latest_tag}."
        exit 1
    fi

    print_info "Latest Verus release: ${LATEST_VERSION} (${LATEST_COMMIT})" >&2
}

#
# Description
#
#   Reads the current Verus version from build/verus-version.
#   Sets CURRENT_VERSION global variable.
#
read_current_version() {
    if [[ ! -f "${VERUS_VERSION_FILE}" ]]; then
        print_error "${VERUS_VERSION_FILE} does not exist."
        exit 1
    fi

    CURRENT_VERSION=$(head -1 "${VERUS_VERSION_FILE}" | tr -d '[:space:]')

    if [[ -z "${CURRENT_VERSION}" ]]; then
        print_error "Could not read current Verus version from ${VERUS_VERSION_FILE}."
        exit 1
    fi

    print_info "Current Verus version: ${CURRENT_VERSION}" >&2
}

#
# Description
#
#   Resolves the newest vstd crate version published by the target Verus release.
#   Sets NEW_VSTD global variable.
#
resolve_vstd_version() {
    print_info "Resolving vstd version for Verus ${LATEST_VERSION}..." >&2

    local versions_json
    versions_json=$(crates_io_api_get "crates/vstd/versions")

    local vstd_version
    vstd_version=$(echo "${versions_json}" \
        | jq -r --arg released_at "${LATEST_RELEASED_AT}" '
            [.versions[]
                | select(.yanked == false)
                | select(.created_at <= $released_at)]
            | sort_by(.created_at)
            | last
            | .num // empty')

    if [[ -z "${vstd_version}" ]]; then
        print_error "Could not determine vstd version for Verus ${LATEST_VERSION}."
        exit 1
    fi

    NEW_VSTD="${vstd_version}"
    print_info "Matching vstd version: ${NEW_VSTD}" >&2
}

#
# Description
#
#   Updates the build/verus-version file with the new version string.
#
update_verus_version_file() {
    printf '%s\n' "${LATEST_VERSION}" > "${VERUS_VERSION_FILE}"
    print_success "Updated ${VERUS_VERSION_FILE} to ${LATEST_VERSION}" >&2
}

#
# Description
#
#   Updates the vstd exact version pin in Cargo.toml.
#   Sets OLD_VSTD global variable.
#
update_vstd_in_cargo_toml() {
    OLD_VSTD=$(grep 'vstd.*version.*=' "${CARGO_TOML_FILE}" \
        | sed 's/.*"=\([^"]*\)".*/\1/' || true)

    if [[ -z "${OLD_VSTD}" ]]; then
        print_error "Could not read current vstd version from ${CARGO_TOML_FILE}."
        exit 1
    fi

    if [[ "${OLD_VSTD}" != "${NEW_VSTD}" ]]; then
        sed -i "s|vstd[[:space:]]*=[[:space:]]*{[[:space:]]*version[[:space:]]*=[[:space:]]*\"=${OLD_VSTD}\"|vstd = { version = \"=${NEW_VSTD}\"|" \
            "${CARGO_TOML_FILE}"

        # Validate the substitution.
        local updated_vstd
        updated_vstd=$(grep 'vstd.*version.*=' "${CARGO_TOML_FILE}" \
            | sed 's/.*"=\([^"]*\)".*/\1/' || true)

        if [[ "${updated_vstd}" != "${NEW_VSTD}" ]]; then
            print_error "Failed to update vstd version in ${CARGO_TOML_FILE}."
            exit 1
        fi

        print_success "Updated vstd: ${OLD_VSTD} -> ${NEW_VSTD}" >&2
    else
        print_info "vstd version already matches (${OLD_VSTD})." >&2
    fi
}

#
# Description
#
#   Regenerates Cargo.lock after the vstd version pin change.
#
update_cargo_lock() {
    print_info "Updating Cargo.lock..." >&2
    (
        cd "${REPO_ROOT_DIR}"
        cargo update --package vstd
    )
    print_success "Cargo.lock updated." >&2
}

#===================================================================================================
# Main
#===================================================================================================

main() {
    # Check required dependencies.
    command -v jq >/dev/null 2>&1 || { print_error "jq is required but not installed."; exit 1; }

    # Parse arguments.
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --help)
                print_help
                exit 0
                ;;
            *)
                print_error "Unknown option: $1"
                print_help
                exit 1
                ;;
        esac
    done

    # Step 1: Resolve latest release.
    resolve_latest_release

    # Step 2: Read current version.
    read_current_version

    # Step 3: Check if update is needed.
    if [[ "${CURRENT_VERSION}" == "${LATEST_VERSION}" ]]; then
        print_info "Verus is already up to date (${CURRENT_VERSION})." >&2
        echo "UPDATED=false"
        exit 0
    fi

    print_info "Update available: ${CURRENT_VERSION} -> ${LATEST_VERSION}" >&2

    # Step 4: Resolve matching vstd version.
    resolve_vstd_version

    # Step 5: Update build/verus-version.
    update_verus_version_file

    # Step 6: Update vstd in Cargo.toml.
    update_vstd_in_cargo_toml

    # Step 7: Regenerate Cargo.lock.
    update_cargo_lock

    # Step 8: Output results for CI.
    echo "UPDATED=true"
    echo "CURRENT_VERSION=${CURRENT_VERSION}"
    echo "LATEST_VERSION=${LATEST_VERSION}"
    echo "OLD_VSTD=${OLD_VSTD}"
    echo "NEW_VSTD=${NEW_VSTD}"
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    main "$@"
fi
