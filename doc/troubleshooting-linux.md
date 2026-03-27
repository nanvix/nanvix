# Troubleshooting (Linux)

This document provides guidance for diagnosing and resolving common issues encountered when
developing, building, or running Nanvix on Linux.

## Table of Contents

- [Resource Leaks](#resource-leaks)
  - [Temporary Directories (`nvx:*`)](#temporary-directories-nvx)
  - [Test Directories (`nanvix-test-*`)](#test-directories-nanvix-test-)
  - [Unix Sockets (`*.socket`)](#unix-sockets-socket)
  - [Network Namespaces (`nvxns-*`)](#network-namespaces-nvxns-)
  - [Cloud Hypervisor Resources](#cloud-hypervisor-resources)
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

### Test Directories (`nanvix-test-*`)

**Description:** Temporary directories created by unit tests in `nanvix-sandbox-cache`.

**Location:** `/tmp/nanvix-test-*`

**Primary Cleanup:** `TempTestDir::Drop` in `src/libs/nanvix-sandbox-cache/src/lib.rs`

**Manual Cleanup:**

```bash
rm -rf /tmp/nanvix-test-*
```

### Unix Sockets (`*.socket`)

**Description:** Unix domain sockets used for inter-process communication between nanvixd
components.

**Location:** `/tmp/*.socket`

**Primary Cleanup:** `SocketListener::Drop` in `src/libs/syscomm/src/socket_listener.rs`

**Manual Cleanup:**

```bash
sudo rm -f /tmp/*.socket
```

### Network Namespaces (`nvxns-*`)

**Description:** Linux network namespaces created for L2 deployments to isolate VM networking.

**Primary Cleanup:** `NetnsHandle::Drop` → `delete_namespace()` in
`src/libs/nanvix-sandbox/src/netns.rs`

**Manual Cleanup:**

```bash
# List stale namespaces
sudo ip netns list | grep 'nvxns-'

# Delete a specific namespace and its veth pair
sudo ip link del nvxgw-h-<ID>
sudo ip netns del nvxns-<ID>

# Delete all stale namespaces
for ns in $(sudo ip netns list | grep -o 'nvxns-[0-9]*'); do
  sudo ip link del "nvxgw-h-${ns#nvxns-}" 2>/dev/null || true
  sudo ip netns del "${ns}" 2>/dev/null || true
done
```

### Cloud Hypervisor Resources

**Description:** Resources created by Cloud Hypervisor during L2 snapshot generation.

**Location:**

- `/tmp/clh-console` - Console output directory
- `/tmp/cloud-hypervisor.sock` - Cloud Hypervisor API socket

**Primary Cleanup:** EXIT trap in `scripts/generate-l2-snapshot.sh`

**Manual Cleanup:**

```bash
sudo rm -rf /tmp/clh-console
sudo rm -f /tmp/cloud-hypervisor.sock
```

### Complete Cleanup Script

To clean all stale Nanvix resources at once:

```bash
#!/bin/bash
# Clean temporary directories
sudo rm -rf /tmp/nvx:*
rm -rf /tmp/nanvix-test-*

# Clean Cloud Hypervisor resources
sudo rm -rf /tmp/clh-console
sudo rm -f /tmp/cloud-hypervisor.sock

# Clean Unix sockets
sudo rm -f /tmp/*.socket

# Clean network namespaces
for ns in $(sudo ip netns list 2>/dev/null | grep -o 'nvxns-[0-9]*' || true); do
  sudo ip link del "nvxgw-h-${ns#nvxns-}" 2>/dev/null || true
  sudo ip netns del "${ns}" 2>/dev/null || true
done

echo "Cleanup complete."
```

## CI/CD Failures

### Stale Resources Causing Test Failures

If CI tests fail with errors related to resources already existing or ports being in use, stale
resources from a previous failed run may be the cause. The CI workflow includes a defensive cleanup
step that runs after every job (see `.github/workflows/self-hosted-ci.yml`), but manual
intervention may be needed on self-hosted runners.

### TIME_WAIT Socket Connections

After L2 deployments, TCP connections may linger in `TIME_WAIT` state. The test framework waits for
these to clear before starting new tests. If tests timeout waiting for port availability:

```bash
# Check for lingering connections on the nanvixd port (default 8181)
ss -tan | grep 8181

# Wait for connections to clear (typically 60-120 seconds)
```
