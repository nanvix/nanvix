#!/bin/bash
# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Install Docker and containerd for Nanvix shim integration testing.
# Run from WSL (Ubuntu 24.04): ./scripts/install-containerd.sh

set -euo pipefail

echo "=== Installing Docker (includes containerd) ==="
if command -v docker &>/dev/null; then
    echo "Docker already installed: $(docker --version)"
else
    curl -fsSL https://get.docker.com | sudo sh
    echo "Docker installed: $(docker --version)"
fi

echo "=== Configuring user ==="
sudo usermod -aG docker "$(whoami)" 2>/dev/null || true
sudo usermod -aG kvm "$(whoami)" 2>/dev/null || true

echo "=== Starting dockerd ==="
if ! pgrep -x dockerd &>/dev/null; then
    # shellcheck disable=SC2024
    # Redirection intentionally runs as current user; /tmp is world-writable.
    sudo dockerd --iptables=false &>/tmp/dockerd.log &
    sleep 3
fi

echo "=== Verifying ==="
sudo docker info 2>/dev/null | head -3 || true
containerd --version
sudo ctr version | grep -E "Version|Revision" || true

echo ""
echo "=== Done ==="
echo "Docker and containerd are ready for integration testing."
