# Running Nanvix

> ℹ️ The instructions in this document assume that you already know how to build Nanvix. For more information on how to build Nanvix, please refer to the [build.md](build.md) document.

This document provides instructions on how to run Nanvix.

## Table of Contents

- [Running Nanvix with `nanvixd` (Preferred Method)](#running-nanvix-with-nanvixd-preferred-method)
  - [Step 1: Run `nanvixd`](#step-1-run-nanvixd)
  - [Step 2: Run an Application](#step-2-run-an-application)
- [Running Nanvix Components Manually (Hyperlight and MicroVM Machines Only)](#running-nanvix-components-manually-hyperlight-and-microvm-machines-only)
  - [Step 1: Run the Linux Daemon](#step-1-run-the-linux-daemon)
  - [Step 2: Run the MicroVM](#step-2-run-the-microvm)
  - [Redirecting Standard Error (Optional)](#redirecting-standard-error-optional)
- [Running Nanvix Through the Build System](#running-nanvix-through-the-build-system)

## Running Nanvix with `nanvixd` (Preferred Method)

Nanvixd is a utility script that manages the deployment of User VMs in Nanvix, and their corresponding linuxd instances.

Nanvixd exposes a unified RESTful API to interact with your deployment. To follow this guide we assume you have `jq` and `curl` installed.

### Step 1: Run `nanvixd`

Open a terminal and run `nanvixd`:

```bash
NANVIX_HTTP_ADDR=127.0.0.1:8080
./bin/nanvixd.elf -http-addr $NANVIX_HTTP_ADDR
```

To enable logging, make sure to prepend the previous command with `RUST_LOG=debug` (or even `RUST_LOG=trace`).

### Step 2: Run an Application

On a new terminal window, you can now spawn and kill applications by sending `POST` requests to nanvixd's HTTP address.

```bash
NANVIX_HTTP_ADDR=127.0.0.1:8080
NEW_JSON=$(jq -n \
    --arg tenant_id "foo" \
    --arg app_name "bar" \
    --arg program "./bin/hello-c.elf" \
    --arg program_args "" \
    '{tenant_id: $tenant_id, app_name: $app_name, program: $program, program_args: $program_args}'
)
NEW_RESPONSE=$(curl \
    --silent \
    --header "Content-Type: application/json" \
    --header "X-NVX-Message-Type: NEW" \
    --request POST \
    --data "${NEW_JSON}" \
    http://${NANVIX_HTTP_ADDR})
VM_ID=$(echo ${NEW_RESPONSE} | jq -r '.user_vm_id')
GATEWAY_SOCKADDR=$(echo ${NEW_RESPONSE} | jq -r '.gateway_sockaddr')
```

Once the user VM is running, you can feed input to its STDIN (and read from its STDOUT) by opening a netcat session to the address returned by `curl`:

```bash
# Interactive session.
nc -U ${GATEWAY_SOCKADDR}
```

```bash
# One-off input.
echo "Hello World!" | nc -U -q 0 ${GATEWAY_SOCKADDR}
```

Once you are done, you can kill the user VM by sending a `KILL` POST request:

```bash
KILL_JSON=$(jq -n \
    --argjson user_vm_id "${VM_ID}" \
    '{user_vm_id: $user_vm_id}'
)
curl \
    --silent \
    --header "Content-Type: application/json" \
    --header "X-NVX-Message-Type: KILL" \
    --request POST \
    --data "${KILL_JSON}" \
    http://${NANVIX_HTTP_ADDR}
```

To gracefully shutdown nanvixd, you can just press `Ctrl-C` in its terminal.

## Running Nanvix Components Manually (Hyperlight and MicroVM Machines Only)

This instructions show you how to run each Nanvix component individually. They require faking some control-plane components that would otherwise be provided by nanvixd.

### Step 1: Run the Linux Daemon

First, open a terminal and start a netcat server to act as control-plane placeholder:

```bash
nc -lU /tmp/control-plane.socket
```

in another terminal, start linuxd:

```bash
./bin/linuxd.elf -control-plane-addr /tmp/control-plane.socket -user-vm-bind-addr /tmp/user-vm-bind.socket
```

all `-x-addr` flags have a corresponding `-x-socket-type` flag to switch between `tcp` or `unix` sockets. The default values are `unix`.

To enable logging, consider prepending the above command with `RUST_LOG=debug` or `RUST_LOG=trace`.

### Step 2: Run the MicroVM

Open another terminal to run the MicroVM. Use the `-initrd` option to specify which application to run, and pass it additional arguments with `-initrd-args`.

```bash
./bin/microvm.elf -user-vm-id 1 -system-vm-addr /tmp/user-vm-bind.socket -kernel bin/kernel.elf -initrd bin/hello-rust-nostd.elf [-gateway-addr /tmp/gw.sock] [-initrd-args <args>]
```

The `-gateway-addr` argument is optional, and should only be used if you want to connect to the VM's stdin/stdout. If you do set it, you next need to open a netcat session to connect to it:

```bash
nc -U /tmp/gw.socket
```

For example, the `./bin/echo-c.elf` binary uses the gateway because it reads input from stdin. Inside the netcat terminal, once you have written your message press `Ctrl-D` so that it is flushed to linuxd.

Alternatively to an interactive netcat session, you can use something like the following:

```bash
echo "Hello world!" | nc -U -q 0 "/tmp/gw-bind.socket" | tr -d '\0'
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
    RUST_LOG=trace ./bin/microvm.elf -user-vm-id 1 -kernel bin/kernel.elf -initrd bin/hello-rust-nostd.elf -stderr /dev/pts/5
    ```

## Running Nanvix Through the Build System

> ℹ️ This runs Nanvix with the default build parameters. Check the [build.md](build.md) document for more information on how to change default build parameters.

To run Nanvix through the build system, simply execute:

```bash
make run
```
