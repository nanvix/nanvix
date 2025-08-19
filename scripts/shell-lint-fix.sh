#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

set -euo pipefail

# Check if shellcheck is available
if ! command -v shellcheck >/dev/null 2>&1; then
    echo "Error: shellcheck is not installed. Please install it first." >&2
    exit 1
fi

# Check if git is available
if ! command -v git >/dev/null 2>&1; then
    echo "Error: git is not available." >&2
    exit 1
fi

# Get all shell scripts in the repository
shell_files=$(git ls-files -- "*.sh")

if [ -z "$shell_files" ]; then
    echo "No shell scripts found in the repository."
    exit 0
fi

echo "Fixing shell script linting issues..."

# Generate diff from shellcheck and apply it
diff_output=$(shellcheck -f diff -S warning $shell_files 2>/dev/null || true)

if [ -n "$diff_output" ]; then
    if echo "$diff_output" | git apply --allow-empty 2>/dev/null; then
        echo "Shell script linting fixes applied successfully."
    else
        echo "Warning: Some fixes could not be applied automatically."
        echo "Please review and fix remaining issues manually."
        exit 1
    fi
else
    echo "No auto-fixable shell script issues found."
    exit 1
fi
