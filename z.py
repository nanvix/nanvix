#!/usr/bin/env python3

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Nanvix Unified Build System Backend.

This script is the single backend that drives all Nanvix build operations on both Linux and
Windows. The shell wrappers (z, z.ps1, z.bat) are thin shims that invoke this script.

All builds are driven through the project Makefile.

Requirements: Python 3.10+, standard library only.
"""

from __future__ import annotations

import os
import platform
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import NoReturn, Sequence

# ==================================================================================================
# Constants
# ==================================================================================================

VALID_MACHINES: tuple[str, ...] = ("microvm",)
VALID_LOG_LEVELS: tuple[str, ...] = ("trace", "debug", "info", "warn", "error", "panic")
VALID_TARGETS: tuple[str, ...] = ("aarch64", "x86", "x86_64")
VALID_MESSAGE_FORMATS: tuple[str, ...] = ("json", "json-diagnostic-rendered-ansi")

DEFAULT_MACHINE = "microvm"
DEFAULT_TARGET = "x86"
DEFAULT_LOG_LEVEL_DEBUG = "trace"
DEFAULT_LOG_LEVEL_RELEASE = "warn"
DEFAULT_TIMEOUT = 600
DEFAULT_MEMORY_SIZE = 128
DEFAULT_IMAGE = "nanvix.img"
DEFAULT_RUN_PROGRAM = "bin/hello-rust-nostd.elf"

# Known Make variables that z.py understands and may inject.
KNOWN_MAKE_VARS: frozenset[str] = frozenset(
    {
        "TARGET",
        "MACHINE",
        "RELEASE",
        "LOG_LEVEL",
        "TIMEOUT",
        "PROFILER",
        "WHP",
        "IMAGE",
        "HOST_CPU",
        "MAKE_NO_PRINT",
        "MEMORY_SIZE",
        "MESSAGE_FORMAT",
        "VERBOSE",
        "SCCACHE",
        "SYSROOT_DIR",
        "VERUS_EXECUTABLE_DIR",
    }
)

# ==================================================================================================
# Logging
# ==================================================================================================

_RED = "\033[31m"
_GREEN = "\033[32m"
_YELLOW = "\033[33m"
_CYAN = "\033[36m"
_RESET = "\033[0m"


def _supports_color() -> bool:
    """Check if the terminal supports ANSI color codes."""
    if os.environ.get("NO_COLOR"):
        return False
    if sys.platform == "win32":
        # Windows Terminal and modern consoles support ANSI.
        return True
    return hasattr(sys.stderr, "isatty") and sys.stderr.isatty()


_COLOR = _supports_color()


def _c(code: str, text: str) -> str:
    return f"{code}{text}{_RESET}" if _COLOR else text


def print_error(msg: str) -> None:
    """Print error message to stderr."""
    print(f"{_c(_RED, '[ERROR]')} {msg}", file=sys.stderr)


def print_success(msg: str) -> None:
    """Print success message to stderr."""
    print(f"{_c(_GREEN, '[OK]')}    {msg}", file=sys.stderr)


def print_info(msg: str) -> None:
    """Print info message to stderr."""
    print(f"{_c(_CYAN, '[INFO]')}  {msg}", file=sys.stderr)


def print_warning(msg: str) -> None:
    """Print warning message to stderr."""
    print(f"{_c(_YELLOW, '[WARN]')}  {msg}", file=sys.stderr)


def die(msg: str) -> NoReturn:
    """Print error and exit with code 1."""
    print_error(msg)
    sys.exit(1)


# ==================================================================================================
# Platform Detection
# ==================================================================================================


@dataclass(frozen=True)
class PlatformInfo:
    """Detected platform information."""

    is_windows: bool
    is_linux: bool
    repo_root: Path
    home_dir: Path

    @staticmethod
    def detect(repo_root: Path) -> PlatformInfo:
        is_win = sys.platform == "win32"
        is_lin = sys.platform.startswith("linux")
        if is_win:
            home = Path(os.environ.get("USERPROFILE", os.environ.get("HOME", "")))
        else:
            home = Path.home()
        return PlatformInfo(
            is_windows=is_win, is_linux=is_lin, repo_root=repo_root, home_dir=home
        )


def _is_windows_server() -> bool:
    """Detect if running on Windows Server (vs desktop Windows)."""
    try:
        result = subprocess.run(
            [
                "powershell",
                "-NoProfile",
                "-Command",
                "(Get-CimInstance Win32_OperatingSystem).ProductType",
            ],
            capture_output=True,
            text=True,
            timeout=10,
        )
        # ProductType: 1=Workstation, 2=Domain Controller, 3=Server
        return result.stdout.strip() != "1"
    except Exception:
        return False


def _assert_windows_version() -> None:
    """Warn if not running Windows 11+."""
    try:
        build = int(platform.version().split(".")[-1])
        if build < 22000:
            print_warning(
                f"Windows build {build} detected. Windows 11 (build 22000+) is recommended."
            )
    except (ValueError, IndexError):
        pass


def _assert_developer_mode() -> None:
    """Check if Windows Developer Mode is enabled."""
    try:
        import winreg  # type: ignore[import-not-found]

        key = winreg.OpenKey(
            winreg.HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock",
        )
        val, _ = winreg.QueryValueEx(key, "AllowDevelopmentWithoutDevLicense")
        winreg.CloseKey(key)
        if val != 1:
            print_warning(
                "Developer Mode is not enabled. Symlinks may not work correctly.\n"
                "  Enable it in Settings > Privacy & Security > For developers."
            )
    except Exception:
        print_warning("Could not check Developer Mode status.")


def _assert_hypervisor_enabled() -> None:
    """Check if Windows Hypervisor Platform is enabled."""
    try:
        result = subprocess.run(
            [
                "powershell",
                "-NoProfile",
                "-Command",
                "(Get-CimInstance Win32_ComputerSystem).HypervisorPresent",
            ],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if "True" not in result.stdout:
            print_warning(
                "Hypervisor is not present. Enable the 'HypervisorPlatform' Windows feature:\n"
                "  Enable-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform"
            )
    except Exception:
        print_warning("Could not check hypervisor status.")


# ==================================================================================================
# Build Configuration
# ==================================================================================================


@dataclass
class BuildConfig:
    """Build configuration assembled from CLI args and platform defaults."""

    machine: str = DEFAULT_MACHINE
    target: str = DEFAULT_TARGET
    release: bool = False
    profile: bool = False
    log_level: str = ""
    timeout: int = DEFAULT_TIMEOUT

    profiler: bool = False
    whp: bool = False

    verus: bool = False

    sysroot_dir: str = ""
    host_cpu: str = ""

    image: str = ""
    memory_size: str = ""
    message_format: str = ""
    verbose: bool = False

    # Raw args to forward to Make (everything after --).
    make_args: list[str] = field(default_factory=list)

    def apply_platform_defaults(self, plat: PlatformInfo) -> None:
        """Apply platform-specific defaults that weren't set by the user."""
        if not self.log_level:
            self.log_level = (
                DEFAULT_LOG_LEVEL_RELEASE if self.release else DEFAULT_LOG_LEVEL_DEBUG
            )
        if plat.is_windows and self.machine == "microvm":
            self.whp = True


# ==================================================================================================
# CLI Argument Parsing
# ==================================================================================================


def _parse_make_var(config: BuildConfig, key: str, val: str) -> None:
    """Apply a known KEY=VALUE pair to the build config."""
    match key:
        case "MACHINE":
            if val not in VALID_MACHINES:
                die(f"Invalid MACHINE={val}. Valid: {', '.join(VALID_MACHINES)}")
            config.machine = val
        case "TARGET":
            if val not in VALID_TARGETS:
                die(f"Invalid TARGET={val}. Valid: {', '.join(VALID_TARGETS)}")
            config.target = val
        case "RELEASE":
            config.release = val.lower() == "yes"
        case "LOG_LEVEL":
            if val not in VALID_LOG_LEVELS:
                die(f"Invalid LOG_LEVEL={val}. Valid: {', '.join(VALID_LOG_LEVELS)}")
            config.log_level = val
        case "TIMEOUT":
            try:
                config.timeout = int(val)
            except ValueError:
                die(f"Invalid TIMEOUT={val}. Must be an integer.")
        case "PROFILER":
            config.profiler = val.lower() == "yes"
        case "WHP":
            config.whp = val.lower() == "yes"
        case "HOST_CPU":
            config.host_cpu = val
        case "IMAGE":
            config.image = val
        case "MEMORY_SIZE":
            try:
                mb = int(val)
                if mb <= 0:
                    die(
                        f"Invalid MEMORY_SIZE={val}. Must be a positive integer (megabytes)."
                    )
            except ValueError:
                die(
                    f"Invalid MEMORY_SIZE={val}. Must be a positive integer (megabytes)."
                )
            config.memory_size = val
        case "MESSAGE_FORMAT":
            if val not in VALID_MESSAGE_FORMATS:
                die(
                    f"Invalid MESSAGE_FORMAT={val}. Valid: {', '.join(VALID_MESSAGE_FORMATS)}"
                )
            config.message_format = val
        case "SYSROOT_DIR":
            config.sysroot_dir = val
        case "VERBOSE":
            config.verbose = val.lower() == "yes"
        case "SCCACHE" | "MAKE_NO_PRINT" | "VERUS_EXECUTABLE_DIR":
            pass  # Passed through to Make verbatim; no z.py-side effect.


def parse_cli(argv: Sequence[str]) -> tuple[str, BuildConfig]:
    """Parse command-line arguments into (command, config).

    Arguments after the command are either recognized options (--profile, --release, --verus)
    or make arguments (targets and KEY=VALUE pairs). The ``--`` separator is optional: the
    first unrecognized non-option argument starts the make_args section. This allows PowerShell
    callers to omit ``--`` (PowerShell strips it in interactive mode).
    """
    if not argv:
        return "help", BuildConfig()

    argv_list = list(argv)
    config = BuildConfig()

    command = ""
    make_args_start = len(argv_list)

    i = 0
    while i < len(argv_list):
        arg = argv_list[i]

        if not command:
            command = arg
            i += 1
            continue

        if arg == "--":
            # Explicit separator: everything after this is make_args.
            make_args_start = i + 1
            break
        elif arg == "--profile":
            config.profile = True
            config.release = True
            config.profiler = True
        elif arg == "--release":
            config.release = True
        elif arg == "--verus":
            config.verus = True
        elif arg.startswith("--"):
            die(f"Unknown option: {arg}")
        else:
            # First non-option argument: this and everything after is make_args.
            make_args_start = i
            break

        i += 1

    if not command:
        return "help", config

    # Store raw make_args (everything after --).
    config.make_args = argv_list[make_args_start:]

    # Extract known KEY=VALUE pairs into config for z.py's own decision-making.
    # The raw args are still passed through to Make verbatim.
    for arg in config.make_args:
        if "=" in arg:
            key, val = arg.split("=", 1)
            if key in KNOWN_MAKE_VARS:
                _parse_make_var(config, key, val)

    return command, config


# ==================================================================================================
# Environment Validation
# ==================================================================================================


def validate_git_context() -> Path:
    """Validate git worktree and return the repo root path."""
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--is-inside-work-tree"],
            capture_output=True,
            text=True,
            check=True,
        )
        if result.stdout.strip() != "true":
            die("Not inside a git work tree.")
    except (subprocess.CalledProcessError, FileNotFoundError):
        die(
            "Not inside a git repository. Ensure git is installed and this is a valid repo."
        )

    try:
        result = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            capture_output=True,
            text=True,
            check=True,
        )
        git_root = Path(result.stdout.strip())
    except subprocess.CalledProcessError:
        die("Failed to determine repository root.")

    cwd = Path.cwd()
    try:
        same_location = cwd.samefile(git_root)
    except (OSError, NotImplementedError):
        same_location = (
            str(cwd.resolve()).casefold() == str(git_root.resolve()).casefold()
        )

    if not same_location:
        die(f"Must run from repo root ({git_root.resolve()}), not {cwd.resolve()}.")

    # Return the caller's cwd (not the canonicalized git path) so that
    # substituted drives on Windows (e.g. `subst N: C:\path`) are preserved.
    # Resolving would expand the alias and cause Make's CURDIR to become a
    # long path, which can blow past cmd.exe's 8191-char command-line limit.
    return cwd


# ==================================================================================================
# Windows Pre-Build Steps
# ==================================================================================================


@dataclass(frozen=True)
class GitSymlinkStub:
    """A tracked symlink expanded from a Windows text stub for a build."""

    path: Path
    contents: str


def restore_git_symlinks(repo_root: Path) -> list[GitSymlinkStub]:
    """Temporarily expand tracked symlink text stubs on Windows."""
    try:
        result = subprocess.run(
            ["git", "ls-files", "-s"],
            capture_output=True,
            text=True,
            check=True,
            cwd=str(repo_root),
        )
    except subprocess.CalledProcessError:
        return []

    expanded: list[GitSymlinkStub] = []

    for line in result.stdout.splitlines():
        # git ls-files -s format: "<mode> <hash> <stage>\t<path>".
        if "\t" not in line:
            continue
        meta, rel_path = line.split("\t", 1)
        meta_parts = meta.split()
        if len(meta_parts) < 3 or meta_parts[0] != "120000":
            continue

        symlink_path = repo_root / rel_path

        if not symlink_path.exists() or symlink_path.is_symlink():
            continue

        try:
            stub_contents = symlink_path.read_text(encoding="utf-8")
        except (OSError, UnicodeError):
            continue

        content = stub_contents.strip()
        # Sanity check: real text stubs are short relative paths.
        if not content or len(content) > 500:
            continue

        target_path = (symlink_path.parent / content).resolve()
        if not target_path.exists():
            continue

        symlink_path.unlink()
        try:
            symlink_path.symlink_to(content, target_is_directory=target_path.is_dir())
        except OSError:
            if target_path.is_file():
                shutil.copy2(str(target_path), str(symlink_path))
            else:
                # Directory junctions do not require Developer Mode or administrator privileges.
                junction = subprocess.run(
                    [
                        os.environ.get("COMSPEC", "cmd.exe"),
                        "/c",
                        "mklink",
                        "/J",
                        str(symlink_path),
                        str(target_path),
                    ],
                    capture_output=True,
                    text=True,
                    check=False,
                )
                if junction.returncode != 0:
                    symlink_path.write_text(stub_contents, encoding="utf-8")
                    print_warning(f"Cannot restore directory symlink: {rel_path}")
                    continue

        expanded.append(GitSymlinkStub(symlink_path, stub_contents))

    return expanded


def collapse_git_symlinks(expanded: Sequence[GitSymlinkStub]) -> None:
    """Restore temporarily expanded symlinks to their tracked text-stub form."""
    for stub in reversed(expanded):
        try:
            if stub.path.is_symlink() or stub.path.is_file():
                stub.path.unlink()
            elif stub.path.exists():
                stub.path.rmdir()
            stub.path.write_text(stub.contents, encoding="utf-8")
        except OSError as error:
            print_warning(f"Cannot restore symlink stub '{stub.path}': {error}")


# ==================================================================================================
# Make Invocation
# ==================================================================================================


def _prepend_path(directory: str) -> None:
    """Add directory to the start of PATH if not already present."""
    entries = os.environ.get("PATH", "").split(os.pathsep)
    if not any(e.casefold() == directory.casefold() for e in entries):
        os.environ["PATH"] = directory + os.pathsep + os.environ.get("PATH", "")


def _append_path(directory: str) -> None:
    """Add directory to the end of PATH if not already present."""
    entries = os.environ.get("PATH", "").split(os.pathsep)
    if not any(e.casefold() == directory.casefold() for e in entries):
        os.environ["PATH"] = os.environ.get("PATH", "") + os.pathsep + directory


def find_make(plat: PlatformInfo) -> str | None:
    """Find the GNU Make binary. Returns path or None."""
    make = shutil.which("make")
    if make:
        return make

    if plat.is_windows:
        local_app_data = os.environ.get("LOCALAPPDATA", "")
        if local_app_data:
            winget_dir = Path(local_app_data) / "Microsoft" / "WinGet" / "Packages"
            if winget_dir.exists():
                for make_dir in winget_dir.glob("*make*"):
                    for candidate in make_dir.rglob("make.exe"):
                        _prepend_path(str(candidate.parent))
                        return str(candidate)

    return None


def find_clang(plat: PlatformInfo) -> str | None:
    """Find the clang C cross-compiler, prepending its directory to PATH.

    The guest C sources (the ported POSIX test suites and other guest C apps)
    are cross-compiled with clang. On Windows they are additionally linked with
    LLVM's ld.lld; on Linux the guest C link uses GNU ld (see
    build/make/guest-c-apps.mk), so only clang matters there.

    The Windows LLVM installer (winget ``LLVM.LLVM`` or the official MSI) drops
    both tools under ``%ProgramFiles%\\LLVM\\bin`` but does not add that
    directory to PATH, so probe the known install locations and prepend the
    first directory that provides BOTH clang and ld.lld — accepting a clang
    without a sibling ld.lld would only defer the failure to the link step.
    Returns the clang path, or None when a usable toolchain cannot be located.
    """
    if not plat.is_windows:
        return shutil.which("clang")

    # On Windows clang alone is insufficient: the guest C link step needs ld.lld
    # from the same LLVM toolchain. Accept what is already on PATH only when both
    # resolve; otherwise probe the standard install locations for a bin directory
    # that ships both and prepend it.
    clang = shutil.which("clang")
    if clang and shutil.which("ld.lld"):
        return clang

    candidates: list[Path] = []
    # Default install location used by the winget package and the MSI.
    for env_var in ("ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"):
        base = os.environ.get(env_var)
        if base:
            candidates.append(Path(base) / "LLVM" / "bin" / "clang.exe")
    # Fallback: a portable winget package under the per-user packages dir.
    local_app_data = os.environ.get("LOCALAPPDATA", "")
    if local_app_data:
        winget_dir = Path(local_app_data) / "Microsoft" / "WinGet" / "Packages"
        if winget_dir.exists():
            for llvm_dir in winget_dir.glob("*LLVM*"):
                candidates.extend(llvm_dir.rglob("clang.exe"))
    for candidate in candidates:
        # Only accept a clang whose directory also provides ld.lld, so the link
        # step does not later fail with a confusing "ld.lld not found".
        if candidate.exists() and (candidate.parent / "ld.lld.exe").exists():
            _prepend_path(str(candidate.parent))
            return str(candidate)

    return None


def _require_make(plat: PlatformInfo) -> str:
    """Find Make or die."""
    make = find_make(plat)
    if make:
        return make
    if plat.is_windows:
        hint = "Run 'z.ps1 setup' to install it."
    else:
        hint = "Run './z setup' to install it."
    die(f"GNU Make not found on PATH. {hint}")


def setup_windows_make_env(plat: PlatformInfo) -> None:
    """Set up the environment for running Make on Windows."""
    # HOME must use forward slashes for Make recipes that reference
    # $(HOME)/.cargo/bin/cargo. Git may set HOME with backslashes, and sh.exe
    # interprets those as escape sequences (e.g., C:\Users → C:Users).
    home = os.environ.get("HOME", os.environ.get("USERPROFILE", ""))
    os.environ["HOME"] = home.replace("\\", "/")

    # Add Git-for-Windows usr/bin to PATH (provides sh.exe and coreutils for Make recipes).
    git_exe = shutil.which("git")
    if git_exe:
        git_root = Path(git_exe).resolve().parent.parent
        git_usr_bin = git_root / "usr" / "bin"
        if git_usr_bin.exists():
            _append_path(str(git_usr_bin))

    # Add Python scripts dir to PATH (for codespell etc.).
    # In a venv, sys.executable is already in the Scripts/ directory alongside
    # codespell.exe, so we add the executable's parent directly. For a system
    # Python install, scripts live in a Scripts/ subdirectory.
    exe_dir = Path(sys.executable).parent
    _append_path(str(exe_dir))
    scripts_subdir = exe_dir / "Scripts"
    if scripts_subdir.is_dir():
        _append_path(str(scripts_subdir))

    # When invoked via git hooks the venv may not be activated, so also check
    # the repo-local .venv/Scripts directory where codespell etc. are installed.
    repo_venv_scripts = plat.repo_root / ".venv" / "Scripts"
    if repo_venv_scripts.is_dir():
        _append_path(str(repo_venv_scripts))

    # Ensure the guest C cross-compiler (clang) and LLVM linker (ld.lld) are on
    # PATH. The LLVM installer does not add itself to PATH on Windows, so probe
    # the default install location and prepend it. Best-effort: only some
    # targets (e.g. run-posix-tests) compile guest C sources, so a missing
    # clang is not fatal here — posix-tests.mk emits a precise error if it is
    # genuinely required.
    find_clang(plat)


def invoke_make(
    plat: PlatformInfo,
    *,
    injected_vars: list[str] | None = None,
    raw_args: list[str] | None = None,
    targets: list[str] | None = None,
    verbose: bool = False,
) -> int:
    """Invoke GNU Make.

    Args:
        plat: Platform information.
        injected_vars: VAR=VALUE strings z.py injects (e.g., RELEASE).
        raw_args: Raw user arguments from after -- (targets and KEY=VALUE pairs).
        targets: Explicit Make targets to append.
        verbose: Print the full command line.

    Returns:
        Make exit code.
    """
    make_bin = _require_make(plat)

    if plat.is_windows:
        setup_windows_make_env(plat)

    cmd: list[str] = [make_bin]
    if injected_vars:
        cmd.extend(injected_vars)
    if raw_args:
        cmd.extend(raw_args)
    if targets:
        cmd.extend(targets)

    if verbose:
        print_info(f"Running: {' '.join(cmd)}")

    result = subprocess.run(cmd, cwd=str(plat.repo_root))
    return result.returncode


# ==================================================================================================
# Build Argument Assembly
# ==================================================================================================


def _assemble_build_make_args(
    plat: PlatformInfo,
    config: BuildConfig,
) -> tuple[list[str], list[str]]:
    """Assemble injected Make variables and filtered user args.

    z.py injects only the variables it needs to add or override. All other user-supplied
    KEY=VALUE pairs and targets are passed through verbatim.

    Returns (injected_vars, filtered_user_args).
    """
    injected: list[str] = []
    user_args = list(config.make_args)

    # --profile: force RELEASE=yes, add PROFILER=yes, strip user-provided RELEASE=.
    if config.profile:
        user_args = [a for a in user_args if not a.startswith("RELEASE=")]
        injected.append("RELEASE=yes")
        injected.append("PROFILER=yes")
    elif config.release:
        if not any(a.startswith("RELEASE=") for a in user_args):
            injected.append("RELEASE=yes")

    # Windows platform defaults.
    if plat.is_windows:
        host_arch = platform.machine().strip().lower()
        target_is_explicit = "TARGET" in os.environ or any(
            a.startswith("TARGET=") for a in user_args
        )
        if host_arch in ("aarch64", "arm64") and not target_is_explicit:
            injected.append("TARGET=aarch64")
        if config.machine == "microvm" and not any(
            a.startswith("WHP=") for a in user_args
        ):
            injected.append("WHP=yes")

    return injected, user_args


# ==================================================================================================
# Subcommand: build
# ==================================================================================================


def cmd_build(plat: PlatformInfo, config: BuildConfig) -> int:
    """Execute the build subcommand."""
    expanded: list[GitSymlinkStub] = (
        restore_git_symlinks(plat.repo_root) if plat.is_windows else []
    )
    try:
        injected, user_args = _assemble_build_make_args(plat, config)

        print_info(f"Build parameters: {' '.join(injected + user_args)}")

        rc = invoke_make(
            plat, injected_vars=injected, raw_args=user_args, verbose=config.verbose
        )
    finally:
        collapse_git_symlinks(expanded)

    if rc == 0:
        print_success("Build complete.")
    else:
        print_error("Build failed.")

    return rc


# ==================================================================================================
# Subcommand: clean / distclean
# ==================================================================================================


def cmd_clean(plat: PlatformInfo, config: BuildConfig) -> int:
    """Execute the clean subcommand."""
    print_info("Running quick cleanup...")

    rc = invoke_make(plat, targets=["clean"])

    if rc == 0:
        print_success("Quick clean complete.")
    else:
        print_error("Quick clean failed.")

    return rc


def cmd_distclean(plat: PlatformInfo, config: BuildConfig) -> int:
    """Execute the distclean subcommand."""
    print_info("Running full cleanup...")

    rc = invoke_make(plat, targets=["distclean"])

    # Windows-specific extra cleanup.
    if plat.is_windows:
        venv_dir = plat.repo_root / ".venv"
        if venv_dir.exists():
            try:
                shutil.rmtree(str(venv_dir))
            except Exception:
                subprocess.run(
                    ["cmd", "/c", "rmdir", "/s", "/q", str(venv_dir)],
                    capture_output=True,
                    check=False,
                )
            print_info("Removed .venv/")

    if rc == 0:
        print_success("Full cleanup complete.")
    else:
        print_error("Full cleanup failed.")

    return rc


# ==================================================================================================
# Subcommand: test
# ==================================================================================================


def cmd_test(plat: PlatformInfo, config: BuildConfig) -> int:
    """Execute the test subcommand."""
    expanded: list[GitSymlinkStub] = (
        restore_git_symlinks(plat.repo_root) if plat.is_windows else []
    )
    try:
        injected, user_args = _assemble_build_make_args(plat, config)

        rc = invoke_make(
            plat,
            injected_vars=injected,
            raw_args=user_args,
            targets=["test"],
            verbose=config.verbose,
        )
    finally:
        collapse_git_symlinks(expanded)

    if rc == 0:
        print_success("Tests passed.")
    else:
        print_error("Tests failed.")

    return rc


# ==================================================================================================
# Subcommand: verify
# ==================================================================================================


def cmd_verify(plat: PlatformInfo, config: BuildConfig) -> int:
    """Execute the verify subcommand (Verus formal verification)."""
    expanded: list[GitSymlinkStub] = (
        restore_git_symlinks(plat.repo_root) if plat.is_windows else []
    )
    try:
        injected, user_args = _assemble_build_make_args(plat, config)

        # If the user did not supply explicit Make goals after `--`, default to the
        # top-level `verify` target. Otherwise, rely solely on the user-specified
        # goals and do not prepend `verify`. Variable assignments (KEY=VALUE) are
        # not Make goals, so they must not suppress the default target.
        has_goals = any("=" not in a for a in user_args)
        targets: list[str] = ["verify"] if not has_goals else []

        rc = invoke_make(
            plat,
            injected_vars=injected,
            raw_args=user_args,
            targets=targets,
            verbose=config.verbose,
        )
    finally:
        collapse_git_symlinks(expanded)

    if rc == 0:
        print_success("Verification complete.")
    else:
        print_error("Verification failed.")

    return rc


# ==================================================================================================
# Subcommand: run
# ==================================================================================================


def cmd_run(plat: PlatformInfo, config: BuildConfig) -> int:
    """Execute the run subcommand."""
    # Parse -program from make_args.
    program = DEFAULT_RUN_PROGRAM
    remaining_args: list[str] = []
    i = 0
    while i < len(config.make_args):
        if config.make_args[i] == "-program" and i + 1 < len(config.make_args):
            program = config.make_args[i + 1]
            i += 2
        else:
            remaining_args.append(config.make_args[i])
            i += 1

    if plat.is_windows:
        # Run nanvixd.exe directly in standalone mode.
        nanvixd = plat.repo_root / "bin" / "nanvixd.exe"
        if not nanvixd.exists():
            die("nanvixd.exe not found in bin/. Run 'z.ps1 build -- all' first.")

        cmd = [str(nanvixd), "--", program]
        print_info(f"Running: {' '.join(cmd)}")
        result = subprocess.run(cmd, cwd=str(plat.repo_root))
        return result.returncode
    else:
        # Linux: delegate to make run. Use remaining_args (with -program
        # stripped) instead of config.make_args to avoid duplicating args.
        injected, _ = _assemble_build_make_args(plat, config)
        return invoke_make(
            plat,
            injected_vars=injected,
            raw_args=remaining_args,
            targets=["run"],
            verbose=config.verbose,
        )


# ==================================================================================================
# Subcommand: bench
# ==================================================================================================


def cmd_bench(plat: PlatformInfo, config: BuildConfig) -> int:
    """Execute the bench subcommand."""
    ext = ".exe" if plat.is_windows else ".elf"
    bench_bin = plat.repo_root / "bin" / f"nanvix-bench{ext}"

    if not bench_bin.exists():
        hint = (
            "z.ps1 build -- nanvix-bench"
            if plat.is_windows
            else "./z build -- all-nanvix-bench"
        )
        die(f"nanvix-bench{ext} not found in bin/. Build it first:\n  {hint}")

    cmd = [str(bench_bin)] + config.make_args
    print_info(f"Running: {' '.join(cmd)}")
    result = subprocess.run(cmd, cwd=str(plat.repo_root))
    return result.returncode


# ==================================================================================================
# Subcommand: help
# ==================================================================================================

HELP_TEXT = (
    """\
Utility for building Nanvix.

Usage:
  z <command> [options] [-- [targets] [PARAM=VALUE ...]]

Commands:
  build       Build Nanvix components.
  test        Run tests.
  verify      Run Verus formal verification on annotated crates.
  clean       Remove build artifacts (quick clean).
  distclean   Remove everything (full clean).
  setup       Install development prerequisites.
  run         Run nanvixd.
  bench       Run benchmarks.
  help        Show this help message.

Options:
  --profile             Enable profiling (implies --release, passes PROFILER=yes).
  --release             Build in release mode.
"""
    "  --verus               Install the Verus formal verification toolchain (setup only).\n"
    """
Build Parameters (after --):
  MACHINE=microvm                Target machine (default: microvm).
  RELEASE=yes|no                 Release mode.
  LOG_LEVEL=LEVEL                trace|debug|info|warn|error|panic.
  PROFILER=yes|no                Enable profiling.
  TIMEOUT=SECONDS                Execution timeout (default: 600).
  WHP=yes|no                     Windows Hypervisor Platform.
  HOST_CPU=CPU                   Target CPU for host builds.
  MEMORY_SIZE=MB                 Memory size in megabytes (default: 128).

Run Options (after --):
  -program <path>       Path to guest binary (default: bin/hello-rust-nostd.elf).

Examples:
  ./z build                               Build everything (Linux defaults).
  ./z build --release -- MACHINE=microvm  Release build for microvm.
  z.ps1 build -- all                      Build everything (Windows).
  z.ps1 build -- guest                    Cross-compile guest only.
  ./z test                                Run all tests.
"""
    "  ./z build -- verify                     Verify all annotated crates.\n"
    "  z.ps1 build -- verify                   Verify all annotated crates on Windows.\n"
    """\
  ./z clean                               Clean build artifacts.
  ./z setup                                Install core dev prerequisites.
"""
    "  ./z setup --verus                        Also install Verus.\n"
)


def cmd_help(plat: PlatformInfo, config: BuildConfig) -> int:
    """Execute the help subcommand."""
    print(HELP_TEXT)
    make = find_make(plat)
    if make:
        print("--- Make Targets and Parameters ---\n")
        invoke_make(plat, targets=["help"])
    else:
        print("(Install GNU Make to see additional Make targets and parameters.)")
    return 0


# ==================================================================================================
# Subcommand: setup
# ==================================================================================================


def _install_git_hooks(repo_root: Path) -> None:
    """Configure git hooks path."""
    try:
        subprocess.run(
            ["git", "config", "--local", "core.hooksPath", ".githooks"],
            cwd=str(repo_root),
            check=True,
            capture_output=True,
        )
        print_success("Git hooks configured (.githooks).")
    except subprocess.CalledProcessError:
        print_warning("Failed to configure git hooks.")


def _refresh_windows_path() -> None:
    """Refresh the session PATH from the registry so newly installed tools are found."""
    try:
        result = subprocess.run(
            [
                "powershell",
                "-NoProfile",
                "-Command",
                "[Environment]::GetEnvironmentVariable('Path','Machine') + ';' + "
                "[Environment]::GetEnvironmentVariable('Path','User')",
            ],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0 and result.stdout.strip():
            os.environ["PATH"] = result.stdout.strip()
    except Exception:
        pass


def _install_chocolatey() -> None:
    """Install the Chocolatey package manager (Windows Server)."""
    print_info("Installing Chocolatey...")
    rc = subprocess.run(
        [
            "powershell",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            "[System.Net.ServicePointManager]::SecurityProtocol = "
            "[System.Net.ServicePointManager]::SecurityProtocol -bor 3072; "
            "iex ((New-Object System.Net.WebClient).DownloadString("
            "'https://community.chocolatey.org/install.ps1'))",
        ],
    ).returncode
    if rc != 0:
        die("Failed to install Chocolatey.")
    _refresh_windows_path()
    if not shutil.which("choco"):
        die("Chocolatey installed but 'choco' not found on PATH.")
    print_success("Chocolatey installed.")


def _choco_install(package_id: str, name: str) -> None:
    """Install a package via Chocolatey."""
    print_info(f"Installing {name} via Chocolatey...")
    rc = subprocess.run(
        ["choco", "install", package_id, "-y", "--no-progress"],
        stdout=subprocess.DEVNULL,
    ).returncode
    if rc != 0:
        die(f"Failed to install {name} via Chocolatey.")
    _refresh_windows_path()


def _winget_install(package_id: str, name: str, *, silent: bool = True) -> None:
    """Install a package via winget."""
    print_info(f"Installing {name} via winget...")
    cmd: list[str] = ["winget", "install", package_id]
    if silent:
        cmd.append("--silent")
    rc = subprocess.run(cmd).returncode
    if rc != 0:
        die(f"Failed to install {name}. Install it manually.")


def _pkg_install(
    name: str,
    *,
    winget_id: str | None = None,
    choco_id: str | None = None,
    use_choco: bool = False,
) -> None:
    """Install a package using the available package manager."""
    if use_choco and choco_id:
        _choco_install(choco_id, name)
    elif winget_id:
        _winget_install(winget_id, name)
    else:
        die(f"No package manager ID available to install {name}. Install it manually.")


def _pkg_install_best_effort(
    name: str,
    *,
    winget_id: str | None = None,
    choco_id: str | None = None,
    use_choco: bool = False,
) -> bool:
    """Best-effort package install; returns True on success, False on failure.

    Unlike `_pkg_install`, a failed install is not fatal — the caller decides
    whether the package is strictly required. Used for tools that only some
    build targets need (e.g. LLVM/Clang, required only when compiling guest C
    sources), so a missing package manager or a failed install must not abort
    setup for unrelated jobs.
    """
    if use_choco and choco_id:
        print_info(f"Installing {name} via Chocolatey...")
        cmd = ["choco", "install", choco_id, "-y", "--no-progress"]
    elif winget_id:
        print_info(f"Installing {name} via winget...")
        cmd = ["winget", "install", winget_id, "--silent"]
    else:
        return False
    rc = subprocess.run(cmd, stdout=subprocess.DEVNULL).returncode
    if rc != 0:
        return False
    _refresh_windows_path()
    return True


def _install_verus(plat: PlatformInfo) -> None:
    """Install Verus with the cross-platform setup script."""
    verus_script = plat.repo_root / "scripts" / "setup" / "verus.py"
    if not verus_script.exists():
        die(f"Verus setup script not found: {verus_script}")

    verus_dir = plat.home_dir / "verus"
    print_info(f"Installing Verus to {verus_dir} ...")
    rc = subprocess.run([sys.executable, str(verus_script), str(verus_dir)]).returncode
    if rc != 0:
        die("Verus setup failed.")


def cmd_setup_linux(plat: PlatformInfo, config: BuildConfig) -> int:
    """Set up the development environment on Linux."""
    print_info("Setting up Nanvix development environment...")

    # Always install core system dependencies.
    core_script = plat.repo_root / "scripts" / "setup" / "ubuntu-core.sh"
    if core_script.exists():
        print_info("Installing core development dependencies (requires sudo)...")
        rc = subprocess.run(["sudo", str(core_script)]).returncode
        if rc != 0:
            die("Failed to install core development dependencies.")
    else:
        print_warning(f"Setup script not found: {core_script}")

    # Verus formal verification toolchain (optional).
    if config.verus:
        _install_verus(plat)

    _install_git_hooks(plat.repo_root)
    print_success("Setup complete.")
    return 0


def cmd_setup_windows(plat: PlatformInfo, config: BuildConfig) -> int:
    """Set up the development environment on Windows."""
    print_info("Setting up Nanvix development environment...")

    # Non-fatal platform assertions.
    _assert_windows_version()
    _assert_developer_mode()
    _assert_hypervisor_enabled()

    # On Windows Server, install Chocolatey as the package manager.
    is_server = _is_windows_server()
    use_choco = False
    if is_server:
        print_info("Windows Server detected.")
        if not shutil.which("choco"):
            _install_chocolatey()
        else:
            print_success("Chocolatey: OK")
        use_choco = True

    # Git.
    _refresh_windows_path()
    if not shutil.which("git"):
        _pkg_install("Git", winget_id="Git.Git", choco_id="git", use_choco=use_choco)
        _refresh_windows_path()
        if not shutil.which("git"):
            die("Git still not found after installation. Add it to PATH manually.")
    print_success("Git: OK")

    # Python.
    if not shutil.which("python"):
        _pkg_install(
            "Python 3.12",
            winget_id="Python.Python.3.12",
            choco_id="python312",
            use_choco=use_choco,
        )
        _refresh_windows_path()
        if not shutil.which("python"):
            die("Python still not found after installation. Add it to PATH manually.")
    print_success("Python: OK")

    # GNU Make.
    if not shutil.which("make"):
        found = False
        local_app_data = os.environ.get("LOCALAPPDATA", "")
        if local_app_data:
            winget_dir = Path(local_app_data) / "Microsoft" / "WinGet" / "Packages"
            if winget_dir.exists():
                for make_dir in winget_dir.glob("*make*"):
                    for candidate in make_dir.rglob("make.exe"):
                        _prepend_path(str(candidate.parent))
                        found = True
                        break
                    if found:
                        break
        if not found:
            _pkg_install(
                "GNU Make",
                winget_id="ezwinports.make",
                choco_id="make",
                use_choco=use_choco,
            )
            _refresh_windows_path()
            if not shutil.which("make"):
                die(
                    "GNU Make still not found after installation. Add it to PATH manually."
                )
    print_success("GNU Make: OK")

    # Visual Studio Build Tools.
    vswhere = (
        Path(os.environ.get("ProgramFiles(x86)", r"C:\Program Files (x86)"))
        / "Microsoft Visual Studio"
        / "Installer"
        / "vswhere.exe"
    )
    if vswhere.exists():
        result = subprocess.run(
            [str(vswhere), "-latest", "-property", "installationPath"],
            capture_output=True,
            text=True,
        )
        if result.returncode == 0 and result.stdout.strip():
            print_success("Visual Studio Build Tools: OK")
        else:
            print_warning(
                "vswhere found but no VS installation detected. "
                "Install the C++ workload from Visual Studio Installer."
            )
    else:
        print_warning(
            "Visual Studio Build Tools not found. Install the "
            "'Desktop development with C++' workload from "
            "https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022"
        )

    # LLVM/Clang: the guest C cross-compiler (clang) and linker (ld.lld) used to
    # build the ported POSIX C test suites and other guest C sources. Best-effort:
    # only some targets compile guest C, so a failed install must not abort setup
    # for unrelated Windows jobs (benchmarks, format checks). find_clang() probes
    # the install locations (the installer does not add %ProgramFiles%\LLVM\bin to
    # PATH) and prepends the first directory that provides BOTH clang and ld.lld;
    # if the toolchain still cannot be provisioned we warn and continue —
    # posix-tests.mk emits a precise error if it is genuinely required.
    if find_clang(plat):
        print_success("LLVM/Clang: OK")
    else:
        _pkg_install_best_effort(
            "LLVM/Clang",
            winget_id="LLVM.LLVM",
            choco_id="llvm",
            use_choco=use_choco,
        )
        if find_clang(plat):
            print_success("LLVM/Clang: OK")
        else:
            print_warning(
                "LLVM/Clang (clang + ld.lld) not found and could not be installed "
                "automatically. It is required only to compile the guest C sources "
                "(e.g. the POSIX C test suites); other targets are unaffected. "
                "Install it with 'winget install LLVM.LLVM' or 'choco install "
                "llvm', or from https://github.com/llvm/llvm-project/releases, and "
                "ensure %ProgramFiles%\\LLVM\\bin is on PATH."
            )

    # Rust toolchain.
    if not shutil.which("rustc"):
        cargo_bin = Path(os.environ.get("USERPROFILE", "")) / ".cargo" / "bin"
        if cargo_bin.exists():
            _prepend_path(str(cargo_bin))
        if not shutil.which("rustc"):
            _winget_install("Rustlang.Rustup", "Rust toolchain")
            _refresh_windows_path()
            if not shutil.which("rustc"):
                die(
                    "Rust toolchain still not found. Add ~/.cargo/bin to PATH manually."
                )
    print_success("Rust toolchain: OK")

    # Verus (optional).
    if config.verus:
        _install_verus(plat)
        print_success("Verus: OK")

    _install_git_hooks(plat.repo_root)
    print_success("Windows setup complete.")
    return 0


def cmd_setup(plat: PlatformInfo, config: BuildConfig) -> int:
    """Execute the setup subcommand."""
    if plat.is_windows:
        return cmd_setup_windows(plat, config)
    elif plat.is_linux:
        return cmd_setup_linux(plat, config)
    else:
        die(f"Unsupported platform: {sys.platform}")


# ==================================================================================================
# Main Entry Point
# ==================================================================================================

COMMANDS = {
    "build": cmd_build,
    "test": cmd_test,
    "verify": cmd_verify,
    "clean": cmd_clean,
    "distclean": cmd_distclean,
    "setup": cmd_setup,
    "run": cmd_run,
    "bench": cmd_bench,
    "help": cmd_help,
}


def main(argv: Sequence[str] | None = None) -> None:
    """Main entry point."""
    if argv is None:
        argv = sys.argv[1:]

    if sys.version_info < (3, 10):
        print(f"[ERROR] Python 3.10+ required. Current: {sys.version}", file=sys.stderr)
        sys.exit(1)

    repo_root = validate_git_context()
    plat = PlatformInfo.detect(repo_root)

    command, config = parse_cli(argv)
    config.apply_platform_defaults(plat)

    if command not in COMMANDS:
        print_error(f"Unknown command: '{command}'")
        print(f"Valid commands: {', '.join(COMMANDS.keys())}")
        sys.exit(1)

    rc = COMMANDS[command](plat, config)
    sys.exit(rc)


if __name__ == "__main__":
    main()
