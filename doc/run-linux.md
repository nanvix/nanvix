# Running Nanvix (Linux)

> **Prerequisite:** You must build Nanvix before running it. See [build-linux.md](build-linux.md)
for instructions.

On Linux, Nanvix instances are launched and managed through `nanvixd`, which operates in one of
two mutually exclusive modes:

- **Interactive Mode**: runs a single application, waits for it to exit, and forwards its exit code.
- **HTTP Mode**: starts a REST server that spawns and kills applications on demand.

## Quick Start

Run a hello-world application and see its output on the terminal:

```bash
./bin/nanvixd.elf -console-file /dev/stdout -- ./bin/hello-rust-nostd.elf
```

## Table of Contents

- [Quick Start](#quick-start)
- [Creating Multibinary Images](#creating-multibinary-images)
- [Interactive Mode](#interactive-mode)
  - [Shim Configuration](#shim-configuration)
- [Running Containers](#running-containers)
  - [Building a Nanvix OCI image](#building-a-nanvix-oci-image)
  - [Importing and running with `ctr`](#importing-and-running-with-ctr)
  - [Enabling Host Networking](#enabling-host-networking)
  - [Mounting a Host Directory](#mounting-a-host-directory)
  - [Passing Kernel Arguments](#passing-kernel-arguments)
- [HTTP Mode](#http-mode)
  - [Starting the Server](#starting-the-server)
  - [Spawning an Application (NEW)](#spawning-an-application-new)
  - [Communicating with an Application](#communicating-with-an-application)
  - [Killing an Application (KILL)](#killing-an-application-kill)
  - [Shutting Down the Server](#shutting-down-the-server)
  - [HTTP API Reference](#http-api-reference)
  - [HTTP Error Codes](#http-error-codes)
- [Logging](#logging)
- [Expert Mode: Standalone User VM](#expert-mode-standalone-user-vm)
  - [Recognised Kernel Arguments](#recognised-kernel-arguments)

---

## Creating Multibinary Images

In standalone deployment mode, guest applications run alongside system daemons (`procd`, `memd`,
and `vfsd`) inside a single VM. These components must be bundled together into a **multibinary
image** using the `mkimage` tool before they can be launched.

The `mkimage` tool takes an output path and a list of `<path>;<name>` pairs, where `<path>` is
the path to the ELF binary and `<name>` is the logical name the kernel uses to identify it at
boot:

```bash
./bin/mkimage.elf -o my-app.img \
    ./bin/procd.elf\;procd \
    ./bin/memd.elf\;memd \
    ./bin/vfsd.elf\;vfsd \
    ./bin/my-app.elf\;my-app
```

The three daemon binaries (`procd.elf`, `memd.elf`, `vfsd.elf`) are shipped in the release
archive under `bin/`. Your application binary must be compiled and linked against `libposix.a`
using the `user.ld` linker script (both also in the release archive).

Once the image is created, pass it to `nanvixd` as the program argument:

```bash
./bin/nanvixd.elf -console-file /dev/stdout -- ./my-app.img
```

> **Important:** The daemon order in the `mkimage` command line matters. Daemons are started in
> the order they appear, and `procd` must be listed first because other daemons depend on it.

## Interactive Mode

Interactive mode runs a single application, waits for it to exit, and forwards its exit code.

### Shim Configuration

| Key              | Description                                    | Default          |
| ---------------- | ---------------------------------------------- | ---------------- |
| `kernel_path`    | Path to `nanvixd.elf` on the host.             | `nanvixd.elf`    |
| `mkramfs_path`   | Path to `mkramfs.elf` on the host.             | `mkramfs.elf`    |
| `temp_dir`       | Directory for generated ramfs images.          | System temp dir  |
| `execution_mode` | Execution mode (`"standalone"` for V1).        | `"standalone"`   |
| `extra_args`     | Additional arguments passed to `nanvixd`.      | `[]`             |

Example:

```toml
kernel_path = "/opt/nanvix/bin/nanvixd.elf"
mkramfs_path = "/opt/nanvix/bin/mkramfs.elf"
temp_dir = "/tmp"
execution_mode = "standalone"
extra_args = ["-console-file", "/dev/null"]
```

> **Note:** When the nanvix containerd shim spawns nanvixd, it always sets
> `-console-file` and `-gateway-sockaddr` itself; any `-console-file` or
> `-gateway-sockaddr` value passed in `extra_args` is stripped with a
> warning.

## Running Containers

### Building a Nanvix OCI image

See [Building Nanvix OCI Images](docker-images.md) for Dockerfile patterns, annotation
conventions, and step-by-step build instructions.

### Importing and running with `ctr`

Once you have a Docker image, import it into containerd and run it with the Nanvix runtime:

```bash
./bin/nanvixd.elf -- ./bin/hello-rust-nostd.elf
```

By default, console output goes to a log file under `logs/`. Use `-console-file /dev/stdout` to
print it to the terminal instead:

```bash
./bin/nanvixd.elf -console-file /dev/stdout -- ./bin/hello-rust-nostd.elf
```

### Enabling Host Networking

By default, networking system calls from the guest are blocked. To allow the guest to access the
host network stack, pass `-allow-host-networking`:

```bash
./bin/nanvixd.elf -allow-host-networking -console-file /dev/stdout -- ./bin/network-rust.elf
```

### Mounting a Host Directory

To make a host directory accessible to the guest at `/mnt`, use the `-mount` flag:

```bash
./bin/nanvixd.elf -mount /path/to/shared/dir -console-file /dev/stdout -- ./bin/file-rust.elf
```

The guest can then read and write files under `/mnt/` which map to the host directory.
See [host-mount.md](host-mount.md) for the design and protocol details.

### Passing Kernel Arguments

To pass kernel arguments (written to guest control registers), use the `-kernel-args` flag:

```bash
./bin/nanvixd.elf -kernel-args snapshot -console-file /dev/stdout -- ./bin/snapshot-rust-nostd.elf
```

See [Recognised Kernel Arguments](#recognised-kernel-arguments) below for available tokens.

Everything after `--` is forwarded to the application as arguments:

```bash
./bin/nanvixd.elf -console-file /dev/stdout -- ./bin/echo-rust-nostd.elf arg1 arg2
```

Arguments and environment variables are packed into a single string separated by `;`. The format
is `<app args>;<env vars>`:

- Everything before the first unescaped `;` becomes command-line arguments.
- Everything after the first unescaped `;` becomes environment variables as space-separated
  `KEY=VALUE` pairs.

Use an empty string when neither is needed. To pass only environment variables, start the string
with `;`:

```bash
# Arguments and environment variables.
./bin/nanvixd.elf -console-file /dev/stdout -- ./bin/echo-rust-nostd.elf "arg1 arg2;VAR1=foo VAR2=bar"

# Environment variables only.
./bin/nanvixd.elf -console-file /dev/stdout -- ./bin/echo-rust-nostd.elf ";VAR1=foo"
```

To include a literal `;` in any section, escape it as `\;`:

```bash
# Argument containing a literal semicolon.
./bin/nanvixd.elf -console-file /dev/stdout -- ./bin/echo-rust-nostd.elf "arg1 with\;semicolon arg2;VAR1=foo"
# args: ["arg1", "with;semicolon", "arg2"]   env: ["VAR1=foo"]
```

> **Note:** Kernel arguments can be passed via `-kernel-args` on `nanvixd` (see
> [Passing Kernel Arguments](#passing-kernel-arguments)) or directly on the UserVM (see
> [Expert Mode: Standalone User VM](#expert-mode-standalone-user-vm)). They are not embedded in
> the initrd arguments string.

## HTTP Mode

HTTP mode starts `nanvixd` as a long-running server that exposes a RESTful API for spawning and
killing applications.

> **Required tools:** `curl` and `jq` (for the examples below).

### Starting the Server

```bash
NANVIX_HTTP_ADDR=127.0.0.1:8080
./bin/nanvixd.elf -http-addr $NANVIX_HTTP_ADDR
```

### Spawning an Application (NEW)

In a **second terminal**, send a `POST` request with `X-NVX-Message-Type: NEW`:

```bash
NANVIX_HTTP_ADDR=127.0.0.1:8080

NEW_JSON=$(jq -n \
    --arg tenant_id "foo" \
    --arg app_name "bar" \
    --arg program "./bin/hello-rust-nostd.elf" \
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

APP_ID=$(echo ${NEW_RESPONSE} | jq -r '.user_vm_id')
GATEWAY_SOCKADDR=$(echo ${NEW_RESPONSE} | jq -r '.gateway_sockaddr')
```

### Communicating with an Application

Use the gateway socket to interact with the application's stdin/stdout via `netcat`:

```bash
nc -U ${GATEWAY_SOCKADDR}
```

```bash
echo "Hello World!" | nc -U -q 0 ${GATEWAY_SOCKADDR}
```

### Killing an Application (KILL)

Send a `POST` request with `X-NVX-Message-Type: KILL` to terminate an application:

```bash
KILL_JSON=$(jq -n \
    --argjson user_vm_id "${APP_ID}" \
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

### Shutting Down the Server

Press `Ctrl-C` in the terminal where `nanvixd` is running.

### HTTP API Reference

All requests are `POST` to `http://<host:port>/`. The message type is specified via the
`X-NVX-Message-Type` header.

#### NEW — Spawn an Application

**Request header:** `X-NVX-Message-Type: NEW`

**Request body:**

| Field          | Type   | Required | Description                              |
| -------------- | ------ | -------- | ---------------------------------------- |
| `tenant_id`    | string | yes      | Tenant identifier for resource isolation.|
| `app_name`     | string | yes      | Application name for identification.     |
| `program`      | string | yes      | Path to the program binary to execute.   |
| `program_args` | string | yes      | Arguments and environment variables.     |

Arguments and environment variables are packed into a single string separated by `;`. The format
is `<app args>;<env vars>`. Everything before the first unescaped semicolon becomes command-line
arguments; everything after the first unescaped semicolon becomes environment variables as
space-separated `KEY=VALUE` pairs (e.g., `"arg1 arg2;VAR1=foo VAR2=bar"`).
Use an empty string when neither is needed. To pass only environment variables, start the string
with `;`. To include a literal `;` in any section, escape it as `\;`.

**Success response (200):**

| Field              | Type    | Description                                |
| ------------------ | ------- | ------------------------------------------ |
| `user_vm_id`       | integer | Unique identifier for the application.     |
| `gateway_sockaddr` | string  | Socket address for the application's I/O.  |

#### KILL — Terminate an Application

**Request header:** `X-NVX-Message-Type: KILL`

**Request body:**

| Field        | Type    | Required | Description                          |
| ------------ | ------- | -------- | ------------------------------------ |
| `user_vm_id` | integer | yes      | Identifier of the application.       |

**Success response (200):**

| Field       | Type    | Description                                   |
| ----------- | ------- | --------------------------------------------- |
| `exit_code` | integer | `0` for success, non-zero for failure.        |

### HTTP Error Codes

All error responses use the `ErrorResponse` schema with a machine-readable `code` and a
human-readable `message`. Callers should branch on `code` and relay `message` for diagnostics.

| Code                   | HTTP Status | Cause                                 |
| ---------------------- | ----------- | ------------------------------------- |
| `MISSING_MESSAGE_TYPE` | 400         | Missing or invalid message type.      |
| `BODY_READ_FAILED`     | 500         | Could not read the request body.      |
| `INVALID_NEW_PAYLOAD`  | 400         | Invalid JSON for a `NEW` request.     |
| `NEW_REQUEST_FAILED`   | 500         | Application creation failed.          |
| `INVALID_KILL_PAYLOAD` | 400         | Invalid JSON for a `KILL` request.    |
| `KILL_REQUEST_FAILED`  | 500         | Application termination failed.       |

**Example error response:**

```json
{
  "code": "INVALID_NEW_PAYLOAD",
  "message": "failed to deserialize request body: missing field `program`"
}
```

## Logging

`nanvixd` uses the `RUST_LOG` environment variable for daemon-level logging (printed to stderr).
This applies to both interactive and HTTP modes:

```bash
RUST_LOG=debug ./bin/nanvixd.elf -console-file /dev/stdout -- ./bin/hello-rust-nostd.elf
RUST_LOG=trace ./bin/nanvixd.elf -http-addr 127.0.0.1:8080
```

By default, `nanvixd`'s own structured (`logrus`) records are written to an auto-named file
(`nanvixd_<timestamp>.log`) inside the log directory (overridable via `-log-dir <dir>`). Pass
`-log-to-stdout` to route those records to stdout instead:

```bash
./bin/nanvixd.elf -log-to-stdout -- ./bin/hello-rust-nostd.elf
```

This is useful when a parent process (for example, the Nanvix containerd shim) captures
`nanvixd`'s stdout and forwards it to its own log sink. `-log-to-stdout` and `-log-dir` are
mutually exclusive.

## Expert Mode: Standalone User VM

> **Warning:** This is an expert-level feature intended for low-level debugging and kernel
> development. Most users should use `nanvixd` instead (see [Interactive Mode](#interactive-mode)).

The `uservm.elf` binary can be launched directly in **standalone mode**, bypassing the full Nanvix
orchestration stack (no `nanvixd`, system VM, control-plane, or gateway connections). In this mode
the guest kernel boots, runs the initrd payload, and exits. Outbound I/O messages from the guest
are silently discarded.

```bash
./bin/uservm.elf -kernel ./bin/kernel.elf -initrd ./bin/hello-rust-nostd.elf -standalone
```

Optional flags:

| Flag                        | Description                                                             |
| --------------------------- | ----------------------------------------------------------------------- |
| `-stderr <file>`            | Redirect guest stderr to a file instead of host stderr.                 |
| `-initrd_args <args>`       | Arguments forwarded to the initrd payload.                              |
| `-kernel-args <args>`       | Kernel arguments written to guest control registers (see below).        |
| `-ramfs <file>`             | Path to a RAM filesystem image exposed to the guest.                    |
| `-user-vm-id <id>`          | VM identifier (defaults to `0` in standalone mode).                     |
| `-log-to-file`              | Write logs to files instead of stdout.                                  |
| `-log-dir <dir>`            | Directory for log files (used with `-log-to-file`).                     |
| `-allow-host-networking`    | Enable host networking for the guest (disabled when omitted).           |

### Recognised Kernel Arguments

The `-kernel-args` flag accepts a space-separated list of tokens:

| Token      | Description                                                                              |
| ---------- | ---------------------------------------------------------------------------------------- |
| `snapshot` | Allow the guest to take exactly one VM snapshot via the `snapshot` kernel call.           |

Example:

```bash
./bin/uservm.elf -kernel ./bin/kernel.elf \
    -initrd ./bin/snapshot-rust-nostd.elf -standalone \
    -kernel-args snapshot
```

Enable verbose logging with `RUST_LOG`:

```bash
RUST_LOG=debug ./bin/uservm.elf -kernel ./bin/kernel.elf \
    -initrd ./bin/hello-rust-nostd.elf -standalone
```
