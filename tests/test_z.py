# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Unit tests for the z.py build backend.

Tests cover CLI parsing, BuildConfig, build argument assembly, environment
validation, path helpers, Windows pre-build steps, and subcommand logic.
Uses only the standard library (unittest + unittest.mock).
"""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import MagicMock, patch

# ---------------------------------------------------------------------------
# Import the module under test
# ---------------------------------------------------------------------------

# z.py lives at the repo root; add it to sys.path so we can import it.
_REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_REPO_ROOT))

import z as zmod  # noqa: E402 — must follow sys.path mutation

# ===========================================================================
# Helpers
# ===========================================================================


def _linux_plat(repo_root: Path | None = None) -> zmod.PlatformInfo:
    """Return a PlatformInfo that looks like Linux."""
    return zmod.PlatformInfo(
        is_windows=False,
        is_linux=True,
        repo_root=repo_root or Path("/repo"),
        home_dir=Path("/home/user"),
    )


def _windows_plat(repo_root: Path | None = None) -> zmod.PlatformInfo:
    """Return a PlatformInfo that looks like Windows."""
    return zmod.PlatformInfo(
        is_windows=True,
        is_linux=False,
        repo_root=repo_root or Path("C:/repo"),
        home_dir=Path("C:/Users/user"),
    )


# ===========================================================================
# CLI Parsing (parse_cli)
# ===========================================================================


class TestParseCli(unittest.TestCase):
    """Tests for the parse_cli function."""

    # --- Subcommand extraction ---

    def test_subcommand_build(self) -> None:
        """Parsing ['build'] extracts 'build' as the subcommand."""
        cmd, _ = zmod.parse_cli(["build"])
        self.assertEqual(cmd, "build")

    def test_subcommand_test(self) -> None:
        """Parsing ['test'] extracts 'test' as the subcommand."""
        cmd, _ = zmod.parse_cli(["test"])
        self.assertEqual(cmd, "test")

    def test_subcommand_clean(self) -> None:
        """Parsing ['clean'] extracts 'clean' as the subcommand."""
        cmd, _ = zmod.parse_cli(["clean"])
        self.assertEqual(cmd, "clean")

    def test_subcommand_distclean(self) -> None:
        """Parsing ['distclean'] extracts 'distclean' as the subcommand."""
        cmd, _ = zmod.parse_cli(["distclean"])
        self.assertEqual(cmd, "distclean")

    def test_subcommand_setup(self) -> None:
        """Parsing ['setup'] extracts 'setup' as the subcommand."""
        cmd, _ = zmod.parse_cli(["setup"])
        self.assertEqual(cmd, "setup")

    def test_subcommand_run(self) -> None:
        """Parsing ['run'] extracts 'run' as the subcommand."""
        cmd, _ = zmod.parse_cli(["run"])
        self.assertEqual(cmd, "run")

    def test_subcommand_verify(self) -> None:
        """Parsing ['verify'] extracts 'verify' as the subcommand."""
        cmd, _ = zmod.parse_cli(["verify"])
        self.assertEqual(cmd, "verify")

    def test_subcommand_bench(self) -> None:
        """Parsing ['bench'] extracts 'bench' as the subcommand."""
        cmd, _ = zmod.parse_cli(["bench"])
        self.assertEqual(cmd, "bench")

    def test_subcommand_help(self) -> None:
        """Parsing ['help'] extracts 'help' as the subcommand."""
        cmd, _ = zmod.parse_cli(["help"])
        self.assertEqual(cmd, "help")

    # --- Empty argv defaults to help ---

    def test_empty_argv_returns_help(self) -> None:
        """An empty argument list defaults to the 'help' subcommand with default config."""
        cmd, cfg = zmod.parse_cli([])
        self.assertEqual(cmd, "help")
        # Config should have all defaults.
        self.assertFalse(cfg.release)
        self.assertFalse(cfg.profile)
        self.assertEqual(cfg.make_args, [])

    # --- --release option ---

    def test_release_flag(self) -> None:
        """The --release flag sets release=True without enabling profiling."""
        cmd, cfg = zmod.parse_cli(["build", "--release"])
        self.assertEqual(cmd, "build")
        self.assertTrue(cfg.release)
        self.assertFalse(cfg.profile)

    # --- --profile option ---

    def test_profile_flag_implies_release_and_profiler(self) -> None:
        """The --profile flag implies both release and profiler modes."""
        cmd, cfg = zmod.parse_cli(["build", "--profile"])
        self.assertEqual(cmd, "build")
        self.assertTrue(cfg.profile)
        self.assertTrue(cfg.release)
        self.assertTrue(cfg.profiler)

    def test_verus_flag(self) -> None:
        """The --verus flag enables the verus config field."""
        _, cfg = zmod.parse_cli(["setup", "--verus"])
        self.assertTrue(cfg.verus)

    # --- -- separator behaviour ---

    def test_explicit_separator(self) -> None:
        """Arguments after an explicit '--' separator are collected as make_args."""
        cmd, cfg = zmod.parse_cli(["build", "--release", "--", "all", "RELEASE=yes"])
        self.assertEqual(cmd, "build")
        self.assertTrue(cfg.release)
        self.assertEqual(cfg.make_args, ["all", "RELEASE=yes"])

    def test_implicit_separator(self) -> None:
        """First non-option arg starts make_args without explicit '--'."""
        cmd, cfg = zmod.parse_cli(["build", "all", "MACHINE=microvm"])
        self.assertEqual(cmd, "build")
        self.assertEqual(cfg.make_args, ["all", "MACHINE=microvm"])
        self.assertEqual(cfg.machine, "microvm")

    # --- KEY=VALUE extraction into BuildConfig ---

    def test_key_value_machine(self) -> None:
        """MACHINE=microvm in make_args sets cfg.machine."""
        _, cfg = zmod.parse_cli(["build", "--", "MACHINE=microvm"])
        self.assertEqual(cfg.machine, "microvm")

    def test_key_value_target(self) -> None:
        """TARGET=x86 in make_args sets cfg.target."""
        _, cfg = zmod.parse_cli(["build", "--", "TARGET=x86"])
        self.assertEqual(cfg.target, "x86")

    def test_key_value_log_level(self) -> None:
        """LOG_LEVEL=warn in make_args sets cfg.log_level."""
        _, cfg = zmod.parse_cli(["build", "--", "LOG_LEVEL=warn"])
        self.assertEqual(cfg.log_level, "warn")

    def test_key_value_timeout(self) -> None:
        """TIMEOUT=120 in make_args sets cfg.timeout to an integer."""
        _, cfg = zmod.parse_cli(["build", "--", "TIMEOUT=120"])
        self.assertEqual(cfg.timeout, 120)

    def test_key_value_release_yes(self) -> None:
        """RELEASE=yes in make_args enables cfg.release."""
        _, cfg = zmod.parse_cli(["build", "--", "RELEASE=yes"])
        self.assertTrue(cfg.release)

    def test_key_value_release_no(self) -> None:
        """RELEASE=no in make_args keeps cfg.release disabled."""
        _, cfg = zmod.parse_cli(["build", "--", "RELEASE=no"])
        self.assertFalse(cfg.release)

    def test_key_value_profiler(self) -> None:
        """PROFILER=yes in make_args enables cfg.profiler."""
        _, cfg = zmod.parse_cli(["build", "--", "PROFILER=yes"])
        self.assertTrue(cfg.profiler)

    def test_key_value_whp(self) -> None:
        """WHP=yes in make_args enables cfg.whp."""
        _, cfg = zmod.parse_cli(["build", "--", "WHP=yes"])
        self.assertTrue(cfg.whp)

    def test_key_value_verbose(self) -> None:
        """VERBOSE=yes in make_args enables cfg.verbose."""
        _, cfg = zmod.parse_cli(["build", "--", "VERBOSE=yes"])
        self.assertTrue(cfg.verbose)

    def test_key_value_host_cpu(self) -> None:
        """HOST_CPU=skylake in make_args sets cfg.host_cpu."""
        _, cfg = zmod.parse_cli(["build", "--", "HOST_CPU=skylake"])
        self.assertEqual(cfg.host_cpu, "skylake")

    def test_key_value_image(self) -> None:
        """IMAGE=custom.img in make_args sets cfg.image."""
        _, cfg = zmod.parse_cli(["build", "--", "IMAGE=custom.img"])
        self.assertEqual(cfg.image, "custom.img")

    def test_key_value_message_format(self) -> None:
        """MESSAGE_FORMAT=json in make_args sets cfg.message_format."""
        _, cfg = zmod.parse_cli(["build", "--", "MESSAGE_FORMAT=json"])
        self.assertEqual(cfg.message_format, "json")

    def test_key_value_sysroot_dir(self) -> None:
        """SYSROOT_DIR=/opt/sysroot in make_args sets cfg.sysroot_dir."""
        _, cfg = zmod.parse_cli(["build", "--", "SYSROOT_DIR=/opt/sysroot"])
        self.assertEqual(cfg.sysroot_dir, "/opt/sysroot")

    def test_passthrough_vars_do_not_change_config(self) -> None:
        """SCCACHE, MAKE_NO_PRINT, VERUS_EXECUTABLE_DIR are pass-through."""
        _, cfg = zmod.parse_cli(
            [
                "build",
                "--",
                "SCCACHE=1",
                "MAKE_NO_PRINT=yes",
                "VERUS_EXECUTABLE_DIR=/opt/verus",
            ]
        )
        # These vars have no z.py-side effect, so config fields remain at defaults.
        self.assertEqual(cfg.machine, zmod.DEFAULT_MACHINE)

    # --- Validation of invalid values ---

    def test_invalid_machine_dies(self) -> None:
        """An unrecognized MACHINE value causes a fatal exit."""
        with patch("z.print_error"), self.assertRaises(SystemExit):
            zmod.parse_cli(["build", "--", "MACHINE=invalid"])

    def test_invalid_target_dies(self) -> None:
        """An unsupported TARGET value causes a fatal exit."""
        with patch("z.print_error"), self.assertRaises(SystemExit):
            zmod.parse_cli(["build", "--", "TARGET=arm"])

    def test_invalid_log_level_dies(self) -> None:
        """An unrecognized LOG_LEVEL value causes a fatal exit."""
        with patch("z.print_error"), self.assertRaises(SystemExit):
            zmod.parse_cli(["build", "--", "LOG_LEVEL=invalid"])

    def test_invalid_timeout_dies(self) -> None:
        """A non-numeric TIMEOUT value causes a fatal exit."""
        with patch("z.print_error"), self.assertRaises(SystemExit):
            zmod.parse_cli(["build", "--", "TIMEOUT=abc"])

    def test_invalid_message_format_dies(self) -> None:
        """An unsupported MESSAGE_FORMAT value causes a fatal exit."""
        with patch("z.print_error"), self.assertRaises(SystemExit):
            zmod.parse_cli(["build", "--", "MESSAGE_FORMAT=xml"])

    # --- Unknown option rejection ---

    def test_unknown_option_dies(self) -> None:
        """An unrecognized --flag causes a fatal exit."""
        with patch("z.print_error"), self.assertRaises(SystemExit):
            zmod.parse_cli(["build", "--unknown-flag"])

    # --- Multiple options combined ---

    def test_combined_release_and_make_args(self) -> None:
        """--release combined with KEY=VALUE make_args sets all config fields correctly."""
        cmd, cfg = zmod.parse_cli(
            ["build", "--release", "--", "all", "MACHINE=microvm", "LOG_LEVEL=info"]
        )
        self.assertEqual(cmd, "build")
        self.assertTrue(cfg.release)
        self.assertEqual(cfg.machine, "microvm")
        self.assertEqual(cfg.log_level, "info")
        self.assertIn("all", cfg.make_args)

    def test_options_before_and_after_separator(self) -> None:
        """Options before -- are z.py flags, everything after is make_args."""
        cmd, cfg = zmod.parse_cli(["build", "--profile", "--", "guest", "TIMEOUT=30"])
        self.assertEqual(cmd, "build")
        self.assertTrue(cfg.profile)
        self.assertTrue(cfg.release)
        self.assertTrue(cfg.profiler)
        self.assertEqual(cfg.timeout, 30)
        self.assertIn("guest", cfg.make_args)


# ===========================================================================
# BuildConfig
# ===========================================================================


class TestBuildConfig(unittest.TestCase):
    """Tests for BuildConfig and apply_platform_defaults."""

    def test_defaults(self) -> None:
        """A default-constructed BuildConfig has expected initial values."""
        cfg = zmod.BuildConfig()
        self.assertEqual(cfg.machine, zmod.DEFAULT_MACHINE)
        self.assertEqual(cfg.target, zmod.DEFAULT_TARGET)
        self.assertFalse(cfg.release)
        self.assertFalse(cfg.profile)
        self.assertEqual(cfg.log_level, "")
        self.assertEqual(cfg.timeout, zmod.DEFAULT_TIMEOUT)
        self.assertFalse(cfg.profiler)
        self.assertFalse(cfg.whp)
        self.assertEqual(cfg.make_args, [])

    # --- apply_platform_defaults on Linux ---

    def test_linux_default_log_level_debug(self) -> None:
        """On Linux in debug mode, the default log level is the debug-mode default."""
        cfg = zmod.BuildConfig()
        cfg.apply_platform_defaults(_linux_plat())
        self.assertEqual(cfg.log_level, zmod.DEFAULT_LOG_LEVEL_DEBUG)

    def test_linux_default_log_level_release(self) -> None:
        """On Linux in release mode, the default log level is the release-mode default."""
        cfg = zmod.BuildConfig(release=True)
        cfg.apply_platform_defaults(_linux_plat())
        self.assertEqual(cfg.log_level, zmod.DEFAULT_LOG_LEVEL_RELEASE)

    def test_linux_no_whp_injection(self) -> None:
        """On Linux, WHP is never auto-enabled even for microvm."""
        cfg = zmod.BuildConfig(machine="microvm")
        cfg.apply_platform_defaults(_linux_plat())
        self.assertFalse(cfg.whp)

    # --- apply_platform_defaults on Windows ---

    def test_windows_whp_auto_injection_for_microvm(self) -> None:
        """On Windows with machine=microvm, WHP is auto-enabled."""
        cfg = zmod.BuildConfig(machine="microvm")
        cfg.apply_platform_defaults(_windows_plat())
        self.assertTrue(cfg.whp)

    # --- --profile implies release + profiler ---

    def test_profile_implies_release_and_profiler(self) -> None:
        """The --profile CLI flag sets profile, release, and profiler together."""
        _, cfg = zmod.parse_cli(["build", "--profile"])
        self.assertTrue(cfg.profile)
        self.assertTrue(cfg.release)
        self.assertTrue(cfg.profiler)

    # --- User-set values are preserved ---

    def test_user_log_level_preserved(self) -> None:
        """A user-set log_level is not overwritten by platform defaults."""
        cfg = zmod.BuildConfig(log_level="error")
        cfg.apply_platform_defaults(_linux_plat())
        self.assertEqual(cfg.log_level, "error")

    # --- Log level based on release mode ---

    def test_default_log_level_debug_mode(self) -> None:
        """In debug mode (release=False), the default log level is 'trace'."""
        cfg = zmod.BuildConfig(release=False)
        cfg.apply_platform_defaults(_linux_plat())
        self.assertEqual(cfg.log_level, "trace")

    def test_default_log_level_release_mode(self) -> None:
        """In release mode, the default log level is 'warn'."""
        cfg = zmod.BuildConfig(release=True)
        cfg.apply_platform_defaults(_linux_plat())
        self.assertEqual(cfg.log_level, "warn")


# ===========================================================================
# Build Argument Assembly (_assemble_build_make_args)
# ===========================================================================


class TestAssembleBuildMakeArgs(unittest.TestCase):
    """Tests for _assemble_build_make_args."""

    # --- Basic injections ---

    def test_release_flag_injects_release_yes(self) -> None:
        """When release=True, RELEASE=yes is injected into the auto-generated args."""
        cfg = zmod.BuildConfig(release=True)
        injected, user = zmod._assemble_build_make_args(_linux_plat(), cfg)
        self.assertIn("RELEASE=yes", injected)

    def test_no_release_no_injection(self) -> None:
        """When release=False, RELEASE=yes is not injected."""
        cfg = zmod.BuildConfig(release=False)
        injected, _ = zmod._assemble_build_make_args(_linux_plat(), cfg)
        self.assertNotIn("RELEASE=yes", injected)

    # --- Profile strips user RELEASE= and injects RELEASE=yes + PROFILER=yes ---

    def test_profile_injects_release_and_profiler(self) -> None:
        """Profile mode injects both RELEASE=yes and PROFILER=yes."""
        cfg = zmod.BuildConfig(profile=True, release=True, make_args=["all"])
        injected, user = zmod._assemble_build_make_args(_linux_plat(), cfg)
        self.assertIn("RELEASE=yes", injected)
        self.assertIn("PROFILER=yes", injected)

    def test_profile_strips_user_release(self) -> None:
        """Profile mode strips any user-provided RELEASE= and forces RELEASE=yes."""
        cfg = zmod.BuildConfig(
            profile=True, release=True, make_args=["RELEASE=no", "all"]
        )
        injected, user = zmod._assemble_build_make_args(_linux_plat(), cfg)
        self.assertIn("RELEASE=yes", injected)
        self.assertIn("PROFILER=yes", injected)
        # User-provided RELEASE= should be stripped.
        self.assertNotIn("RELEASE=no", user)
        # But 'all' should remain.
        self.assertIn("all", user)

    # --- Release does not inject if user provides RELEASE= ---

    def test_release_no_duplicate_if_user_provides(self) -> None:
        """If the user already supplies RELEASE=, z.py does not inject a duplicate."""
        cfg = zmod.BuildConfig(release=True, make_args=["RELEASE=yes"])
        injected, user = zmod._assemble_build_make_args(_linux_plat(), cfg)
        # z.py should NOT inject because user already provided one.
        self.assertNotIn("RELEASE=yes", injected)
        self.assertIn("RELEASE=yes", user)

    # --- Windows-specific defaults ---

    def test_windows_injects_whp_yes_for_microvm(self) -> None:
        """On Windows with machine=microvm, WHP=yes is auto-injected."""
        cfg = zmod.BuildConfig(machine="microvm", make_args=["all"])
        with patch("z.platform.machine", return_value="AMD64"):
            injected, _ = zmod._assemble_build_make_args(_windows_plat(), cfg)
        self.assertIn("WHP=yes", injected)

    def test_windows_arm64_injects_aarch64_target(self) -> None:
        """Native Windows ARM64 builds default to the AArch64 guest target."""
        cfg = zmod.BuildConfig(machine="microvm", make_args=["all"])
        with patch.dict(zmod.os.environ, {}, clear=True), patch(
            "z.platform.machine", return_value="ARM64"
        ):
            injected, _ = zmod._assemble_build_make_args(_windows_plat(), cfg)
        self.assertIn("TARGET=aarch64", injected)

    def test_windows_arm64_preserves_explicit_target(self) -> None:
        """An explicit Windows ARM64 target is not overridden."""
        cfg = zmod.BuildConfig(machine="microvm", make_args=["TARGET=x86_64"])
        with patch("z.platform.machine", return_value="ARM64"):
            injected, user = zmod._assemble_build_make_args(_windows_plat(), cfg)
        self.assertNotIn("TARGET=aarch64", injected)
        self.assertIn("TARGET=x86_64", user)

    def test_windows_arm64_preserves_environment_target(self) -> None:
        """A TARGET supplied through the environment is not overridden."""
        cfg = zmod.BuildConfig(machine="microvm", make_args=["all"])
        with patch.dict(zmod.os.environ, {"TARGET": "x86_64"}, clear=True), patch(
            "z.platform.machine", return_value="ARM64"
        ):
            injected, _ = zmod._assemble_build_make_args(_windows_plat(), cfg)
        self.assertNotIn("TARGET=aarch64", injected)

    def test_windows_no_duplicate_whp(self) -> None:
        """User-supplied WHP= should not be overridden."""
        cfg = zmod.BuildConfig(machine="microvm", make_args=["WHP=no"])
        with patch("z.platform.machine", return_value="AMD64"):
            injected, user = zmod._assemble_build_make_args(_windows_plat(), cfg)
        self.assertNotIn("WHP=yes", injected)
        self.assertIn("WHP=no", user)

    # --- Linux does not inject Windows defaults ---

    def test_linux_no_whp_injection(self) -> None:
        """On Linux, WHP=yes is never auto-injected."""
        cfg = zmod.BuildConfig(machine="microvm", make_args=["all"])
        injected, _ = zmod._assemble_build_make_args(_linux_plat(), cfg)
        self.assertNotIn("WHP=yes", injected)

    # --- User args pass through verbatim ---

    def test_user_args_pass_through(self) -> None:
        """User-provided make_args appear verbatim in the user portion of the output."""
        cfg = zmod.BuildConfig(make_args=["all", "MACHINE=microvm", "CUSTOM=val"])
        _, user = zmod._assemble_build_make_args(_linux_plat(), cfg)
        self.assertIn("all", user)
        self.assertIn("MACHINE=microvm", user)
        self.assertIn("CUSTOM=val", user)


# ===========================================================================
# Environment Validation
# ===========================================================================


class TestValidateGitContext(unittest.TestCase):
    """Tests for validate_git_context."""

    @patch("z.print_error")
    @patch("z.subprocess.run")
    def test_not_a_git_repo_dies(self, mock_run: MagicMock, _: MagicMock) -> None:
        """Running outside a git repository causes a fatal exit."""
        mock_run.side_effect = subprocess.CalledProcessError(128, "git")
        with self.assertRaises(SystemExit):
            zmod.validate_git_context()

    @patch("z.print_error")
    @patch("z.subprocess.run")
    def test_git_not_found_dies(self, mock_run: MagicMock, _: MagicMock) -> None:
        """A missing git executable causes a fatal exit."""
        mock_run.side_effect = FileNotFoundError("git")
        with self.assertRaises(SystemExit):
            zmod.validate_git_context()

    @patch("z.print_error")
    @patch("z.Path.cwd")
    @patch("z.subprocess.run")
    def test_wrong_cwd_dies(
        self, mock_run: MagicMock, mock_cwd: MagicMock, _: MagicMock
    ) -> None:
        """Running from a directory other than the repo root causes a fatal exit."""
        # First call: --is-inside-work-tree → true
        work_tree_result = MagicMock()
        work_tree_result.stdout = "true\n"
        # Second call: --show-toplevel → /repo
        toplevel_result = MagicMock()
        toplevel_result.stdout = "/repo\n"

        mock_run.side_effect = [work_tree_result, toplevel_result]

        # cwd returns a different directory.
        mock_cwd.return_value = Path("/some/other/dir")

        with self.assertRaises(SystemExit):
            zmod.validate_git_context()

    @patch("z.Path.cwd")
    @patch("z.subprocess.run")
    def test_valid_context_returns_repo_root(
        self, mock_run: MagicMock, mock_cwd: MagicMock
    ) -> None:
        """When cwd matches the repo root, the function returns the root path."""
        work_tree_result = MagicMock()
        work_tree_result.stdout = "true\n"
        toplevel_result = MagicMock()
        toplevel_result.stdout = str(_REPO_ROOT) + "\n"

        mock_run.side_effect = [work_tree_result, toplevel_result]
        mock_cwd.return_value = _REPO_ROOT

        root = zmod.validate_git_context()
        self.assertEqual(root, _REPO_ROOT)


# ===========================================================================
# Path Helpers
# ===========================================================================


class TestPathHelpers(unittest.TestCase):
    """Tests for _prepend_path and _append_path."""

    def setUp(self) -> None:
        self._original_path = os.environ.get("PATH", "")

    def tearDown(self) -> None:
        os.environ["PATH"] = self._original_path

    # --- _prepend_path ---

    def test_prepend_path_adds_to_front(self) -> None:
        """_prepend_path inserts the directory at the beginning of PATH."""
        os.environ["PATH"] = os.pathsep.join(["/usr/bin", "/usr/local/bin"])
        zmod._prepend_path("/my/dir")
        entries = os.environ["PATH"].split(os.pathsep)
        self.assertEqual(entries[0], "/my/dir")

    def test_prepend_path_idempotent(self) -> None:
        """_prepend_path does not duplicate an already-present directory."""
        os.environ["PATH"] = os.pathsep.join(["/my/dir", "/usr/bin"])
        zmod._prepend_path("/my/dir")
        count = os.environ["PATH"].split(os.pathsep).count("/my/dir")
        self.assertEqual(count, 1)

    def test_prepend_path_case_insensitive(self) -> None:
        """_prepend_path treats paths as case-insensitive when deduplicating."""
        # Intentional cross-platform design: z.py uses case-insensitive
        # comparison so PATH dedup works on both Windows and Linux.
        os.environ["PATH"] = os.pathsep.join(["/My/Dir", "/usr/bin"])
        zmod._prepend_path("/my/dir")
        # Should not add because case-fold matches.
        entries = os.environ["PATH"].split(os.pathsep)
        self.assertNotIn("/my/dir", entries)

    # --- _append_path ---

    def test_append_path_adds_to_end(self) -> None:
        """_append_path appends the directory at the end of PATH."""
        os.environ["PATH"] = os.pathsep.join(["/usr/bin", "/usr/local/bin"])
        zmod._append_path("/my/dir")
        entries = os.environ["PATH"].split(os.pathsep)
        self.assertEqual(entries[-1], "/my/dir")

    def test_append_path_idempotent(self) -> None:
        """_append_path does not duplicate an already-present directory."""
        os.environ["PATH"] = os.pathsep.join(["/usr/bin", "/my/dir"])
        zmod._append_path("/my/dir")
        count = os.environ["PATH"].split(os.pathsep).count("/my/dir")
        self.assertEqual(count, 1)

    def test_append_path_case_insensitive(self) -> None:
        """_append_path treats paths as case-insensitive when deduplicating."""
        # Intentional cross-platform design: z.py uses case-insensitive
        # comparison so PATH dedup works on both Windows and Linux.
        os.environ["PATH"] = os.pathsep.join(["/usr/bin", "/My/Dir"])
        zmod._append_path("/my/dir")
        entries = os.environ["PATH"].split(os.pathsep)
        self.assertNotIn("/my/dir", entries)

    # --- Empty PATH edge case ---

    def test_prepend_path_empty_path(self) -> None:
        """_prepend_path works correctly when PATH is initially empty."""
        os.environ["PATH"] = ""
        zmod._prepend_path("/my/dir")
        self.assertTrue(os.environ["PATH"].startswith("/my/dir"))

    def test_append_path_empty_path(self) -> None:
        """_append_path works correctly when PATH is initially empty."""
        os.environ["PATH"] = ""
        zmod._append_path("/my/dir")
        self.assertTrue(os.environ["PATH"].endswith("/my/dir"))


# ===========================================================================
# Windows Pre-Build: restore_git_symlinks
# ===========================================================================


class TestRestoreGitSymlinks(unittest.TestCase):
    """Tests for restore_git_symlinks."""

    def test_stub_detection_and_copy(self) -> None:
        """A text stub pointing at a real file should be replaced by a copy."""
        with tempfile.TemporaryDirectory() as repo:
            repo_path = Path(repo)

            # Create a target file.
            target = repo_path / "real_file.txt"
            target.write_text("real content", encoding="utf-8")

            # Create a text stub that points at the target (relative path).
            stub = repo_path / "link.txt"
            stub.write_text("real_file.txt", encoding="utf-8")

            # Mock git ls-files -s output: mode 120000 = symlink.
            ls_output = "120000 abc123 0\tlink.txt\n"

            mock_result = MagicMock()
            mock_result.stdout = ls_output
            mock_result.returncode = 0

            with patch("z.subprocess.run", return_value=mock_result):
                zmod.restore_git_symlinks(repo_path)

            # The stub should now contain the real file's content.
            self.assertEqual(stub.read_text(encoding="utf-8"), "real content")

    def test_real_symlink_is_skipped(self) -> None:
        """Files that are already real symlinks should be skipped."""
        with tempfile.TemporaryDirectory() as repo:
            repo_path = Path(repo)

            target = repo_path / "real_file.txt"
            target.write_text("real content", encoding="utf-8")

            link = repo_path / "link.txt"
            try:
                link.symlink_to(target)
            except OSError:
                self.skipTest("Symlink creation not supported on this platform")

            ls_output = "120000 abc123 0\tlink.txt\n"
            mock_result = MagicMock()
            mock_result.stdout = ls_output

            with patch("z.subprocess.run", return_value=mock_result):
                zmod.restore_git_symlinks(repo_path)

            # Should still be a symlink.
            self.assertTrue(link.is_symlink())

    def test_git_command_failure_returns_silently(self) -> None:
        """If git ls-files fails, the function should return without error."""
        with patch(
            "z.subprocess.run",
            side_effect=subprocess.CalledProcessError(1, "git"),
        ):
            # Should not raise.
            zmod.restore_git_symlinks(Path("/nonexistent"))

    def test_non_symlink_mode_is_skipped(self) -> None:
        """Lines with mode != 120000 should be ignored."""
        with tempfile.TemporaryDirectory() as repo:
            repo_path = Path(repo)
            f = repo_path / "normal.txt"
            f.write_text("content", encoding="utf-8")

            ls_output = "100644 abc123 0\tnormal.txt\n"
            mock_result = MagicMock()
            mock_result.stdout = ls_output

            with patch("z.subprocess.run", return_value=mock_result):
                zmod.restore_git_symlinks(repo_path)

            # File should be unchanged.
            self.assertEqual(f.read_text(encoding="utf-8"), "content")

    def test_stub_with_empty_content_is_skipped(self) -> None:
        """Empty stubs should be skipped."""
        with tempfile.TemporaryDirectory() as repo:
            repo_path = Path(repo)
            stub = repo_path / "empty_link.txt"
            stub.write_text("", encoding="utf-8")

            ls_output = "120000 abc123 0\tempty_link.txt\n"
            mock_result = MagicMock()
            mock_result.stdout = ls_output

            with patch("z.subprocess.run", return_value=mock_result):
                zmod.restore_git_symlinks(repo_path)

            # Should still be empty.
            self.assertEqual(stub.read_text(encoding="utf-8"), "")

    def test_directory_symlink_restore_failure_warns(self) -> None:
        """Failed directory symlink and junction creation should restore the stub and warn."""
        with tempfile.TemporaryDirectory() as repo:
            repo_path = Path(repo)

            target_dir = repo_path / "subdir"
            target_dir.mkdir()

            stub = repo_path / "link_to_dir.txt"
            stub.write_text("subdir", encoding="utf-8")

            ls_output = "120000 abc123 0\tlink_to_dir.txt\n"
            git_result = MagicMock()
            git_result.stdout = ls_output
            junction_result = MagicMock()
            junction_result.returncode = 1

            with patch("z.Path.symlink_to", side_effect=OSError):
                with patch(
                    "z.subprocess.run", side_effect=[git_result, junction_result]
                ):
                    with patch("z.print_warning") as mock_warn:
                        expanded = zmod.restore_git_symlinks(repo_path)

            self.assertEqual(expanded, [])
            self.assertEqual(stub.read_text(encoding="utf-8"), "subdir")
            mock_warn.assert_called_once()
            self.assertIn("link_to_dir.txt", mock_warn.call_args[0][0])

    @unittest.skipIf(os.name == "nt", "requires POSIX symlink support")
    def test_directory_symlink_is_restored_when_supported(self) -> None:
        """Directory targets should become real symlinks when the host permits it."""
        with tempfile.TemporaryDirectory() as repo:
            repo_path = Path(repo)
            target_dir = repo_path / "subdir"
            target_dir.mkdir()
            stub = repo_path / "link_to_dir.txt"
            stub.write_text("subdir", encoding="utf-8")

            git_result = MagicMock()
            git_result.stdout = "120000 abc123 0\tlink_to_dir.txt\n"

            with patch("z.subprocess.run", return_value=git_result):
                with patch("z.print_warning") as mock_warn:
                    expanded = zmod.restore_git_symlinks(repo_path)

            self.assertTrue(stub.is_symlink())
            self.assertEqual(len(expanded), 1)
            mock_warn.assert_not_called()


# ===========================================================================
# Subcommand: cmd_run
# ===========================================================================


@patch("z.print_info")
class TestCmdRun(unittest.TestCase):
    """Tests for cmd_run."""

    def test_default_program(self, _info: MagicMock) -> None:
        """Without -program, the default binary should be used."""
        plat = _linux_plat()
        cfg = zmod.BuildConfig(make_args=[])
        with patch("z.invoke_make", return_value=0) as mock_make:
            rc = zmod.cmd_run(plat, cfg)
        self.assertEqual(rc, 0)
        # invoke_make is called with targets=["run"]
        mock_make.assert_called_once()
        _, kwargs = mock_make.call_args
        self.assertIn("run", kwargs.get("targets", []))

    def test_program_argument_parsing(self, _info: MagicMock) -> None:
        """The -program arg and its value should be stripped from raw_args."""
        plat = _linux_plat()
        cfg = zmod.BuildConfig(make_args=["-program", "bin/custom.elf", "extra"])
        with patch("z.invoke_make", return_value=0) as mock_make:
            rc = zmod.cmd_run(plat, cfg)
        self.assertEqual(rc, 0)
        _, kwargs = mock_make.call_args
        # -program and its value should be stripped from raw_args.
        raw = kwargs.get("raw_args", [])
        self.assertNotIn("-program", raw)
        self.assertNotIn("bin/custom.elf", raw)
        self.assertIn("extra", raw)

    def test_windows_run_uses_nanvixd(self, _info: MagicMock) -> None:
        """On Windows, cmd_run invokes nanvixd.exe directly."""
        with tempfile.TemporaryDirectory() as repo:
            repo_path = Path(repo)
            bin_dir = repo_path / "bin"
            bin_dir.mkdir()
            nanvixd = bin_dir / "nanvixd.exe"
            nanvixd.write_text("fake", encoding="utf-8")

            plat = _windows_plat(repo_path)
            cfg = zmod.BuildConfig(make_args=[])

            mock_result = MagicMock()
            mock_result.returncode = 0

            with patch("z.subprocess.run", return_value=mock_result) as mock_run:
                rc = zmod.cmd_run(plat, cfg)

            self.assertEqual(rc, 0)
            args = mock_run.call_args[0][0]
            self.assertEqual(args[0], str(nanvixd))
            self.assertIn("--", args)
            self.assertIn(zmod.DEFAULT_RUN_PROGRAM, args)

    @patch("z.print_error")
    def test_windows_run_missing_nanvixd_dies(
        self, _err: MagicMock, _info: MagicMock
    ) -> None:
        """On Windows, dies if nanvixd.exe is not found."""
        with tempfile.TemporaryDirectory() as repo:
            repo_path = Path(repo)
            plat = _windows_plat(repo_path)
            cfg = zmod.BuildConfig(make_args=[])

            with self.assertRaises(SystemExit):
                zmod.cmd_run(plat, cfg)


# ===========================================================================
# Subcommand: cmd_bench
# ===========================================================================


@patch("z.print_info")
class TestCmdBench(unittest.TestCase):
    """Tests for cmd_bench."""

    @patch("z.print_error")
    def test_binary_not_found_dies_linux(
        self, _err: MagicMock, _info: MagicMock
    ) -> None:
        """On Linux, dies with a helpful message if nanvix-bench.elf is missing."""
        with tempfile.TemporaryDirectory() as repo:
            repo_path = Path(repo)
            plat = _linux_plat(repo_path)
            cfg = zmod.BuildConfig(make_args=[])

            with self.assertRaises(SystemExit):
                zmod.cmd_bench(plat, cfg)

    @patch("z.print_error")
    def test_binary_not_found_dies_windows(
        self, _err: MagicMock, _info: MagicMock
    ) -> None:
        """On Windows, dies with a helpful message if nanvix-bench.exe is missing."""
        with tempfile.TemporaryDirectory() as repo:
            repo_path = Path(repo)
            plat = _windows_plat(repo_path)
            cfg = zmod.BuildConfig(make_args=[])

            with self.assertRaises(SystemExit):
                zmod.cmd_bench(plat, cfg)

    def test_bench_runs_binary_with_args(self, _info: MagicMock) -> None:
        """cmd_bench should run the benchmark binary with make_args appended."""
        with tempfile.TemporaryDirectory() as repo:
            repo_path = Path(repo)
            bin_dir = repo_path / "bin"
            bin_dir.mkdir()
            bench_bin = bin_dir / "nanvix-bench.elf"
            bench_bin.write_text("fake", encoding="utf-8")

            plat = _linux_plat(repo_path)
            cfg = zmod.BuildConfig(make_args=["--boot-time", "--iterations=5"])

            mock_result = MagicMock()
            mock_result.returncode = 0

            with patch("z.subprocess.run", return_value=mock_result) as mock_run:
                rc = zmod.cmd_bench(plat, cfg)

            self.assertEqual(rc, 0)
            args = mock_run.call_args[0][0]
            self.assertEqual(args[0], str(bench_bin))
            self.assertIn("--boot-time", args)
            self.assertIn("--iterations=5", args)

    def test_bench_windows_extension(self, _info: MagicMock) -> None:
        """On Windows, looks for .exe extension."""
        with tempfile.TemporaryDirectory() as repo:
            repo_path = Path(repo)
            bin_dir = repo_path / "bin"
            bin_dir.mkdir()
            bench_bin = bin_dir / "nanvix-bench.exe"
            bench_bin.write_text("fake", encoding="utf-8")

            plat = _windows_plat(repo_path)
            cfg = zmod.BuildConfig(make_args=[])

            mock_result = MagicMock()
            mock_result.returncode = 0

            with patch("z.subprocess.run", return_value=mock_result) as mock_run:
                rc = zmod.cmd_bench(plat, cfg)

            self.assertEqual(rc, 0)
            args = mock_run.call_args[0][0]
            self.assertIn("nanvix-bench.exe", args[0])


# ===========================================================================
# Subcommand: cmd_build
# ===========================================================================


class TestCmdBuild(unittest.TestCase):
    """Tests for cmd_build."""

    @patch("z.print_success")
    @patch("z.print_info")
    @patch("z.invoke_make", return_value=0)
    def test_build_calls_invoke_make(
        self, mock_make: MagicMock, _: MagicMock, _s: MagicMock
    ) -> None:
        """cmd_build delegates to invoke_make with assembled args."""
        plat = _linux_plat()
        cfg = zmod.BuildConfig(release=True, make_args=["all"])
        rc = zmod.cmd_build(plat, cfg)
        self.assertEqual(rc, 0)
        mock_make.assert_called_once()
        _, kwargs = mock_make.call_args
        injected = kwargs.get("injected_vars", [])
        self.assertIn("RELEASE=yes", injected)

    @patch("z.print_error")
    @patch("z.print_info")
    @patch("z.invoke_make", return_value=2)
    def test_build_propagates_make_failure(
        self, mock_make: MagicMock, _: MagicMock, _e: MagicMock
    ) -> None:
        """cmd_build returns the non-zero exit code from invoke_make."""
        plat = _linux_plat()
        cfg = zmod.BuildConfig(make_args=["all"])
        rc = zmod.cmd_build(plat, cfg)
        self.assertEqual(rc, 2)

    @patch("z.print_success")
    @patch("z.print_info")
    @patch("z.invoke_make", return_value=0)
    @patch("z.restore_git_symlinks")
    def test_build_windows_restores_symlinks(
        self,
        mock_symlinks: MagicMock,
        _make: MagicMock,
        _info: MagicMock,
        _s: MagicMock,
    ) -> None:
        """On Windows, cmd_build calls restore_git_symlinks before building."""
        plat = _windows_plat()
        cfg = zmod.BuildConfig(make_args=["all"])
        zmod.cmd_build(plat, cfg)
        mock_symlinks.assert_called_once_with(plat.repo_root)


# ===========================================================================
# Subcommand: cmd_test
# ===========================================================================


class TestCmdTest(unittest.TestCase):
    """Tests for cmd_test."""

    @patch("z.print_success")
    @patch("z.invoke_make", return_value=0)
    def test_test_invokes_make_with_test_target(
        self, mock_make: MagicMock, _: MagicMock
    ) -> None:
        """cmd_test calls invoke_make with targets=['test']."""
        plat = _linux_plat()
        cfg = zmod.BuildConfig(make_args=[])
        rc = zmod.cmd_test(plat, cfg)
        self.assertEqual(rc, 0)
        mock_make.assert_called_once()
        _, kwargs = mock_make.call_args
        self.assertIn("test", kwargs.get("targets", []))

    @patch("z.print_success")
    @patch("z.invoke_make", return_value=0)
    @patch("z.restore_git_symlinks")
    def test_test_windows_restores_symlinks(
        self, mock_symlinks: MagicMock, _make: MagicMock, _s: MagicMock
    ) -> None:
        """On Windows, cmd_test calls restore_git_symlinks before testing."""
        plat = _windows_plat()
        cfg = zmod.BuildConfig(make_args=[])
        zmod.cmd_test(plat, cfg)
        mock_symlinks.assert_called_once_with(plat.repo_root)


# ===========================================================================
# Subcommand: cmd_clean / cmd_distclean
# ===========================================================================


class TestCmdClean(unittest.TestCase):
    """Tests for cmd_clean and cmd_distclean."""

    @patch("z.print_success")
    @patch("z.print_info")
    @patch("z.invoke_make", return_value=0)
    def test_clean_calls_make_clean(
        self, mock_make: MagicMock, _: MagicMock, _s: MagicMock
    ) -> None:
        """cmd_clean invokes make with targets=['clean']."""
        plat = _linux_plat()
        cfg = zmod.BuildConfig()
        rc = zmod.cmd_clean(plat, cfg)
        self.assertEqual(rc, 0)
        _, kwargs = mock_make.call_args
        self.assertIn("clean", kwargs.get("targets", []))

    @patch("z.print_success")
    @patch("z.print_info")
    @patch("z.invoke_make", return_value=0)
    def test_distclean_calls_make_distclean(
        self, mock_make: MagicMock, _: MagicMock, _s: MagicMock
    ) -> None:
        """cmd_distclean invokes make with targets=['distclean']."""
        plat = _linux_plat()
        cfg = zmod.BuildConfig()
        rc = zmod.cmd_distclean(plat, cfg)
        self.assertEqual(rc, 0)
        _, kwargs = mock_make.call_args
        self.assertIn("distclean", kwargs.get("targets", []))


# ===========================================================================
# Subcommand: cmd_setup
# ===========================================================================


class TestCmdSetup(unittest.TestCase):
    """Tests for cmd_setup."""

    @patch("z.print_info")
    @patch("z.subprocess.run")
    def test_install_verus_uses_python_script(
        self, mock_run: MagicMock, _info: MagicMock
    ) -> None:
        """Linux and Windows use the current Python interpreter for Verus setup."""
        mock_run.return_value.returncode = 0

        with tempfile.TemporaryDirectory() as temp_dir:
            repo_root = Path(temp_dir)
            verus_script = repo_root / "scripts" / "setup" / "verus.py"
            verus_script.parent.mkdir(parents=True)
            verus_script.touch()

            for plat in (_linux_plat(repo_root), _windows_plat(repo_root)):
                with self.subTest(is_windows=plat.is_windows):
                    mock_run.reset_mock()
                    zmod._install_verus(plat)
                    mock_run.assert_called_once_with(
                        [
                            sys.executable,
                            str(verus_script),
                            str(plat.home_dir / "verus"),
                        ]
                    )

    @patch("z.cmd_setup_linux", return_value=0)
    def test_setup_dispatches_to_linux(self, mock_linux: MagicMock) -> None:
        """On Linux, cmd_setup delegates to cmd_setup_linux."""
        plat = _linux_plat()
        cfg = zmod.BuildConfig()
        rc = zmod.cmd_setup(plat, cfg)
        self.assertEqual(rc, 0)
        mock_linux.assert_called_once_with(plat, cfg)

    @patch("z.cmd_setup_windows", return_value=0)
    def test_setup_dispatches_to_windows(self, mock_win: MagicMock) -> None:
        """On Windows, cmd_setup delegates to cmd_setup_windows."""
        plat = _windows_plat()
        cfg = zmod.BuildConfig()
        rc = zmod.cmd_setup(plat, cfg)
        self.assertEqual(rc, 0)
        mock_win.assert_called_once_with(plat, cfg)


# ===========================================================================
# Subcommand: cmd_help
# ===========================================================================


class TestCmdHelp(unittest.TestCase):
    """Tests for cmd_help."""

    @patch("z.invoke_make", return_value=0)
    @patch("z.find_make", return_value="make")
    @patch("builtins.print")
    def test_help_prints_help_text(
        self, mock_print: MagicMock, _find: MagicMock, _make: MagicMock
    ) -> None:
        """cmd_help prints HELP_TEXT and returns 0."""
        plat = _linux_plat()
        cfg = zmod.BuildConfig()
        rc = zmod.cmd_help(plat, cfg)
        self.assertEqual(rc, 0)
        # The first print call should contain the help text.
        printed = mock_print.call_args_list[0][0][0]
        self.assertIn("Usage:", printed)


# ===========================================================================
# _parse_make_var edge cases
# ===========================================================================


class TestParseMakeVar(unittest.TestCase):
    """Tests for _parse_make_var."""

    def test_valid_log_levels(self) -> None:
        """All values in VALID_LOG_LEVELS are accepted by _parse_make_var."""
        for level in zmod.VALID_LOG_LEVELS:
            cfg = zmod.BuildConfig()
            zmod._parse_make_var(cfg, "LOG_LEVEL", level)
            self.assertEqual(cfg.log_level, level)

    def test_valid_message_formats(self) -> None:
        """All values in VALID_MESSAGE_FORMATS are accepted by _parse_make_var."""
        for fmt in zmod.VALID_MESSAGE_FORMATS:
            cfg = zmod.BuildConfig()
            zmod._parse_make_var(cfg, "MESSAGE_FORMAT", fmt)
            self.assertEqual(cfg.message_format, fmt)

    def test_release_case_insensitive(self) -> None:
        """RELEASE accepts case-insensitive 'YES'/'No' values."""
        cfg = zmod.BuildConfig()
        zmod._parse_make_var(cfg, "RELEASE", "YES")
        self.assertTrue(cfg.release)
        zmod._parse_make_var(cfg, "RELEASE", "No")
        self.assertFalse(cfg.release)

    def test_profiler_yes_no(self) -> None:
        """PROFILER toggles cfg.profiler between True and False."""
        cfg = zmod.BuildConfig()
        zmod._parse_make_var(cfg, "PROFILER", "yes")
        self.assertTrue(cfg.profiler)
        zmod._parse_make_var(cfg, "PROFILER", "no")
        self.assertFalse(cfg.profiler)

    def test_whp_yes_no(self) -> None:
        """WHP toggles cfg.whp between True and False."""
        cfg = zmod.BuildConfig()
        zmod._parse_make_var(cfg, "WHP", "yes")
        self.assertTrue(cfg.whp)
        zmod._parse_make_var(cfg, "WHP", "no")
        self.assertFalse(cfg.whp)

    def test_verbose_yes_no(self) -> None:
        """VERBOSE toggles cfg.verbose between True and False."""
        cfg = zmod.BuildConfig()
        zmod._parse_make_var(cfg, "VERBOSE", "yes")
        self.assertTrue(cfg.verbose)
        zmod._parse_make_var(cfg, "VERBOSE", "no")
        self.assertFalse(cfg.verbose)

    def test_timeout_valid(self) -> None:
        """A numeric TIMEOUT string is parsed into an integer."""
        cfg = zmod.BuildConfig()
        zmod._parse_make_var(cfg, "TIMEOUT", "42")
        self.assertEqual(cfg.timeout, 42)

    def test_sysroot_dir(self) -> None:
        """SYSROOT_DIR sets cfg.sysroot_dir to the given path."""
        cfg = zmod.BuildConfig()
        zmod._parse_make_var(cfg, "SYSROOT_DIR", "/opt/sysroot")
        self.assertEqual(cfg.sysroot_dir, "/opt/sysroot")


# ===========================================================================
# PlatformInfo
# ===========================================================================


class TestPlatformInfo(unittest.TestCase):
    """Tests for PlatformInfo.detect."""

    @patch("z.sys")
    def test_detect_linux(self, mock_sys: MagicMock) -> None:
        """PlatformInfo.detect identifies a Linux platform correctly."""
        mock_sys.platform = "linux"
        plat = zmod.PlatformInfo.detect(Path("/repo"))
        self.assertTrue(plat.is_linux)
        self.assertFalse(plat.is_windows)

    @patch("z.sys")
    def test_detect_windows(self, mock_sys: MagicMock) -> None:
        """PlatformInfo.detect identifies a Windows platform correctly."""
        mock_sys.platform = "win32"
        with patch.dict(os.environ, {"USERPROFILE": "C:\\Users\\user"}):
            plat = zmod.PlatformInfo.detect(Path("C:/repo"))
        self.assertTrue(plat.is_windows)
        self.assertFalse(plat.is_linux)


# ===========================================================================
# COMMANDS dictionary
# ===========================================================================


class TestCommands(unittest.TestCase):
    """Tests for the COMMANDS dispatch table."""

    def test_all_expected_commands_present(self) -> None:
        """The COMMANDS dict contains exactly the expected set of subcommands."""
        expected = {
            "build",
            "test",
            "verify",
            "clean",
            "distclean",
            "setup",
            "run",
            "bench",
            "help",
        }
        self.assertEqual(set(zmod.COMMANDS.keys()), expected)

    def test_command_values_are_callable(self) -> None:
        """Every entry in COMMANDS maps to a callable handler function."""
        for name, func in zmod.COMMANDS.items():
            self.assertTrue(callable(func), f"COMMANDS['{name}'] is not callable")


if __name__ == "__main__":
    unittest.main()
