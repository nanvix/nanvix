# Running Nanvix

> ℹ️ The instructions in this document assume that you already know how to build Nanvix. For more information on how to build Nanvix, please refer to the [build.md](build.md) document.

This document provides instructions on how to run Nanvix.

## Table of Contents

- [Running Nanvix Through the Build System](#running-nanvix-through-the-build-system)
- [Running Nanvix in the MicroVM (Hyperlight and MicroVM Machines Only)](#running-nanvix-in-the-microvm-hyperlight-and-microvm-machines-only)
  - [Step 1: Run the Linux Daemon](#step-1-run-the-linux-daemon)
  - [Step 2: Run the MicroVM](#step-2-run-the-microvm)
  - [Enabling Logging (Optional)](#enabling-logging-optional)
  - [Redirecting Standard Error (Optional)](#redirecting-standard-error-optional)
- [Running Nanvix with `nanvixd` (MicroVM and Hyperlight Machines Only)](#running-nanvix-with-nanvixd-microvm-and-hyperlight-machines-only)
  - [Step 1: Run `nanvixd`](#step-1-run-nanvixd)
  - [Step 2: Run an Application](#step-2-run-an-application)

## Running Nanvix Through the Build System

> ℹ️ This runs Nanvix with the default build parameters. Check the [build.md](build.md) document for more information on how to change default build parameters.

To run Nanvix through the build system, simply execute:

```bash
make run
```

## Running Nanvix in the MicroVM (Hyperlight and MicroVM Machines Only)

### Step 1: Run the Linux Daemon

Open a terminal and run the Linux Daemon:

```bash
./bin/linuxd.elf -user-vm-bind-addr 127.0.0.1:1234
```

### Step 2: Run the MicroVM

Open a second terminal and run the MicroVM. Use the `-initrd` option to specify which application to run:

```bash
sudo -E ./bin/microvm.elf -kernel bin/kernel.elf -initrd bin/hello-rust-nostd.elf -gateway 127.0.0.1:1234
```

### Enabling Logging (Optional)

To enable logging, set the `RUST_LOG` environment variable to `trace` when running the Linux Daemon and/or the MicroVM:

```bash
# Open a terminal and run the Linuxd Daemon.
RUST_LOG=trace sudo -E ./bin/microvm.elf -kernel bin/kernel.elf -initrd bin/hello-rust-nostd.elf -gateway 127.0.0.1:1234

# Open a scond terminal and run the MicroVM.
RUST_LOG=trace ./bin/linuxd.elf -user-vm-bind-addr 127.0.0.1:1234
```

### Redirecting Standard Error (Optional)

Redirecting the standard error of the MicroVM to another terminal can be useful for debugging.

1. Open a new terminal and get its tty path:

    ```bash
    $ tty
    /dev/pts/5
    ```

2. Run the MicroVM with the `-stderr` option, specifying the tty path:

    ```bash
    # Assuming /dev/pts/5 is the tty of the new terminal.
    sudo -E RUST_LOG=trace ./bin/microvm.elf -kernel bin/kernel.elf -initrd bin/hello-rust-nostd.elf -stderr /dev/pts/5
    ```

## Running Nanvix with `nanvixd` (MicroVM and Hyperlight Machines Only)

### Step 1: Run `nanvixd`

Open a terminal and run `nanvixd`:

```bash
sudo -E ./bin/nanvixd.elf -http-addr 127.0.0.1:8080 -linuxd-addr 127.0.0.1:7070 -sandbox-addr 127.0.0.1:1234 -keep-alive 0
```

### Step 2: Run an Application

Open a second terminal and run an application using `curl`:

```bash
curl -w "\n" \
  --header "Content-Type: application/json" \
  --request POST \
  --data '{"clientid":1, "program":"bin/hello-rust-nostd.elf", "args":[]}' http://localhost:8080
```
