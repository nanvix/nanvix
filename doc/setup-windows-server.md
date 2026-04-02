# Setting Up Your Development Environment (Windows Server 2022)

This guide covers setting up the Nanvix development environment on Windows Server 2022/2025
Datacenter. The workflow is identical to Windows 11 except that `z.ps1 setup` automatically
installs [Chocolatey](https://chocolatey.org/) and uses it as the package manager instead of
winget.

## Table of Contents

- [1. Enable Developer Mode](#1-enable-developer-mode)
- [2. Enable Windows Hypervisor Platform](#2-enable-windows-hypervisor-platform)
- [3. Clone This Repository](#3-clone-this-repository)
- [4. Run Setup](#4-run-setup)
- [5. Remote Development with VS Code Tunnel (Optional)](#5-remote-development-with-vs-code-tunnel-optional)

---

## 1. Enable Developer Mode

Run in an **elevated PowerShell prompt** (once per machine):

```powershell
reg add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock" `
    /t REG_DWORD /f /v AllowDevelopmentWithoutDevLicense /d 1
```

## 2. Enable Windows Hypervisor Platform

Run in an **elevated PowerShell prompt** (once per machine):

```powershell
Enable-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform -All
```

Restart the machine after enabling the feature.

> **Note:** The VM must support nested virtualization to use WHP. On Azure, use Dv5, Ev5, or
> newer series (e.g., `Standard_D4s_v5`).

## 3. Clone This Repository

```powershell
$env:WORKDIR = "$env:USERPROFILE\nanvix"                                 # Set the directory for the source tree.
New-Item -ItemType Directory -Path $env:WORKDIR -Force; cd $env:WORKDIR  # Create the workspace and navigate to it.
git clone -c core.symlinks=true https://github.com/nanvix/nanvix.git     # Clone the repository.
cd nanvix                                                                # Navigate to the Nanvix source tree.
```

> **Note:** The `-c core.symlinks=true` flag ensures Git creates native symlinks instead of text
> stub files. This requires Developer Mode to be enabled. If you already cloned without this flag,
> prefer re-cloning after enabling Developer Mode so your existing worktree is not overwritten.

## 4. Run Setup

```powershell
.\z.ps1 setup
```

On Windows Server, this command automatically detects the server environment and:

1. Installs [Chocolatey](https://chocolatey.org/) (if not already installed).
2. Installs Git via Chocolatey (if not already installed).
3. Installs Python 3.12 via Chocolatey (if not already installed).
4. Installs GNU Make via Chocolatey (if not already installed).
5. Checks for Visual Studio Build Tools (warns if missing).
6. Installs the Rust toolchain via rustup (if not already installed).
7. Configures the repository Git hooks from `.githooks`.

> **Note:** `z.ps1 setup` emits a non-fatal warning about the Windows build number being below
> 22000. This is expected on Server 2022 and does not affect functionality.

---

## 5. Remote Development with VS Code Tunnel (Optional)

Windows Server environments are typically headless. You can use
[VS Code Tunnel](https://code.visualstudio.com/docs/remote/tunnels) to connect your local IDE to
the server without opening inbound ports.

### Install the VS Code CLI

```powershell
New-Item -ItemType Directory -Path "$env:USERPROFILE\bin" -Force | Out-Null
$ProgressPreference = 'SilentlyContinue'
Invoke-WebRequest -Uri "https://code.visualstudio.com/sha/download?build=stable&os=cli-win32-x64" `
    -OutFile "$env:TEMP\vscode.zip" -UseBasicParsing
Expand-Archive "$env:TEMP\vscode.zip" -DestinationPath "$env:TEMP\vscode" -Force
Move-Item "$env:TEMP\vscode\code.exe" "$env:USERPROFILE\bin\code.exe" -Force
Remove-Item "$env:TEMP\vscode.zip", "$env:TEMP\vscode" -Recurse -Force -ErrorAction SilentlyContinue
```

Add `$env:USERPROFILE\bin` to your PATH:

```powershell
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$env:USERPROFILE\bin*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$env:USERPROFILE\bin", "User")
}
```

Reopen your PowerShell session, then start the tunnel:

```powershell
code.exe tunnel
```

Follow the authentication prompts. Once the tunnel is active, open VS Code on your local machine
and connect to the server via the **Remote - Tunnels** extension.
