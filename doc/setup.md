# Setting Up Your Development Environment

> ℹ️ Some instructions in this document assume that you have superuser privileges on the system.

This document guides you through setting up your development environment to
build and run Nanvix. In summary, you will:

1. Clone the Nanvix repository.
2. Install dependencies for building development tools.
3. Install the Rust toolchain for building Nanvix kernel and system components.
4. Get a C/C++ cross-compiler toolchain for building some user-space applications and libraries.

## Table of Contents

- [Clone this Repository](#clone-this-repository)
- [Getting Dependencies](#getting-dependencies)
  - [Ubuntu 22.04](#ubuntu-2204)
  - [Arch Linux](#arch-linux)
- [Getting the Rust Toolchain](#getting-the-rust-toolchain)
- [Getting a C/C++ Cross Compiler Toolchain](#getting-a-cc-cross-compiler-toolchain)
  - [Building from the Sources (Recommended)](#building-from-the-sources-recommended)
  - [Building a Docker Image from the Sources](#building-a-docker-image-from-the-sources)
  - [Getting a Pre-Built Docker Image](#getting-a-pre-built-docker-image)
- [Building JaveScript to WebAssembly Toolchain (Optional)](#building-javescript-to-webassembly-toolchain-optional)
- [Building QEMU (Optional)](#building-qemu-optional)

## Clone this Repository

```bash
export WORKDIR=$HOME/nanvix                     # Set this to the directory where the source tree will be cloned.
mkdir -p $WORKDIR && cd $WORKDIR                # Create workspace and switch to it.
git clone https://github.com/nanvix/nanvix.git  # Clone repository.
cd nanvix                                       # Switch to Nanvix source tree.
```

## Getting Dependencies

### Ubuntu 22.04

```bash
# Assuming you are in the project's root directory.
cat build/scripts/setup/ubuntu.sh                # Inspect what is going to be installed.
sudo -E ./build/scripts/setup/ubuntu.sh --extra  # Install dependencies.
```

### Arch Linux

```bash
# Assuming you are in the project's root directory.
cat build/scripts/setup/arch.sh                # Inspect what is going to be installed.
sudo -E ./build/scripts/setup/arch.sh --extra  # Install dependencies.
```

## Getting the Rust Toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
exec $SHELL # Restart shell to update path.
rustup component add rust-src
rustup target add wasm32-wasip1
```

## Getting a C/C++ Cross Compiler Toolchain

You can follow any of the approaches below.

### Building from the Sources (Recommended)

> ⚠️ This step may take some time to complete.

```bash
# Assuming you are in the project's root directory.
export TOOLCHAIN_DIR=$PWD                          # Set this to the directory where the toolchain will be installed.
./build/scripts/setup/toolchain.sh $TOOLCHAIN_DIR  # Build GCC, Binutils, and GDB.
```

### Building a Docker Image from the Sources

> ⚠️ This step may take some time to complete.

```bash
# Assuming you are in the project's root directory.
docker build -t nanvix/toolchain build/scripts/setup/
```

### Getting a Pre-Built Docker Image

```bash
docker pull nanvix/toolchain
```

## Building JaveScript to WebAssembly Toolchain (Optional)

> ⚠️ This step may take some time to complete.

Follow this step if you want to build the JavaScript to WebAssembly (Javy) toolchain for Nanvix.

```bash
git clone https://github.com/nanvix/javy
cargo build -p javy-plugin --target=wasm32-wasip1 -r
cargo install --path crates/cli
```

## Building QEMU (Optional)

> ⚠️ This step may take some time to complete.

Follow this step if you want to use a version of QEMU that is known to work with Nanvix.

```bash
# Assuming you are in the project's root directory.
export TARGET=x86                      # Select x86 as your target architecture.
./build/scripts/setup/qemu.sh $TARGET  # Build QEMU.
```
