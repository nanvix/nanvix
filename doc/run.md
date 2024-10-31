# Running Nanvix

This document instructs you on how to run Nanvix.

> The instructions in this document assume that you have already built Nanvix. For more information on how to build Nanvix, please refer to the [Building Nanvix](build.md) document.

## Table of Contents

- [Running Nanvix in QEMU with Default Parameters](#running-nanvix-in-qemu-with-default-parameters)
  - [List of Optional Run Parameters](#list-of-optional-run-parameters)
- [Running Nanvix in MicroVM](#running-nanvix-in-microvm)
  - [Redirect Standard Error (Optional)](#redirect-standard-error-optional)

## Running Nanvix in QEMU with Default Parameters

```bash
# Run Nanvix in QEMU with default parameters:
# make TARGET=x86 LOG_LEVEL=warn MACHINE=qemu-pc TIMEOUT=90 run
make run
```

### List of Optional Run Parameters

- `LOG_LEVEL=<trace|info|warn|error>`: Set the output log level.
- `RELEASE=<yes|no>`: Enable/Disable release build.
- `TARGET=x86`: Set target CPU architecture.
- `TIMEOUT=<seconds>`: Set the timeout for the run script.

## Running Nanvix in MicroVM

> ⚠️ This step assumes that you have superuser privileges on the system.

```bash
sudo -E RUST_LOG=trace ./bin/microvm.elf -kernel bin/kernel.elf -initrd bin/boottime.elf
```

### Redirect Standard Error (Optional)

Is possible redirect the standard error of the MicroVM to another terminal. This
is useful for debugging.

To do it, open a new terminal and get its tty path:

```bash
$ tty
/dev/pts/5
```

Now, in the first terminal, run the MicroVM with the `-stderr` option:

```bash
# Assuming /dev/pts/5 is the tty of the new terminal.
sudo -E RUST_LOG=trace ./bin/microvm.elf -kernel bin/kernel.elf -initrd bin/boottime.elf -stderr dev/pts/5
```
