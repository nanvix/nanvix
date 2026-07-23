# Setting Up Your Development Environment (Linux)

This guide will help you set up your development environment to build and run Nanvix on Linux.

## Table of Contents

- [1. Check Your System and Permissions](#1-check-your-system-and-permissions)
- [2. Clone This Repository](#2-clone-this-repository)
- [3. Install Dependencies for Development Tools](#3-install-dependencies-for-development-tools)
- [4. Setup KVM](#4-setup-kvm)
- [5. Setup SCCACHE (Optional)](#5-setup-sccache-optional)
- [6. Set Up GDB Debugging (Optional)](#6-set-up-gdb-debugging-optional)
- [7. Setting Up Development Tools for the First Time](#7-setting-up-development-tools-for-the-first-time)
  - [Step 1: Install the Rust Toolchain](#step-1-install-the-rust-toolchain)
  - [Step 2: Set Up Development Tools](#step-2-set-up-development-tools)
- [8. Updating Your Development Tools](#8-updating-your-development-tools)
- [9. Verus (Formal Verification)](#9-verus-formal-verification)
  - [Using a Custom Verus Installation](#using-a-custom-verus-installation)
- [10. Setup Your IDE (Optional)](#10-setup-your-ide-optional)
  - [Visual Studio Code](#visual-studio-code)

---

## 1. Check Your System and Permissions

- Ensure you are running Ubuntu 24.04
- Make sure you have sudo privileges

## 2. Clone This Repository

```bash
export WORKDIR=$HOME/nanvix                     # Set the directory for the source tree.
mkdir -p $WORKDIR && cd $WORKDIR                # Create the workspace and navigate to it.
git clone https://github.com/nanvix/nanvix.git  # Clone the repository.
cd nanvix                                       # Navigate to the Nanvix source tree.
```

## 3. Install Dependencies for Development Tools

```bash
# Ensure you are in the project's root directory.
cat ./scripts/setup/ubuntu-core.sh      # Review the installation script.
sudo -E ./scripts/setup/ubuntu-core.sh  # Run the script to install core dependencies.
```

## 4. Setup KVM

```bash
# Check if KVM is enabled.
sudo kvm-ok
sudo lsmod | grep kvm

# Add user to KVM group.
sudo usermod -aG kvm $USER

# Re-login and check if groups changed.
newgrp kvm
groups
```

## 5. Setup SCCACHE (Optional)

Install `sccache` to enable caching of compilation artifacts, which can significantly speed up
builds.

```bash
# Set the sccache version and filename.
SCCACHE_VERSION="v0.10.0"
SCCACHE_FILENAME="sccache-${SCCACHE_VERSION}-x86_64-unknown-linux-musl"
SCCACHE_TAR="${SCCACHE_FILENAME}.tar.gz"
SCCACHE_INSTALL_PATH="/usr/local/bin/sccache"

# Get pre-compiled binaries for sccache.
wget "https://github.com/mozilla/sccache/releases/download/${SCCACHE_VERSION}/${SCCACHE_TAR}"

# Extract and install sccache.
tar -xzf "${SCCACHE_TAR}"
sudo mv "${SCCACHE_FILENAME}/sccache" ${SCCACHE_INSTALL_PATH}

# Clean up the downloaded files.
rm -rf "${SCCACHE_TAR}" "${SCCACHE_FILENAME}"

# Add sccache to PATH (if not already in PATH)
echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

## 6. Set Up GDB Debugging (Optional)

The repository includes a `.gdbinit` file that automatically configures GDB for debugging. By
default, GDB refuses to auto-load `.gdbinit` files from arbitrary directories. To allow it for
this project, add the repository path to your GDB auto-load safe-path:

```bash
echo "add-auto-load-safe-path $(pwd)/.gdbinit" >> ~/.gdbinit
```

After this, launching `gdb-multiarch` from the project root will automatically pick up the
project's `.gdbinit`.

For full debugging instructions with GDB, see [doc/gdb.md](gdb.md).

---

## 7. Setting Up Development Tools for the First Time

> ⚠️ **Note:** This process may take some time to complete.

### Step 1: Install the Rust Toolchain

```bash
# Install Rust.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Restart the shell to update the PATH.
exec $SHELL
```

### Step 2: Set Up Development Tools

```bash
# Ensure you are in the project's root directory.
./z setup
```

## 8. Updating Your Development Tools

When a new major release of Nanvix is available, you must update your development tools to ensure
compatibility. Follow these steps to update your environment:

1. **Update system dependencies**: [Re-install dependencies for development tools](#3-install-dependencies-for-development-tools).
2. **Rebuild development tools**: [Re-build the development tools](#7-setting-up-development-tools-for-the-first-time).

> **Note:** This update process is only required for major releases, not for minor updates or patches.

## 9. Verus (Formal Verification)

Verification requires `VERUS_EXECUTABLE_DIR` to point to the directory containing the `verus`
binary. When unset, `make verify` is a no-op. The expected version is pinned in
`build/verus-version`.

The easiest way to install Verus is via the setup command:

```bash
./z setup --verus
./z build -- verify
```

This installs the pinned Verus release to `~/verus`. You can also invoke the setup script
directly to choose a custom install directory:

```bash
# Download the pinned release and run verification.
python3 scripts/setup/verus.py ~/toolchain/verus
./z build -- verify VERUS_EXECUTABLE_DIR=~/toolchain/verus
```

### Using a Custom Verus Installation

If you need a specific Verus version (e.g., to test a new feature or a nightly commit), you can
build Verus from source and point `VERUS_EXECUTABLE_DIR` at the resulting binaries:

```bash
# 1. Clone and build Verus (see https://github.com/verus-lang/verus/blob/main/INSTALL.md).
git clone https://github.com/verus-lang/verus.git ~/verus-src
cd ~/verus-src/source
# Follow the Verus build instructions to produce binaries.

# 2. From the Nanvix repo root, run verification pointing at the verus binary directory.
cd /path/to/nanvix
./z build -- verify VERUS_EXECUTABLE_DIR=~/verus-src/source/target-verus/release
```

> **Note:** The build validates that a `verus` binary exists at the given path (the directory
> is used read-only; nothing is written to it). You are responsible for ensuring the Verus
> version is compatible with the `vstd` crate version pinned in `Cargo.toml`.

---

## 10. Setup Your IDE (Optional)

Choose one of the following options to set up your IDE for Nanvix development.

### Visual Studio Code

Use the host-specific settings template below. The Linux template invokes `./z`.

```bash
mkdir -p .vscode && cd .vscode
ln -s ../scripts/setup/vscode/settings-linux.json settings.json
```
