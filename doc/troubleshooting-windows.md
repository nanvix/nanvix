# Troubleshooting (Windows)

This document provides guidance for diagnosing and resolving common issues encountered when
developing, building, or running Nanvix on Windows 11.

## Table of Contents

- [Windows Hypervisor Platform Not Enabled](#windows-hypervisor-platform-not-enabled)
- [Docker Build Fails with Symlink Errors](#docker-build-fails-with-symlink-errors)
- [Stale `.venv` Directory Blocks Docker Build](#stale-venv-directory-blocks-docker-build)
- [Cleaning on Windows](#cleaning-on-windows)

---

## Windows Hypervisor Platform Not Enabled

**Description:** The UserVM fails to start because WHP is not enabled.

**Symptom:** Error messages mentioning WHP or hypervisor platform when running `uservm.exe`.

**Fix:**

```powershell
# Run in an elevated PowerShell prompt.
Enable-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform -All
# Restart your machine after enabling the feature.
```

Verify it is enabled:

```powershell
Get-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform
```

## Docker Build Fails with Symlink Errors

**Description:** The repository uses symbolic links. If cloned without `core.symlinks=true` or
without Developer Mode enabled, Git checks out symlinks as small text files. The `z.ps1` script
attempts to restore these as file copies before Docker builds, but issues may arise if the
restoration fails.

**Fix:** Re-clone with symlinks enabled (requires [Developer Mode](setup-windows.md#2-enable-developer-mode)):

```powershell
git clone -c core.symlinks=true https://github.com/nanvix/nanvix.git
```

If you must repair an existing clone, back up or stash any local changes first because refreshing
the worktree will overwrite unstaged files:

```powershell
git config core.symlinks true
git checkout -- .
```

## Stale `.venv` Directory Blocks Docker Build

**Description:** A previous Docker build may leave a broken `.venv` directory with Linux symlinks
(e.g., `lib64 -> lib`) that Windows cannot handle, causing the Docker output exporter to fail.

**Fix:** Remove the `.venv` directory before building:

```powershell
# cmd handles broken reparse points more reliably.
cmd /c "rmdir /s /q .venv"
```

> **Note:** The `z.ps1` script performs this cleanup automatically before each Docker build.

## Cleaning on Windows

```powershell
# Quick clean (removes UserVM build artifacts and cache).
.\z.ps1 clean

# Full clean (cargo clean + remove all build artifacts).
.\z.ps1 distclean
```
