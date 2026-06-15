#!/usr/bin/env bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#
# Convenience wrapper to build the `nanvixd-vmm` crate directly with cargo, for
# quick iteration. The canonical, fully-integrated build goes through the Nanvix
# build system instead:
#
#   ./z build -- all-nanvixd-vmm     # from the Nanvix repo root
#
# The crate's OpenVMM sources are git dependencies pinned to a revision in
# Cargo.toml (no local OpenVMM checkout needed for sources), and the required
# `[patch.crates-io]` entries live in the workspace root. Two things a bare
# `cargo build` does not provide on its own:
#
#   - MEMORY_SIZE_BYTES : consumed by the Nanvix `config` crate's build script
#                         (normally exported by the Nanvix Makefile). Defaults
#                         to 128 MiB here.
#   - PROTOC            : the Protocol Buffers compiler, required by transitive
#                         OpenVMM build dependencies (e.g. `tdisp_proto` via
#                         `prost-build`). Resolved from $PROTOC, then a system
#                         `protoc`, then OpenVMM's restored copy.
#
# Any extra arguments are forwarded to `cargo build` (e.g. `--release`).
#
# Usage:
#   ./build.sh                # debug build
#   ./build.sh --release      # release build
#

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NANVIX_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"

# Memory size (MiB) -> bytes, mirroring the Nanvix Makefile default.
MEMORY_SIZE_MB="${MEMORY_SIZE:-128}"
export MEMORY_SIZE_BYTES="${MEMORY_SIZE_BYTES:-$((MEMORY_SIZE_MB * 1048576))}"

# Resolve protoc: explicit PROTOC, then system protoc, then OpenVMM's restored copy.
if [[ -z "${PROTOC:-}" ]]; then
    if command -v protoc &>/dev/null; then
        PROTOC="$(command -v protoc)"
        export PROTOC
    else
        CANDIDATE="${NANVIX_ROOT}/../OpenVMM/.packages/Google.Protobuf.Tools/tools/protoc"
        if [[ -x "${CANDIDATE}" ]]; then
            export PROTOC="${CANDIDATE}"
        else
            echo "[ERROR] protoc not found. Install it (e.g. 'apt-get install protobuf-compiler')," >&2
            echo "        set PROTOC, or restore OpenVMM's packages." >&2
            exit 1
        fi
    fi
fi

echo "[nanvixd-vmm] MEMORY_SIZE_BYTES=${MEMORY_SIZE_BYTES} PROTOC=${PROTOC}"
exec cargo build --manifest-path "${NANVIX_ROOT}/Cargo.toml" -p nanvixd-vmm "$@"
