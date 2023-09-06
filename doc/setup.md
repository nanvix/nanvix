# Development Environment Setup

This document instructs you on how to setup your development environment.

> ⚠️ Follow these instructions on a fresh Ubuntu 22.04 system.

> ⚠️ These instructions assume that you have superuser privileges on the system.

## 1. Clone This Repository

```bash
export WORKDIR=$HOME/nanvix                         # Change this if you want.
export INSTALL_DIR=$WORKDIR                         # Change this if you want.
mkdir -p $WORKDIR                                   # Create workspace.
cd $WORKDIR                                         # Switch to workspace.
git clone https://github.com/Bois-Barganhados-Studio/nanvix-user-level-thread.git .   # Clone repository.
```

## 2. Install Dependencies

```bash
cat ./tools/dev/setup/ubuntu.sh     # Checkout what is going to be installed.
sudo -E ./tools/dev/setup/ubuntu.sh # Install dependencies.
```

## 3. Build Toolchain

> ⚠️ This step may take some time to complete.

```bash
./tools/dev/setup/binutils.sh $INSTALL_DIR  # Build Binutils.
./tools/dev/setup/gcc.sh $INSTALL_DIR       # Build GCC.
./tools/dev/setup/gdb.sh $INSTALL_DIR       # Build GDB.
```

## 4. Build System Simulator

> ⚠️ This step may take some time to complete.

You may chose to between QEMU or Bochs.

### 4.1. Build QEMU (Preferred)

```bash
./tools/dev/setup/qemu.sh  # Build QEMU.
```

### 4.2. Build Bochs (Alternative)

```bash
./tools/dev/setup/bochs.sh  # Build Bochs.
```
