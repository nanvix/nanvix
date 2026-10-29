# Troubleshooting (Linux)

This document provides guidance for diagnosing and resolving common issues encountered when
developing, building, or running Nanvix on Linux.

## Table of Contents

- [Resource Leaks](#resource-leaks)
  - [Temporary Directories (`nvx:*`)](#temporary-directories-nvx)
  - [Test Directories (`nanvix-test-*`)](#test-directories-nanvix-test-)
  - [Gateway Sockets (`nvx-standalone-gw-*.sock`)](#gateway-sockets-nvx-standalone-gw-sock)
  - [Complete Cleanup Script](#complete-cleanup-script)
- [Cross-Platform Issues](#cross-platform-issues)
  - [Cached Build Options Stale (`./z`)](#cached-build-options-stale-z)
- [CI/CD Failures](#cicd-failures)
  - [Stale Resources Causing Test Failures](#stale-resources-causing-test-failures)
  - [TIME_WAIT Socket Connections](#time_wait-socket-connections)

---

## Resource Leaks

Nanvix creates various temporary resources during operation. Under normal circumstances, these
resources are automatically cleaned up by their owners. However, crashes or abnormal termination
may leave stale resources behind that require manual cleanup.

### Temporary Directories (`nvx:*`)

**Description:** Runtime temporary directories created by `nanvixd` for each sandbox instance.

**Location:** `/tmp/nvx:*`

**Primary Cleanup:** `TemporaryDirectory::Drop` in `src/utils/nanvixd/src/tempdir.rs`

**Manual Cleanup:**

```bash
sudo rm -rf /tmp/nvx:*
```

### Gateway Sockets (`nvx-standalone-gw-*.sock`)

**Description:** Unix domain sockets that `nanvixd` binds in HTTP mode, one for each User VM,
through which a host-side consumer (for instance, the containerd shim or `nanvix-test`) exchanges
the standard input and output of the guest. Unless `-gateway-sockaddr` gives another path, the
socket of a User VM is bound at `/tmp/nvx-standalone-gw-<pid>.sock`, where `<pid>` is the process ID
of `nanvixd`.

**Location:** `/tmp/nvx-standalone-gw-*.sock`

**Primary Cleanup:** `GatewayEndpointPath::Drop` in `src/libs/nanvix-http/src/client/standalone.rs`,
which runs whenever a User VM is torn down: when it is killed, and when `nanvixd` shuts down
gracefully on `SIGINT`, `SIGTERM`, `SIGHUP`, or `SIGQUIT`. Gateway sockets leak only when `nanvixd`
is killed with `SIGKILL` or crashes.

**Manual Cleanup:** Make sure that no `nanvixd` process is running first, as the commands below also
match the sockets of running instances.

```bash
# List leaked gateway sockets.
./scripts/cleanup-nanvixd.sh --list-sockets

# Remove leaked gateway sockets.
./scripts/cleanup-nanvixd.sh --sockets
```

### Complete Cleanup Script

To clean all stale Nanvix resources at once:

```bash
#!/bin/bash
# Clean temporary directories
sudo rm -rf /tmp/nvx:*
rm -rf /tmp/nanvix-test-*

# Clean gateway sockets
rm -f /tmp/nvx-standalone-gw-*.sock

echo "Cleanup complete."
```

## CI/CD Failures

### Stale Resources Causing Test Failures

If CI tests fail with errors related to resources already existing or ports being in use, stale
resources from a previous failed run may be the cause. The CI workflow includes a defensive cleanup
step that runs after every job (see `.github/workflows/self-hosted-ci.yml`), but manual
intervention may be needed on self-hosted runners.

### TIME_WAIT Socket Connections

After HTTP-mode runs, TCP connections may linger in `TIME_WAIT` state. The test framework waits for
these to clear before starting new tests. If tests timeout waiting for port availability:

```bash
# Check for lingering connections on the nanvixd port (default 8181)
ss -tan | grep 8181

# Wait for connections to clear (typically 60-120 seconds)
```
