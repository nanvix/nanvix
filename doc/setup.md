# Setting Up Your Development Environment

This document instructs you on how to setup your development environment to build and run Nanvix.

> ℹ️ Some instructions instructions in this document assume that you have superuser privileges on the system.

## Table of Contents

- [Clone this Repository](#clone-this-repository)
- [Installing System-Wide Dependencies](#installing-system-wide-dependencies)
  - [Ubuntu 22.04](#ubuntu-2204)
  - [Arch Linux](#arch-linux)
- [Installing Rust toolchain](#installing-rust-toolchain)
- [Building C/C++ Toolchain](#building-cc-toolchain)
- [Building QEMU (Optional)](#building-qemu-optional)

## Clone this Repository

```bash
export WORKDIR=$HOME/nanvix                     # Set this to the directory where the source tree will be cloned.
mkdir -p $WORKDIR && cd $WORKDIR                # Create workspace and switch to it.
git clone https://github.com/nanvix/nanvix.git  # Clone repository.
cd nanvix                                       # Switch to Nanvix source tree.
```

## Installing System-Wide Dependencies

### Ubuntu 22.04

```bash
cat build/scripts/setup/ubuntu.sh                # Inspect what is going to be installed.
sudo -E ./build/scripts/setup/ubuntu.sh --extra  # Install dependencies.
```

### Arch Linux

```bash
cat build/scripts/setup/arch.sh                # Inspect what is going to be installed.
sudo -E ./build/scripts/setup/arch.sh --extra  # Install dependencies.
```

## Installing Rust toolchain

``` bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
exec $SHELL # Restart shell to update path.
rustup component add rust-src
rustup target add wasm32-wasip1
```

## Building C/C++ Toolchain

> ⚠️ This step may take some time to complete.

```bash
export TOOLCHAIN_DIR=$PWD                          # Set this to the directory where the toolchain will be installed.
./build/scripts/setup/toolchain.sh $TOOLCHAIN_DIR  # Build GCC, Binutils, and GDB.
```

## Building QEMU (Optional)

Follow this step if you want to use a version of QEMU that is known to work with Nanvix.

> ⚠️ This step may take some time to complete.

```bash
export TARGET=x86                      # Select x86 as your target architecture.
./build/scripts/setup/qemu.sh $TARGET  # Build QEMU.
```
