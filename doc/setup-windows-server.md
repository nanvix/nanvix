# Setting Up Your Development Environment (Windows Server 2022)

This guide covers setting up the Nanvix development environment on Windows Server 2022/2025 Datacenter.
Phase 1 installs system-wide prerequisites in an elevated PowerShell session. Phase 2 installs
per-user tools and configures the repository as a regular user.

## Phase 1: Administrator Setup

Run in an **elevated PowerShell prompt** (once per machine). This installs Chocolatey, Git,
Python 3.12, GNU Make, Visual Studio Build Tools (C++ workload), enables Developer Mode, and
enables the Windows Hypervisor Platform.

```powershell
.\scripts\setup\windows-server-admin.ps1
```

Reboot after running if Windows Hypervisor Platform was just enabled.

> **Note:** The VM must support nested virtualization to use WHP. On Azure, use Dv5, Ev5, or
> newer series (e.g., `Standard_D4s_v5`).

## Phase 2: User Setup

Run in a **regular (non-elevated) PowerShell prompt** (once per user). This installs the Rust
toolchain under the current user profile, clones the Nanvix repository with symlink support, and
runs `z.ps1 setup` to validate the environment and configure Git hooks.

```powershell
.\scripts\setup\windows-server-user.ps1
```

## 3. Clone This Repository

```powershell
$env:WORKDIR = "$env:USERPROFILE\nanvix"                                 # Set the directory for the source tree.
New-Item -ItemType Directory -Path $env:WORKDIR -Force; cd $env:WORKDIR  # Create the workspace and navigate to it.
git clone -c core.symlinks=true https://github.com/nanvix/nanvix.git     # Clone the repository.
cd nanvix                                                                # Navigate to the Nanvix source tree.
```

> **Note:** The `-c core.symlinks=true` flag ensures Git creates native symlinks instead of text
> stub files. This requires Developer Mode to be enabled (Phase 1). If you already cloned without
> this flag, prefer re-cloning after enabling Developer Mode so your existing worktree is not
> overwritten.

## 4. Run Setup

```powershell
.\z.ps1 setup
```

This validates the development environment and configures the repository Git hooks from
`.githooks`.

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
