# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Unit tests for the inline test migration tool (scripts/migrate-inline-tests.py)."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parent.parent
_SCRIPTS = _REPO_ROOT / "scripts"
sys.path.insert(0, str(_SCRIPTS))

from testlayout import baseline as bl  # noqa: E402
from testlayout import migrate, policy  # noqa: E402
from testlayout.paths import BASELINE_FILE, crate_roots  # noqa: E402
from testlayout.rustlex import tokenize  # noqa: E402


def _load_cli(name: str):
    spec = importlib.util.spec_from_file_location(
        name.replace("-", "_"), _SCRIPTS / f"{name}.py"
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


INLINE = """\
use core::fmt;

pub fn add(a: u32, b: u32) -> u32 {
    a + b
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn adds() {
        assert_eq!(add(1, 2), 3);
    }
}
"""


class RepoCase(unittest.TestCase):
    """Base class that builds a throw-away crate on disk."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = Path(self._tmp.name)
        self.write("crates/a/Cargo.toml", '[package]\nname = "a"\n')

    def write(self, rel: str, text: str) -> None:
        """Writes a file below the repository root."""
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "w", encoding="utf-8", newline="\n") as handle:
            handle.write(text)

    def read(self, rel: str) -> str:
        """Reads a file below the repository root."""
        return (self.root / rel).read_text(encoding="utf-8")

    def plan(self, text: str, path: str = "crates/a/src/lib.rs") -> migrate.FilePlan:
        """Plans the migration of one file of the crate."""
        files = ["crates/a/Cargo.toml", path]
        return migrate.plan_file(
            self.root, path, text, set(files), crate_roots(self.root, files)
        )


class TestPlan(RepoCase):
    """Tests for the migration of a single file."""

    def test_moves_module_to_sibling_file(self) -> None:
        """The module body moves, dedented, below a license header."""
        plan = self.plan(INLINE)
        self.assertEqual(plan.skips, [])
        self.assertEqual(
            [(m.name, m.target, m.path_attr) for m in plan.moves],
            [("tests", "crates/a/src/tests.rs", None)],
        )
        self.assertEqual(
            plan.moves[0].source,
            migrate.LICENSE_HEADER
            + "\n"
            + "use super::*;\n"
            + "\n"
            + "#[test]\n"
            + "fn adds() {\n"
            + "    assert_eq!(add(1, 2), 3);\n"
            + "}\n",
        )
        self.assertEqual(
            plan.text,
            INLINE[: INLINE.index("#[cfg(all")]
            + '#[cfg(all(test, feature = "std"))]\nmod tests;\n',
        )

    def test_files_without_inline_tests_are_untouched(self) -> None:
        """Out-of-line modules and mainline code stay as they are."""
        text = "pub fn f() {}\n#[cfg(test)]\nmod tests;\n"
        plan = self.plan(text)
        self.assertEqual((plan.moves, plan.skips, plan.text), ([], [], text))

    def test_visibility_and_extra_attributes_are_kept(self) -> None:
        """Everything before the module keyword stays on the declaration."""
        text = "#[cfg(test)]\n#[allow(dead_code)]\npub(crate) mod tests {\n    fn f() {}\n}\n"
        plan = self.plan(text)
        self.assertEqual(
            plan.text, "#[cfg(test)]\n#[allow(dead_code)]\npub(crate) mod tests;\n"
        )

    def test_comments_around_the_module_stay(self) -> None:
        """Banners and doc comments above the attribute remain in the mainline file."""
        text = (
            "// ----- tests -----\n\n/// Docs.\n#[cfg(test)]\n"
            "mod tests {\n    fn f() {}\n}\n// tail\n"
        )
        plan = self.plan(text)
        self.assertEqual(
            plan.text,
            "// ----- tests -----\n\n/// Docs.\n#[cfg(test)]\nmod tests;\n// tail\n",
        )

    def test_multi_line_strings_are_not_dedented(self) -> None:
        """Dedenting must not change string literal contents."""
        text = (
            "#[cfg(test)]\nmod tests {\n"
            '    const A: &str = "one\n'
            '    two";\n'
            '    const B: &str = r#"x\n'
            '        y"#;\n'
            "    fn f() {}\n"
            "}\n"
        )
        source = self.plan(text).moves[0].source
        self.assertIn('const A: &str = "one\n    two";', source)
        self.assertIn('const B: &str = r#"x\n        y"#;', source)
        self.assertIn("\nfn f() {}\n", source)

    def test_inner_attributes_move_with_the_body(self) -> None:
        """Inner attributes of the module belong to the new file."""
        text = "#[cfg(test)]\nmod tests {\n    #![allow(dead_code)]\n    fn f() {}\n}\n"
        self.assertIn("#![allow(dead_code)]\n", self.plan(text).moves[0].source)

    def test_two_modules_in_one_file(self) -> None:
        """Every inline test module of a file is moved, last to first."""
        text = (
            "#[cfg(test)]\nmod tests {\n    fn a() {}\n}\n"
            "pub fn mid() {}\n"
            "#[cfg(test)]\nmod more_tests {\n    fn b() {}\n}\n"
        )
        plan = self.plan(text)
        self.assertEqual([m.name for m in plan.moves], ["tests", "more_tests"])
        self.assertEqual(
            plan.text,
            "#[cfg(test)]\nmod tests;\npub fn mid() {}\n#[cfg(test)]\nmod more_tests;\n",
        )

    def test_odd_name_uses_a_path_attribute(self) -> None:
        """A module whose name does not mark a test file keeps its name via #[path]."""
        text = "#[cfg(test)]\nmod errno_shim {\n    pub fn f() {}\n}\n"
        plan = self.plan(text)
        move = plan.moves[0]
        self.assertEqual(move.target, "crates/a/src/tests/errno_shim.rs")
        self.assertEqual(move.path_attr, "tests/errno_shim.rs")
        self.assertEqual(
            plan.text,
            '#[cfg(test)]\n#[path = "tests/errno_shim.rs"]\nmod errno_shim;\n',
        )

    def test_path_is_relative_to_the_declaring_file(self) -> None:
        """For a non-root file the #[path] goes through the file's module directory."""
        text = "#[cfg(test)]\nmod benchmarks {\n    pub fn f() {}\n}\n"
        plan = self.plan(text, "crates/a/src/vec_deque.rs")
        self.assertEqual(
            plan.moves[0].target, "crates/a/src/vec_deque/tests/benchmarks.rs"
        )
        self.assertEqual(plan.moves[0].path_attr, "vec_deque/tests/benchmarks.rs")

    def test_modules_that_cannot_move_are_skipped(self) -> None:
        """Bodies that depend on their location are reported, not moved."""
        cases = {
            "include_str": (
                '#[cfg(test)]\nmod tests {\n    const D: &str = include_str!("x.txt");\n}\n'
            ),
            "line!": "#[cfg(test)]\nmod tests {\n    fn f() -> u32 { line!() }\n}\n",
            "#[path]": '#[cfg(test)]\nmod tests {\n    #[path = "x.rs"]\n    mod x;\n}\n',
            "nested": (
                "mod outer {\n    #[cfg(test)]\n    mod tests {\n        fn f() {}\n    }\n}\n"
            ),
        }
        for needle, text in cases.items():
            with self.subTest(case=needle):
                plan = self.plan(text)
                self.assertEqual(plan.moves, [])
                self.assertEqual(len(plan.skips), 1)
                self.assertIn(needle, plan.skips[0].reason)
                self.assertEqual(plan.text, text)

    def test_children_of_path_modules_are_skipped(self) -> None:
        """Child modules would resolve differently from a #[path] file."""
        text = "#[cfg(test)]\nmod errno_shim {\n    mod helper;\n}\n"
        plan = self.plan(text)
        self.assertEqual(plan.moves, [])
        self.assertIn("child modules", plan.skips[0].reason)

    def test_children_of_regular_modules_are_fine(self) -> None:
        """A `tests.rs` file resolves children the way an inline `tests` module did."""
        text = "#[cfg(test)]\nmod tests {\n    mod helper;\n}\n"
        self.assertEqual(len(self.plan(text).moves), 1)

    def test_existing_target_is_a_conflict(self) -> None:
        """An existing file is never overwritten."""
        self.write("crates/a/src/tests.rs", "// existing\n")
        files = {"crates/a/Cargo.toml", "crates/a/src/lib.rs", "crates/a/src/tests.rs"}
        plan = migrate.plan_file(
            self.root,
            "crates/a/src/lib.rs",
            INLINE,
            files,
            crate_roots(self.root, sorted(files)),
        )
        self.assertEqual(plan.moves, [])
        self.assertIn("already exists", plan.skips[0].reason)

    def test_dedent_helper(self) -> None:
        """Only lines that start in code are dedented; blank lines are emptied."""
        body = '    a\n      \n    b("x\n    y")\n'
        self.assertEqual(
            migrate.dedent_body(body, tokenize(body)), 'a\n\nb("x\n    y")\n'
        )


class TestMigrate(RepoCase):
    """Tests for migrating a tree and for the command-line tool."""

    def setUp(self) -> None:
        super().setUp()
        self.write("crates/a/src/lib.rs", INLINE)
        self.cli = _load_cli("migrate-inline-tests")

    def run_cli(self, *args: str) -> tuple[int, str]:
        """Runs the migration CLI against the throw-away repository."""
        out = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
            code = self.cli.main(["--root", str(self.root), "--no-format", *args])
        return code, out.getvalue()

    def test_dry_run_writes_nothing(self) -> None:
        """--dry-run only reports."""
        code, out = self.run_cli("--dry-run", "crates")
        self.assertEqual(code, 0)
        self.assertIn(
            "would move: crates/a/src/lib.rs: mod tests -> crates/a/src/tests.rs", out
        )
        self.assertEqual(self.read("crates/a/src/lib.rs"), INLINE)
        self.assertFalse((self.root / "crates/a/src/tests.rs").exists())

    def test_migration_satisfies_the_policy_and_shrinks_the_baseline(self) -> None:
        """After migrating, the module's baseline entry is gone and the check passes."""
        self.write(BASELINE_FILE, "R2 crates/a/src/lib.rs mod tests tests=1\n")
        code, out = self.run_cli("crates/a")
        self.assertEqual(code, 0, out)
        self.assertEqual(bl.load(self.root / BASELINE_FILE), {})
        self.assertTrue(
            policy.check(
                self.root,
                ["crates/a/Cargo.toml", "crates/a/src/lib.rs", "crates/a/src/tests.rs"],
            ).ok
        )
        self.assertIn("mod tests;", self.read("crates/a/src/lib.rs"))
        self.assertIn("fn adds()", self.read("crates/a/src/tests.rs"))

    def test_second_run_is_a_no_op(self) -> None:
        """Migrating twice does nothing the second time."""
        self.run_cli("crates")
        before = self.read("crates/a/src/lib.rs")
        code, out = self.run_cli("crates")
        self.assertEqual(code, 0)
        self.assertIn("0 module(s) moved", out)
        self.assertEqual(self.read("crates/a/src/lib.rs"), before)

    def test_skips_make_the_exit_code_fail(self) -> None:
        """A module left behind is reported with a non-zero exit code."""
        self.write(
            "crates/a/src/lib.rs",
            "#[cfg(test)]\nmod tests {\n    const F: &str = file!();\n}\n",
        )
        code, out = self.run_cli("crates")
        self.assertEqual(code, 1)
        self.assertIn("skipped: crates/a/src/lib.rs:1: mod tests: it uses `file!`", out)

    def test_path_is_required(self) -> None:
        """Without a path the tool refuses to guess."""
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            self.cli.main(["--root", str(self.root)])

    def test_only_selected_paths_are_migrated(self) -> None:
        """Paths limit the migration."""
        self.write("crates/b/Cargo.toml", '[package]\nname = "b"\n')
        self.write("crates/b/src/lib.rs", INLINE)
        self.run_cli("crates/a")
        self.assertIn("mod tests {", self.read("crates/b/src/lib.rs"))

    def test_editions_follow_the_owning_crate(self) -> None:
        """rustfmt must use the edition of the crate that owns the file."""
        self.write("Cargo.toml", '[workspace.package]\nedition = "2021"\n')
        self.write(
            "crates/a/Cargo.toml", '[package]\nname = "a"\nedition.workspace = true\n'
        )
        self.write("crates/b/Cargo.toml", '[package]\nname = "b"\nedition = "2024"\n')
        self.assertEqual(migrate.rust_edition(self.root, "crates/a/src/lib.rs"), "2021")
        self.assertEqual(
            migrate.rust_edition(self.root, "crates/b/src/deep/x.rs"), "2024"
        )
        self.assertEqual(migrate.rust_edition(self.root, "loose.rs"), "2021")


class TestNormalizedTokens(unittest.TestCase):
    """Tests for the layout-insensitive token comparison."""

    def test_layout_and_trailing_commas_are_ignored(self) -> None:
        """Formatting may join lines and drop trailing commas."""
        wrapped = "f(\n    a,\n    b,\n);"
        joined = "f(a, b);"
        self.assertEqual(
            migrate.normalized_tokens(wrapped), migrate.normalized_tokens(joined)
        )

    def test_string_continuations_are_ignored(self) -> None:
        """Re-wrapping a string at a line continuation keeps its value."""
        wrapped = 's("abc \\\n        def");'
        joined = 's("abc def");'
        self.assertEqual(
            migrate.normalized_tokens(wrapped), migrate.normalized_tokens(joined)
        )

    def test_real_differences_are_kept(self) -> None:
        """Different code and different raw strings stay different."""
        self.assertNotEqual(
            migrate.normalized_tokens("f(a);"), migrate.normalized_tokens("f(b);")
        )
        raw_a = 'r"abc \\\n def"'
        raw_b = 'r"abc def"'
        self.assertNotEqual(
            migrate.normalized_tokens(raw_a), migrate.normalized_tokens(raw_b)
        )


@unittest.skipIf(shutil.which("git") is None, "git is not available")
class TestVerify(RepoCase):
    """Tests for proving that a tree is a pure migration of a revision."""

    def git(self, *args: str) -> None:
        """Runs git in the throw-away repository."""
        env = dict(
            os.environ, GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_SYSTEM=os.devnull
        )
        subprocess.run(
            [
                "git",
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
                *args,
            ],
            cwd=self.root,
            check=True,
            capture_output=True,
            env=env,
        )

    def setUp(self) -> None:
        super().setUp()
        self.write("crates/a/src/lib.rs", INLINE)
        self.git("init", "-q")
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "base")
        migrate.migrate(self.root, ["crates"], apply=True, format_output=False)

    def test_pure_migration_verifies(self) -> None:
        """A migration made by the tool verifies against its base."""
        self.assertEqual(migrate.verify(self.root, "HEAD"), [])

    def test_edited_test_body_is_detected(self) -> None:
        """A changed assertion is not a mechanical move."""
        text = self.read("crates/a/src/tests.rs").replace("(1, 2), 3", "(1, 2), 4")
        self.write("crates/a/src/tests.rs", text)
        problems = migrate.verify(self.root, "HEAD")
        self.assertEqual(len(problems), 1)
        self.assertIn("crates/a/src/tests.rs", problems[0])

    def test_edited_mainline_code_is_detected(self) -> None:
        """Changes to mainline code do not belong in a migration."""
        text = self.read("crates/a/src/lib.rs").replace("a + b", "a - b")
        self.write("crates/a/src/lib.rs", text)
        problems = migrate.verify(self.root, "HEAD")
        self.assertEqual(len(problems), 1)
        self.assertIn("crates/a/src/lib.rs", problems[0])

    def test_missing_target_is_detected(self) -> None:
        """Deleting the moved file loses the tests."""
        (self.root / "crates/a/src/tests.rs").unlink()
        problems = migrate.verify(self.root, "HEAD")
        self.assertTrue(any("missing" in p for p in problems), problems)


if __name__ == "__main__":
    unittest.main()
