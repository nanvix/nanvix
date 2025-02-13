# Running Nanvix

This document instructs you on how to run Nanvix.

> ℹ️ The instructions in this document assume that you have already built
Nanvix. For more information on how to build Nanvix, please refer to the
[Building Nanvix](build.md) document.

## Table of Contents

- [Running Nanvix in QEMU with Default Parameters](#running-nanvix-in-qemu-with-default-parameters)
  - [List of Optional Run Parameters](#list-of-optional-run-parameters)
- [Running Nanvix in MicroVM](#running-nanvix-in-microvm)
  - [Redirect Standard Error (Optional)](#redirect-standard-error-optional)
  - [Running the Linux Daemon (Optional)](#running-the-linux-daemon-optional)

## Running Nanvix in QEMU with Default Parameters

```bash
# Run Nanvix in QEMU with default parameters:
# make TARGET=x86 MACHINE=qemu-pc VERBOSE=no RELEASE=no TIMEOUT=10 run
make run
```

### List of Optional Run Parameters

- `LOG_LEVEL=<trace|info|warn|error>`: Set the output log level.
- `RELEASE=<yes|no>`: Enable/Disable release build.
- `TARGET=x86`: Set target CPU architecture.
- `TIMEOUT=<seconds>`: Set the execution timeout.

## Running Nanvix in MicroVM

> ⚠️ This step assumes that you have superuser privileges on the system.

```bash
sudo -E RUST_LOG=trace ./bin/microvm.elf -kernel bin/kernel.elf -initrd bin/noop-rust-nostd.elf
```

### Redirect Standard Error (Optional)

It's possible to redirect the standard error of the MicroVM to another terminal. This
is useful for debugging.

To do it, open a new terminal and get its tty path:

```bash
$ tty
/dev/pts/5
```

Now, in the first terminal, run the MicroVM with the `-stderr` option:

```bash
# Assuming /dev/pts/5 is the tty of the new terminal.
sudo -E RUST_LOG=trace ./bin/microvm.elf -kernel bin/kernel.elf -initrd bin/noop-rust-nostd.elf -stderr dev/pts/5
```

### Running the Linux Daemon (Optional)

There are more binaries available other than `noop-rust-nostd.elf`. One of them is `linux-app.elf`.
In order to run it, get the terminal's tty path:

```bash
$ tty
/dev/pts/5
```

Now open a second terminal and run the daemon on it:

```bash
rmdir foo ; rm -f *.tmp ; RUST_LOG=trace ./bin/linuxd.elf -server 127.0.0.1:1234
```

Removing the directory and files sets up the environment for the tests run in `linux-app.elf`.
Now open a third terminal and run the MicroVM on it, redirecting stderr to the first terminal:

```bash
sudo -E RUST_LOG=trace ./bin/microvm.elf -kernel bin/kernel.elf -stderr /dev/pts/5 -initrd bin/linux-app.elf -gateway 127.0.0.1:1234
```
