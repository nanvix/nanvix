# Setting Up Your Development Environment

> ℹ️ **Note:** Some instructions in this document assume that you have superuser privileges on your system.
> ℹ️ **Note:** Ensure that your system supports KVM (Kernel-based Virtual Machine), and that it is enabled.

This guide will help you set up your development environment to build and run Nanvix. Here's a quick overview of the steps:

1. Clone the Nanvix repository.
2. Install dependencies for development tools.
3. Set up development tools.

## Table of Contents

- [Clone the Repository](#clone-the-repository)
- [Installing Dependencies for Development Tools](#installing-dependencies-for-development-tools)
  - [For Ubuntu 24.04](#for-ubuntu-2404)
  - [For Arch Linux](#for-arch-linux)
- [Setting Up Development Tools](#setting-up-development-tools)
  - [Option 1: Build Development Tools Locally (Recommended)](#option-1-build-development-tools-locally-recommended)
  - [Option 2: Use a Pre-Built Docker Image](#option-2-use-a-pre-built-docker-image)
  - [Option 3: Build a Docker Image](#option-3-build-a-docker-image)

---

## Clone the Repository

Start by cloning the Nanvix repository:

```bash
export WORKDIR=$HOME/nanvix                     # Set the directory for the source tree.
mkdir -p $WORKDIR && cd $WORKDIR                # Create the workspace and navigate to it.
git clone https://github.com/nanvix/nanvix.git  # Clone the repository.
cd nanvix                                       # Navigate to the Nanvix source tree.
```

---

## Installing Dependencies for Development Tools

### For Ubuntu 24.04

To install dependencies on Ubuntu 24.04:

```bash
# Ensure you are in the project's root directory.
cat ./scripts/setup/ubuntu.sh              # Review the installation script.
sudo -E ./scripts/setup/ubuntu.sh --extra  # Run the script to install dependencies.
```

### For Arch Linux

To install dependencies on Arch Linux:

```bash
# Ensure you are in the project's root directory.
cat ./scripts/setup/arch.sh              # Review the installation script.
sudo -E ./scripts/setup/arch.sh --extra  # Run the script to install dependencies.
```

---

## Setting Up Development Tools

Choose one of the following methods to set up the development tools for Nanvix.

### Option 1: Build Development Tools Locally (Recommended)

If you prefer to build the tools directly on your system, follow these steps:

#### Step 1: Install the Rust Toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
exec $SHELL # Restart the shell to update the PATH.
rustup component add rust-src
rustup target add wasm32-wasip1
```

#### Step 2: Build a C/C++ Cross Compiler Toolchain

> ⚠️ **Note:** This process may take some time to complete.

```bash
# Ensure you are in the project's root directory.
export TOOLCHAIN_DIR=$PWD/toolchain          # Set the directory for the toolchain.
./scripts/setup/toolchain.sh $TOOLCHAIN_DIR  # Build GCC, Binutils, and GDB.
```

#### Step 3: Build the JavaScript to WebAssembly Toolchain (Optional)

> ⚠️ **Note:** This step  may take some time to complete.

If you need the JavaScript to WebAssembly (Javy) toolchain:

```bash
# Run these commands in a separate directory.
git clone https://github.com/nanvix/javy

# Building javy requires the `clang` package.
cargo build -p javy-plugin --target=wasm32-wasip1 -r
cargo install --path crates/cli
```

#### Step 4: Build QEMU (Optional)

> ⚠️ **Note:** This step may take some time to complete.

If you need a version of QEMU known to work with Nanvix:

```bash
# Ensure you are in the project's root directory.
export TARGET=x86                # Set the target architecture (e.g., x86).
./scripts/setup/qemu.sh $TARGET  # Build QEMU.
```

### Option 2: Use a Pre-Built Docker Image

This is the easiest and fastest way to get started:

```bash
docker pull nanvix/toolchain
```

### Option 3: Build a Docker Image

> ⚠️ **Note:** This process may take some time to complete.

To build a Docker image with the required tools:

```bash
# Ensure you are in the project's root directory.
docker build --no-cache -t nanvix/toolchain ./scripts/setup/
```
