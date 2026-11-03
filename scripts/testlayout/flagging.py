# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Detection of changes to existing tests in a pull request.

Adding a test is always fine. Changing, removing or renaming one is a different matter: a failing
test can be "fixed" by weakening it, so those changes deserve a careful review. The flagger compares
the *test items* (see `testitems`) of every changed file at the merge base and at the head of the
pull request:

- an item that still exists unchanged, in the same file or in any other changed file, is fine: new
  tests, reordering, moving tests between files and comment or formatting edits are not flagged;
- an item that is gone while its name lives on in the same file with different content was
  *modified*;
- an item that is gone for good was *removed* (or *deleted* together with its file);
- renaming a test file is flagged, and so is growing the layout allowlist.

The result is a heuristic safety net for reviewers, not a security boundary.
"""

from __future__ import annotations

import posixpath
from collections import Counter, defaultdict
from dataclasses import asdict, dataclass, field
from pathlib import Path

from . import gitdiff, paths, violations
from .paths import ALLOWLIST_FILE
from .testitems import (
    RUST,
    ExtractError,
    Item,
    extract,
    is_test_file,
    language,
    rust_module_refs,
)

# Hidden marker that identifies the sticky pull request comment.
MARKER = "<!-- test-change-review -->"

# Findings listed in the pull request comment before the rest is summarized.
DEFAULT_COMMENT_LIMIT = 40

# Larger files are not analyzed; they are reported as unparsed instead.
MAX_FILE_BYTES = 2 * 1024 * 1024

MODIFIED = "modified"
REMOVED = "removed"
DELETED = "deleted"
RENAMED = "renamed"
POLICY_GROWTH = "policy-growth"
UNPARSED = "unparsed"

_DETAILS = {
    MODIFIED: "existing test code was changed",
    REMOVED: "existing test code was removed or renamed",
    DELETED: "test file was deleted",
    RENAMED: "test file was renamed",
    POLICY_GROWTH: "the layout exceptions grew",
    UNPARSED: "the file could not be analyzed",
}


@dataclass
class Finding:
    """One change to an existing test."""

    kind: str
    path: str
    name: str
    base_line: int = 0
    head_line: int = 0
    detail: str = ""
    old_path: str = ""


@dataclass
class Report:
    """The outcome of comparing the head of a pull request with its merge base."""

    base: str
    head: str
    merge_base: str
    files_scanned: int = 0
    findings: list[Finding] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)

    @property
    def flagged(self) -> bool:
        """Tells whether the pull request changes any existing test."""
        return bool(self.findings)

    def to_json(self) -> dict[str, object]:
        """Returns the report as a JSON-serializable dictionary."""
        return {
            "flagged": self.flagged,
            "base": self.base,
            "head": self.head,
            "merge_base": self.merge_base,
            "files_scanned": self.files_scanned,
            "findings": [asdict(f) for f in self.findings],
            "notes": list(self.notes),
        }


@dataclass
class _Pair:
    """The test items of one changed file at both revisions."""

    change: gitdiff.Change
    base_items: list[Item]
    head_items: list[Item]
    # False when one of the revisions could not be analyzed.
    comparable: bool = True


def analyze(root: Path, base: str, head: str) -> Report:
    """Compares the tests of `head` with those of its merge base with `base`."""
    base_sha = gitdiff.resolve(root, base)
    head_sha = gitdiff.resolve(root, head)
    merge = gitdiff.merge_base(root, base_sha, head_sha)
    report = Report(base=base, head=head, merge_base=merge)
    changes = gitdiff.changed_files(root, merge, head_sha)
    with gitdiff.BlobReader(root) as blobs:
        pairs = _load_pairs(root, blobs, merge, head_sha, changes, report)
        _check_policy_files(blobs, merge, head_sha, changes, report)
    _match(pairs, report)
    report.findings.sort(key=lambda f: (f.path, f.base_line, f.head_line, f.name))
    return report


def _extract(path: str, data: bytes, context: tuple[str, ...] = ()) -> list[Item]:
    if len(data) > MAX_FILE_BYTES:
        raise ExtractError(f"file is larger than {MAX_FILE_BYTES} bytes")
    return extract(path, data.decode("utf-8", "replace"), context)


def _try_extract(
    path: str, data: bytes | None, context: tuple[str, ...] = ()
) -> tuple[list[Item], str]:
    """Extracts the items of one revision of a file; returns them and an error message."""
    if data is None:
        return [], ""
    try:
        return _extract(path, data, context), ""
    except ExtractError as err:
        return [], str(err)


def _module_contexts(
    root: Path, blobs: gitdiff.BlobReader, revision: str, sources: dict[str, bytes]
) -> tuple[dict[str, tuple[str, ...]], dict[str, str]]:
    """Follows changed module declarations so unchanged moves retain their guards."""
    rust = {path: data for path, data in sources.items() if language(path) == RUST}
    manifests: set[str] = set()
    for path in rust:
        directory = posixpath.dirname(path)
        while True:
            manifests.add(posixpath.join(directory, "Cargo.toml"))
            if not directory:
                break
            directory = posixpath.dirname(directory)
    texts: dict[str, str] = {}
    for manifest in sorted(manifests):
        data = blobs.read(revision, manifest)
        if data is not None:
            texts[manifest] = data.decode("utf-8", "replace")
    roots = paths.crate_roots(root, sorted({*rust, *texts}), texts.__getitem__)
    parents: dict[str, list[tuple[str, tuple[str, ...]]]] = defaultdict(list)
    errors: dict[str, str] = {}
    for path, data in rust.items():
        try:
            if len(data) > MAX_FILE_BYTES:
                raise ExtractError(f"file is larger than {MAX_FILE_BYTES} bytes")
            for module in rust_module_refs(data.decode("utf-8", "replace")):
                candidates = paths.module_candidates(
                    path, module.name, roots, module.path, module.chain
                )
                found = [candidate for candidate in candidates if candidate in rust]
                if len(found) > 1:
                    raise ExtractError(f"ambiguous module '{module.name}'")
                for child in found:
                    parents[child].append((path, module.context))
        except ExtractError as err:
            errors[path] = str(err)

    contexts: dict[str, tuple[str, ...]] = {}

    def context_for(path: str, visiting: frozenset[str]) -> tuple[str, ...]:
        if path in errors:
            raise ExtractError(errors[path])
        if path in contexts:
            return contexts[path]
        if path in visiting:
            raise ExtractError(f"cyclic module reference to '{path}'")
        options = {
            context_for(parent, visiting | {path}) + context
            for parent, context in parents[path]
        }
        if len(options) > 1:
            raise ExtractError(f"multiple module guard contexts for '{path}'")
        contexts[path] = next(iter(options), ())
        return contexts[path]

    for path in rust:
        try:
            context_for(path, frozenset())
        except ExtractError as err:
            errors[path] = str(err)
    return contexts, errors


def _load_pairs(
    root: Path,
    blobs: gitdiff.BlobReader,
    base: str,
    head: str,
    changes: list[gitdiff.Change],
    report: Report,
) -> list[_Pair]:
    pairs: list[_Pair] = []
    loaded: list[tuple[gitdiff.Change, bytes | None, bytes | None]] = []
    base_sources: dict[str, bytes] = {}
    head_sources: dict[str, bytes] = {}
    for change in changes:
        if language(change.old_path) is None and language(change.path) is None:
            continue
        report.files_scanned += 1
        if change.status == "R" and is_test_file(change.old_path):
            report.findings.append(
                Finding(
                    RENAMED,
                    change.path,
                    f"from {change.old_path}",
                    detail=_DETAILS[RENAMED],
                    old_path=change.old_path,
                )
            )
        old = blobs.read(base, change.old_path) if change.status != "A" else None
        new = blobs.read(head, change.path) if change.status != "D" else None
        if old is not None and old == new:
            continue
        loaded.append((change, old, new))
        if old is not None:
            base_sources[change.old_path] = old
        if new is not None:
            head_sources[change.path] = new
    base_contexts, base_errors = _module_contexts(root, blobs, base, base_sources)
    head_contexts, head_errors = _module_contexts(root, blobs, head, head_sources)
    for change, old, new in loaded:
        base_items, base_error = _try_extract(
            change.old_path, old, base_contexts.get(change.old_path, ())
        )
        head_items, head_error = _try_extract(
            change.path, new, head_contexts.get(change.path, ())
        )
        base_error = base_error or base_errors.get(change.old_path, "")
        head_error = head_error or head_errors.get(change.path, "")
        error = base_error or head_error
        if error and change.status != "A":
            report.findings.append(
                Finding(
                    UNPARSED,
                    change.path,
                    change.path,
                    detail=f"{_DETAILS[UNPARSED]}: {error}",
                    old_path=change.old_path if change.status == "R" else "",
                )
            )
        elif error:
            report.notes.append(f"{change.path}: {error}")
        pairs.append(_Pair(change, base_items, head_items, comparable=not error))
    return pairs


def _subtract(items: list[Item], others: list[Item]) -> list[Item]:
    """Returns the items that remain after removing one match per item of `others`."""
    budget = Counter(item.fingerprint for item in others)
    left: list[Item] = []
    for item in items:
        if budget[item.fingerprint] > 0:
            budget[item.fingerprint] -= 1
        else:
            left.append(item)
    return left


def _match(pairs: list[_Pair], report: Report) -> None:
    """Pairs the base items that changed with their new versions, or reports them as gone."""
    pool: Counter[str] = Counter()
    orphans: list[tuple[_Pair, Item]] = []
    for pair in pairs:
        if not pair.comparable:
            pool.update(item.fingerprint for item in pair.head_items)
            continue
        base_left = _subtract(pair.base_items, pair.head_items)
        head_left = _subtract(pair.head_items, pair.base_items)
        by_name: dict[str, list[int]] = defaultdict(list)
        for index, item in enumerate(head_left):
            by_name[item.name].append(index)
        consumed: set[int] = set()
        for item in base_left:
            candidates = by_name.get(item.name)
            if not candidates:
                orphans.append((pair, item))
                continue
            index = candidates.pop(0)
            consumed.add(index)
            report.findings.append(
                Finding(
                    MODIFIED,
                    pair.change.path,
                    item.name,
                    item.line,
                    head_left[index].line,
                    _DETAILS[MODIFIED],
                    pair.change.old_path if pair.change.status == "R" else "",
                )
            )
        pool.update(
            item.fingerprint
            for index, item in enumerate(head_left)
            if index not in consumed
        )
    # An item that left its file is fine if it shows up unchanged in another changed file.
    for pair, item in orphans:
        if pool[item.fingerprint] > 0:
            pool[item.fingerprint] -= 1
            continue
        kind = DELETED if pair.change.status == "D" else REMOVED
        report.findings.append(
            Finding(
                kind,
                pair.change.path,
                item.name,
                item.line,
                0,
                _DETAILS[kind],
                pair.change.old_path if pair.change.status == "R" else "",
            )
        )


def _check_policy_files(
    blobs: gitdiff.BlobReader,
    base: str,
    head: str,
    changes: list[gitdiff.Change],
    report: Report,
) -> None:
    """Flags new or raised entries of the layout allowlist."""
    if ALLOWLIST_FILE not in {change.path for change in changes}:
        return
    old = blobs.read(base, ALLOWLIST_FILE)
    new = blobs.read(head, ALLOWLIST_FILE)
    if new is None or old == new:
        return
    try:
        before = violations.parse_text(
            old.decode("utf-8", "replace") if old else "", f"{ALLOWLIST_FILE} (base)"
        )
        after = violations.parse_text(new.decode("utf-8", "replace"), ALLOWLIST_FILE)
    except violations.AllowlistError as err:
        report.findings.append(
            Finding(
                UNPARSED,
                ALLOWLIST_FILE,
                ALLOWLIST_FILE,
                detail=f"{_DETAILS[UNPARSED]}: {err}",
            )
        )
        return
    for key, value in after.items():
        previous = before.get(key)
        if previous is None:
            detail = f"entry added with a value of {value}"
        elif value > previous:
            detail = f"entry raised from {previous} to {value}"
        else:
            continue
        report.findings.append(
            Finding(POLICY_GROWTH, ALLOWLIST_FILE, key, detail=detail)
        )


# ---------------------------------------------------------------------------------------------
# Rendering
# ---------------------------------------------------------------------------------------------


def _where(finding: Finding) -> str:
    if finding.head_line:
        return f"{finding.path}:{finding.head_line}"
    if finding.base_line:
        return f"{finding.path}:{finding.base_line} (base)"
    return finding.path


def render_text(report: Report) -> str:
    """Renders the report as plain text for terminals and logs."""
    lines = [f"{_where(f)}: {f.kind}: {f.name} - {f.detail}" for f in report.findings]
    lines.extend(f"note: {note}" for note in report.notes)
    if report.flagged:
        lines.append(
            f"{len(report.findings)} change(s) to existing tests in {report.files_scanned} "
            "changed test file(s)."
        )
    else:
        lines.append(
            f"No existing tests were changed ({report.files_scanned} changed test file(s) "
            "checked)."
        )
    return "\n".join(lines) + "\n"


def _code(text: str, limit: int = 100) -> str:
    """Formats untrusted text as a Markdown code span that fits in a table cell."""
    text = " ".join(text.replace("`", "'").split())
    if len(text) > limit:
        text = text[: limit - 1] + "…"
    return f"`{text.replace('|', chr(92) + '|')}`"


def render_markdown(
    report: Report, limit: int | None = DEFAULT_COMMENT_LIMIT, marker: bool = True
) -> str:
    """Renders the report as Markdown for the pull request comment and the job summary.

    Findings derive from untrusted pull request content, so they are only ever emitted inside code
    spans.
    """
    out: list[str] = [MARKER, ""] if marker else []
    if not report.flagged:
        out.append("### ✅ No existing tests were changed")
        out.append("")
        out.append("This pull request only adds tests or leaves them untouched.")
        return "\n".join(out) + "\n"
    count = len(report.findings)
    out.append("### ⚠️ Existing tests were changed: careful review requested")
    out.append("")
    out.append(
        f"This pull request changes **{count}** piece(s) of existing test code. A failing test "
        "can be made to pass by weakening it, so please check that each change below is "
        "intended and that no test lost coverage. Adding new tests is never flagged."
    )
    out.append("")
    out.append("| Change | File | Item | Line |")
    out.append("|---|---|---|---|")
    shown = report.findings if limit is None else report.findings[:limit]
    for finding in shown:
        line = ""
        if finding.head_line:
            line = str(finding.head_line)
        elif finding.base_line:
            line = f"{finding.base_line} (base)"
        out.append(
            f"| {finding.kind} | {_code(finding.path, 80)} | "
            f"{_code(finding.name, 60)} | {line} |"
        )
    if len(shown) < count:
        out.append("")
        out.append(
            f"…and {count - len(shown)} more. See the job summary of the "
            "`Test Change Review` workflow run for the full list."
        )
    if report.notes:
        out.append("")
        out.append("Files that could not be analyzed:")
        out.extend(f"- {_code(note, 150)}" for note in report.notes[:10])
    out.append("")
    out.append(
        "<details><summary>How this check works</summary>\n\n"
        "Every test is fingerprinted from its content, ignoring comments, formatting and "
        "position. A test that disappears, or whose name lives on with a different content, is "
        "flagged; moving a test to another file is not. Growing the test layout allowlist is "
        "flagged too. See `doc/test.md` for the details, and run "
        "`python3 scripts/flag-test-changes.py --base <base> --head <head>` to reproduce "
        "this report locally.\n\n</details>"
    )
    return "\n".join(out) + "\n"
