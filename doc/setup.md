# Development Environment Setup

This document instructs you on how to setup your development environment and run Nanvix.

> ⚠️ These instructions assume that you have superuser privileges on the machine.

## 1. Clone this Repository and Submodules

```bash
export WORKDIR=$HOME/nanvix                                          # Change this if you want.
mkdir -p $WORKDIR                                                    # Create workspace.
cd $WORKDIR                                                          # Switch to workspace.
git clone --recurse-submodules https://github.com/nanvix/nanvix.git  # Clone repository.
cd nanvix                                                            # Switch to nanvix source tree.
```

## 2. Install Dependencies

> ⚠️ For this step, case you will build QEMU, remove QEMU from shell scripts.

### Ubuntu 22.04
```bash
cat build/scripts/setup/ubuntu.sh        # Inspect what is going to be installed.
sudo -E ./build/scripts/setup/ubuntu.sh  # Install dependencies.
```

### Arch Linux
```bash
cat build/scripts/setup/arch.sh        # Inspect what is going to be installed.
sudo -E ./build/scripts/setup/arch.sh  # Install dependencies.
```

### Install Rust toolchain
``` bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
exec $SHELL  # Restart shell to update path.
rustup component add rust-src
```

## Optional

> ⚠️ Next instructions are necessary only if you want compile QEMU and C language toolchain.

### Build Toolchain

> ⚠️ This step may take some time to complete.

```bash
export TARGET=x86                           # Select x86 as your target architecture.
./build/scripts/setup/toolchain.sh $TARGET  # Build GCC, Binutils, and GDB.
```

### Build QEMU

> ⚠️ This step may take some time to complete.

```bash
export TARGET=x86                      # Select x86 as your target architecture.
./build/scripts/setup/qemu.sh $TARGET  # Build QEMU.
```
# Run Nanvix

This section instructs you on how to compile and run Nanvix. 

## Using Microvm

Compile Microvm:

```bash
cd $WORKDIR/nanvix/src/microvm
make all
```

Compile Kernel:

```bash
cd $WORKDIR/nanvix/src/kernel
make TARGET=x86 LOG_LEVEL=trace MACHINE=microvm all
```

Run Nanvix:

```bash
cd $WORKDIR/nanvix/src/kernel
make MICROVM_PATH=$WORKDIR/nanvix/src/microvm/bin TARGET=x86 LOG_LEVEL=trace MACHINE=microvm TIMEOUT=90 run
```

## Using QEMU

Compile Kernel:

```bash
cd $WORKDIR/nanvix/src/kernel
make TARGET=x86 LOG_LEVEL=trace MACHINE=qemu-pc all
```

Run Nanvix:

```bash
cd $WORKDIR/nanvix/src/kernel
make TARGET=x86 LOG_LEVEL=trace MACHINE=qemu-pc TIMEOUT=90 run
```

# Make Variables

See `nanvix/src/kernel/Cargo.toml` for possible values and the Makefiles for other variables.

`TARGET` => Set the CPU architecture.

`LOG_LEVEL` => Set the output log level.

`MACHINE` => Set the target machine.

`RELEASE` => Enable release build.

`PROFILER` => Print benchmark output.

`TIMEOUT` => Set timeout for run command

`TOOLCHAIN_DIR` => Path to toolchain directory.

`MICROVM_PATH` => Path to Microvm Binary.
