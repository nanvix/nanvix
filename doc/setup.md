# Setting up your Development Environment

This document instructs you on how to setup your development environment to build and run Nanvix.

> ⚠️ Some instructions instructions in this document assume that you have superuser privileges on the system.

## Table of Contents

- [Clone this Repository](#clone-this-repository)
- [Installing Dependencies](#installing-dependencies)
  - [Installing System-Wide Packages](#installing-system-wide-packages)
  - [Installing Rust toolchain](#installing-rust-toolchain)
  - [Building QEMU (Optional)](#building-qemu-optional)
  - [Building C Toolchain (Optional)](#building-c-toolchain-optional)

## Clone this Repository

```bash
export WORKDIR=$HOME/nanvix                                          # Change this if you want.
mkdir -p $WORKDIR                                                    # Create workspace.
cd $WORKDIR                                                          # Switch to workspace.
git clone --recurse-submodules https://github.com/nanvix/nanvix.git  # Recursively clone repository.
cd nanvix                                                            # Switch to nanvix source tree.
```

## Installing Dependencies

### Installing System-Wide Packages

#### Ubuntu 22.04

```bash
cat build/scripts/setup/ubuntu.sh        # Inspect what is going to be installed.
sudo -E ./build/scripts/setup/ubuntu.sh  # Install dependencies.
```

#### Arch Linux

```bash
cat build/scripts/setup/arch.sh        # Inspect what is going to be installed.
sudo -E ./build/scripts/setup/arch.sh  # Install dependencies.
```

### Installing Rust toolchain

``` bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
exec $SHELL  # Restart shell to update path.
rustup component add rust-src
```

### Building QEMU (Optional)

Follow this step if you want to use a specific version of QEMU to run Nanvix.

> ⚠️ This step may take some time to complete.

```bash
export TARGET=x86                      # Select x86 as your target architecture.
./build/scripts/setup/qemu.sh $TARGET  # Build QEMU.
```

### Building C Toolchain (Optional)

Follow this step if you want to build write C applications for Nanvix.

> ⚠️ This step may take some time to complete.

```bash
export TARGET=x86                           # Select x86 as your target architecture.
./build/scripts/setup/toolchain.sh $TARGET  # Build GCC, Binutils, and GDB.
```
