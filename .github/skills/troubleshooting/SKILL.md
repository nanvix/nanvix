---
name: troubleshooting
description: Guide for diagnosing Nanvix build, runtime, and test failures, including cleanup and debugging workflows. Use this when asked to investigate errors or unstable behavior.
---

# Troubleshooting Nanvix

Use this skill when the user reports problems, errors, or unexpected behavior while developing,
building, or running Nanvix. This covers common failure modes and their resolutions.

## Resource Leaks and Cleanup

Crashes or abnormal termination may leave stale resources.  Clean them up with:

### Quick Cleanup

```bash
# Remove all stale Nanvix resources.
sudo rm -rf /tmp/nvx:*
rm -rf /tmp/nanvix-test-*
sudo rm -rf /tmp/clh-console
sudo rm -f /tmp/cloud-hypervisor.sock
sudo rm -f /tmp/*.socket

# Clean up stale network namespaces (L2 deployments).
for ns in $(sudo ip netns list 2>/dev/null \
    | grep -o 'nvxns-[0-9]*' || true); do
  sudo ip link del "nvxgw-h-${ns#nvxns-}" \
      2>/dev/null || true
  sudo ip netns del "${ns}" 2>/dev/null || true
done
```

There is also a cleanup script for `nanvixd` resources:

```bash
./scripts/cleanup-nanvixd.sh
```

### Resource Types

| Resource           | Location            | Created By     |
|--------------------|---------------------|----------------|
| Temp directories   | `/tmp/nvx:*`        | `nanvixd`      |
| Test directories   | `/tmp/nanvix-test-*`| Unit tests     |
| Unix sockets       | `/tmp/*.socket`     | `syscomm`      |
| Network namespaces | `nvxns-*`           | L2 deployments |
| Cloud Hypervisor   | `/tmp/clh-console`  | L2 snapshot    |

## Build Failures

### Toolchain Not Found

**Symptom**: Cargo toolchain errors or missing tools.

**Fix**: Ensure the `toolchain/` symlink points to a valid installation:

```bash
ls -la toolchain/
# If missing, rebuild:
./z setup --toolchain-dir $HOME/toolchain
ln -s $HOME/toolchain toolchain
```

### Full Reset (Last Resort)

**Symptom**: Build or test issues persist after normal cleanup.

**Fix**: Perform a full cleanup, then rebuild:

```bash
./z distclean

./z build -- all
```

### sccache Crashes

**Symptom**: Intermittent build failures with sccache errors.

**Fix**: sccache is automatically disabled for non-artifact targets. For manual builds, unset it:

```bash
./z build -- all SCCACHE=
```

## Runtime Failures

### KVM Not Available

**Symptom**: `Could not access KVM kernel module` or
similar.

**Fix**:

```bash
sudo kvm-ok
sudo lsmod | grep kvm
sudo usermod -aG kvm $USER
newgrp kvm
```

### Port Already in Use

**Symptom**: `Address already in use` when starting `nanvixd` in HTTP mode.

**Fix**: Kill existing processes or use a different port:

```bash
lsof -i :8080
kill <PID>
# Or use different port:
./bin/nanvixd.elf -http-addr 127.0.0.1:9090
```

### TIME_WAIT Socket Connections

**Symptom**: After L2 deployments, connections linger.

**Fix**: Check and wait for them to clear:

```bash
ss -tan | grep 8181
```

### Tests Fail Due to Stale Resources

**Symptom**: CI tests fail with resource-exists or port-in-use errors.

**Fix**: Run the full cleanup script (see Resource Leaks section above).

## Debugging

### Enable Verbose Logging

```bash
# Nanvixd daemon logging.
RUST_LOG=debug ./bin/nanvixd.elf \
    -console-file /dev/stdout \
    -- ./bin/hello-rust-nostd.elf
RUST_LOG=trace ./bin/nanvixd.elf \
    -http-addr 127.0.0.1:8080

# Build with trace-level guest logging.
./z build -- all LOG_LEVEL=trace
```

### Standalone UserVM Debugging

Bypass the full stack for low-level kernel debugging:

```bash
RUST_LOG=debug ./bin/uservm.elf \
    -kernel ./bin/kernel.elf \
    -initrd ./bin/hello-rust-nostd.elf \
    -standalone
```

### Check Build Errors

```bash
# Rust check with JSON output for IDE integration.
./z build -- check
```

### Log Files

Runtime logs are stored under `logs/` with timestamped
directory names (e.g., `logs/2026_02_24_13_04_54/`).

## Windows-Specific Issues

### WHP Not Enabled

**Symptom**: UserVM fails to start with hypervisor-related errors.

**Fix**:

```powershell
# Run in an elevated PowerShell prompt.
Enable-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform -All
# Restart the machine.
```

### Docker Build Fails on Windows

**Symptom**: Docker build fails with symlink or file-access errors, or the Docker output exporter
fails because a previous build left a `.venv` directory with Linux symlinks (`lib64 -> lib`) that
Windows cannot handle.

**Fix**:

1. Ensure Docker Desktop is running with Linux containers enabled: `docker info`.
2. Remove stale `.venv` directory (cmd handles broken reparse points more reliably):
   ```powershell
   cmd /c "rmdir /s /q .venv"
   ```
3. Delete build cache and retry:
    `Remove-Item .z.cache -ErrorAction SilentlyContinue; .\z.ps1 build -- all`.

> **Note:** The `z.ps1` script removes `.venv` automatically before each Docker build.

### Symlink Errors on Windows Clone

**Symptom**: Build or tool errors because Git checked out symlinks as text files.

**Fix**: Re-clone with Developer Mode enabled and symlinks flag:

```powershell
git clone -c core.symlinks=true https://github.com/nanvix/nanvix.git
```

To repair an existing clone (including an optional backup of local changes):

```powershell
# Optional: back up any local changes before resetting the working tree.
git status
# Either commit your work-in-progress, or stash it:
# git commit -am "WIP backup"
# git stash -u

git config core.symlinks true
git checkout -- .
```

### Cached Build Options Stale (Windows)

**Symptom**: Build uses wrong parameters on Windows.

**Fix**: Delete the cache file and rebuild:

```powershell
Remove-Item .z.cache -ErrorAction SilentlyContinue
.\z.ps1 build -- all
```
