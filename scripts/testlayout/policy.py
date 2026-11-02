# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""The test-layout policy: tests live in their own files, never in mainline source.

Rules (applied outside test files):

- R1: no `#[test]`-style attributes.
- R2: test-only gated modules must be out of line and resolve to a test file.
- R3: other test-only gated code ("seams") must be listed in the allowlist.
- R4: no executable doc-tests.
- R5: Python tests live under `tests/`; C/C++ tests live under `src/tests/`.
- R6: end-to-end test manifests live under `test/`.

Every violation fails the check. Only R3 seams can be allowlisted, and an allowlist entry whose code
is gone fails the check as well, so the allowlist cannot silently outlive the code it covers.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

from . import violations as vl
from .cfgexpr import CfgParseError
from .paths import (
    ALLOWLIST_FILE,
    C_EXTENSIONS,
    crate_roots,
    is_c_test_file,
    is_python_test_file,
    is_rust_test_file,
    list_repo_files,
    looks_like_e2e_manifest,
    module_candidates,
    python_looks_like_test,
)
from .rustlex import LexError
from .scan import GatedMod, scan_rust

DOC_HINT = "see doc/test.md ('Test layout')"


@dataclass
class Finding:
    """A reportable problem, anchored to a file and line."""

    path: str
    line: int
    rule: str
    message: str


@dataclass
class Report:
    """Result of one policy evaluation."""

    findings: list[Finding] = field(default_factory=list)
    current: dict[str, vl.Aggregate] = field(default_factory=dict)
    allowlist: dict[str, int] = field(default_factory=dict)

    @property
    def ok(self) -> bool:
        """Tells whether the policy holds."""
        return not self.findings


def expected_module_file(
    file: str, name: str, roots: set[str], chain: tuple[str, ...]
) -> str:
    """Returns the file a moved test module is expected to live in."""
    return module_candidates(file, name, roots, None, chain)[0]


def collect_violations(
    root: Path, files: list[str] | None = None
) -> list[vl.Violation]:
    """Scans the repository and returns every policy violation (before allowlist filtering)."""
    names = files if files is not None else list_repo_files(root)
    known = set(names)
    roots = crate_roots(root, names)
    out: list[vl.Violation] = []
    for path in names:
        lower = path.lower()
        if lower.endswith(".rs"):
            if not is_rust_test_file(path):
                out.extend(_check_rust(root, path, known, roots))
        elif lower.endswith(".py"):
            out.extend(_check_python(root, path))
        elif lower.endswith(C_EXTENSIONS):
            if not is_c_test_file(path):
                continue
            if not path.startswith("src/tests/"):
                out.append(
                    _misplaced(
                        "R5", path, "c", "C/C++ test files must live under src/tests/"
                    )
                )
        elif lower.endswith(".toml"):
            if looks_like_e2e_manifest(path) and not path.startswith(
                ("test/", "src/tests/")
            ):
                out.append(
                    _misplaced(
                        "R6",
                        path,
                        "toml",
                        "end-to-end test manifests must live in test/",
                    )
                )
    return out


def _misplaced(rule: str, path: str, kind: str, message: str) -> vl.Violation:
    return vl.Violation(rule, path, 1, f"{rule} {path} {kind}", 1, f"{rule} {message}")


def _check_python(root: Path, path: str) -> list[vl.Violation]:
    if path.startswith("tests/"):
        return []
    named = is_python_test_file(path)
    if not named:
        try:
            named = python_looks_like_test((root / path).read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError):
            return []
    if not named:
        return []
    return [_misplaced("R5", path, "py", "Python test files must live under tests/")]


def _check_rust(
    root: Path, path: str, known: set[str], roots: set[str]
) -> list[vl.Violation]:
    try:
        text = (root / path).read_text(encoding="utf-8")
        result = scan_rust(text)
    except (OSError, UnicodeDecodeError) as err:
        return [
            vl.Violation(
                "R0", path, 1, f"R0 {path} read", 1, f"R0 cannot read file: {err}"
            )
        ]
    except (LexError, CfgParseError) as err:
        return [
            vl.Violation(
                "R0", path, 1, f"R0 {path} parse", 1, f"R0 cannot parse file: {err}"
            )
        ]
    out: list[vl.Violation] = []
    if result.file_gate_line:
        out.append(
            vl.Violation(
                "R2",
                path,
                result.file_gate_line,
                f"R2 {path} file gate",
                1,
                "R2 file-level test gate in a non-test file; move the test code to a test file",
            )
        )
    for mod in result.mods:
        out.extend(_check_mod(path, mod, known, roots))
    for seam in result.seams:
        out.append(
            vl.Violation(
                "R3",
                path,
                seam.line,
                f"R3 {path} {seam.kind} {seam.name}",
                1,
                f"R3 test-only {seam.kind} '{seam.name}' in mainline source; move it into the "
                f"child test module or list it in {ALLOWLIST_FILE} "
                "(`check-test-layout.py --print-allowlist` prints the entries)",
            )
        )
    if result.test_attr_lines:
        out.extend(
            vl.Violation(
                "R1",
                path,
                line,
                f"R1 {path}",
                1,
                "R1 test attribute outside a test file; tests must live in their own files "
                f"({DOC_HINT})",
            )
            for line in result.test_attr_lines
        )
    out.extend(
        vl.Violation(
            "R4",
            path,
            line,
            f"R4 {path}",
            1,
            "R4 executable doc-test; mark the fence `text` or `ignore`, or write a real test "
            "in a test file",
        )
        for line in result.doctest_lines
    )
    return out


def _check_mod(
    path: str, mod: GatedMod, known: set[str], roots: set[str]
) -> list[vl.Violation]:
    if mod.inline:
        target = expected_module_file(path, mod.name, roots, mod.chain)
        return [
            vl.Violation(
                "R2",
                path,
                mod.line,
                f"R2 {path} mod {mod.name}",
                mod.tests,
                f"R2 inline test module '{mod.name}' ({mod.tests} tests); move it to {target}",
            )
        ]
    candidates = module_candidates(path, mod.name, roots, mod.path_attr, mod.chain)
    resolved = next((c for c in candidates if c in known), None)
    if resolved is not None and is_rust_test_file(resolved):
        return []
    if resolved is None:
        reason = f"it does not resolve to a file (tried {', '.join(candidates)})"
    else:
        reason = (
            f"'{resolved}' is not a test file; name it 'tests.rs'/'test.rs' or place it in a "
            "'tests/' directory"
        )
    return [
        vl.Violation(
            "R2",
            path,
            mod.line,
            f"R2 {path} modfile {mod.name}",
            1,
            f"R2 test-only module '{mod.name}' must resolve to a test file, but {reason}",
        )
    ]


def evaluate(violations: list[vl.Violation], allow: dict[str, int]) -> Report:
    """Compares violations against the allowlist; returns the report."""
    report = Report(allowlist=allow)
    report.current = vl.aggregate(violations)
    for key, agg in sorted(
        report.current.items(), key=lambda kv: (kv[1].path, kv[1].first_line)
    ):
        if agg.rule == "R3" and key in allow:
            if agg.value > allow[key]:
                report.findings.append(
                    Finding(
                        agg.path,
                        agg.first_line,
                        "R3",
                        f"R3 {agg.value} occurrences of '{key}' exceed the {allow[key]} "
                        f"allowed in {ALLOWLIST_FILE}",
                    )
                )
            continue
        report.findings.append(Finding(agg.path, agg.first_line, agg.rule, agg.message))
    for key, allowed in sorted(allow.items()):
        agg = report.current.get(key)
        if agg is None or agg.value < allowed:
            now = 0 if agg is None else agg.value
            report.findings.append(
                Finding(
                    _key_path(key),
                    1,
                    "R3",
                    f"R3 stale allowlist entry '{key}' ({allowed} -> {now}); remove it from "
                    f"{ALLOWLIST_FILE}",
                )
            )
    return report


def _key_path(key: str) -> str:
    parts = key.split(" ", 2)
    return parts[1] if len(parts) > 1 else ALLOWLIST_FILE


def check(root: Path, files: list[str] | None = None) -> Report:
    """Runs the full policy check against the repository at `root`."""
    violations = collect_violations(root, files)
    return evaluate(violations, vl.load(root / ALLOWLIST_FILE))


def uncovered_seams(report: Report) -> list[str]:
    """Returns the allowlist lines that would cover the seams the allowlist does not cover yet.

    A seam that is already listed with a smaller count shows up with its full, raised count, so the
    line replaces the existing entry.
    """
    lines = [
        vl.render(key, agg.value)
        for key, agg in report.current.items()
        if agg.rule == "R3" and agg.value > report.allowlist.get(key, 0)
    ]
    return sorted(lines, key=lambda line: (line.split(" ", 2)[1], line))
