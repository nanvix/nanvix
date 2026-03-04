#!/usr/bin/env bash
# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Retry wrapper for Docker-based build commands.
#
# Usage: docker-retry.sh <context-label> <command> [args...]
#
# Retries the given command up to 3 times with exponential backoff (30s base).
# On exhaustion, emits a GitHub Actions ::error:: annotation that includes
# <context-label> for easier triage, then exits 1.

set -euo pipefail

if [[ $# -lt 2 ]]; then
    echo "Usage: $0 <context-label> <command> [args...]" >&2
    exit 2
fi

context_label="$1"
shift

max_retries=3
for attempt in $(seq 1 "$max_retries"); do
    if "$@"; then
        exit 0
    fi
    if [[ "$attempt" -eq "$max_retries" ]]; then
        echo "::error::Docker build failed after $max_retries attempts ($context_label)."
        exit 1
    fi
    echo "::warning::Docker build attempt $attempt/$max_retries failed ($context_label). Retrying in $((attempt * 30))s..."
    sleep $((attempt * 30))
done
