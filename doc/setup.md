# Setting Up Your Development Environment

This guide will help you set up your development environment to build and run Nanvix.

## Table of Contents

- [Linux Setup](#linux-setup)
  - [1. Check Your System and Permissions](#1-check-your-system-and-permissions)
  - [2. Clone This Repository](#2-clone-this-repository)
  - [3. Install Dependencies for Development Tools](#3-install-dependencies-for-development-tools)
  - [4. Setup KVM](#4-setup-kvm)
  - [5. Setup Docker (Optional)](#5-setup-docker-optional)
  - [6. Setup SCCACHE (Optional)](#6-setup-sccache-optional)
  - [7. Set Up GDB Debugging (Optional)](#7-set-up-gdb-debugging-optional)
- [Setting Up Development Tools for the First Time](#setting-up-development-tools-for-the-first-time)
  - [Option 1: Build Development Tools Locally (Preferred Method)](#option-1-build-development-tools-locally-preferred-method)
  - [Option 2: Use a Pre-Built Docker Image](#option-2-use-a-pre-built-docker-image)
  - [Option 3: Build a Docker Image](#option-3-build-a-docker-image)
- [Updating Your Development Tools](#updating-your-development-tools)
  - [Verus (Formal Verification)](#verus-formal-verification)
- [Windows Setup](#windows-setup)
  - [1. Enable Windows Hypervisor Platform](#1-enable-windows-hypervisor-platform)
  - [2. Enable Developer Mode](#2-enable-developer-mode)
  - [3. Install Docker Desktop](#3-install-docker-desktop)
  - [4. Install the Rust Toolchain](#4-install-the-rust-toolchain)
  - [5. Clone This Repository](#5-clone-this-repository)
  - [6. Pull the Docker Toolchain Image](#6-pull-the-docker-toolchain-image)
- [Setup Your IDE (Optional)](#setup-your-ide-optional)
  - [Visual Studio Code](#visual-studio-code)

---

## Linux Setup

### 1. Check Your System and Permissions

- Ensure you are running Ubuntu 24.04
- Make sure you have sudo privileges

### 2. Clone This Repository

```bash
export WORKDIR=$HOME/nanvix                     # Set the directory for the source tree.
mkdir -p $WORKDIR && cd $WORKDIR                # Create the workspace and navigate to it.
git clone https://github.com/nanvix/nanvix.git  # Clone the repository.
cd nanvix                                       # Navigate to the Nanvix source tree.
```

### 3. Install Dependencies for Development Tools

```bash
# Ensure you are in the project's root directory.
cat ./scripts/setup/ubuntu.sh      # Review the installation script.
sudo -E ./scripts/setup/ubuntu.sh  # Run the script to install dependencies.
```

### 4. Setup KVM

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

### 5. Setup Docker (Optional)

```bash
# Install Docker.
curl -fsSL https://get.docker.com | sh

# Add user to Docker group.
sudo usermod -aG docker $USER

# Re-login and check if groups changed.
newgrp docker
groups
```

### 6. Setup SCCACHE (Optional)

Install `sccache` to enable caching of compilation artifacts and can significantly speed up builds.

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

### 7. Set Up GDB Debugging (Optional)

The repository includes a `.gdbinit` file that automatically configures GDB for debugging. By
default, GDB refuses to auto-load `.gdbinit` files from arbitrary directories. To allow it for
this project, add the repository path to your GDB auto-load safe-path:

```bash
echo "add-auto-load-safe-path $(pwd)/.gdbinit" >> ~/.gdbinit
```

After this, launching `gdb-multiarch` from the project root will automatically pick up the
project's `.gdbinit`.

For full debugging instructions, see [doc/gdb.md](gdb.md).

---

## Setting Up Development Tools for the First Time

> ⚠️ **Note:** This process may take some time to complete.

Choose one of the following methods to set up the development tools for Nanvix.

> **Tip:** If you plan to actively contribute to Nanvix, building the tools locally (Option 1) is
> recommended. It provides the fastest edit-build-test cycle and full access to debugging and
> profiling tools.

### Option 1: Build Development Tools Locally (Preferred Method)

To build the tools directly on your system, follow these steps.

#### Step 1: Install the Rust Toolchain

```bash
# Install Rust.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Restart the shell to update the PATH.
exec $SHELL
```

#### Step 2: Build Cross Compiler Toolchain

```bash
# Ensure you are in the project's root directory.
./z setup --toolchain-dir $HOME/toolchain    # Build the cross compiler toolchain and place it in $HOME/toolchain.
ln -T -s $HOME/toolchain toolchain           # Create symbolic link for toolchain for convenience.
```

> **Note:** The toolchain directory must be located outside the repository root.
> Use `./z setup --toolchain-dir <path>` to specify a valid location.

#### Step 3: Build QEMU (Optional)

To run Nanvix on QEMU, you need to build it:

```bash
# Ensure you are in the project's root directory.
export TARGET=x86                # Set the target architecture (e.g., x86).
./scripts/setup/qemu.sh $TARGET  # Build QEMU.
```

### Option 2: Use a Pre-Built Docker Image

This is the easiest and fastest way to get started:

```bash
# Ensure you are in the project's root directory.
./z setup --with-docker
```

### Option 3: Build a Docker Image

To build a Docker image with the required tools:

```bash
# Ensure you are in the project's root directory.
./scripts/setup/docker.sh
```

## Updating Your Development Tools

When a new major release of Nanvix is available, you must update your development tools to ensure
compatibility. Follow these steps to update your environment:

1. **Update system dependencies**: [Re-install dependencies for development tools](#3-install-dependencies-for-development-tools).
2. **Rebuild development tools**: [Re-build the development tools](#option-1-build-development-tools-locally-preferred-method).

> **Note:** This update process is only required for major releases, not for minor updates or patches.

### Verus (Formal Verification)

Verus is installed automatically when you run `make verify`. The expected version is pinned in
`build/verus-version` and installed to `toolchain/verus`. No manual setup is required.

#### Using a Custom Verus Installation

If you need a specific Verus version (e.g., to test a new feature or a nightly commit), you can
build Verus from source and point `VERUS_EXECUTABLE_DIR` at the resulting binaries:

```bash
# 1. Clone and build Verus (see https://github.com/verus-lang/verus/blob/main/INSTALL.md).
git clone https://github.com/verus-lang/verus.git ~/verus-src
cd ~/verus-src/source
# Follow the Verus build instructions to produce binaries.

# 2. Point VERUS_EXECUTABLE_DIR at the directory containing the verus binary.
make verify VERUS_EXECUTABLE_DIR=~/verus-src/source/target-verus/release
```

> **Note:** When `VERUS_EXECUTABLE_DIR` is overridden, the automatic download is skipped and
> the build validates that a `verus` binary exists at the given path (the directory is used
> read-only; nothing is written to it). You are responsible for ensuring the Verus version is
> compatible with the `vstd` crate version pinned in `Cargo.toml`.

## Windows Setup

Nanvix currently supports Windows 11 on the host side. On Windows 11, guest components (kernel,
user binaries) are cross-compiled inside Docker while the UserVM is built natively using the
Windows Hypervisor Platform (WHP) backend. The `z.ps1` PowerShell script mirrors the Linux `z`
utility interface.

### 1. Enable Windows Hypervisor Platform

The Windows Hypervisor Platform (WHP) feature is required to run the UserVM natively. This step
requires an elevated PowerShell session.

```powershell
# Run in an elevated PowerShell prompt.
Enable-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform -All
```

Restart your machine after enabling the feature.

### 2. Enable Developer Mode

This repository uses symbolic links. On Windows, Git needs Developer Mode enabled to create native
symlinks during clone. Without it, symlinks are checked out as small text files and tools that
expect real files may fail.

1. Open **Settings > Privacy & Security > For developers**.
2. Turn on **Developer Mode**.

Using the Settings app may prompt for approval, but it does not require you to manually open an
elevated PowerShell session.

Alternatively, enable it from PowerShell. This command does require an elevated PowerShell
session:

```powershell
reg add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock" /t REG_DWORD /f /v AllowDevelopmentWithoutDevLicense /d 1
```

### 3. Install Docker Desktop

Install [Docker Desktop for Windows](https://www.docker.com/products/docker-desktop/) and ensure
it is configured to use **Linux containers** (the default).

Verify Docker is working:

```powershell
docker info
```

### 4. Install the Rust Toolchain

Install Rust via [rustup](https://rustup.rs):

```powershell
# Download and run the rustup installer.
winget install Rustlang.Rustup
```

After installation, restart your terminal and verify:

```powershell
rustc --version
cargo --version
```

### 5. Clone This Repository

```powershell
git clone -c core.symlinks=true https://github.com/nanvix/nanvix.git
cd nanvix
```

> **Note:** The `-c core.symlinks=true` flag ensures Git creates native symlinks instead of text
> stub files. This requires [Developer Mode](#2-enable-developer-mode) to be enabled. If you
> already cloned without this flag, prefer re-cloning after enabling Developer Mode so your
> existing worktree is not overwritten.
>
> **Note:** Cloning the repository does not require an elevated PowerShell session once Developer
> Mode is enabled.

### 6. Pull the Docker Toolchain Image

```powershell
.\z.ps1 setup
```

This pulls the pre-built minimal Docker toolchain image required for cross-compiling guest
components. To use the full (non-minimal) image instead:

```powershell
.\z.ps1 setup --with-docker
```

---

## Setup Your IDE (Optional)

Choose one of the following options to set up your IDE for Nanvix development.

### Visual Studio Code

Use the host-specific settings template below. The Linux template invokes `./z`, while the
Windows template invokes `./z.bat` and also routes Rust Analyzer build-script discovery through the
Windows Docker workflow. Without the Windows override, Rust Analyzer falls back to native `cargo`
for build-script metadata and guest crates such as `kernel` fail because `gcc`/`ar` are not
available on the host `PATH`.

**Linux:**

```bash
mkdir -p .vscode && cd .vscode
ln -s ../scripts/setup/vscode/settings-linux.json settings.json
```

**Windows (PowerShell):**

```powershell
New-Item -ItemType Directory -Path .vscode -Force
Copy-Item scripts\setup\vscode\settings-windows.json .vscode\settings.json
```
