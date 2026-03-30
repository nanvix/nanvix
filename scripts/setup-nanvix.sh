#!/bin/bash
# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

set -e

NANVIX_DIR="${NANVIX_DIR:-$HOME/nanvix}"

echo "Using NANVIX_DIR=${NANVIX_DIR}"

echo "=== Step 1: Install system dependencies ==="
cd "${NANVIX_DIR}"
sudo -E ./scripts/setup/ubuntu-core.sh
sudo -E ./scripts/setup/ubuntu-sdk.sh

echo "=== Step 2: Install sccache ==="
SCCACHE_VERSION="v0.10.0"
SCCACHE_FILENAME="sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl"
SCCACHE_TAR="${SCCACHE_FILENAME}.tar.gz"
cd /tmp
wget -q "https://github.com/mozilla/sccache/releases/download/${SCCACHE_VERSION}/${SCCACHE_TAR}"
tar -xzf "${SCCACHE_TAR}"
sudo mv "${SCCACHE_FILENAME}/sccache" /usr/local/bin/sccache
rm -rf "${SCCACHE_TAR}" "${SCCACHE_FILENAME}"
echo "sccache installed: $(sccache --version)"
export RUSTC_WRAPPER=sccache

echo "=== Step 3: Build cross-compiler toolchain ==="
cd "${NANVIX_DIR}"
./z setup --nanvix-sdk --toolchain-dir $HOME/toolchain
ln -T -sf $HOME/toolchain toolchain

echo "=== Step 4: Build Nanvix ==="
cd "${NANVIX_DIR}"
./z build -- all

echo "=== Step 5: Quick test ==="
cd "${NANVIX_DIR}"
./bin/nanvixd.elf -console-file /dev/stdout -- ./bin/hello-rust-nostd.elf

echo ""
echo "=== Done! Nanvix is built and tested ==="
echo "Binaries are in ${NANVIX_DIR}/bin/"
echo ""
echo "To re-run the test:"
echo "  cd ${NANVIX_DIR}"
echo "  ./bin/nanvixd.elf -console-file /dev/stdout -- ./bin/hello-rust-nostd.elf"
