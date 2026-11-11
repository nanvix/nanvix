# Setting Up Your Development Environment (Windows)

This guide will help you set up your development environment to build and run Nanvix on Windows.

## Table of Contents

- [1. Check Your System and Permissions](#1-check-your-system-and-permissions)
- [2. Enable Developer Mode](#2-enable-developer-mode)
- [3. Clone This Repository](#3-clone-this-repository)
- [4. Enable Windows Hypervisor Platform](#4-enable-windows-hypervisor-platform)
- [5. Run Setup](#5-run-setup)
- [6. Verus (Formal Verification, Optional)](#6-verus-formal-verification-optional)
- [7. Setup Your IDE (Optional)](#7-setup-your-ide-optional)
  - [Visual Studio Code](#visual-studio-code)

---

## 1. Check Your System and Permissions

- Ensure you are running Windows 11
- Ensure you have administrator privileges

## 2. Enable Developer Mode

This repository uses symbolic links. On Windows, Git needs Developer Mode enabled to create native
symlinks during clone. Without it, symlinks are checked out as small text files and tools that
expect real files may fail.

1. Open **Settings > System > Advanced > For developers**.
2. Turn on **Developer Mode**.

Using the Settings app may prompt for approval, but it does not require you to manually open an
elevated PowerShell session.

Alternatively, enable it from PowerShell. This command does require an elevated PowerShell
session:

```powershell
reg add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock" /t REG_DWORD /f /v AllowDevelopmentWithoutDevLicense /d 1
```

## 3. Clone This Repository

```powershell
$env:WORKDIR = "$env:USERPROFILE\nanvix"                                 # Set the directory for the source tree.
New-Item -ItemType Directory -Path $env:WORKDIR -Force; cd $env:WORKDIR  # Create the workspace and navigate to it.
git clone -c core.symlinks=true https://github.com/nanvix/nanvix.git     # Clone the repository.
cd nanvix                                                                # Navigate to the Nanvix source tree.
```

> **Note:** The `-c core.symlinks=true` flag ensures Git creates native symlinks instead of text
> stub files. This requires [Developer Mode](#2-enable-developer-mode) to be enabled. If you
> already cloned without this flag, prefer re-cloning after enabling Developer Mode so your
> existing worktree is not overwritten.
>
> **Note:** Cloning the repository does not require an elevated PowerShell session once Developer
> Mode is enabled.

## 4. Enable Windows Hypervisor Platform

The Windows Hypervisor Platform (WHP) feature is required to run the UserVM natively. This step
requires an elevated PowerShell session.

```powershell
# Run in an elevated PowerShell prompt.
Enable-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform -All
```

Restart your machine after enabling the feature.

## 5. Run Setup

```powershell
.\z.ps1 setup
```

This command validates prerequisites and configures the development environment:

1. Verifies you are running Windows 11.
2. Verifies Developer Mode is enabled.
3. Verifies the Windows Hypervisor Platform is active.
4. Installs Git via winget (if not already installed).
5. Installs Python 3.12 via winget (if not already installed).
6. Installs GNU Make via winget (if not already installed).
7. Checks for Visual Studio Build Tools (warns if missing).
8. Installs LLVM/Clang via winget (if not already installed).
9. Installs the Rust toolchain via winget (if not already installed).
10. Installs ShellCheck via winget (if not already installed).
11. Configures the repository Git hooks from `.githooks`.

> **Note:** If winget is not available, install the prerequisites manually before running setup:
>
> - Git: `winget install Git.Git` (see [git-scm.com](https://git-scm.com))
> - Python 3.10+: `winget install Python.Python.3.12` (see [python.org](https://www.python.org))
> - GNU Make: `winget install ezwinports.make`
> - LLVM/Clang: `winget install LLVM.LLVM`
> - Rust: `winget install Rustlang.Rustup` (see [rustup.rs](https://rustup.rs))
> - ShellCheck: `winget install koalaman.shellcheck`
>
> **Note:** ShellCheck is required only to lint shell scripts (the `lint` and `lint-check` targets;
> the pre-commit hook runs `lint-check`). If setup cannot install it, it prints a warning and
> continues.

---

## 6. Verus (Formal Verification, Optional)

Verus formal verification is optional. When `VERUS_EXECUTABLE_DIR` is not set, verification
is skipped.

The easiest way to install Verus is via the setup command:

```powershell
.\z.ps1 setup --verus
```

This installs the pinned Verus release to `%USERPROFILE%\verus`. You can also invoke the
setup script directly to choose a custom install directory:

```powershell
python .\scripts\setup\verus.py C:\verus
```

The script downloads the prebuilt Windows binary from the Verus GitHub releases page,
validates the archive, and installs it to the given directory. The default setup location is
used automatically:

```powershell
.\z.ps1 build -- verify
```

Set `VERUS_EXECUTABLE_DIR` when using a custom installation directory:

```powershell
.\z.ps1 build -- verify VERUS_EXECUTABLE_DIR=C:\verus
```

> **Note:** The expected version is pinned in `build/verus-version`. Re-run `setup --verus`
> or the setup script after version bumps to update the installation.

---

## 7. Setup Your IDE (Optional)

Choose one of the following options to set up your IDE for Nanvix development.

### Visual Studio Code

Use the host-specific templates below. The settings template invokes `./z.bat` and routes Rust
Analyzer build-script discovery through the Windows build workflow. It also enables the `whp`
feature, like Windows builds do, so that Rust Analyzer analyzes the WHP backend of the UserVM. The
other templates add tasks that run `z.ps1`, debug configurations for the UserVM, and a list of
recommended extensions.

```powershell
New-Item -ItemType Directory -Path .vscode -Force
Copy-Item scripts\setup\vscode\settings-windows.json .vscode\settings.json
Copy-Item scripts\setup\vscode\tasks-windows.json .vscode\tasks.json
Copy-Item scripts\setup\vscode\launch-windows.json .vscode\launch.json
Copy-Item scripts\setup\vscode\extensions-windows.json .vscode\extensions.json
```

Then run **Extensions: Show Recommended Extensions** from the Command Palette and install the
recommended extensions: Rust Analyzer, C/C++ (which provides the `cppvsdbg` debugger), Even Better
TOML, PowerShell, Python, and ShellCheck.

#### Tasks

Run tasks with **Terminal > Run Task**. Each task runs `z.ps1` in Windows PowerShell with the
default build parameters. To change them, append parameters such as `RELEASE=yes` to the `args` of
the task in `.vscode\tasks.json`.

| Task                 | Command                             |
| -------------------- | ----------------------------------- |
| `Build: all`         | `.\z.ps1 build -- all`              |
| `Build: uservm`      | `.\z.ps1 build -- all-uservm`       |
| `Build: nanvixd`     | `.\z.ps1 build -- all-nanvixd`      |
| `Run: guest program` | `.\z.ps1 run -- -program <program>` |
| `Test: all`          | `.\z.ps1 test`                      |
| `Test: unit`         | `.\z.ps1 build -- run-unit-tests`   |
| `Test: integration`  | `.\z.ps1 build -- run-nanvix-tests` |

`Build: all` is the default build task (`Ctrl+Shift+B`), and `Test: all` is the default test task.
`Run: guest program` prompts for the program to run and does not build anything, so run
`Build: all` first.

#### Debugging

The launch template debugs the host process with `cppvsdbg`, the native Windows debugger of the
C/C++ extension. `uservm.exe` and `nanvixd.exe` are built with the MSVC toolchain and carry PDB
debug information, which CodeLLDB supports only partially. Before you debug, build Nanvix once with
the `Build: all` task, so that `bin\` holds the kernel and the guest programs.

| Configuration                 | Debugs                                                    |
| ----------------------------- | --------------------------------------------------------- |
| `Debug UserVM (uservm.exe)`   | `bin\uservm.exe -kernel bin\kernel.elf -initrd <program>` |
| `Debug nanvixd (nanvixd.exe)` | `bin\nanvixd.exe -- <program>`                            |

Both configurations prompt for the guest program (`bin/hello-rust-nostd.elf` by default) and rebuild
the binary that they debug before they start. `nanvixd.exe` runs the VM in its own process through
the UserVM library, so breakpoints in that library are hit in both configurations. Code in
`src\uservm\src\main.rs` only runs in `uservm.exe`.

Debug information is only available in debug builds, the default. Release builds (`RELEASE=yes`)
are stripped. Debug builds are optimized as well (`opt-level = 3` in the `dev` profile of
`Cargo.toml`), so some lines have no code that a breakpoint can bind to, and some variables are
optimized out.
