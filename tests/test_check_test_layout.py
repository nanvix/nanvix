# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Unit tests for the test-layout checker (scripts/check-test-layout.py)."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import sys
import tempfile
import unittest
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parent.parent
_SCRIPTS = _REPO_ROOT / "scripts"
sys.path.insert(0, str(_SCRIPTS))

from testlayout import policy  # noqa: E402
from testlayout import violations as vl  # noqa: E402
from testlayout.cfgexpr import is_test_only, parse_cfg_args  # noqa: E402
from testlayout.paths import (  # noqa: E402
    ALLOWLIST_FILE,
    is_c_test_file,
    is_python_test_file,
    is_rust_test_file,
    module_candidates,
    module_directory,
)
from testlayout.rustlex import IDENT, STRING, LexError, tokenize  # noqa: E402
from testlayout.scan import scan_rust  # noqa: E402


def _load_cli():
    spec = importlib.util.spec_from_file_location(
        "check_test_layout", _SCRIPTS / "check-test-layout.py"
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _test_only(expr: str) -> bool:
    toks = tokenize(f"cfg({expr})")
    return is_test_only(parse_cfg_args(toks, 1))


class TestLexer(unittest.TestCase):
    """Tests for the Rust tokenizer."""

    def test_raw_string_is_one_token(self) -> None:
        """A raw string may contain quotes and comment markers."""
        toks = tokenize('let s = r#"a "quoted" // not a comment"#;')
        strings = [toks.src(i) for i in range(len(toks)) if toks.kinds[i] == STRING]
        self.assertEqual(strings, ['r#"a "quoted" // not a comment"#'])

    def test_nested_block_comments_are_skipped(self) -> None:
        """Block comments nest, so a stray terminator does not end them early."""
        toks = tokenize("a /* x /* y */ z */ b")
        words = [toks.src(i) for i in range(len(toks)) if toks.kinds[i] == IDENT]
        self.assertEqual(words, ["a", "b"])

    def test_lifetimes_and_chars(self) -> None:
        """Lifetimes do not swallow code, and a `}` char literal is not a bracket."""
        text = "fn f<'a>(x: &'a str) -> char { '}' }"
        toks = tokenize(text)
        brace = [i for i in range(len(toks)) if toks.is_punct(i, "{")][0]
        self.assertEqual(toks.ends[toks.partner[brace]], len(text))

    def test_unbalanced_brackets_are_rejected(self) -> None:
        """Unbalanced source cannot be scanned reliably."""
        with self.assertRaises(LexError):
            tokenize("fn f() { ")


class TestCfg(unittest.TestCase):
    """Tests for the cfg predicate evaluation."""

    def test_test_only_predicates(self) -> None:
        """Predicates that are false whenever `test` is false are test-only."""
        for expr in (
            "test",
            'feature = "test"',
            'all(test, feature = "std")',
            'all(feature = "std", test, not(feature = "x"))',
            'any(test, feature = "test")',
            'all(any(test, feature = "test"), feature = "std")',
        ):
            with self.subTest(expr=expr):
                self.assertTrue(_test_only(expr))

    def test_other_predicates(self) -> None:
        """Predicates that can hold outside tests are mainline code."""
        for expr in (
            "not(test)",
            'any(test, feature = "std")',
            'feature = "std"',
            'any(feature = "syscall", test)',
            "unix",
        ):
            with self.subTest(expr=expr):
                self.assertFalse(_test_only(expr))


class TestScan(unittest.TestCase):
    """Tests for the Rust scanner."""

    def test_inline_module_counts_tests(self) -> None:
        """An inline test module records its extent and its number of tests."""
        text = (
            "pub fn add(a: u32, b: u32) -> u32 { a + b }\n"
            "\n"
            '#[cfg(all(test, feature = "std"))]\n'
            "mod tests {\n"
            "    use super::*;\n"
            "\n"
            "    #[test]\n"
            "    fn one() {}\n"
            "\n"
            "    #[test]\n"
            "    fn two() {}\n"
            "}\n"
        )
        result = scan_rust(text)
        self.assertEqual(len(result.mods), 1)
        mod = result.mods[0]
        self.assertEqual(
            (mod.name, mod.inline, mod.tests, mod.line), ("tests", True, 2, 3)
        )
        self.assertEqual(text[mod.start_off : mod.start_off + 11], "#[cfg(all(t")
        self.assertEqual(text[mod.body_open], "{")
        self.assertEqual(text[mod.body_close], "}")
        self.assertEqual(result.test_attr_lines, [])

    def test_out_of_line_module(self) -> None:
        """A declaration without a body is an out-of-line module."""
        result = scan_rust("#[cfg(test)]\nmod tests;\n")
        self.assertEqual([(m.name, m.inline) for m in result.mods], [("tests", False)])

    def test_multiple_cfg_attributes_are_conjoined(self) -> None:
        """Individually broad cfg predicates can jointly imply a test build."""
        attrs = '#[cfg(any(feature = "test", feature = "std"))]\n#[cfg(not(feature = "std"))]\n'
        result = scan_rust(attrs + "mod checks { fn test() {} }\n")
        self.assertEqual([m.name for m in result.mods], ["checks"])
        result = scan_rust(attrs.replace("#[", "#![") + "fn helper() {}\n")
        self.assertEqual(result.file_gate_line, 1)

    def test_conditional_test_attributes_are_detected(self) -> None:
        """Tests declared through nested cfg_attr attributes remain tests."""
        for attr in (
            "cfg_attr(test, test)",
            "cfg_attr(test, allow(dead_code), test)",
            "cfg_attr(test, cfg_attr(unix, tokio::test))",
        ):
            with self.subTest(attr=attr):
                result = scan_rust(f"#[{attr}]\nfn example() {{}}\n")
                self.assertEqual(result.test_attr_lines, [1])

    def test_path_attribute_is_recorded(self) -> None:
        """A `#[path]` attribute overrides the default location."""
        result = scan_rust('#[cfg(test)]\n#[path = "tests/x.rs"]\nmod x;\n')
        self.assertEqual(result.mods[0].path_attr, "tests/x.rs")

    def test_seams(self) -> None:
        """Test-only items other than modules are seams; imports are wiring."""
        text = (
            "#[cfg(test)]\nfn helper() {}\n"
            "#[cfg(test)]\nuse std::vec::Vec;\n"
            "#[cfg(test)]\nextern crate alloc;\n"
            "struct S {\n    #[cfg(test)]\n    hook: u32,\n}\n"
        )
        result = scan_rust(text)
        self.assertEqual(
            [(s.kind, s.name) for s in result.seams][:1],
            [("fn", "helper")],
        )
        self.assertFalse(any(s.kind in ("use", "extern_crate") for s in result.seams))

    def test_mainline_gates_are_ignored(self) -> None:
        """Gates that also hold outside of tests are not test code."""
        text = (
            "#[cfg(not(test))]\nmod real {}\n"
            '#[cfg(any(test, feature = "std"))]\nmod shared {}\n'
        )
        result = scan_rust(text)
        self.assertEqual((result.mods, result.seams), ([], []))

    def test_decoys_in_strings_and_comments(self) -> None:
        """Attributes inside strings and comments are not code."""
        text = (
            'const S: &str = "#[cfg(test)] mod tests { }";\n'
            "// #[cfg(test)] mod hidden { }\n"
            "/* #[cfg(test)] mod hidden2 { } */\n"
            'const R: &str = r#"#[test] fn x() {}"#;\n'
        )
        result = scan_rust(text)
        self.assertEqual(
            (result.mods, result.seams, result.test_attr_lines), ([], [], [])
        )

    def test_stray_test_attribute(self) -> None:
        """A test attribute outside a gated item is reported with its line."""
        result = scan_rust("fn f() {}\n\n#[test]\nfn t() {}\n")
        self.assertEqual(result.test_attr_lines, [3])

    def test_file_level_gate(self) -> None:
        """A crate-level test gate marks the whole file as test code."""
        result = scan_rust('#![cfg(all(test, feature = "std"))]\nfn x() {}\n')
        self.assertEqual(result.file_gate_line, 1)

    def test_doc_tests(self) -> None:
        """Fenced blocks that rustdoc would run are reported; others are not."""
        text = (
            "/// ```\n/// assert!(true);\n/// ```\nfn a() {}\n"
            "/// ```text\n/// not code\n/// ```\nfn b() {}\n"
            "/// ```ignore\n/// let x = 1;\n/// ```\nfn c() {}\n"
            "/// ```rust,no_run\n/// let y = 2;\n/// ```\nfn d() {}\n"
            "/// ```sh\n/// echo hi\n/// ```\nfn e() {}\n"
        )
        self.assertEqual(scan_rust(text).doctest_lines, [1, 13])

    def test_target_specific_doc_ignores_are_not_universal(self) -> None:
        """A target-specific ignore leaves an executable test on other targets."""
        for info in ("ignore-x86_64", "rust,ignore-x86_64", "rust,ignore"):
            with self.subTest(info=info):
                text = f"/// ```{info}\n/// assert!(true);\n/// ```\nfn f() {{}}\n"
                expected = [] if info == "rust,ignore" else [1]
                self.assertEqual(scan_rust(text).doctest_lines, expected)

    def test_indented_doc_tests(self) -> None:
        """Indented code blocks are tests in both line and block documentation."""
        examples = (
            ("///     assert!(true);\nfn f() {}\n", [1]),
            ("/// Example.\n///\n///     assert!(true);\nfn f() {}\n", [3]),
            ("/** Example.\n\n    assert!(true);\n*/\nfn f() {}\n", [3]),
            ("/** Example.\n *\n *     assert!(true);\n */\nfn f() {}\n", [3]),
        )
        for text, expected in examples:
            with self.subTest(text=text):
                self.assertEqual(scan_rust(text).doctest_lines, expected)

    def test_indentation_inside_paragraphs_and_text_fences_is_not_code(self) -> None:
        """Paragraph continuations and ignored fenced contents stay non-executable."""
        for text in (
            "/// Paragraph\n///     continuation\nfn f() {}\n",
            "/// ```text\n///     example\n/// ```\nfn f() {}\n",
        ):
            with self.subTest(text=text):
                self.assertEqual(scan_rust(text).doctest_lines, [])


class TestPaths(unittest.TestCase):
    """Tests for path classification and module resolution."""

    def test_rust_test_files(self) -> None:
        """Test files are named or placed as such."""
        for path in (
            "src/libs/a/src/tests.rs",
            "src/libs/a/src/test.rs",
            "src/libs/a/src/foo_tests.rs",
            "src/libs/a/src/test_foo.rs",
            "src/libs/a/src/tests/foo.rs",
            "src/tests/integration/x/src/main.rs",
        ):
            with self.subTest(path=path):
                self.assertTrue(is_rust_test_file(path))
        for path in (
            "src/libs/a/src/lib.rs",
            "src/libs/a/src/latest.rs",
            "src/libs/a/src/contest.rs",
        ):
            with self.subTest(path=path):
                self.assertFalse(is_rust_test_file(path))

    def test_python_and_c_test_files(self) -> None:
        """Python and C test files follow their own conventions."""
        self.assertTrue(is_python_test_file("tests/test_z.py"))
        self.assertTrue(is_python_test_file("scripts/test_helper.py"))
        self.assertFalse(is_python_test_file("scripts/run-smoke-test.py"))
        self.assertTrue(is_c_test_file("src/libs/a/test_foo.c"))
        self.assertFalse(is_c_test_file("src/libs/a/foo.c"))

    def test_module_resolution(self) -> None:
        """Child modules live next to roots and mod.rs, and under the stem otherwise."""
        roots = {"crates/a/src/lib.rs"}
        self.assertEqual(module_directory("crates/a/src/lib.rs", roots), "crates/a/src")
        self.assertEqual(
            module_directory("crates/a/src/foo/mod.rs", roots), "crates/a/src/foo"
        )
        self.assertEqual(
            module_directory("crates/a/src/foo.rs", roots), "crates/a/src/foo"
        )
        self.assertEqual(
            module_candidates("crates/a/src/foo.rs", "tests", roots),
            ["crates/a/src/foo/tests.rs", "crates/a/src/foo/tests/mod.rs"],
        )
        self.assertEqual(
            module_candidates("crates/a/src/foo.rs", "x", roots, "tests/x.rs"),
            ["crates/a/src/tests/x.rs"],
        )


class TestAllowlistFormat(unittest.TestCase):
    """Tests for the allowlist file format."""

    def test_round_trip(self) -> None:
        """Every seam entry survives rendering and parsing."""
        entries = {
            "R3 a/b.rs fn helper": 1,
            "R3 a/b.rs code let x = 1;": 2,
            "R3 a/b.rs struct S::field: bool,": 3,
        }
        for key, value in entries.items():
            with self.subTest(key=key):
                self.assertEqual(vl.parse_line(vl.render(key, value)), (key, value))

    def test_malformed_line(self) -> None:
        """Unrecognized lines are rejected."""
        with self.assertRaises(vl.AllowlistError):
            vl.parse_line("whatever")

    def test_only_seams_can_be_allowlisted(self) -> None:
        """Every rule other than R3 is strict: it cannot be listed."""
        for line in (
            "R1 a/b.rs",
            "R2 a/b.rs mod tests",
            "R2 a/b.rs file gate",
            "R4 a/b.rs",
            "R5 tools/test_x.py py",
        ):
            with self.subTest(line=line):
                with self.assertRaises(vl.AllowlistError):
                    vl.parse_line(line)

    def test_duplicates_are_rejected(self) -> None:
        """A key may appear only once."""
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp, "allowlist.txt")
            path.write_text("R3 a.rs fn f\nR3 a.rs fn f\n", encoding="utf-8")
            with self.assertRaises(vl.AllowlistError):
                vl.load(path)

    def test_missing_file_is_empty(self) -> None:
        """A file that does not exist is an empty allowlist."""
        self.assertEqual(vl.load(Path("does-not-exist.txt")), {})


class RepoCase(unittest.TestCase):
    """Base class that builds a throw-away repository on disk."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = Path(self._tmp.name)
        self.files: list[str] = []
        self.write("crates/a/Cargo.toml", '[package]\nname = "a"\n')

    def write(self, rel: str, text: str) -> None:
        """Writes a file below the repository root."""
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "w", encoding="utf-8", newline="\n") as handle:
            handle.write(text)
        if rel not in self.files:
            self.files.append(rel)

    def check(self) -> policy.Report:
        """Runs the policy over the files written so far."""
        return policy.check(self.root, sorted(self.files))

    def rules(self) -> list[str]:
        """Returns the rules of the current findings."""
        return [f.rule for f in self.check().findings]


class TestRules(RepoCase):
    """Tests for rules R1-R6."""

    def test_clean_layout(self) -> None:
        """A crate with its tests in tests.rs passes."""
        self.write("crates/a/src/lib.rs", "pub fn f() {}\n#[cfg(test)]\nmod tests;\n")
        self.write("crates/a/src/tests.rs", "use super::*;\n#[test]\nfn t() { f(); }\n")
        self.assertTrue(self.check().ok, self.check().findings)

    def test_inline_module_is_flagged(self) -> None:
        """R2: an inline test module is a violation."""
        self.write(
            "crates/a/src/lib.rs",
            "#[cfg(test)]\nmod tests {\n #[test]\n fn t() {}\n}\n",
        )
        findings = self.check().findings
        self.assertEqual(
            [(f.rule, f.path, f.line) for f in findings],
            [("R2", "crates/a/src/lib.rs", 1)],
        )
        self.assertIn("crates/a/src/tests.rs", findings[0].message)

    def test_module_must_resolve_to_a_test_file(self) -> None:
        """R2: an out-of-line module has to live in a test file."""
        self.write("crates/a/src/lib.rs", "#[cfg(test)]\nmod checks;\n")
        self.write("crates/a/src/checks.rs", "pub fn t() {}\n")
        self.assertEqual(self.rules(), ["R2"])

    def test_path_attribute_into_tests_directory(self) -> None:
        """A `#[path]` into a tests/ directory is accepted."""
        self.write(
            "crates/a/src/lib.rs",
            '#[cfg(test)]\n#[path = "tests/checks.rs"]\nmod checks;\n',
        )
        self.write("crates/a/src/tests/checks.rs", "#[test]\nfn t() {}\n")
        self.assertTrue(self.check().ok, self.check().findings)

    def test_missing_module_file(self) -> None:
        """R2: a declaration that resolves to nothing is reported."""
        self.write("crates/a/src/lib.rs", "#[cfg(test)]\nmod tests;\n")
        self.assertEqual(self.rules(), ["R2"])

    def test_module_in_mod_rs_and_nested_file(self) -> None:
        """Modules resolve below the declaring file's module directory."""
        self.write("crates/a/src/lib.rs", "mod foo;\n")
        self.write("crates/a/src/foo.rs", "#[cfg(test)]\nmod tests;\n")
        self.write("crates/a/src/foo/tests.rs", "#[test]\nfn t() {}\n")
        self.assertTrue(self.check().ok, self.check().findings)

    def test_binary_children_are_not_crate_roots(self) -> None:
        """Auto-discovery only treats binary entry points as crate roots."""
        self.write("crates/a/src/bin/cli.rs", "mod helper;\n")
        self.write("crates/a/src/bin/cli/helper.rs", "#[cfg(test)]\nmod tests;\n")
        self.write("crates/a/src/bin/cli/helper/tests.rs", "#[test]\nfn t() {}\n")
        self.write("crates/a/src/bin/tool/main.rs", "#[cfg(test)]\nmod tests;\n")
        self.write("crates/a/src/bin/tool/tests.rs", "#[test]\nfn t() {}\n")
        self.assertTrue(self.check().ok, self.check().findings)

    def test_inline_path_attribute_uses_the_module_directory(self) -> None:
        """A path inside an inline module is relative to that module's directory."""
        self.write("crates/a/src/lib.rs", "mod foo;\n")
        self.write(
            "crates/a/src/foo.rs",
            'mod outer {\n#[cfg(test)]\n#[path = "tests/checks.rs"]\nmod checks;\n}\n',
        )
        self.write("crates/a/src/foo/outer/tests/checks.rs", "#[test]\nfn t() {}\n")
        self.assertTrue(self.check().ok, self.check().findings)

    def test_file_level_gate_is_flagged(self) -> None:
        """R2: a crate-level test gate in a mainline file is a violation."""
        self.write("crates/a/src/lib.rs", "pub mod x;\n")
        self.write("crates/a/src/x.rs", "#![cfg(test)]\nfn t() {}\n")
        self.assertEqual(self.rules(), ["R2"])

    def test_seam_needs_allowlist(self) -> None:
        """R3: test-only code in mainline source must be allowlisted."""
        self.write("crates/a/src/lib.rs", "#[cfg(test)]\nfn helper() {}\n")
        self.assertEqual(self.rules(), ["R3"])
        self.write(ALLOWLIST_FILE, "R3 crates/a/src/lib.rs fn helper\n")
        self.assertTrue(self.check().ok, self.check().findings)

    def test_allowlist_limits_occurrences(self) -> None:
        """R3: an allowlist entry covers only as many occurrences as it states."""
        self.write(
            "crates/a/src/lib.rs",
            "#[cfg(test)]\nfn helper() {}\n#[cfg(test)]\nfn helper() {}\n",
        )
        self.write(ALLOWLIST_FILE, "R3 crates/a/src/lib.rs fn helper\n")
        self.assertEqual(self.rules(), ["R3"])
        self.write(ALLOWLIST_FILE, "R3 crates/a/src/lib.rs fn helper x2\n")
        self.assertTrue(self.check().ok, self.check().findings)

    def test_stale_allowlist_entry(self) -> None:
        """R3: an allowlist entry whose code is gone is reported."""
        self.write("crates/a/src/lib.rs", "pub fn f() {}\n")
        self.write(ALLOWLIST_FILE, "R3 crates/a/src/lib.rs fn helper\n")
        findings = self.check().findings
        self.assertEqual([f.rule for f in findings], ["R3"])
        self.assertIn("stale allowlist", findings[0].message)

    def test_partial_removal_requires_lowering_the_allowlist_count(self) -> None:
        """A seam allowlist entry must match exactly, not merely bound the count."""
        self.write("crates/a/src/lib.rs", "#[cfg(test)]\nfn helper() {}\n")
        self.write(ALLOWLIST_FILE, "R3 crates/a/src/lib.rs fn helper x2\n")
        findings = self.check().findings
        self.assertEqual(len(findings), 1)
        self.assertIn("stale allowlist", findings[0].message)
        self.assertIn("(2 -> 1)", findings[0].message)

    def test_stray_test_attribute(self) -> None:
        """R1: a test attribute in a mainline file is a violation, in a test file it is not."""
        self.write("crates/a/src/lib.rs", "mod util;\n")
        self.write("crates/a/src/util.rs", "#[test]\nfn t() {}\n")
        self.assertEqual(self.rules(), ["R1"])

    def test_doc_test_is_flagged(self) -> None:
        """R4: an executable doc-test is a violation."""
        self.write(
            "crates/a/src/lib.rs",
            "/// ```\n/// assert!(true);\n/// ```\npub fn f() {}\n",
        )
        self.assertEqual(self.rules(), ["R4"])

    def test_python_tests_live_under_tests(self) -> None:
        """R5: Python tests belong in tests/."""
        self.write("tools/test_thing.py", "import unittest\n")
        self.write(
            "tools/helper.py",
            "import unittest\n\nclass T(unittest.TestCase):\n    pass\n",
        )
        self.write("tools/plain.py", "print('hi')\n")
        self.write("tests/test_ok.py", "import unittest\n")
        findings = self.check().findings
        self.assertEqual(
            sorted((f.rule, f.path) for f in findings),
            [("R5", "tools/helper.py"), ("R5", "tools/test_thing.py")],
        )

    def test_c_tests_live_under_src_tests(self) -> None:
        """R5: C tests belong in src/tests/."""
        self.write("src/libs/x/test_x.c", "int main(void) { return 0; }\n")
        self.write("src/tests/unit/test_y.c", "int main(void) { return 0; }\n")
        self.write("src/libs/x/x.c", "int x(void) { return 0; }\n")
        self.assertEqual(
            [(f.rule, f.path) for f in self.check().findings],
            [("R5", "src/libs/x/test_x.c")],
        )

    def test_e2e_manifests_live_under_test(self) -> None:
        """R6: end-to-end manifests belong in test/."""
        self.write("docs/test-smoke.toml", "[x]\n")
        self.write("test/test-smoke.toml", "[x]\n")
        self.assertEqual(
            [(f.rule, f.path) for f in self.check().findings],
            [("R6", "docs/test-smoke.toml")],
        )

    def test_unparsable_file_is_reported(self) -> None:
        """A file that cannot be scanned is a finding, not a crash."""
        self.write("crates/a/src/lib.rs", "fn f() {\n")
        self.assertEqual(self.rules(), ["R0"])


class TestAllowlist(RepoCase):
    """Tests for the allowlist as the only exception to the policy."""

    INLINE = (
        "#[cfg(test)]\nmod tests {\n #[test]\n fn t() {}\n #[test]\n fn u() {}\n}\n"
    )

    def test_inline_module_cannot_be_allowlisted(self) -> None:
        """Only seams can be listed, so an inline test module always fails."""
        self.write("crates/a/src/lib.rs", self.INLINE)
        self.assertEqual(self.rules(), ["R2"])
        self.write(ALLOWLIST_FILE, "R2 crates/a/src/lib.rs mod tests\n")
        with self.assertRaises(vl.AllowlistError):
            self.check()

    def test_uncovered_seams_lists_missing_entries_only(self) -> None:
        """The printed lines cover exactly the seams that are not allowed yet."""
        self.write(
            "crates/a/src/lib.rs",
            "#[cfg(test)]\nfn one() {}\n#[cfg(test)]\nfn two() {}\n#[cfg(test)]\nfn two() {}\n",
        )
        self.write(ALLOWLIST_FILE, "R3 crates/a/src/lib.rs fn one\n")
        self.assertEqual(
            policy.uncovered_seams(self.check()), ["R3 crates/a/src/lib.rs fn two x2"]
        )

    def test_uncovered_seams_raises_a_short_entry(self) -> None:
        """A seam listed with too small a count is printed with its full count."""
        self.write(
            "crates/a/src/lib.rs",
            "#[cfg(test)]\nfn two() {}\n#[cfg(test)]\nfn two() {}\n",
        )
        self.write(ALLOWLIST_FILE, "R3 crates/a/src/lib.rs fn two\n")
        self.assertEqual(
            policy.uncovered_seams(self.check()), ["R3 crates/a/src/lib.rs fn two x2"]
        )

    def test_uncovered_seams_ignores_other_rules(self) -> None:
        """Violations that cannot be allowlisted are not offered as entries."""
        self.write("crates/a/src/lib.rs", self.INLINE)
        self.assertEqual(policy.uncovered_seams(self.check()), [])


class TestCli(RepoCase):
    """Tests for the command-line interface."""

    def setUp(self) -> None:
        super().setUp()
        self.cli = _load_cli()

    def run_cli(self, *args: str) -> tuple[int, str]:
        """Runs the CLI against the throw-away repository."""
        out = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
            code = self.cli.main(["--root", str(self.root), *args])
        return code, out.getvalue()

    def test_exit_codes(self) -> None:
        """0 for a clean tree, 1 for violations, 2 for a malformed allowlist."""
        self.write("crates/a/src/lib.rs", "pub fn f() {}\n")
        self.assertEqual(self.run_cli()[0], 0)
        self.write("crates/a/src/lib.rs", "#[cfg(test)]\nmod tests {}\n")
        code, out = self.run_cli()
        self.assertEqual(code, 1)
        self.assertIn("crates/a/src/lib.rs:1:", out)
        self.write(ALLOWLIST_FILE, "garbage\n")
        self.assertEqual(self.run_cli()[0], 2)

    def test_github_format(self) -> None:
        """The github format emits workflow annotations."""
        self.write("crates/a/src/lib.rs", "#[cfg(test)]\nmod tests {}\n")
        code, out = self.run_cli("--format", "github")
        self.assertEqual(code, 1)
        self.assertTrue(out.startswith("::error file=crates/a/src/lib.rs,line=1,"))

    def test_print_allowlist_covers_the_seams(self) -> None:
        """The printed lines make a tree that only has seams pass."""
        self.write(
            "crates/a/src/lib.rs",
            "#[cfg(test)]\nfn h() {}\n#[cfg(test)]\nfn g() {}\n#[cfg(test)]\nfn g() {}\n",
        )
        self.assertEqual(self.run_cli()[0], 1)
        code, out = self.run_cli("--print-allowlist")
        self.assertEqual(code, 0)
        self.assertEqual(
            out.splitlines(),
            ["R3 crates/a/src/lib.rs fn g x2", "R3 crates/a/src/lib.rs fn h"],
        )
        self.write(ALLOWLIST_FILE, out)
        self.assertEqual(self.run_cli()[0], 0)
        self.assertEqual(self.run_cli("--print-allowlist")[1], "")


if __name__ == "__main__":
    unittest.main()
