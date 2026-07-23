# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Unit tests for the cross-platform Verus setup script."""

from __future__ import annotations

import io
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest.mock import patch

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


if __name__ == "__main__":
    unittest.main()
