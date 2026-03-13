#!/bin/bash
set -e

echo "=== Step 1: Install system dependencies ==="
cd ~/nanvix/nanvix
sudo -E ./scripts/setup/ubuntu.sh

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
cd ~/nanvix/nanvix
./z setup --toolchain-dir $HOME/toolchain
ln -T -sf $HOME/toolchain toolchain

echo "=== Step 4: Build Nanvix ==="
cd ~/nanvix/nanvix
./z build -- all

echo "=== Step 5: Quick test ==="
cd ~/nanvix/nanvix
./bin/nanvixd.elf -console-file /dev/stdout -- ./bin/hello-rust-nostd.elf

echo ""
echo "=== Done! Nanvix is built and tested ==="
echo "Binaries are in ~/nanvix/nanvix/bin/"
echo ""
echo "To re-run the test:"
echo "  cd ~/nanvix/nanvix"
echo "  ./bin/nanvixd.elf -console-file /dev/stdout -- ./bin/hello-rust-nostd.elf"
