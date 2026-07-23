#!/usr/bin/env python3

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Install the pinned Verus release on Linux or Windows.

The installer reads the expected version from ``build/verus-version``, reuses
validated archives from the local cache, and installs the platform-specific
Verus release when the requested version is not already present.

Usage:
    python scripts/setup/verus.py [--download-only] [install-dir]

Environment variables:
    GITHUB_TOKEN: Token used for authenticated GitHub downloads.
    GH_TOKEN: Alternative to ``GITHUB_TOKEN``.
    VERUS_ZIP_CACHE_DIR: Override the default ``.verus-cache`` directory.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.request
import zipfile
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path, PurePosixPath, PureWindowsPath

# ==================================================================================================
# Constants
# ==================================================================================================

REPO_ROOT_DIR = Path(__file__).resolve().parents[2]
VERUS_VERSION_FILE = REPO_ROOT_DIR / "build" / "verus-version"
VERUS_RELEASE_URL = "https://github.com/verus-lang/verus/releases/download/release"
DEFAULT_CACHE_DIR = REPO_ROOT_DIR / ".verus-cache"

DOWNLOAD_RETRY_COUNT = 5
DOWNLOAD_RETRY_DELAY_SECONDS = 10
DOWNLOAD_TIMEOUT_SECONDS = 300


@dataclass(frozen=True)
class VerusPlatform:
    """Platform-specific Verus release metadata."""

    name: str
    archive_suffix: str
    archive_subdir: str

    def archive_name(self, version: str) -> str:
        """Return the release archive name for ``version``."""
        return f"verus-{version}-{self.archive_suffix}.zip"


LINUX = VerusPlatform(
    name="linux",
    archive_suffix="x86-linux",
    archive_subdir="verus-x86-linux",
)
WINDOWS = VerusPlatform(
    name="windows",
    archive_suffix="x86-win",
    archive_subdir="verus-x86-win",
)


class VerusSetupError(RuntimeError):
    """Error raised when Verus setup cannot be completed."""


# ==================================================================================================
# Logging
# ==================================================================================================


def print_info(message: str) -> None:
    """Print an informational message."""
    print(f"[INFO] {message}")


def print_success(message: str) -> None:
    """Print a success message."""
    print(f"[OK]   {message}")


def print_warning(message: str) -> None:
    """Print a warning message."""
    print(f"[WARN] {message}", file=sys.stderr)


def print_error(message: str) -> None:
    """Print an error message."""
    print(f"[ERROR] {message}", file=sys.stderr)


# ==================================================================================================
# Platform Helpers
# ==================================================================================================


def detect_platform(platform_name: str = sys.platform) -> VerusPlatform:
    """Return Verus metadata for the current operating system.

    Args:
        platform_name: Python platform identifier. This argument is injectable for tests.

    Raises:
        VerusSetupError: If the operating system is unsupported.
    """
    if platform_name.startswith("linux"):
        return LINUX
    if platform_name in {"win32", "cygwin", "msys"}:
        return WINDOWS
    raise VerusSetupError(f"unsupported host platform: {platform_name}")


def get_default_install_dir(platform_config: VerusPlatform) -> Path:
    """Return the default installation directory for a platform."""
    if platform_config == WINDOWS:
        user_profile = os.environ.get("USERPROFILE")
        if user_profile:
            return Path(user_profile) / "verus"
    return Path.home() / "verus"


def get_cache_dir() -> Path:
    """Return the configured Verus archive cache directory."""
    configured_dir = os.environ.get("VERUS_ZIP_CACHE_DIR")
    return Path(configured_dir).expanduser() if configured_dir else DEFAULT_CACHE_DIR


# ==================================================================================================
# Version Helpers
# ==================================================================================================


def get_expected_version(version_file: Path = VERUS_VERSION_FILE) -> str:
    """Read the expected Verus version from ``version_file``.

    Raises:
        VerusSetupError: If the file does not exist or contains no version.
    """
    if not version_file.is_file():
        raise VerusSetupError(f"Verus version file not found: {version_file}")

    version = version_file.read_text(encoding="utf-8").strip()
    if not version:
        raise VerusSetupError(f"Verus version file is empty: {version_file}")
    return version


def get_installed_version(install_dir: Path) -> str:
    """Return the installed Verus version, or an empty string when absent."""
    version_file = install_dir / "version.txt"
    if not version_file.is_file():
        return ""
    return version_file.read_text(encoding="utf-8").strip()


# ==================================================================================================
# Archive Helpers
# ==================================================================================================


def get_auth_headers(environment: Mapping[str, str] = os.environ) -> dict[str, str]:
    """Return GitHub authorization headers when a token is available."""
    token = environment.get("GITHUB_TOKEN") or environment.get("GH_TOKEN")
    return {"Authorization": f"Bearer {token}"} if token else {}


def validate_zip_archive(archive_path: Path) -> bool:
    """Return whether ``archive_path`` is a readable, non-corrupt zip archive."""
    try:
        with zipfile.ZipFile(archive_path) as archive:
            return archive.testzip() is None
    except (OSError, zipfile.BadZipFile, zipfile.LargeZipFile):
        return False


def _validate_archive_members(members: Sequence[zipfile.ZipInfo]) -> None:
    """Reject archive members that could escape the extraction directory."""
    for member in members:
        posix_path = PurePosixPath(member.filename)
        windows_path = PureWindowsPath(member.filename)
        if (
            posix_path.is_absolute()
            or windows_path.is_absolute()
            or windows_path.drive
            or ".." in posix_path.parts
            or ".." in windows_path.parts
        ):
            raise VerusSetupError(f"unsafe path in Verus archive: {member.filename}")


def _extract_archive(archive_path: Path, destination: Path) -> list[zipfile.ZipInfo]:
    """Extract an archive after validating all member paths."""
    with zipfile.ZipFile(archive_path) as archive:
        members = archive.infolist()
        _validate_archive_members(members)
        archive.extractall(destination)
        return members


def extract_verus_linux(archive_path: Path, destination: Path) -> None:
    """Extract a Linux Verus archive and restore Unix permission bits."""
    members = _extract_archive(archive_path, destination)
    for member in members:
        permissions = (member.external_attr >> 16) & 0o7777
        extracted_path = destination / member.filename
        if permissions and extracted_path.exists() and not extracted_path.is_symlink():
            extracted_path.chmod(permissions)


def extract_verus_windows(archive_path: Path, destination: Path) -> None:
    """Extract a Windows Verus archive."""
    _extract_archive(archive_path, destination)


def _persist_archive(archive_path: Path, cached_archive: Path) -> None:
    """Atomically copy a downloaded archive into the local cache."""
    cached_archive.parent.mkdir(parents=True, exist_ok=True)
    temporary_cache = cached_archive.with_name(
        f".{cached_archive.name}.{os.getpid()}.tmp"
    )
    try:
        shutil.copy2(archive_path, temporary_cache)
        os.replace(temporary_cache, cached_archive)
    finally:
        temporary_cache.unlink(missing_ok=True)


def download_verus_archive(
    destination: Path,
    version: str,
    platform_config: VerusPlatform,
    cache_dir: Path,
) -> None:
    """Copy or download the requested platform's Verus release archive.

    A valid cached archive is preferred. Fresh downloads are validated before
    they are persisted to the cache.

    Raises:
        VerusSetupError: If the archive cannot be downloaded or validated.
    """
    archive_name = platform_config.archive_name(version)
    cached_archive = cache_dir / archive_name

    if cached_archive.is_file():
        if cached_archive.stat().st_size > 0 and validate_zip_archive(cached_archive):
            print_info(f"Using cached Verus archive from {cached_archive}")
            shutil.copy2(cached_archive, destination)
            return
        print_warning(
            f"Cached Verus archive at {cached_archive} is corrupted; "
            "deleting and re-downloading."
        )
        cached_archive.unlink(missing_ok=True)

    download_url = f"{VERUS_RELEASE_URL}/{version}/{archive_name}"
    print_info(f"Downloading Verus {version} from {download_url} ...")
    request = urllib.request.Request(download_url, headers=get_auth_headers())
    last_error: Exception | None = None

    for attempt in range(1, DOWNLOAD_RETRY_COUNT + 1):
        try:
            with urllib.request.urlopen(
                request, timeout=DOWNLOAD_TIMEOUT_SECONDS
            ) as response, destination.open("wb") as output:
                shutil.copyfileobj(response, output)

            if not validate_zip_archive(destination):
                raise VerusSetupError("downloaded archive failed zip validation")
            last_error = None
            break
        except (OSError, VerusSetupError) as error:
            last_error = error
            destination.unlink(missing_ok=True)
            if attempt < DOWNLOAD_RETRY_COUNT:
                print_warning(
                    f"Download attempt {attempt} failed; retrying in "
                    f"{DOWNLOAD_RETRY_DELAY_SECONDS}s..."
                )
                time.sleep(DOWNLOAD_RETRY_DELAY_SECONDS)

    if last_error is not None:
        raise VerusSetupError(
            f"failed to download Verus from {download_url} after "
            f"{DOWNLOAD_RETRY_COUNT} attempts: {last_error}"
        ) from last_error

    _persist_archive(destination, cached_archive)


# ==================================================================================================
# Installation Helpers
# ==================================================================================================


def _validate_install_dir(install_dir: Path) -> None:
    """Ensure the destructive installation target looks intentional."""
    if "verus" not in str(install_dir).lower():
        raise VerusSetupError(
            f"install directory '{install_dir}' does not contain 'verus'; aborting for safety"
        )


def _clear_directory(directory: Path) -> None:
    """Remove all children from ``directory`` without removing the directory itself."""
    directory.mkdir(parents=True, exist_ok=True)
    for child in directory.iterdir():
        if child.is_symlink() or child.is_file():
            child.unlink()
        else:
            shutil.rmtree(child)


def _install_verus(
    install_dir: Path,
    version: str,
    cache_dir: Path,
    platform_config: VerusPlatform,
    extractor: Callable[[Path, Path], None],
) -> None:
    """Install one platform-specific Verus archive."""
    _validate_install_dir(install_dir)
    archive_name = platform_config.archive_name(version)

    with tempfile.TemporaryDirectory(prefix="verus-install-") as temporary_dir_name:
        temporary_dir = Path(temporary_dir_name)
        archive_path = temporary_dir / archive_name
        extract_dir = temporary_dir / "extract"
        extract_dir.mkdir()

        download_verus_archive(archive_path, version, platform_config, cache_dir)
        print_info(f"Extracting Verus to {install_dir} ...")
        extractor(archive_path, extract_dir)

        archive_subdir = extract_dir / platform_config.archive_subdir
        if not archive_subdir.is_dir():
            raise VerusSetupError(
                f"expected directory '{platform_config.archive_subdir}' not found in archive"
            )

        _clear_directory(install_dir)
        shutil.copytree(archive_subdir, install_dir, dirs_exist_ok=True, symlinks=True)

    print_success(f"Verus {version} installed to {install_dir}.")


def install_verus_linux(install_dir: Path, version: str, cache_dir: Path) -> None:
    """Install the Linux Verus release."""
    _install_verus(
        install_dir,
        version,
        cache_dir,
        LINUX,
        extract_verus_linux,
    )


def install_verus_windows(install_dir: Path, version: str, cache_dir: Path) -> None:
    """Install the Windows Verus release."""
    _install_verus(
        install_dir,
        version,
        cache_dir,
        WINDOWS,
        extract_verus_windows,
    )


def install_verus(
    install_dir: Path,
    version: str,
    cache_dir: Path,
    platform_config: VerusPlatform,
) -> None:
    """Dispatch Verus installation to the host-specific implementation."""
    if platform_config == LINUX:
        install_verus_linux(install_dir, version, cache_dir)
        return
    if platform_config == WINDOWS:
        install_verus_windows(install_dir, version, cache_dir)
        return
    raise VerusSetupError(f"unsupported Verus platform: {platform_config.name}")


def download_only(
    version: str, cache_dir: Path, platform_config: VerusPlatform
) -> None:
    """Populate and validate the archive cache without installing Verus."""
    archive_name = platform_config.archive_name(version)
    with tempfile.TemporaryDirectory(prefix="verus-download-") as temporary_dir_name:
        archive_path = Path(temporary_dir_name) / archive_name
        download_verus_archive(archive_path, version, platform_config, cache_dir)

    cached_archive = cache_dir / archive_name
    if not cached_archive.is_file() or cached_archive.stat().st_size == 0:
        raise VerusSetupError(
            f"Verus archive not found in cache after download: {cached_archive}"
        )
    if not validate_zip_archive(cached_archive):
        cached_archive.unlink(missing_ok=True)
        raise VerusSetupError(
            f"downloaded Verus archive failed validation: {cached_archive}"
        )

    print_success(f"Verus {version} archive cached in {cache_dir}.")


# ==================================================================================================
# Toolchain Helpers
# ==================================================================================================


def ensure_verus_toolchain(install_dir: Path) -> None:
    """Install the Rust toolchain specified by Verus when it is missing."""
    version_json = install_dir / "version.json"
    rustup = shutil.which("rustup")
    if not version_json.is_file() or rustup is None:
        return

    try:
        version_data = json.loads(version_json.read_text(encoding="utf-8"))
        verus_data = version_data.get("verus", {})
        required_toolchain = verus_data.get("toolchain", "")
    except (AttributeError, json.JSONDecodeError, OSError, TypeError):
        print_warning(f"Failed to parse {version_json}; skipping toolchain check.")
        return

    if not isinstance(required_toolchain, str) or not required_toolchain:
        return

    probe = subprocess.run(
        [rustup, "run", required_toolchain, "rustc", "--version"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if probe.returncode == 0:
        return

    print_info(f"Installing Rust toolchain '{required_toolchain}' required by Verus...")
    subprocess.run(
        [
            rustup,
            "toolchain",
            "install",
            required_toolchain,
            "--profile",
            "minimal",
            "--component",
            "rust-src",
        ],
        check=True,
    )


# ==================================================================================================
# Command-Line Interface
# ==================================================================================================


def parse_args(arguments: Sequence[str] | None = None) -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser(
        description="Install the pinned Verus release for Linux or Windows."
    )
    parser.add_argument(
        "install_dir",
        nargs="?",
        type=Path,
        help="installation directory (default: ~/verus)",
    )
    parser.add_argument(
        "--download-only",
        "-DownloadOnly",
        action="store_true",
        help="download the Verus archive to the local cache without installing it",
    )
    return parser.parse_args(arguments)


def run(arguments: argparse.Namespace) -> None:
    """Execute the Verus setup operation."""
    platform_config = detect_platform()
    install_dir = (
        arguments.install_dir.expanduser()
        if arguments.install_dir is not None
        else get_default_install_dir(platform_config)
    )
    cache_dir = get_cache_dir()
    expected_version = get_expected_version()

    if arguments.download_only:
        download_only(expected_version, cache_dir, platform_config)
        return

    installed_version = get_installed_version(install_dir)
    if installed_version == expected_version:
        print_info(f"Verus {expected_version} is already installed in {install_dir}.")
        ensure_verus_toolchain(install_dir)
        return

    if installed_version:
        print_info(
            f"Verus version mismatch (found '{installed_version}', expected "
            f"'{expected_version}'). Updating..."
        )
    else:
        print_info(f"Verus not found in {install_dir}. Installing...")

    install_verus(install_dir, expected_version, cache_dir, platform_config)
    ensure_verus_toolchain(install_dir)


def main(arguments: Sequence[str] | None = None) -> int:
    """Run the command-line interface and return a process exit code."""
    try:
        run(parse_args(arguments))
    except (OSError, subprocess.SubprocessError, VerusSetupError) as error:
        print_error(str(error))
        return 1
    except KeyboardInterrupt:
        print_error("interrupted")
        return 130
    return 0


if __name__ == "__main__":
    sys.exit(main())
