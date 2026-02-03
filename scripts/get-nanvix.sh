#!/bin/bash
# shellcheck shell=bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Description: Downloads the latest release of Nanvix from GitHub.
# This script uses only tools commonly available on Ubuntu systems by default.

set -euo pipefail

# Exit codes.
readonly EXIT_SUCCESS=0
readonly EXIT_FAILURE=1

# Configuration.
readonly GITHUB_REPO="nanvix/nanvix"
readonly GITHUB_API_URL="https://api.github.com/repos/${GITHUB_REPO}/releases/latest"
readonly CONNECT_TIMEOUT="${NANVIX_CONNECT_TIMEOUT:-30}"
readonly MAX_TIMEOUT="${NANVIX_MAX_TIMEOUT:-300}"
readonly FORCE_DOWNLOAD="${NANVIX_FORCE_DOWNLOAD:-false}"

# Print usage information.
usage() {
    local script_name
    script_name=$(basename "$0")
    echo "Usage: $script_name [options] [output_directory]"
    echo ""
    echo "Downloads the latest release of Nanvix from GitHub."
    echo ""
    echo "Arguments:"
    echo "  output_directory  Directory to save downloaded files (default: current directory)"
    echo ""
    echo "Options:"
    echo "  -f, --force       Force download even if files already exist"
    echo "  -h, --help        Show this help message and exit"
    echo ""
    echo "Environment Variables:"
    echo "  NANVIX_CONNECT_TIMEOUT   Connection timeout in seconds (default: 30)"
    echo "  NANVIX_MAX_TIMEOUT       Maximum total timeout in seconds (default: 300)"
    echo "  NANVIX_FORCE_DOWNLOAD    Force download if 'true' (default: false)"
    echo ""
    echo "Examples:"
    echo "  $script_name /tmp/nanvix"
    echo "  $script_name --force /tmp/nanvix"
    echo "  NANVIX_CONNECT_TIMEOUT=60 $script_name /tmp/nanvix"
}

# Print an error message and exit.
error() {
    echo "Error: $1" >&2
    exit "$EXIT_FAILURE"
}

# Print an informational message.
info() {
    echo "[INFO] $1"
}

# Print a warning message.
warn() {
    echo "[WARN] $1" >&2
}

# Check if a command exists.
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Determine which download tool to use.
get_download_tool() {
    if command_exists curl; then
        echo "curl"
    elif command_exists wget; then
        echo "wget"
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
}

# Download a URL to stdout using available tools.
# Note: curl uses --connect-timeout for connection and --max-time for overall timeout.
# wget's --timeout sets all timeouts (connect, read, DNS) to the same value.
# We use --dns-timeout and --connect-timeout for wget to match curl behavior more closely.
download_to_stdout() {
    local url="$1"
    local tool
    local exit_code
    tool=$(get_download_tool)

    case "$tool" in
        curl)
            curl -sL --fail --max-redirs 5 --connect-timeout "$CONNECT_TIMEOUT" --max-time "$MAX_TIMEOUT" "$url"
            exit_code=$?
            if (( exit_code != 0 )); then
                warn "curl failed with exit code $exit_code for URL: $url"
                return 1
            fi
            ;;
        wget)
            wget -qO- --max-redirect=5 --dns-timeout="$CONNECT_TIMEOUT" --connect-timeout="$CONNECT_TIMEOUT" --timeout="$MAX_TIMEOUT" "$url"
            exit_code=$?
            if (( exit_code != 0 )); then
                warn "wget failed with exit code $exit_code for URL: $url"
                return 1
            fi
            ;;
    esac
}

# Download a URL to a file using available tools.
download_to_file() {
    local url="$1"
    local output="$2"
    local tool
    local exit_code
    tool=$(get_download_tool)

    case "$tool" in
        curl)
            curl -sL --fail --max-redirs 5 --connect-timeout "$CONNECT_TIMEOUT" --max-time "$MAX_TIMEOUT" -o "$output" "$url"
            exit_code=$?
            if (( exit_code != 0 )); then
                warn "curl failed with exit code $exit_code for URL: $url"
                return 1
            fi
            ;;
        wget)
            wget -q --max-redirect=5 --dns-timeout="$CONNECT_TIMEOUT" --connect-timeout="$CONNECT_TIMEOUT" --timeout="$MAX_TIMEOUT" -O "$output" "$url"
            exit_code=$?
            if (( exit_code != 0 )); then
                warn "wget failed with exit code $exit_code for URL: $url"
                return 1
            fi
            ;;
    esac
}

# Extract a JSON value using basic shell tools.
# Note: This is a simple parser that works for the GitHub API response format.
# It does not handle multi-line values, escaped quotes, or nested objects.
# WARNING: Do not use for security-sensitive parsing of untrusted input.
extract_json_value() {
    local json="$1"
    local key="$2"
    # Use || true to prevent set -e from exiting if no match is found.
    echo "$json" | { grep -o "\"${key}\"[[:space:]]*:[[:space:]]*\"[^\"]*\"" || true; } | head -n 1 | sed "s/\"${key}\"[[:space:]]*:[[:space:]]*\"\([^\"]*\)\"/\1/"
}

# Extract all download URLs from the JSON response.
extract_asset_urls() {
    local json="$1"
    # Use || true to prevent set -e from exiting if no match is found.
    echo "$json" | { grep -o '"browser_download_url"[[:space:]]*:[[:space:]]*"[^"]*"' || true; } | sed 's/"browser_download_url"[[:space:]]*:[[:space:]]*"\([^"]*\)"/\1/'
}

# Global variable for tracking current download (used by cleanup).
CURRENT_DOWNLOAD=""

# Cleanup partial downloads on failure or interruption.
# shellcheck disable=SC2317  # Function is invoked indirectly via trap.
cleanup() {
    if [ -n "${CURRENT_DOWNLOAD:-}" ] && [ -f "$CURRENT_DOWNLOAD" ]; then
        rm -f "$CURRENT_DOWNLOAD" 2>/dev/null
    fi
}

# Main function.
main() {
    local force_download="$FORCE_DOWNLOAD"
    local output_dir="."

    # Parse command line arguments.
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -h|--help)
                usage
                exit "$EXIT_SUCCESS"
                ;;
            -f|--force)
                force_download="true"
                shift
                ;;
            -*)
                error "Unknown option: $1. Use --help for usage information."
                ;;
            *)
                if [[ "$output_dir" != "." ]]; then
                    error "Multiple output directories specified. Use --help for usage."
                fi
                output_dir="$1"
                shift
                ;;
        esac
    done

    # Validate and normalize output directory path.
    # Check for path traversal components (e.g., '../', '/../', '/..').
    if [[ "$output_dir" == "../"* || "$output_dir" == *"/../"* || "$output_dir" == *"/.." ]]; then
        error "Invalid output directory path: contains path traversal component '..'."
    fi
    output_dir=$(realpath -m "$output_dir" 2>/dev/null) || error "Invalid output directory path."

    info "Fetching latest release information from GitHub..."

    # Download the release information.
    local release_info
    release_info=$(download_to_stdout "$GITHUB_API_URL")

    if [ -z "$release_info" ]; then
        error "Failed to fetch release information from GitHub."
    fi

    # Check for API rate limit or errors.
    if [[ "$release_info" == *'"message"'* ]]; then
        local message
        message=$(extract_json_value "$release_info" "message")
        error "GitHub API error: $message"
    fi

    # Extract release information.
    local tag_name
    tag_name=$(extract_json_value "$release_info" "tag_name")

    if [ -z "$tag_name" ]; then
        error "Could not determine the latest release version."
    fi

    info "Latest release: $tag_name"

    # Create output directory if it does not exist.
    mkdir -p "$output_dir"
    info "Using output directory: $output_dir"

    # Set up cleanup trap for partial downloads.
    trap cleanup ERR EXIT INT TERM

    # Extract asset URLs.
    local asset_urls
    asset_urls=$(extract_asset_urls "$release_info")

    if [ -z "$asset_urls" ]; then
        info "No binary assets found. Downloading source archive instead..."

        # Download source tarball.
        local tarball_url="https://github.com/${GITHUB_REPO}/archive/refs/tags/${tag_name}.tar.gz"
        local tarball_name="nanvix-${tag_name#v}.tar.gz"
        local tarball_path="${output_dir}/${tarball_name}"

        # Check if tarball already exists.
        local tarball_downloaded="false"
        if [[ -f "$tarball_path" ]] && [[ "$force_download" != "true" ]]; then
            info "File already exists, skipping: $tarball_name (use --force to re-download)"
        else
            info "Downloading source archive: $tarball_name"
            CURRENT_DOWNLOAD="$tarball_path"
            download_to_file "$tarball_url" "$tarball_path"
            tarball_downloaded="true"
        fi

        if [ -f "$tarball_path" ] && [ -s "$tarball_path" ]; then
            if [[ "$tarball_downloaded" == "true" ]]; then
                info "Downloaded: $tarball_path"
            fi
            CURRENT_DOWNLOAD=""

            # Validate archive before extraction.
            if ! gzip -t "$tarball_path" 2>/dev/null; then
                error "Downloaded file is not a valid gzip archive."
            fi

            # Extract if tar is available.
            if command_exists tar; then
                local extract_dir="${output_dir}/nanvix-${tag_name#v}"
                if [[ -d "$extract_dir" ]] && [[ "$force_download" != "true" ]]; then
                    info "Directory already exists, skipping extraction: $extract_dir"
                else
                    info "Extracting archive..."
                    # Use --no-same-owner to prevent permission issues when running as root.
                    # Use --skip-old-files to avoid overwriting existing files unless forced.
                    if [[ "$force_download" == "true" ]]; then
                        tar --no-same-owner -xzf "$tarball_path" -C "$output_dir"
                    else
                        tar --no-same-owner --skip-old-files -xzf "$tarball_path" -C "$output_dir"
                    fi
                    info "Extracted to: $extract_dir"
                fi
            else
                info "tar not found. Archive saved but not extracted."
            fi
        else
            error "Failed to download source archive."
        fi
    else
        # Download all release assets.
        info "Downloading release assets..."

        local download_failed=0
        local url filename filepath
        while IFS= read -r url; do
            if [ -n "$url" ]; then
                # Validate URL is from expected GitHub releases domain.
                local expected_prefix="https://github.com/${GITHUB_REPO}/releases/download/"
                if [[ "$url" != "${expected_prefix}"* ]]; then
                    warn "Skipping URL from unexpected domain: $url"
                    continue
                fi

                # Sanitize filename: use -- to handle names starting with -, filter unsafe chars.
                filename=$(basename -- "$url" | tr -cd '[:alnum:]._-')

                # Validate that filename is not empty after sanitization.
                if [ -z "$filename" ]; then
                    warn "Could not determine filename for: $url"
                    download_failed=1
                    continue
                fi

                filepath="${output_dir}/${filename}"

                # Check if file already exists.
                if [[ -f "$filepath" ]] && [[ "$force_download" != "true" ]]; then
                    info "File already exists, skipping: $filename (use --force to re-download)"
                    continue
                fi

                CURRENT_DOWNLOAD="$filepath"

                info "Downloading: $filename"
                download_to_file "$url" "$filepath"

                # TODO: Add checksum verification when GitHub release includes checksums.

                if [ -f "$filepath" ] && [ -s "$filepath" ]; then
                    info "Downloaded: $filepath"
                    CURRENT_DOWNLOAD=""
                else
                    warn "Failed to download $filename"
                    CURRENT_DOWNLOAD=""
                    download_failed=1
                fi
            fi
        done <<< "$asset_urls"

        if (( download_failed == 1 )); then
            error "One or more downloads failed."
        fi
    fi

    info "Download complete."
    info "Files saved to: $output_dir"
}

# Run main function.
main "$@"
