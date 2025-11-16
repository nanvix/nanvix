# Setting Up Your Development Environment

This guide will help you set up your development environment to build and run Nanvix.

## Table of Contents

- [Setting Up Your System](#setting-up-your-system)
  - [1. Check Your System and Permissions](#1-check-your-system-and-permissions)
  - [2. Clone This Repository](#2-clone-this-repository)
  - [3. Install Dependencies for Development Tools](#3-install-dependencies-for-development-tools)
  - [4. Setup KVM](#4-setup-kvm)
  - [5. Setup Docker (Optional)](#5-setup-docker-optional)
  - [6. Setup SCCACHE (Optional)](#6-setup-sccache-optional)
- [Setting Up Development Tools for the First Time](#setting-up-development-tools-for-the-first-time)
  - [Option 1: Build Development Tools Locally (Preferred Method)](#option-1-build-development-tools-locally-preferred-method)
  - [Option 2: Use a Pre-Built Docker Image](#option-2-use-a-pre-built-docker-image)
  - [Option 3: Build a Docker Image](#option-3-build-a-docker-image)
- [Updating Your Development Tools](#updating-your-development-tools)
- [Setup Your IDE (Optional)](#setup-your-ide-optional)
  - [Visual Studio Code](#visual-studio-code)

---

## Setting Up Your System

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

---

## Setting Up Development Tools for the First Time

> ⚠️ **Note:** This process may take some time to complete.

Choose one of the following methods to set up the development tools for Nanvix.

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
ln -s $HOME/toolchain toolchain              # Create symbolic link for toolchain for convenience.
```

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
docker build --no-cache -t nanvix/toolchain ./scripts/setup/
```

## Updating Your Development Tools

When a new major release of Nanvix is available, you must update your development tools to ensure
compatibility. Follow these steps to update your environment:

1. **Update system dependencies**: [Re-install dependencies for development tools](#3-install-dependencies-for-development-tools).
2. **Rebuild development tools**: [Re-build the development tools](#option-1-build-development-tools-locally-preferred-method).

> **Note:** This update process is only required for major releases, not for minor updates or patches.

## Setup Your IDE (Optional)

Choose one of the following options to set up your IDE for Nanvix development.

### Visual Studio Code

```bash
mkdir -p .vscode && cd .vscode
ln -s ../scripts/setup/vscode/settings.json settings.json
```
