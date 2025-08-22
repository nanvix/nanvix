#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#
# Utility for creating a minor release of Nanvix.
#
# Run './create-minor-release.sh --help' for usage information.
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

#===================================================================================================
# Global Variables
#===================================================================================================

# Options
HELP_OPT_NAME="--help"
PUSH_OPT_NAME="--push"

# Directories.
REPO_ROOT_DIR=$(git rev-parse --show-toplevel) # /

# Cargo.toml file path.
CARGO_TOML_FILE_PATH="${REPO_ROOT_DIR}/Cargo.toml"

#===================================================================================================
# Functions
#===================================================================================================

print_help() {
    cat << EOF
Utility for creating a minor release of Nanvix.

Usage: $0 [OPTIONS]

Options
  --help   Print the help information.
  --push   Pushes the release commit to the remote origin.
EOF
}

#
# DESCRIPTION
#   Extracts the current version from the Cargo.toml file.
#
# ARGUMENTS
#   $1 - The path to the Cargo.toml file.
#
# RETURNS
#   The current version as a string in the format MAJOR.MINOR.PATCH.
#
# USAGE EXAMPLE
#   current_version=$(extract_current_version "path/to/Cargo.toml")
#
extract_current_version() {
    local cargo_toml="$1"

    # Check if the Cargo.toml file does not exist.
    if [[ ! -f "$cargo_toml" ]]; then
        print_error "Cargo.toml not found at $cargo_toml"
        exit 1
    fi

    local current_version
    current_version=$(sed -n 's/^[[:space:]]*version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$cargo_toml" | head -n1)

    # Check if version was not extracted successfully.
    if [[ -z "$current_version" ]]; then
        print_error "Could not extract version from Cargo.toml"
        exit 1
    fi

    echo "$current_version"
}

#
# DESCRIPTION
#   Increments the current version to create a new minor release.
#
# ARGUMENTS
#   $1 - The current version as a string in the format MAJOR.MINOR.PATCH.
#
# RETURNS
#   The new version as a string in the format MAJOR.MINOR.PATCH.
#
# USAGE EXAMPLE
#   new_version=$(increment_version "1.2.3")
#
increment_version() {
    local current_version="$1"
    local major minor patch

    # Split the version into major, minor, and patch components.
    major=$(echo "$current_version" | cut -d. -f1)
    minor=$(echo "$current_version" | cut -d. -f2)
    patch=$(echo "$current_version" | cut -d. -f3)

    # Validate version format.
    if [[ ! "$major" =~ ^[0-9]+$ ]] || [[ ! "$minor" =~ ^[0-9]+$ ]] || [[ ! "$patch" =~ ^[0-9]+$ ]]; then
        print_error "Invalid version format '$current_version'"
        exit 1
    fi

    # Increment patch version.
    local new_patch new_version
    new_patch=$((patch + 1))
    new_version="${major}.${minor}.${new_patch}"

    echo "$new_version"
}

#
# DESCRIPTION
#   Updates the version field in the Cargo.toml file.
#
# ARGUMENTS
#   $1 - Path to the Cargo.toml file.
#   $2 - New version string to set.
#
# RETURNS
#   None. Exits on error.
#
# USAGE EXAMPLE
#   update_cargo_toml "path/to/Cargo.toml" "1.2.4"
#
update_cargo_toml() {
    local cargo_toml="$1"
    local new_version="$2"

    # Create a backup of Cargo.toml
    cp "$cargo_toml" "${cargo_toml}.bak"

    # Update the version in Cargo.toml
    sed -i "s/^[[:space:]]*version[[:space:]]*=[[:space:]]*\"[0-9]\+\.[0-9]\+\.[0-9]\+\"/version = \"$new_version\"/" "$cargo_toml"

    # Check if version was not updated successfully.
    local updated_version
    updated_version=$(extract_current_version "$cargo_toml")
    if [[ "$updated_version" != "$new_version" ]]; then
        print_error "Failed to update version in Cargo.toml"

        # Restore backup.
        mv "${cargo_toml}.bak" "$cargo_toml"

        exit 1
    fi

    # Remove backup of Cargo.toml
    rm "${cargo_toml}.bak"

    print_success "Updated Cargo.toml with version: $new_version"
}

#
# DESCRIPTION
#   Checks if git user.name and user.email are configured.
#
# RETURNS
#   - 1 (false) if git is not configured.
#   - 0 (true) if git is configured.
#
# USAGE EXAMPLE
#   if git_is_configured; then
#       echo "Git is not configured.
#   else
#       echo "Git is configured."
#   fi
#
git_is_configured() {
    if [[ -z "$(git config --get user.name)" ]] || [[ -z "$(git config --get user.email)" ]]; then
        # Git is not configured.
        return 1
    else
        # Git is configured.
        return 0
    fi
}

#
# DESCRIPTION
#   Commits the version bump to the repository if there are changes.
#
# ARGUMENTS
#   $1 - Path to the Cargo.toml file.
#   $2 - New version string.
#
# USAGE EXAMPLE
#   git_commit "path/to/Cargo.toml" "1.2.4"
#
git_commit() {
    local cargo_toml="$1"
    local new_version="$2"

    # Check if there are changes to commit
    if ! git diff --quiet "$cargo_toml"; then
        print_message "Committing new version..."

        # Add the modified Cargo.toml
        git add "$cargo_toml"

        # Commit with the standardized message format.
        git commit --no-verify -m "Nanvix $new_version"

        print_success "Successfully created minor release: $new_version"
    else
        print_message "No changes to commit"
    fi
}

#
# DESCRIPTION
#   Pushes committed changes to the remote repository on the current branch.
#
# USAGE EXAMPLE
#   git_push
#
git_push() {
    print_message "Pushing changes to remote..."

    # Check if git is not configured.
    if ! git_is_configured; then
        print_error "Git is not configured. Please set user.name and user.email."
        exit 1
    fi

    # Get current branch name
    local current_branch
    current_branch=$(git rev-parse --abbrev-ref HEAD)
    print_message "Pushing to branch: $current_branch"

    # Push to current branch
    git push --no-verify -u origin "$current_branch"
}


#
# DESCRIPTION
#   Checks if the given commit is a merge commit.
#
# ARGUMENTS
#   $1 - Commit hash to check.
#
# RETURNS
#   - 1 (false) if the commit is not a merge commit.
#   - 0 (true) if the commit is a merge commit.
#
# USAGE EXAMPLE
#   if is_merge_commit "<commit_hash>"; then
#       echo "This is a merge commit."
#   else
#       echo "This is not a merge commit."
#   fi
#
is_merge_commit() {

    local commit="$1"
    # Check if the commit has more than one parent
    if git rev-parse -q --verify "${commit}^2" >/dev/null; then
        # It is a merge commit.
        return 0
    else
        # It is not a merge commit.
        return 1
    fi
}

#
# DESCRIPTION
#   Checks if the current branch is the default branch.
#
# RETURNS
#   - 1 (false) if the current branch is not the default branch.
#   - 0 (true) if the current branch is the default branch.
#
# EXAMPLE USAGE
#   if is_default_branch; then
#       echo "You are on the default branch."
#   else
#       echo "You are not on the default branch."
#   fi
#
is_default_branch() {
    local current_branch
    current_branch=$(git rev-parse --abbrev-ref HEAD)

    # Check if remote 'origin' does not exist.
    if ! git remote | grep -q '^origin$'; then
        print_error "Remote 'origin' does not exist."
        exit 1
    fi
    print_message "Current branch: $current_branch"

    # Get the default branch and ensure it exists.
    local default_branch
    default_branch=$(git remote show origin 2>/dev/null | grep 'HEAD branch' | awk '{print $NF}')
    if [[ -z "$default_branch" ]]; then
        print_error "Could not determine default branch from remote 'origin'."
        exit 1
    fi
    print_message "Default branch: $default_branch"

    if [[ "$current_branch" == "$default_branch" ]]; then
        # Current branch is the default branch.
        return 0
    else
        # Current branch is not the default branch.
        return 1
    fi
}

#===================================================================================================
# Main Script
#===================================================================================================

main() {
    local push_flag

    push_flag=false
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            "${HELP_OPT_NAME}")
                print_help
                exit 0
                ;;
            "${PUSH_OPT_NAME}")
                push_flag=true
                shift
                ;;
            *)
                print_error "Unknown option '$1'"
                print_help
                exit 1
                ;;
        esac
    done

    # Check if the current branch is not the default branch.
    if ! is_default_branch; then
        print_error "This script must be run on the default branch."
        exit 1
    fi

    # Check if the HEAD commit is not a merge commit.
    if ! is_merge_commit "$(git rev-parse HEAD)"; then
        print_error "The HEAD commit is not a merge commit."
        exit 1
    fi

    print_message "Creating minor release..."

    local current_version new_version
    current_version=$(extract_current_version "$CARGO_TOML_FILE_PATH")
    print_message "Current version: $current_version"
    new_version=$(increment_version "$current_version")
    print_message "Incrementing version: $current_version -> $new_version"
    update_cargo_toml "$CARGO_TOML_FILE_PATH" "$new_version"
    git_commit "$CARGO_TOML_FILE_PATH" "$new_version"

    if $push_flag; then
        git_push
    else
        print_message "Changes committed locally."
    fi
}

# Call main function with all arguments
main "$@"
