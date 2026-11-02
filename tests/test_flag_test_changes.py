# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Unit tests for the PR flagger (scripts/flag-test-changes.py)."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

_REPO_ROOT = Path(__file__).resolve().parent.parent
_SCRIPTS = _REPO_ROOT / "scripts"
sys.path.insert(0, str(_SCRIPTS))

from testlayout import flagging, gitdiff, testitems  # noqa: E402
from testlayout.clex import DIRECTIVE, tokenize_c  # noqa: E402
from testlayout.paths import ALLOWLIST_FILE  # noqa: E402
from testlayout.rustlex import LexError, normalized_tokens  # noqa: E402


def _load_cli():
    spec = importlib.util.spec_from_file_location(
        "flag_test_changes", _SCRIPTS / "flag-test-changes.py"
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _items(path: str, text: str) -> list[testitems.Item]:
    return testitems.extract(path, text)


def _fingerprints(path: str, text: str) -> list[str]:
    return sorted(item.fingerprint for item in _items(path, text))


def _names(path: str, text: str) -> list[str]:
    return [item.name for item in _items(path, text)]


RUST_TEST = "crates/a/tests/it.rs"
C_TEST = "src/tests/integration/test-c-x/main.c"
PY_TEST = "tests/test_x.py"
MANIFEST = "test/test-standalone.toml"

ONE = """\
#[test]
fn one() {
    assert_eq!(1, 1);
}
"""

TWO = """\
#[test]
fn two() {
    assert_eq!(2, 2);
}
"""


class TestNormalizedTokens(unittest.TestCase):
    """The layout-insensitive token view of test code."""

    def test_trailing_commas_are_ignored(self) -> None:
        self.assertEqual(normalized_tokens("f(a, b,);"), normalized_tokens("f(a, b);"))

    def test_comma_before_a_non_closer_is_kept(self) -> None:
        self.assertNotEqual(normalized_tokens("f(a, b);"), normalized_tokens("f(a b);"))

    def test_string_continuations_are_joined(self) -> None:
        self.assertEqual(
            normalized_tokens('let s = "ab\\\n    cd";'),
            normalized_tokens('let s = "abcd";'),
        )

    def test_raw_strings_are_verbatim(self) -> None:
        self.assertNotEqual(
            normalized_tokens('let s = r"ab\\\n cd";'),
            normalized_tokens('let s = r"abcd";'),
        )


class TestCTokenizer(unittest.TestCase):
    """The C tokenizer."""

    def test_directive_is_one_token_with_continuations(self) -> None:
        toks = tokenize_c("#define X(a) \\\n    ((a) + 1)\nint y;\n")
        self.assertEqual(toks.kinds[0], DIRECTIVE)
        self.assertEqual(toks.src(1), "int")

    def test_hash_inside_a_macro_is_not_a_directive(self) -> None:
        toks = tokenize_c("#define STR(x) #x\n")
        self.assertEqual(len(toks), 1)

    def test_braces_in_strings_chars_and_comments_are_ignored(self) -> None:
        toks = tokenize_c("int f(void) { char c = '{'; /* } */ return \"}\"[0]; }\n")
        self.assertEqual(toks.src(len(toks) - 1), "}")

    def test_block_comments_do_not_nest(self) -> None:
        toks = tokenize_c("/* a /* b */ int x;\n")
        self.assertEqual(toks.src(0), "int")

    def test_unbalanced_brace_is_an_error(self) -> None:
        with self.assertRaises(LexError):
            tokenize_c("void f(void) {\n")


class TestRustItems(unittest.TestCase):
    """Rust test items."""

    def test_every_unit_of_a_test_file_is_an_item(self) -> None:
        text = ONE + "const CASES: &[u32] = &[1, 2];\nfn helper() {}\nuse std::fmt;\n"
        self.assertEqual(_names(RUST_TEST, text), ["one", "CASES", "helper"])

    def test_mainline_code_is_not_an_item(self) -> None:
        text = "pub fn add(a: u32) -> u32 { a }\n// test\n"
        self.assertEqual(_items("crates/a/src/lib.rs", text), [])

    def test_inline_test_module_items_are_tracked_one_by_one(self) -> None:
        text = (
            "pub fn add() {}\n#[cfg(test)]\nmod tests {\n    use super::*;\n"
            "    fn helper() {}\n    #[test]\n    fn adds() {}\n}\n"
        )
        self.assertEqual(
            _names("crates/a/src/lib.rs", text), ["tests::helper", "tests::adds"]
        )

    def test_item_is_the_same_inline_and_in_a_test_file(self) -> None:
        inline = (
            "#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n"
            "    fn adds() {\n        assert_eq!(add(1, 2), 3);\n    }\n}\n"
        )
        moved = (
            "use super::*;\n\n#[test]\nfn adds() {\n    assert_eq!(add(1, 2), 3);\n}\n"
        )
        self.assertEqual(
            _fingerprints("crates/a/src/lib.rs", inline),
            _fingerprints("crates/a/src/tests.rs", moved),
        )

    def test_out_of_line_test_module_declaration_is_an_item(self) -> None:
        text = "#[cfg(test)]\nmod tests;\n"
        self.assertEqual(_names("crates/a/src/lib.rs", text), ["tests"])

    def test_gated_seams_are_items(self) -> None:
        text = (
            "struct S {\n    a: u8,\n    #[cfg(test)]\n    hook: u8,\n}\n"
            "impl S {\n    #[cfg(test)]\n    fn peek(&self) -> u8 { self.a }\n}\n"
            "fn run() {\n    #[cfg(test)]\n    { trace(); }\n}\n"
        )
        names = _names("crates/a/src/lib.rs", text)
        self.assertEqual(len(names), 3)
        self.assertIn("S::hook: u8,", names)
        self.assertIn("<S>::peek", names)

    def test_test_marked_function_in_mainline_is_an_item(self) -> None:
        text = "#[test]\nfn stray() {}\n#[tokio::test]\nasync fn other() {}\n"
        self.assertEqual(_names("crates/a/src/lib.rs", text), ["stray", "other"])

    def test_gated_use_is_wiring(self) -> None:
        text = "#[cfg(test)]\nuse std::fmt;\n"
        self.assertEqual(_items("crates/a/src/lib.rs", text), [])

    def test_whole_file_gate_makes_the_file_test_code(self) -> None:
        text = "#![cfg(test)]\nfn helper() {}\n"
        self.assertEqual(_names("crates/a/src/support.rs", text), ["#![..]", "helper"])

    def test_impl_context_is_part_of_the_fingerprint(self) -> None:
        a = "impl A {\n    fn f() {}\n}\n"
        b = "impl B {\n    fn f() {}\n}\n"
        self.assertNotEqual(_fingerprints(RUST_TEST, a), _fingerprints(RUST_TEST, b))

    def test_comments_and_layout_do_not_change_the_fingerprint(self) -> None:
        a = "#[test]\nfn one() {\n    assert_eq!(1, 1);\n}\n"
        b = "#[test]\n/// Docs.\nfn one() { // c\n    assert_eq!(1,\n        1,);\n}\n"
        self.assertEqual(_fingerprints(RUST_TEST, a), _fingerprints(RUST_TEST, b))

    def test_attributes_are_part_of_the_fingerprint(self) -> None:
        a = "#[test]\nfn one() {}\n"
        b = "#[test]\n#[ignore]\nfn one() {}\n"
        self.assertNotEqual(_fingerprints(RUST_TEST, a), _fingerprints(RUST_TEST, b))

    def test_unbalanced_source_is_an_error(self) -> None:
        with self.assertRaises(testitems.ExtractError):
            _items(RUST_TEST, "fn broken() {\n")


class TestPythonItems(unittest.TestCase):
    """Python test items."""

    TEXT = '''\
"""Module docs."""

import unittest

LIMIT = 3


class TestThing(unittest.TestCase):
    """Class docs."""

    def test_a(self) -> None:
        """Method docs."""
        self.assertEqual(1, 1)


if __name__ == "__main__":
    unittest.main()
'''

    def test_items(self) -> None:
        self.assertEqual(
            _names(PY_TEST, self.TEXT),
            ["LIMIT", "TestThing", "TestThing::test_a", "if"],
        )

    def test_docstrings_and_comments_are_ignored(self) -> None:
        other = self.TEXT.replace("Method docs.", "Other docs.").replace(
            "LIMIT = 3", "LIMIT = 3  # A comment."
        )
        self.assertEqual(
            _fingerprints(PY_TEST, self.TEXT), _fingerprints(PY_TEST, other)
        )

    def test_assertions_are_not_ignored(self) -> None:
        other = self.TEXT.replace("assertEqual(1, 1)", "assertEqual(1, 2)")
        self.assertNotEqual(
            _fingerprints(PY_TEST, self.TEXT), _fingerprints(PY_TEST, other)
        )

    def test_non_test_python_is_not_tracked(self) -> None:
        self.assertEqual(_items("scripts/tool.py", "x = 1\n"), [])

    def test_syntax_error_is_an_error(self) -> None:
        with self.assertRaises(testitems.ExtractError):
            _items(PY_TEST, "def broken(:\n")


class TestCItems(unittest.TestCase):
    """C test items."""

    TEXT = """\
#include <assert.h>
#define LIMIT 4

typedef struct {
    int a;
} pair_t;

static int helper(int x)
{
    return x + 1;
}

int table[] = { 1, 2 };

int main(void)
{
    assert(helper(1) == 2);
    return 0;
}
"""

    def test_items(self) -> None:
        self.assertEqual(
            _names(C_TEST, self.TEXT),
            ["LIMIT", "pair_t", "helper", "table", "main"],
        )

    def test_comments_and_layout_do_not_change_the_fingerprint(self) -> None:
        other = self.TEXT.replace(
            "    return x + 1;", "    /* one more */\n    return x +\n        1;"
        ).replace("#define LIMIT 4", "#define LIMIT   4 // four")
        self.assertEqual(_fingerprints(C_TEST, self.TEXT), _fingerprints(C_TEST, other))

    def test_macro_value_is_not_ignored(self) -> None:
        other = self.TEXT.replace("#define LIMIT 4", "#define LIMIT 5")
        self.assertNotEqual(
            _fingerprints(C_TEST, self.TEXT), _fingerprints(C_TEST, other)
        )

    def test_macro_invocations_are_named_after_their_arguments(self) -> None:
        text = "TEST(suite, first) { assert(1); }\nTEST(suite, second) { assert(2); }\n"
        self.assertEqual(
            _names(C_TEST, text),
            ["TEST(suite, first) { assert(1); }", "TEST(suite, second) { assert(2); }"],
        )

    def test_extern_c_block_is_recursed(self) -> None:
        text = 'extern "C" {\nint a(void);\nint b(void);\n}\n'
        self.assertEqual(_names(C_TEST, text), ["extern::a", "extern::b"])

    def test_non_test_c_is_not_tracked(self) -> None:
        self.assertEqual(_items("src/libs/x/src/x.c", "int x;\n"), [])


class TestTomlItems(unittest.TestCase):
    """End-to-end manifest items."""

    TEXT = """\
# Header comment.
[runner]
port_num = 9999

[[tests]]
name = "empty/1"
build_modes = ["debug",
    "release"]

[[tests]]
name = "empty/2"
note = '''
[[not-a-table]]
'''
"""

    def test_items(self) -> None:
        self.assertEqual(
            _names(MANIFEST, self.TEXT),
            ["[runner]", '[[tests]] name="empty/1"', '[[tests]] name="empty/2"'],
        )

    def test_root_keys_form_an_item(self) -> None:
        self.assertEqual(
            _names(MANIFEST, 'a = 1\n[[tests]]\nname = "x"\n'),
            ["(root)", '[[tests]] name="x"'],
        )

    def test_comments_blank_lines_layout_and_table_order_are_ignored(self) -> None:
        other = (
            "[[tests]]\n  name = \"empty/2\"  # Second.\nnote = '''\n[[not-a-table]]\n'''\n"
            "\n[runner]\nport_num = 9999\n\n"
            '[[tests]]\nname = "empty/1"\nbuild_modes = ["debug",\n  "release"]\n'
        )
        self.assertEqual(
            _fingerprints(MANIFEST, self.TEXT), _fingerprints(MANIFEST, other)
        )

    def test_reflowing_a_multi_line_value_is_conservatively_flagged(self) -> None:
        other = self.TEXT.replace('["debug",\n    "release"]', '["debug", "release"]')
        self.assertNotEqual(
            _fingerprints(MANIFEST, self.TEXT), _fingerprints(MANIFEST, other)
        )

    def test_values_are_not_ignored(self) -> None:
        other = self.TEXT.replace("9999", "9998")
        self.assertNotEqual(
            _fingerprints(MANIFEST, self.TEXT), _fingerprints(MANIFEST, other)
        )

    def test_other_toml_is_not_tracked(self) -> None:
        self.assertEqual(_items("crates/a/Cargo.toml", "[package]\n"), [])


class GitCase(unittest.TestCase):
    """Base class that builds a throw-away git repository."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = Path(self._tmp.name)
        self.git("init", "-q")

    def git(self, *args: str) -> str:
        """Runs git in the repository."""
        proc = subprocess.run(
            [
                "git",
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.autocrlf=false",
                *args,
            ],
            cwd=self.root,
            check=True,
            capture_output=True,
            text=True,
        )
        return proc.stdout.strip()

    def commit(self, files: dict[str, str | None]) -> str:
        """Writes (or deletes, for `None`) files and commits them; returns the commit."""
        for rel, text in files.items():
            path = self.root / rel
            if text is None:
                path.unlink()
                continue
            path.parent.mkdir(parents=True, exist_ok=True)
            with open(path, "w", encoding="utf-8", newline="\n") as handle:
                handle.write(text)
        self.git("add", "-A")
        self.git("commit", "-q", "-m", "commit")
        return self.git("rev-parse", "HEAD")

    def flag(self, base: str) -> flagging.Report:
        """Analyzes the changes between `base` and the current commit."""
        return flagging.analyze(self.root, base, "HEAD")

    def summary(self, report: flagging.Report) -> list[tuple[str, str]]:
        """Returns the kind and name of every finding, sorted."""
        return sorted((f.kind, f.name) for f in report.findings)


class TestFlagger(GitCase):
    """The comparison of two revisions."""

    def test_adding_a_test_to_an_existing_file_is_not_flagged(self) -> None:
        base = self.commit({RUST_TEST: ONE})
        self.commit({RUST_TEST: ONE + "\n" + TWO})
        self.assertFalse(self.flag(base).flagged)

    def test_adding_a_test_file_is_not_flagged(self) -> None:
        base = self.commit({RUST_TEST: ONE})
        self.commit({"crates/a/tests/other.rs": TWO})
        self.assertFalse(self.flag(base).flagged)

    def test_modifying_a_test_is_flagged(self) -> None:
        base = self.commit({RUST_TEST: ONE})
        self.commit({RUST_TEST: ONE.replace("assert_eq!(1, 1)", "assert_eq!(1, 2)")})
        report = self.flag(base)
        self.assertEqual(self.summary(report), [("modified", "one")])
        finding = report.findings[0]
        self.assertEqual(
            (finding.path, finding.base_line, finding.head_line), (RUST_TEST, 1, 1)
        )

    def test_inserting_a_statement_into_a_test_is_flagged(self) -> None:
        base = self.commit({RUST_TEST: ONE})
        self.commit({RUST_TEST: ONE.replace("}\n", "    assert!(true);\n}\n")})
        self.assertEqual(self.summary(self.flag(base)), [("modified", "one")])

    def test_removing_a_test_is_flagged(self) -> None:
        base = self.commit({RUST_TEST: ONE + "\n" + TWO})
        self.commit({RUST_TEST: TWO})
        self.assertEqual(self.summary(self.flag(base)), [("removed", "one")])

    def test_renaming_a_test_function_is_flagged(self) -> None:
        base = self.commit({RUST_TEST: ONE})
        self.commit({RUST_TEST: ONE.replace("fn one", "fn uno")})
        self.assertEqual(self.summary(self.flag(base)), [("removed", "one")])

    def test_ignoring_a_test_is_flagged(self) -> None:
        base = self.commit({RUST_TEST: ONE})
        self.commit({RUST_TEST: "#[ignore]\n" + ONE})
        self.assertEqual(self.summary(self.flag(base)), [("modified", "one")])

    def test_editing_a_test_table_is_flagged(self) -> None:
        table = "const CASES: &[(u32, u32)] = &[(1, 2), (3, 4)];\n"
        base = self.commit({RUST_TEST: table})
        self.commit({RUST_TEST: table.replace("(3, 4)", "(3, 5)")})
        self.assertEqual(self.summary(self.flag(base)), [("modified", "CASES")])

    def test_formatting_and_comment_edits_are_not_flagged(self) -> None:
        base = self.commit({RUST_TEST: ONE})
        self.commit(
            {
                RUST_TEST: "#[test]\n// Now documented.\nfn one() { assert_eq!(1,\n 1,); }\n"
            }
        )
        self.assertFalse(self.flag(base).flagged)

    def test_reordering_tests_is_not_flagged(self) -> None:
        base = self.commit({RUST_TEST: ONE + "\n" + TWO})
        self.commit({RUST_TEST: TWO + "\n" + ONE})
        self.assertFalse(self.flag(base).flagged)

    def test_moving_a_test_to_another_file_is_not_flagged(self) -> None:
        base = self.commit({RUST_TEST: ONE + "\n" + TWO})
        self.commit({RUST_TEST: TWO, "crates/a/tests/moved.rs": ONE})
        self.assertFalse(self.flag(base).flagged)

    def test_a_copy_does_not_hide_a_modification(self) -> None:
        base = self.commit({RUST_TEST: ONE})
        self.commit(
            {
                RUST_TEST: ONE.replace("assert_eq!(1, 1)", "assert!(true)"),
                "crates/a/tests/copy.rs": ONE,
            }
        )
        self.assertEqual(self.summary(self.flag(base)), [("modified", "one")])

    def test_deleting_a_test_file_is_flagged(self) -> None:
        base = self.commit({RUST_TEST: ONE, "crates/a/tests/keep.rs": TWO})
        self.commit({RUST_TEST: None})
        self.assertEqual(self.summary(self.flag(base)), [("deleted", "one")])

    def test_moving_every_test_out_of_a_file_is_not_flagged(self) -> None:
        base = self.commit({RUST_TEST: ONE})
        self.commit({RUST_TEST: None, "crates/a/tests/moved.rs": ONE + "\n" + TWO})
        self.assertFalse(self.flag(base).flagged)

    def test_renaming_a_test_file_is_flagged(self) -> None:
        base = self.commit({RUST_TEST: ONE})
        self.commit({RUST_TEST: None, "crates/a/tests/renamed.rs": ONE})
        report = self.flag(base)
        self.assertEqual(self.summary(report), [("renamed", f"from {RUST_TEST}")])
        self.assertEqual(report.findings[0].old_path, RUST_TEST)

    def test_unparsable_head_is_flagged(self) -> None:
        base = self.commit({RUST_TEST: ONE})
        self.commit({RUST_TEST: ONE + "fn broken() {\n"})
        report = self.flag(base)
        self.assertEqual([f.kind for f in report.findings], ["unparsed"])

    def test_unparsable_new_file_is_only_a_note(self) -> None:
        base = self.commit({RUST_TEST: ONE})
        self.commit({"crates/a/tests/new.rs": "fn broken() {\n"})
        report = self.flag(base)
        self.assertFalse(report.flagged)
        self.assertEqual(len(report.notes), 1)

    def test_mainline_changes_outside_tests_are_not_flagged(self) -> None:
        lib = "pub fn add(a: u32) -> u32 {\n    a + 1\n}\n\n#[cfg(test)]\nmod tests;\n"
        base = self.commit({"crates/a/src/lib.rs": lib, "crates/a/src/tests.rs": ONE})
        self.commit({"crates/a/src/lib.rs": lib.replace("a + 1", "a + 2")})
        self.assertFalse(self.flag(base).flagged)

    def test_editing_an_inline_test_is_flagged(self) -> None:
        lib = (
            "pub fn add(a: u32) -> u32 { a }\n\n#[cfg(test)]\nmod tests {\n"
            "    #[test]\n    fn adds() {\n        assert_eq!(add(1), 1);\n    }\n}\n"
        )
        base = self.commit({"crates/a/src/lib.rs": lib})
        self.commit({"crates/a/src/lib.rs": lib.replace("add(1), 1", "add(1), 2")})
        self.assertEqual(self.summary(self.flag(base)), [("modified", "tests::adds")])

    def test_editing_a_gated_seam_is_flagged(self) -> None:
        lib = (
            "struct S { a: u8 }\nimpl S {\n    #[cfg(test)]\n"
            "    fn peek(&self) -> u8 { self.a }\n}\n"
        )
        base = self.commit({"crates/a/src/lib.rs": lib})
        self.commit({"crates/a/src/lib.rs": lib.replace("self.a }", "0 }")})
        self.assertEqual(self.summary(self.flag(base)), [("modified", "<S>::peek")])

    def test_moving_an_inline_module_to_a_test_file_is_not_flagged(self) -> None:
        inline = (
            "pub fn add(a: u32) -> u32 { a }\n\n#[cfg(test)]\nmod tests {\n"
            "    use super::*;\n\n    #[test]\n    fn adds() {\n"
            "        assert_eq!(add(1), 1);\n    }\n}\n"
        )
        base = self.commit({"crates/a/src/lib.rs": inline})
        self.commit(
            {
                "crates/a/src/lib.rs": (
                    "pub fn add(a: u32) -> u32 { a }\n\n#[cfg(test)]\nmod tests;\n"
                ),
                "crates/a/src/tests.rs": (
                    "use super::*;\n\n#[test]\nfn adds() {\n    assert_eq!(add(1), 1);\n}\n"
                ),
            }
        )
        self.assertFalse(self.flag(base).flagged)

    def test_dropping_the_test_module_declaration_is_flagged(self) -> None:
        lib = "pub fn add() {}\n\n#[cfg(test)]\nmod tests;\n"
        base = self.commit({"crates/a/src/lib.rs": lib, "crates/a/src/tests.rs": ONE})
        self.commit({"crates/a/src/lib.rs": "pub fn add() {}\n"})
        self.assertEqual(self.summary(self.flag(base)), [("removed", "tests")])

    def test_files_that_are_not_tests_are_ignored(self) -> None:
        base = self.commit(
            {"crates/a/src/lib.rs": "pub fn add() {}\n", "README.md": "a\n"}
        )
        self.commit({"crates/a/src/lib.rs": "pub fn add() { }\n", "README.md": "b\n"})
        report = self.flag(base)
        self.assertFalse(report.flagged)
        self.assertEqual(report.files_scanned, 1)

    def test_python_changes(self) -> None:
        text = (
            "import unittest\n\n\nclass TestX(unittest.TestCase):\n"
            "    def test_a(self):\n        self.assertEqual(1, 1)\n"
        )
        base = self.commit({PY_TEST: text})
        self.commit(
            {PY_TEST: text + "\n    def test_b(self):\n        self.assertTrue(1)\n"}
        )
        self.assertFalse(self.flag(base).flagged)
        self.commit({PY_TEST: text.replace("assertEqual(1, 1)", "assertTrue(1)")})
        self.assertEqual(self.summary(self.flag(base)), [("modified", "TestX::test_a")])

    def test_c_changes(self) -> None:
        text = "int main(void)\n{\n    return 0;\n}\n"
        base = self.commit({C_TEST: text})
        self.commit({C_TEST: text + "\nint other(void) { return 1; }\n"})
        self.assertFalse(self.flag(base).flagged)
        self.commit({C_TEST: text.replace("return 0", "return 1")})
        self.assertEqual(self.summary(self.flag(base)), [("modified", "main")])

    def test_manifest_changes(self) -> None:
        text = '[[tests]]\nname = "a"\nexpected_exit_code = 0\n'
        base = self.commit({MANIFEST: text})
        self.commit(
            {MANIFEST: text + '\n[[tests]]\nname = "b"\nexpected_exit_code = 1\n'}
        )
        self.assertFalse(self.flag(base).flagged)
        self.commit({MANIFEST: text.replace("= 0", "= 1")})
        self.assertEqual(
            self.summary(self.flag(base)), [("modified", '[[tests]] name="a"')]
        )
        self.commit({MANIFEST: ""})
        self.assertEqual(
            self.summary(self.flag(base)), [("removed", '[[tests]] name="a"')]
        )


class TestPolicyFiles(GitCase):
    """The layout allowlist."""

    ENTRY = "R3 crates/a/src/lib.rs fn helper\n"

    def test_growing_the_allowlist_is_flagged(self) -> None:
        base = self.commit({ALLOWLIST_FILE: "# Header.\n"})
        self.commit({ALLOWLIST_FILE: "# Header.\n" + self.ENTRY})
        report = self.flag(base)
        self.assertEqual(
            self.summary(report),
            [("policy-growth", "R3 crates/a/src/lib.rs fn helper")],
        )
        self.assertEqual({f.path for f in report.findings}, {ALLOWLIST_FILE})

    def test_raising_an_entry_is_flagged(self) -> None:
        base = self.commit({ALLOWLIST_FILE: self.ENTRY})
        self.commit(
            {
                ALLOWLIST_FILE: self.ENTRY.replace("helper", "helper x3")
                + "R3 crates/b/src/lib.rs fn other\n"
            }
        )
        self.assertEqual(
            self.summary(self.flag(base)),
            [
                ("policy-growth", "R3 crates/a/src/lib.rs fn helper"),
                ("policy-growth", "R3 crates/b/src/lib.rs fn other"),
            ],
        )

    def test_shrinking_the_allowlist_is_not_flagged(self) -> None:
        base = self.commit(
            {
                ALLOWLIST_FILE: self.ENTRY.replace("helper", "helper x2")
                + "R3 b.rs fn f\n"
            }
        )
        self.commit({ALLOWLIST_FILE: self.ENTRY})
        self.assertFalse(self.flag(base).flagged)
        self.commit({ALLOWLIST_FILE: None})
        self.assertFalse(self.flag(base).flagged)

    def test_unreadable_allowlist_is_flagged(self) -> None:
        base = self.commit({ALLOWLIST_FILE: self.ENTRY})
        self.commit({ALLOWLIST_FILE: "not an allowlist line\n"})
        self.assertEqual([f.kind for f in self.flag(base).findings], ["unparsed"])


class TestGitDiff(GitCase):
    """The git access layer."""

    def test_blob_reader(self) -> None:
        commit = self.commit({"a/b c.txt": "hello\n"})
        with gitdiff.BlobReader(self.root) as blobs:
            self.assertEqual(blobs.read(commit, "a/b c.txt"), b"hello\n")
            self.assertIsNone(blobs.read(commit, "missing.txt"))
            self.assertEqual(blobs.read(commit, "a/b c.txt"), b"hello\n")

    def test_changed_files(self) -> None:
        base = self.commit(
            {"keep.txt": "k\n", "old.txt": "o\n" * 20, "gone.txt": "g\n"}
        )
        self.commit(
            {
                "new.txt": "n\n",
                "old.txt": None,
                "moved.txt": "o\n" * 20,
                "gone.txt": None,
            }
        )
        changes = {
            (c.status, c.path, c.old_path)
            for c in gitdiff.changed_files(self.root, base, "HEAD")
        }
        self.assertEqual(
            changes,
            {
                ("A", "new.txt", "new.txt"),
                ("D", "gone.txt", "gone.txt"),
                ("R", "moved.txt", "old.txt"),
            },
        )

    def test_unknown_revision_is_an_error(self) -> None:
        self.commit({"a.txt": "a\n"})
        with self.assertRaises(gitdiff.GitError):
            gitdiff.resolve(self.root, "no-such-revision")

    def test_merge_base_is_the_fork_point(self) -> None:
        base = self.commit({"a.txt": "a\n"})
        self.git("checkout", "-q", "-b", "topic")
        self.commit({"b.txt": "b\n"})
        self.git("checkout", "-q", "-")
        self.commit({"c.txt": "c\n"})
        self.assertEqual(gitdiff.merge_base(self.root, "topic", "HEAD"), base)

    def test_changes_made_to_the_base_since_the_fork_are_ignored(self) -> None:
        base = self.commit({RUST_TEST: ONE})
        self.git("checkout", "-q", "-b", "topic")
        self.commit({"crates/a/tests/new.rs": TWO})
        self.git("checkout", "-q", "-")
        self.commit({RUST_TEST: ONE.replace("1, 1", "1, 2")})
        report = flagging.analyze(self.root, "HEAD", "topic")
        self.assertEqual(report.merge_base, base)
        self.assertFalse(report.flagged)


class TestRendering(unittest.TestCase):
    """The Markdown, text and JSON reports."""

    @staticmethod
    def report(*findings: flagging.Finding) -> flagging.Report:
        return flagging.Report("base", "head", "abc", 3, list(findings))

    def test_clean_report(self) -> None:
        text = flagging.render_markdown(self.report())
        self.assertTrue(text.startswith(flagging.MARKER))
        self.assertIn("No existing tests were changed", text)

    def test_untrusted_text_stays_inside_code_spans(self) -> None:
        finding = flagging.Finding(
            "modified", "a|b`c.rs", "@octocat <b>|x|</b> `", head_line=4
        )
        text = flagging.render_markdown(self.report(finding))
        self.assertIn(
            "| modified | `a\\|b'c.rs` | `@octocat <b>\\|x\\|</b> '` | 4 |", text
        )

    def test_long_reports_are_truncated_in_the_comment_only(self) -> None:
        findings = [flagging.Finding("removed", "t.rs", f"t{i}", i) for i in range(50)]
        comment = flagging.render_markdown(self.report(*findings), limit=10)
        self.assertIn("…and 40 more", comment)
        self.assertEqual(comment.count("| removed |"), 10)
        full = flagging.render_markdown(
            self.report(*findings), limit=None, marker=False
        )
        self.assertEqual(full.count("| removed |"), 50)
        self.assertFalse(full.startswith(flagging.MARKER))

    def test_removed_items_show_their_base_line(self) -> None:
        text = flagging.render_markdown(
            self.report(flagging.Finding("removed", "t.rs", "t", 7))
        )
        self.assertIn("| 7 (base) |", text)

    def test_text_report(self) -> None:
        text = flagging.render_text(
            self.report(flagging.Finding("modified", "t.rs", "t", 7, 8, "changed"))
        )
        self.assertIn("t.rs:8: modified: t - changed", text)
        self.assertIn("1 change(s)", text)

    def test_json_report(self) -> None:
        data = self.report(flagging.Finding("removed", "t.rs", "t", 7)).to_json()
        self.assertTrue(data["flagged"])
        self.assertEqual(data["findings"][0]["path"], "t.rs")
        self.assertEqual(json.loads(json.dumps(data)), data)


class TestCli(GitCase):
    """The command line interface."""

    def setUp(self) -> None:
        super().setUp()
        self.cli = _load_cli()
        self.base = self.commit({RUST_TEST: ONE})

    def run_cli(self, *args: str, env: dict[str, str] | None = None) -> tuple[int, str]:
        out = io.StringIO()
        with contextlib.redirect_stdout(out), mock.patch.dict(os.environ, env or {}):
            status = self.cli.main(["--root", str(self.root), *args])
        return status, out.getvalue()

    def test_flagged_run_is_not_a_failure(self) -> None:
        self.commit({RUST_TEST: ONE.replace("1, 1", "1, 2")})
        status, out = self.run_cli("--base", self.base)
        self.assertEqual(status, 0)
        self.assertIn("modified: one", out)

    def test_json_to_stdout(self) -> None:
        self.commit({RUST_TEST: TWO})
        status, out = self.run_cli("--base", self.base, "--json")
        self.assertEqual(status, 0)
        data = json.loads(out)
        self.assertTrue(data["flagged"])
        self.assertEqual(data["findings"][0]["kind"], "removed")

    def test_markdown_to_file(self) -> None:
        self.commit({RUST_TEST: TWO})
        target = self.root / "comment.md"
        self.run_cli("--base", self.base, "--markdown", str(target))
        self.assertTrue(target.read_text(encoding="utf-8").startswith(flagging.MARKER))

    def test_github_integration(self) -> None:
        self.commit({RUST_TEST: TWO})
        output = self.root / "output.txt"
        summary = self.root / "summary.md"
        self.run_cli(
            "--base",
            self.base,
            "--github-output",
            "--step-summary",
            env={"GITHUB_OUTPUT": str(output), "GITHUB_STEP_SUMMARY": str(summary)},
        )
        self.assertEqual(
            output.read_text(encoding="utf-8").split(), ["flagged=true", "count=1"]
        )
        self.assertIn(
            "Existing tests were changed", summary.read_text(encoding="utf-8")
        )

    def test_clean_run(self) -> None:
        self.commit({"crates/a/tests/new.rs": TWO})
        output = self.root / "output.txt"
        status, out = self.run_cli(
            "--base", self.base, "--github-output", env={"GITHUB_OUTPUT": str(output)}
        )
        self.assertEqual(status, 0)
        self.assertIn("No existing tests were changed", out)
        self.assertEqual(
            output.read_text(encoding="utf-8").split(), ["flagged=false", "count=0"]
        )

    def test_unknown_revision_fails(self) -> None:
        with contextlib.redirect_stderr(io.StringIO()):
            status, _ = self.run_cli("--base", "no-such-revision")
        self.assertEqual(status, 2)


if __name__ == "__main__":
    unittest.main()
