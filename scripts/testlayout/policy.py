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

A ratchet baseline records the violations that exist today. New violations, grown counts and stale
baseline entries all fail the check, so the baseline can only shrink.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

from . import baseline as bl
from .cfgexpr import CfgParseError
from .paths import (
    ALLOWLIST_FILE,
    BASELINE_FILE,
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

MIGRATE_HINT = "scripts/migrate-inline-tests.py"
DOC_HINT = "see doc/test.md ('Test layout')"

BASELINE_HEADER = [
    "Ratchet baseline for scripts/check-test-layout.py.",
    "",
    "Lists test code that still lives in mainline source. Entries may only be removed: fix the",
    "code (scripts/migrate-inline-tests.py), then run",
    "`scripts/check-test-layout.py --update-baseline`.",
    "Never add an entry; put new tests in their own files instead (see doc/test.md).",
]


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
    current: dict[str, bl.Aggregate] = field(default_factory=dict)
    baseline: dict[str, int] = field(default_factory=dict)
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
) -> list[bl.Violation]:
    """Scans the repository and returns every policy violation (before baseline filtering)."""
    names = files if files is not None else list_repo_files(root)
    known = set(names)
    roots = crate_roots(root, names)
    out: list[bl.Violation] = []
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


def _misplaced(rule: str, path: str, kind: str, message: str) -> bl.Violation:
    return bl.Violation(rule, path, 1, f"{rule} {path} {kind}", 1, f"{rule} {message}")


def _check_python(root: Path, path: str) -> list[bl.Violation]:
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
) -> list[bl.Violation]:
    try:
        text = (root / path).read_text(encoding="utf-8")
        result = scan_rust(text)
    except (OSError, UnicodeDecodeError) as err:
        return [
            bl.Violation(
                "R0", path, 1, f"R0 {path} read", 1, f"R0 cannot read file: {err}"
            )
        ]
    except (LexError, CfgParseError) as err:
        return [
            bl.Violation(
                "R0", path, 1, f"R0 {path} parse", 1, f"R0 cannot parse file: {err}"
            )
        ]
    out: list[bl.Violation] = []
    if result.file_gate_line:
        out.append(
            bl.Violation(
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
            bl.Violation(
                "R3",
                path,
                seam.line,
                f"R3 {path} {seam.kind} {seam.name}",
                1,
                f"R3 test-only {seam.kind} '{seam.name}' in mainline source; move it into the "
                f"child test module or list it in {ALLOWLIST_FILE}",
            )
        )
    if result.test_attr_lines:
        out.extend(
            bl.Violation(
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
        bl.Violation(
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
) -> list[bl.Violation]:
    if mod.inline:
        target = expected_module_file(path, mod.name, roots, mod.chain)
        return [
            bl.Violation(
                "R2",
                path,
                mod.line,
                f"R2 {path} mod {mod.name}",
                mod.tests,
                f"R2 inline test module '{mod.name}' ({mod.tests} tests); move it to {target} "
                f"with {MIGRATE_HINT} {path}",
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
        bl.Violation(
            "R2",
            path,
            mod.line,
            f"R2 {path} modfile {mod.name}",
            1,
            f"R2 test-only module '{mod.name}' must resolve to a test file, but {reason}",
        )
    ]


def evaluate(
    violations: list[bl.Violation], baseline: dict[str, int], allow: dict[str, int]
):
    """Compares violations against the baseline and allowlist; returns the report."""
    report = Report(baseline=baseline, allowlist=allow)
    report.current = bl.aggregate(violations)
    ratchet_hint = "run `python scripts/check-test-layout.py --update-baseline`"
    for key, agg in sorted(
        report.current.items(), key=lambda kv: (kv[1].path, kv[1].first_line)
    ):
        if agg.rule == "R0":
            report.findings.append(Finding(agg.path, agg.first_line, "R0", agg.message))
            continue
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
        allowed = baseline.get(key)
        if allowed is None:
            report.findings.append(
                Finding(agg.path, agg.first_line, agg.rule, agg.message)
            )
        elif agg.value > allowed:
            report.findings.append(
                Finding(
                    agg.path,
                    agg.first_line,
                    agg.rule,
                    f"{agg.rule} count grew from {allowed} to {agg.value} for '{key}': "
                    f"{agg.message[3:]}",
                )
            )
    for key, allowed in sorted(baseline.items()):
        agg = report.current.get(key)
        if key in allow and key.startswith("R3 "):
            report.findings.append(
                Finding(
                    _key_path(key),
                    1,
                    "R3",
                    f"R3 '{key}' is both baselined and allowlisted; remove it from "
                    f"{BASELINE_FILE} ({ratchet_hint})",
                )
            )
        elif agg is None or agg.value < allowed:
            now = 0 if agg is None else agg.value
            report.findings.append(
                Finding(
                    _key_path(key),
                    1,
                    key.split(" ", 1)[0],
                    f"{key.split(' ', 1)[0]} stale baseline entry '{key}' ({allowed} -> {now}): "
                    f"the violation was fixed; {ratchet_hint} to drop it from {BASELINE_FILE}",
                )
            )
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
    return parts[1] if len(parts) > 1 else BASELINE_FILE


def check(root: Path, files: list[str] | None = None) -> Report:
    """Runs the full policy check against the repository at `root`."""
    violations = collect_violations(root, files)
    baseline = bl.load(root / BASELINE_FILE)
    allow = bl.load(root / ALLOWLIST_FILE)
    return evaluate(violations, baseline, allow)


def shrink_baseline(report: Report) -> dict[str, int]:
    """Returns the baseline with fixed entries removed and shrunken counts lowered.

    Entries are never added or raised, which keeps the ratchet one-directional.
    """
    out: dict[str, int] = {}
    for key, allowed in report.baseline.items():
        if key in report.allowlist and key.startswith("R3 "):
            continue
        agg = report.current.get(key)
        if agg is None:
            continue
        out[key] = min(allowed, agg.value)
    return out


def write_baseline(root: Path, entries: dict[str, int]) -> None:
    """Writes the baseline file (UTF-8, LF line endings)."""
    with open(root / BASELINE_FILE, "w", encoding="utf-8", newline="\n") as handle:
        handle.write(bl.dump(entries, BASELINE_HEADER))


def shrink_baseline_file(root: Path) -> tuple[int, int]:
    """Drops fixed entries from the baseline file; returns the entry counts before and after."""
    report = check(root)
    shrunk = shrink_baseline(report)
    if shrunk != report.baseline:
        write_baseline(root, shrunk)
    return len(report.baseline), len(shrunk)
