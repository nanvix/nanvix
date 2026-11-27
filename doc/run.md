# Running Nanvix

> **Prerequisite:** You must build Nanvix before running it. See [build](build.md)
for instructions.

This document guides you through running Nanvix. Choose the guide that matches your development
environment:

- **[Linux](run-linux.md)** — Full runtime workflow on Linux. Supports interactive mode, HTTP mode,
  container integration, and standalone User VM debugging. Uses KVM as the backend.

- **[Windows](run-windows.md)** — Standalone interactive mode on Windows 11. Uses WHP as the
backend.

## Standalone Terminal Output

On both hosts, guest application output is drained independently of blocking writes to host
stdout. A 64 MiB byte queue absorbs host back-pressure; exhausting this queue reports an error
instead of dropping application bytes or allowing unbounded memory growth. The existing bounded
VM channels and small writer staging buffers are additional to this queue budget.

Output remains ordered, prompts are flushed when idle, and pending output is flushed before EOF,
including after the guest exits. Host stdout must eventually become writable for lossless
completion. The Windows debug-console sink uses the same host-output buffering module but retains
its separate 4 MiB, best-effort policy: excess debug bytes may be dropped and counted.
Host-output errors, including closed pipes, request VM shutdown and are returned to the caller
after the vCPU has stopped.
