# Troubleshooting (Windows)

This document provides guidance for diagnosing and resolving common issues encountered when
developing, building, or running Nanvix on Windows 11.

## Table of Contents

- [Windows Hypervisor Platform Not Enabled](#windows-hypervisor-platform-not-enabled)
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

## Cleaning on Windows

```powershell
# Quick clean (removes UserVM build artifacts and cache).
.\z.ps1 clean

# Full clean (cargo clean + remove all build artifacts).
.\z.ps1 distclean
```
