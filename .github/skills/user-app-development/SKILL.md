---
name: user-app-development
description: Guide for developing, building, and running Nanvix user-space applications across supported runtimes and languages. Use this when asked about guest app implementation or execution.
---

# User Application Development

Use this skill when the user asks about developing, building, or running user-space applications on
Nanvix.  User applications are programs that run inside the Nanvix guest environment.

## Supported Languages and Examples

| Application        | Path                         | Lang       | Runtime    |
|--------------------|------------------------------|------------|------------|
| `hello-rust-nostd` | `src/user/hello-rust-nostd/` | Rust       | Bare-metal |

## Running Applications

### Interactive Mode (recommended for development)

```bash
# Run a native application with console output.
./bin/nanvixd.elf \
    -console-file /dev/stdout \
    -- ./bin/hello-rust-nostd.elf

# Pass arguments to the application.
./bin/nanvixd.elf \
    -console-file /dev/stdout \
    -- ./bin/echo-rust-nostd.elf arg1 arg2

# Pass arguments and environment variables.
./bin/nanvixd.elf \
    -console-file /dev/stdout \
    -- ./bin/echo-rust-nostd.elf \
    "arg1 arg2;VAR1=foo VAR2=bar"

# Pass arguments, environment variables, and kernel arguments.
# Format: "<app args>;<env vars>;<kernel args>"
./bin/nanvixd.elf \
    -console-file /dev/stdout \
    -- ./bin/echo-rust-nostd.elf \
    "arg1 arg2;VAR1=foo;feature1 feature2"
```

### HTTP Mode (for multi-application scenarios)

```bash
# Start nanvixd in HTTP mode.
NANVIX_HTTP_ADDR=127.0.0.1:8080
./bin/nanvixd.elf -http-addr $NANVIX_HTTP_ADDR

# Spawn an application via HTTP (in another terminal).
curl --silent \
    --header "Content-Type: application/json" \
    --header "X-NVX-Message-Type: NEW" \
    --request POST \
    --data '{"tenant_id":"foo",
             "app_name":"bar",
             "program":"./bin/hello-rust-nostd.elf",
             "program_args":""}' \
    http://${NANVIX_HTTP_ADDR}
```

### Standalone UserVM Mode (expert/debugging)

```bash
./bin/uservm.elf \
    -kernel ./bin/kernel.elf \
    -initrd ./bin/hello-rust-nostd.elf \
    -standalone
```

### Running on Windows

On Windows, `nanvixd` supports standalone interactive mode (no HTTP mode). Both `nanvixd` and the
UserVM are built natively with the WHP backend:

```powershell
# Run via nanvixd (recommended).
.\bin\nanvixd.exe -- .\bin\hello-rust-nostd.elf
```

You can also launch a run via `z.ps1`:

```powershell
# Run with default options.
.\z.ps1 run

# Run with a custom guest binary.
.\z.ps1 run -- -program bin\hello-rust-nostd.elf
```

For low-level debugging, the standalone UserVM is still available:

```powershell
.\bin\uservm.exe -kernel .\bin\kernel.elf -initrd .\bin\hello-rust-nostd.elf -standalone
```

> **Note:** HTTP mode is Linux-only. On Windows, only standalone interactive mode
> via `nanvixd` is supported.

## Creating a New Rust Application (no_std)

1. Create directory at `src/user/<name>/`.

2. Add `Cargo.toml`:

   ```toml
   [package]
   name = "<name>"
   version.workspace = true
   license-file.workspace = true
   authors.workspace = true
   edition.workspace = true

   [[bin]]
   name = "<name>"
   path = "src/main.rs"

   [dependencies]
   nvx = { workspace = true }
   sys = { workspace = true }
   syslog = { workspace = true }
   ```

3. Create `src/main.rs`:

   ```rust
   // Copyright(c) The Maintainers of Nanvix.
   // Licensed under the MIT License.

   #![no_std]
   #![no_main]

   extern crate alloc;

   // Application entry point.
   ```

4. Add to workspace `members` in root `Cargo.toml`.

5. Add to `ALL_GUEST_APPLICATIONS` in the `Makefile`.

## Creating a New C Application

1. Create directory at `src/user/<name>/`.
2. Create a `Makefile` following the pattern in existing C applications.
3. Write source files using the Nanvix POSIX layer (`#include <nanvix/...>`).
4. The application links against `libc.a` and the custom linker script at
   `build/user/linker/x86/user.ld`.

## Logging

- Use `RUST_LOG` environment variable for `nanvixd` daemon-level logging.
- Guest-side logging uses the `syslog` crate (`error!`, `warn!`, `info!`, `debug!`, `trace!`).
- Console output defaults to `logs/` directory; use `-console-file /dev/stdout` to redirect to
  terminal.
