# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Regression tests for configuration changes in the shared build output directory."""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

_REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_REPO_ROOT))

import z  # noqa: E402

_INPUT_MTIME = 1_600_000_000
_PROGRAMS = ("hello-rust-nostd", "test-rust-kernel", "test-c-misc")


class BuildConfigurationTest(unittest.TestCase):
    """Exercise the real Make dependency graph without compiling guest binaries."""

    def setUp(self) -> None:
        environment = mock.patch.dict(os.environ)
        environment.start()
        self.addCleanup(environment.stop)
        os.environ.pop("MAKEFLAGS", None)
        os.environ.pop("MFLAGS", None)
        plat = z.PlatformInfo.detect(_REPO_ROOT)
        if plat.is_windows:
            z.setup_windows_make_env(plat)
        make = z.find_make(plat)
        if make is None:
            self.fail("GNU Make is required for build configuration regression tests")

        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.binaries = self.directory / "bin"
        self.binaries.mkdir()
        self.images = [self.binaries / f"{name}.initrd" for name in _PROGRAMS]
        for name in (*_PROGRAMS, "procd", "memd", "vfsd"):
            self._write_input(self.binaries / f"{name}.elf", "initial")
        self.posix_input = self.directory / "posix-input"
        self._write_input(self.posix_input, "initial")
        mkimage = self.binaries / ("mkimage.exe" if plat.is_windows else "mkimage.elf")
        self._write_input(mkimage, "unused")

        # Substitute the compiler/linker inputs and recipes, but keep the real guard dependencies.
        recipes = self.directory / "recipes.mk"
        recipes.write_text(
            "\n".join(
                f"{image.as_posix()}:\n"
                f"\t@cat {image.with_suffix('.elf').as_posix()} > $@\n"
                for image in self.images
            )
            + f"{self.binaries.as_posix()}/test-c-misc.elf:\n"
            + f"\t@cat {self.posix_input.as_posix()} > $@\n",
            encoding="utf-8",
        )
        self.command = [
            make,
            "--no-print-directory",
            "-j4",
            "-f",
            str(_REPO_ROOT / "Makefile"),
            "-f",
            str(recipes),
            "-o",
            "all-guest-binaries",
            "-o",
            "all-host-binaries-mkimage",
            f"BINARIES_DIR={self.binaries.as_posix()}",
            f"POSIX_TESTS_SRCDIR={self.directory.as_posix()}",
            f"POSIX_TEST_CRT_OBJ={self.posix_input.as_posix()}",
            f"GUEST_C_APP_LIBC={self.posix_input.as_posix()}",
            f"GUEST_C_APP_LIBM={self.posix_input.as_posix()}",
            f"LIBNVX_CRT0={self.posix_input.as_posix()}",
            f"GUEST_C_APP_LD_SCRIPT={self.posix_input.as_posix()}",
            "RELEASE_VERSION=test",
            "SCCACHE=",
        ]

    def _write_input(self, path: Path, contents: str) -> None:
        path.write_text(contents, encoding="utf-8")
        os.utime(path, (_INPUT_MTIME, _INPUT_MTIME))

    def _build(self, **configuration: str) -> None:
        options = {
            "TARGET": "x86",
            "RELEASE": "no",
            "PROFILER": "no",
            "LOG_LEVEL": "trace",
            "MEMORY_SIZE": "256",
        }
        options.update(configuration)
        result = subprocess.run(
            self.command
            + [f"{key}={value}" for key, value in options.items()]
            + [image.as_posix() for image in self.images],
            cwd=_REPO_ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_configuration_round_trips_rebuild_older_inputs(self) -> None:
        for option, value in (
            ("RELEASE", "yes"),
            ("LOG_LEVEL", "error"),
            ("MEMORY_SIZE", "128"),
            ("TARGET", "x86_64"),
            ("PROFILER", "yes"),
        ):
            with self.subTest(option=option):
                baseline = {"RELEASE": "yes"} if option == "PROFILER" else {}
                for index, configuration in enumerate(
                    (baseline, baseline | {option: value}, baseline)
                ):
                    contents = f"{option}-{index}"
                    for name in _PROGRAMS:
                        if name != "test-c-misc":
                            self._write_input(self.binaries / f"{name}.elf", contents)
                    self._write_input(self.posix_input, contents)
                    self._build(**configuration)
                    for image in self.images:
                        self.assertTrue(image.exists(), f"Missing image: {image.name}")
                        self.assertEqual(image.read_text(encoding="utf-8"), contents)
                # The next subtest starts with a fresh image, not the previous configuration.
                for image in self.images:
                    image.unlink()
                (self.binaries / "test-c-misc.elf").unlink()

    def test_unchanged_configuration_preserves_images(self) -> None:
        self._build()
        before = [image.stat().st_mtime_ns for image in self.images]
        self._build()
        self.assertEqual([image.stat().st_mtime_ns for image in self.images], before)

    def test_switch_removes_unused_shared_outputs_only(self) -> None:
        self._build()
        stale = [
            self.binaries / "unused.initrd",
            self.binaries / "test-c-unused.elf",
            self.binaries / "posix-tests-ramfs-unused.img",
        ]
        preserved = [
            self.binaries / "kernel.elf",
            self.binaries / "user-data.img",
        ]
        for path in stale + preserved:
            self._write_input(path, "preserve only unrelated files")
        self._build(LOG_LEVEL="error")
        for path in stale:
            self.assertFalse(path.exists(), f"Stale output was retained: {path.name}")
        for path in preserved:
            self.assertTrue(path.exists(), f"Unrelated output was removed: {path.name}")

    def test_legacy_target_sentinel_does_not_preserve_old_images(self) -> None:
        (self.binaries / ".last-build-target").write_text("x86\n", encoding="utf-8")
        for image in self.images:
            image.write_text("stale", encoding="utf-8")
        self._build()
        for image in self.images:
            self.assertEqual(image.read_text(encoding="utf-8"), "initial")


if __name__ == "__main__":
    unittest.main()
