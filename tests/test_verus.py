# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Unit tests for the cross-platform Verus setup script."""

from __future__ import annotations

import io
import subprocess
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest.mock import call, patch

from scripts.setup import verus as verusmod


class TestArchiveValidation(unittest.TestCase):
    """Tests for Verus archive member validation."""

    def test_rejects_windows_drive_qualified_relative_path(self) -> None:
        """Drive-qualified relative paths cannot escape extraction validation."""
        with tempfile.TemporaryDirectory() as temp_dir:
            root_dir = Path(temp_dir)
            archive_path = root_dir / "archive.zip"
            with zipfile.ZipFile(archive_path, "w") as archive:
                archive.writestr("C:outside", b"archive contents")

            with self.assertRaisesRegex(verusmod.VerusSetupError, "unsafe path"):
                verusmod.extract_verus_windows(archive_path, root_dir / "destination")


class TestDownloadVerusArchive(unittest.TestCase):
    """Tests for Verus archive downloads."""

    def test_corrupt_cache_can_disappear_before_cleanup(self) -> None:
        """A concurrently removed corrupt cache entry does not abort the download."""
        version = "test-version"
        archive_contents = b"replacement archive"

        with tempfile.TemporaryDirectory() as temp_dir:
            root_dir = Path(temp_dir)
            cache_dir = root_dir / "cache"
            cache_dir.mkdir()
            cached_archive = cache_dir / verusmod.LINUX.archive_name(version)
            cached_archive.write_bytes(b"corrupt archive")
            destination = root_dir / "download.zip"

            def validate_archive(archive_path: Path) -> bool:
                if archive_path == cached_archive:
                    archive_path.unlink()
                    return False
                return True

            with (
                patch.object(
                    verusmod, "validate_zip_archive", side_effect=validate_archive
                ),
                patch.object(
                    verusmod.urllib.request,
                    "urlopen",
                    return_value=io.BytesIO(archive_contents),
                ),
                patch.object(verusmod, "print_warning"),
            ):
                verusmod.download_verus_archive(
                    destination,
                    version,
                    verusmod.LINUX,
                    cache_dir,
                )

            self.assertEqual(destination.read_bytes(), archive_contents)
            self.assertEqual(cached_archive.read_bytes(), archive_contents)


class TestEnsureVerusToolchain(unittest.TestCase):
    """Tests for installing the Rust toolchain required by Verus."""

    def test_strips_rustup_annotation_before_install(self) -> None:
        """Rustup annotations in Verus metadata are not part of the toolchain name."""
        toolchain = "1.98.0-x86_64-unknown-linux-gnu"
        annotated_toolchain = (
            f"{toolchain} (overridden by environment variable RUSTUP_TOOLCHAIN)"
        )

        with tempfile.TemporaryDirectory() as temp_dir:
            install_dir = Path(temp_dir)
            (install_dir / "version.json").write_text(
                f'{{"verus": {{"toolchain": "{annotated_toolchain}"}}}}',
                encoding="utf-8",
            )

            with (
                patch.object(verusmod.shutil, "which", return_value="rustup"),
                patch.object(
                    verusmod.subprocess,
                    "run",
                    side_effect=[
                        subprocess.CompletedProcess(args=[], returncode=1),
                        subprocess.CompletedProcess(args=[], returncode=0),
                    ],
                ) as run_command,
                patch.object(verusmod, "print_warning") as print_warning,
            ):
                verusmod.ensure_verus_toolchain(install_dir)

        print_warning.assert_called_once_with(
            "Verus Rust toolchain metadata contains an annotation; "
            f"using '{toolchain}'."
        )
        self.assertEqual(
            run_command.call_args_list,
            [
                call(
                    ["rustup", "run", toolchain, "rustc", "--version"],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    check=False,
                ),
                call(
                    [
                        "rustup",
                        "toolchain",
                        "install",
                        toolchain,
                        "--profile",
                        "minimal",
                        "--component",
                        "rust-src",
                    ],
                    check=True,
                ),
            ],
        )


if __name__ == "__main__":
    unittest.main()
