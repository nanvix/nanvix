#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#
# Updates hyperlight crates to the latest commit on the default branch.
#
# This script resolves the default branch of hyperlight-dev/hyperlight via the
# GitHub API, fetches the latest commit SHA on that branch, and updates the
# three hyperlight workspace dependencies in Cargo.toml to use git+rev
# references.
#
# The script prints key=value pairs to stdout for CI consumption.
# All log messages are sent to stderr so stdout contains only machine-readable output.
# When no update is needed, it prints UPDATED=false and exits 0.
#
# Environment Variables:
#   GITHUB_TOKEN / GH_TOKEN  Optional GitHub token for authenticated API requests.
#
# Run './scripts/update-hyperlight.sh --help' for usage information.
#

#===================================================================================================

# Fast fail on errors, unset variables, and pipe failures.
set -euo pipefail

#==================================================================================================
# Imports
#==================================================================================================

IMPORT_DIR="$(cd "$(dirname "$0")" && pwd)/common"

source "${IMPORT_DIR}/logging.sh"
source "${IMPORT_DIR}/utils.sh"

#===================================================================================================
# Constants
#===================================================================================================

readonly HYPERLIGHT_REPO="hyperlight-dev/hyperlight"
readonly HYPERLIGHT_GIT_URL="https://github.com/${HYPERLIGHT_REPO}"
readonly GITHUB_API="https://api.github.com"

# Crate names to update.
readonly HYPERLIGHT_CRATES=("hyperlight-common" "hyperlight-host" "hyperlight-guest")

#===================================================================================================
# Global Variables
#===================================================================================================

REPO_ROOT_DIR=$(get_repo_root)
readonly REPO_ROOT_DIR

readonly CARGO_TOML_FILE="${REPO_ROOT_DIR}/Cargo.toml"

#===================================================================================================
# Functions
#===================================================================================================

print_help() {
    cat << EOF
Updates hyperlight crates to the latest commit on the default branch.

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
#   $1 - API path (e.g., "repos/hyperlight-dev/hyperlight/commits/main").
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
#   Resolves the latest commit SHA and workspace version on the default branch
#   of hyperlight-dev/hyperlight.
#   Sets LATEST_SHA and LATEST_VERSION global variables.
#
resolve_latest_commit() {
    # Resolve the default branch name dynamically.
    print_info "Resolving default branch for ${HYPERLIGHT_REPO}..." >&2

    local repo_json
    repo_json=$(github_api_get "repos/${HYPERLIGHT_REPO}")

    local default_branch
    default_branch=$(echo "${repo_json}" | jq -r '.default_branch')

    if [[ -z "${default_branch}" || "${default_branch}" == "null" ]]; then
        print_error "Could not determine default branch for ${HYPERLIGHT_REPO}."
        exit 1
    fi

    print_info "Default branch: ${default_branch}" >&2
    print_info "Fetching latest commit on ${HYPERLIGHT_REPO}/${default_branch}..." >&2

    local commit_json
    commit_json=$(github_api_get "repos/${HYPERLIGHT_REPO}/commits/${default_branch}")

    LATEST_SHA=$(echo "${commit_json}" | jq -r '.sha')

    if [[ -z "${LATEST_SHA}" || "${LATEST_SHA}" == "null" ]]; then
        print_error "Could not resolve latest commit for ${HYPERLIGHT_REPO}."
        exit 1
    fi

    print_info "Latest commit: ${LATEST_SHA}" >&2

    # Resolve the workspace version from the remote Cargo.toml.
    print_info "Resolving hyperlight workspace version..." >&2

    local contents_json
    contents_json=$(github_api_get \
        "repos/${HYPERLIGHT_REPO}/contents/Cargo.toml?ref=${LATEST_SHA}")

    LATEST_VERSION=$(echo "${contents_json}" \
        | jq -r '.content' \
        | base64 -d \
        | awk '
            /^\[workspace\.package\]/ { in_section=1; next }
            /^\[/ && in_section { in_section=0 }
            in_section && $1 ~ /^version[[:space:]]*=/ {
                if (match($0, /"[^"]*"/)) {
                    v = substr($0, RSTART + 1, RLENGTH - 2);
                    print v;
                    exit;
                }
            }
        ')

    if [[ -z "${LATEST_VERSION}" ]]; then
        print_error "Could not determine hyperlight workspace version."
        exit 1
    fi

    print_info "Hyperlight version: ${LATEST_VERSION}" >&2
}

#
# Description
#
#   Reads the current hyperlight reference from Cargo.toml.
#   Detects whether entries use version pins or git+rev references.
#   Sets CURRENT_REF global variable (a version string or SHA).
#   Sets CURRENT_FORMAT global variable ("version" or "git").
#
read_current_ref() {
    local first_crate="${HYPERLIGHT_CRATES[0]}"
    local line
    line=$(grep "^${first_crate}" "${CARGO_TOML_FILE}" || true)

    if [[ -z "${line}" ]]; then
        print_error "Could not find ${first_crate} in ${CARGO_TOML_FILE}."
        exit 1
    fi

    if echo "${line}" | grep -q 'rev = '; then
        CURRENT_FORMAT="git"
        CURRENT_REF=$(echo "${line}" | sed 's/.*rev = "\([^"]*\)".*/\1/')
    elif echo "${line}" | grep -q 'version = '; then
        CURRENT_FORMAT="version"
        CURRENT_REF=$(echo "${line}" | sed 's/.*version = "\([^"]*\)".*/\1/')
    else
        print_error "Unrecognized dependency format for ${first_crate}."
        exit 1
    fi

    if [[ -z "${CURRENT_REF}" ]]; then
        print_error "Could not extract current reference for ${first_crate}."
        exit 1
    fi

    print_info "Current format: ${CURRENT_FORMAT}, ref: ${CURRENT_REF}" >&2
}

#
# Description
#
#   Updates a single hyperlight crate entry in Cargo.toml from either
#   version pin or git+rev format to the new git+rev reference.
#
# Arguments
#
#   $1 - Crate name (e.g., "hyperlight-common").
#   $2 - New commit SHA.
#
update_crate_entry() {
    local crate="$1"
    local new_sha="$2"
    local git_ref="git = \"${HYPERLIGHT_GIT_URL}\", rev = \"${new_sha}\""

    if [[ "${CURRENT_FORMAT}" == "version" ]]; then
        # Before: hyperlight-common = { version = "0.13.1", default-features ...
        # After:  hyperlight-common = { git = "...", rev = "...", default-features ...
        sed -i "s|${crate} = {[[:space:]]*version = \"[^\"]*\",|${crate} = { ${git_ref},|" \
            "${CARGO_TOML_FILE}"
    else
        # Replace: rev = "OLD_SHA" → rev = "NEW_SHA"
        sed -i "/${crate}/s|rev = \"[^\"]*\"|rev = \"${new_sha}\"|" \
            "${CARGO_TOML_FILE}"
    fi
}

#
# Description
#
#   Updates all hyperlight crate entries in Cargo.toml.
#
update_cargo_toml() {
    for crate in "${HYPERLIGHT_CRATES[@]}"; do
        update_crate_entry "${crate}" "${LATEST_SHA}"
    done

    # Validate that all entries now contain the new SHA.
    for crate in "${HYPERLIGHT_CRATES[@]}"; do
        if ! grep -q "${crate}.*rev = \"${LATEST_SHA}\"" "${CARGO_TOML_FILE}"; then
            print_error "Failed to update ${crate} in ${CARGO_TOML_FILE}."
            exit 1
        fi
    done

    print_success "Updated hyperlight crates to rev ${LATEST_SHA}" >&2
}

#
# Description
#
#   Regenerates Cargo.lock after the dependency changes.
#
update_cargo_lock() {
    print_info "Updating Cargo.lock..." >&2
    (
        cd "${REPO_ROOT_DIR}"
        cargo update -p hyperlight-common -p hyperlight-host -p hyperlight-guest
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

    # Step 1: Resolve latest commit on the default branch.
    resolve_latest_commit

    # Step 2: Read current reference from Cargo.toml.
    read_current_ref

    # Step 3: Check if update is needed.
    if [[ "${CURRENT_FORMAT}" == "git" && "${CURRENT_REF}" == "${LATEST_SHA}" ]]; then
        print_info "Hyperlight is already up to date (${CURRENT_REF:0:12})." >&2
        echo "UPDATED=false"
        exit 0
    fi

    print_info "Update available: ${CURRENT_REF:0:12} -> ${LATEST_SHA:0:12}" >&2

    # Step 4: Update Cargo.toml.
    update_cargo_toml

    # Step 5: Regenerate Cargo.lock.
    update_cargo_lock

    # Step 6: Output results for CI.
    echo "UPDATED=true"
    echo "OLD_REF=${CURRENT_REF}"
    echo "NEW_REV=${LATEST_SHA}"
    echo "VERSION=${LATEST_VERSION}"
}

main "$@"
