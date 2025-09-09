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

# Get all shell scripts in the repository (store in array to preserve filenames safely).
mapfile -t shell_files < <(git ls-files -- '*.sh')

if [ "${#shell_files[@]}" -eq 0 ]; then
    echo "No shell scripts found in the repository."
    exit 0
fi

echo "Fixing shell script linting issues..."

# First pass: generate diff for auto-fixable issues.
diff_output=$(shellcheck -f diff -S warning "${shell_files[@]}" 2>/dev/null || true)

if [ -n "$diff_output" ]; then
    if echo "$diff_output" | git apply --allow-empty 2>/dev/null; then
        echo "Applied auto-fixable shell script linting changes."
    else
        echo "Error: failed to apply some auto-fixable shell script changes." >&2
        echo "Please review shellcheck output manually." >&2
        exit 1
    fi
else
    echo "No auto-fixable shell script issues found."
fi

# Second pass: run shellcheck normally to detect any remaining issues (including non-fixable).
if shellcheck -S warning "${shell_files[@]}" >/dev/null 2>&1; then
    if [ -n "$diff_output" ]; then
        echo "All shell script issues resolved after auto-fixes."
    else
        echo "No shell script issues found."
    fi
    exit 0
else
    if [ -n "$diff_output" ]; then
        echo "Error: remaining shell script issues after applying fixes." >&2
    else
        echo "Error: shell script issues detected (none auto-fixable)." >&2
    fi
    # Print re-runnable command with proper quoting.
    printf 'Run: shellcheck -S warning ' >&2
    printf '%q ' "${shell_files[@]}" >&2
    printf '\n' >&2
    exit 1
fi
