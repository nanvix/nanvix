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
| `hello-c`          | `src/user/hello-c/`          | C          | Newlib     |
| `hello-cpp`        | `src/user/hello-cpp/`        | C++        | Libstdc++  |
| `hello-rust-nostd` | `src/user/hello-rust-nostd/` | Rust       | Bare-metal |
| `hello-js`         | `src/user/hello-js/`         | JavaScript | QuickJS    |
| `hello-python`     | `src/user/hello-python/`     | Python     | Python 3   |
| `hello-wasm`       | `src/user/hello-wasm/`       | Rust/WASM  | wasmd      |
| `webpage-js`       | `src/user/webpage-js/`       | JavaScript | QuickJS    |

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
    -- ./bin/echo-c.elf arg1 arg2

# Pass arguments and environment variables.
./bin/nanvixd.elf \
    -console-file /dev/stdout \
    -- ./bin/echo-c.elf \
    "arg1 arg2;VAR1=foo VAR2=bar"
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
             "program":"./bin/hello-c.elf",
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
4. The application links against `libposix.a`, `libc.a`, and the custom linker script at
   `build/user/linker/x86/user.ld`.

## Creating a WASM Application

1. Create directory at `src/user/<name>/` or `src/benchmarks/<name>/`.
2. Set target to `wasm32-wasip1` in `Cargo.toml`.
3. Add to `ALL_WASM_BINARIES` in the `Makefile`.
4. WASM binaries are executed by the `wasmd` daemon.

## Logging

- Use `RUST_LOG` environment variable for `nanvixd` daemon-level logging.
- Guest-side logging uses the `syslog` crate (`error!`, `warn!`, `info!`, `debug!`, `trace!`).
- Console output defaults to `logs/` directory; use `-console-file /dev/stdout` to redirect to
  terminal.
