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

- [Interactive Mode](#interactive-mode)
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

---

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

> **Note:** When containerd provides a stdout path (e.g., via `ctr run`), the shim
> automatically overrides `-console-file` from `extra_args` to redirect guest console
> output to containerd's stdout pipe.

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

Everything after `--` is forwarded to the application as arguments:

```bash
./bin/nanvixd.elf -console-file /dev/stdout -- ./bin/echo-rust-nostd.elf arg1 arg2
```

Arguments and environment variables are packed into a single string separated by `;`. Everything
before the semicolon becomes command-line arguments; everything after it becomes environment
variables as space-separated `KEY=VALUE` pairs (e.g., `"arg1 arg2;VAR1=foo VAR2=bar"`). Use an empty
string when neither is needed. To pass only environment variables, start the string with `;`:

```bash
# Arguments and environment variables.
./bin/nanvixd.elf -console-file /dev/stdout -- ./bin/echo-rust-nostd.elf "arg1 arg2;VAR1=foo VAR2=bar"

# Environment variables only.
./bin/nanvixd.elf -console-file /dev/stdout -- ./bin/echo-rust-nostd.elf ";VAR1=foo"
```

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

Arguments and environment variables are packed into a single string separated by `;`. Everything
before the semicolon becomes command-line arguments; everything after it becomes environment
variables as space-separated `KEY=VALUE` pairs (e.g., `"arg1 arg2;VAR1=foo VAR2=bar"`). Use an empty
string when neither is needed. To pass only environment variables, start the string with `;`.

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
| `-ramfs <file>`             | Path to a RAM filesystem image exposed to the guest.                    |
| `-user-vm-id <id>`          | VM identifier (defaults to `0` in standalone mode).                     |
| `-log-to-file`              | Write logs to files instead of stdout.                                  |
| `-log-dir <dir>`            | Directory for log files (used with `-log-to-file`).                     |
| `-allow-host-networking`    | Enable host networking for the guest (disabled when omitted).           |

Enable verbose logging with `RUST_LOG`:

```bash
RUST_LOG=debug ./bin/uservm.elf -kernel ./bin/kernel.elf \
    -initrd ./bin/hello-rust-nostd.elf -standalone
```
