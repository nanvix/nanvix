# Building Nanvix

This document instructs you on how to build Nanvix.

> ⚠️ These instructions assume that you have already setup your development environment. See [setup.md](setup.md) for details.

## 1. Set Toolchain Location

```bash
# Change this variable accordingly.
# This variable should point to the directory where you have installed your
# development toolchain. If you have not changed the default installation
# location, then you can run the following command unmodified.
export TOOLCHAIN_DIR=$HOME/nanvix/toolchain
```

## 2. Build Binaries

```bash
make nanvix  # Build binaries.
```

## 3. Build System Image

```bash
make image  # Build system image.
```

### 4. Build Documentation (Optional)

```bash
make documentation  # Build Doxygen documentation.
```

### 4. Clean Build (Optional)

```bash
make clean  # Clean up all build artifacts.
```
