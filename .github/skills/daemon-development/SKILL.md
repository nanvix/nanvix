---
name: daemon-development
description: Guide for developing and debugging Nanvix daemons, including guest daemons and host linuxd behavior. Use this when asked about daemon architecture or daemon changes.
---

# Daemon Development

Use this skill when the user asks about developing, modifying, or debugging system daemons in
Nanvix. Daemons are long-running system services that run in user-space and provide core OS
functionality.

## Daemon Overview

| Daemon   | Path                  | Target | Purpose              |
|----------|-----------------------|--------|----------------------|
| `memd`   | `src/daemons/memd/`   | Guest  | Memory management.   |
| `procd`  | `src/daemons/procd/`  | Guest  | Process management.  |
| `linuxd` | `src/daemons/linuxd/` | Host   | L2 VM management.    |

## Guest Daemons (`memd`, `procd`)

Guest daemons are `#![no_std]`, `#![no_main]` Rust binaries that run inside the Nanvix microkernel
environment. They communicate with the kernel through kernel calls (`sys::kcall`) and handle system
events/messages.

### Memory Daemon (`memd`)

- Handles page faults by terminating faulting processes and resuming exception events.
- Processes IPC requests for memory management.
- Uses `sys::kcall::pm::terminate()` and `sys::kcall::event::resume()`.

### Process Daemon (`procd`)

- Manages process lifecycle (creation, termination, scheduling).
- Handles IPC-based process management requests.

## Host Daemon (`linuxd`)

- Runs on the host Linux system with full `std` support.
- Manages L2 VM deployment and host-side resources.
- Configuration in `build/linuxd_config.toml`.
- Only built for `microvm` machine.

## Building Daemons

```bash
# Build all daemons as part of the full build.
./z build -- all

# Guest daemons: GUEST_CARGO_BUILD_CMD.
# Host daemons (linuxd): HOST_CARGO_BUILD_CMD.
```

## Creating a New Guest Daemon

1. Create directory at `src/daemons/<name>/`.

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
   sys = { workspace = true }
   syslog = { workspace = true }
   # Additional dependencies as needed.
   ```

3. Create `src/main.rs` with:

   ```rust
   // Copyright(c) The Maintainers of Nanvix.
   // Licensed under the MIT License.

   #![no_std]
   #![no_main]

   extern crate alloc;

   // Daemon implementation.
   ```

4. Add the crate to the workspace `members` in root
   `Cargo.toml`.

5. Add the daemon name to `ALL_GUEST_DAEMONS` in the
   `Makefile`.

## Coding Rules (Daemon-Specific)

- Guest daemons must be `#![no_std]` and `#![no_main]`.
- Use kernel calls (`sys::kcall::*`) for system interactions.
- Handle IPC messages by parsing `SystemMessage` structures.
- Always handle errors gracefully — daemons must not crash.
- Log all errors with `error!` before returning.
- Use the `syslog` crate for logging within guest daemons.
