# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Violation keys and the ratchet baseline / allowlist files.

A violation is identified by a canonical *key* and carries a *value* (a count). Keys look like
`R2 src/a.rs mod tests`; the on-disk line format appends the value when it is meaningful:

    R1 <path> <attributes>             test attributes outside test files
    R2 <path> mod <name> tests=<N>     inline test-gated module holding N tests
    R2 <path> modfile <name>           test-gated `mod x;` that does not resolve to a test file
    R2 <path> file gate                file-level test gate in a non-test file
    R3 <path> <kind> <name> [x<N>]     test-only seam (N occurrences when more than one)
    R4 <path> <blocks>                 executable doc-test blocks
    R5 <path> <kind>                   test file outside its designated location
    R6 <path> <kind>                   end-to-end manifest outside `test/`
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path


class BaselineError(Exception):
    """Raised when a baseline or allowlist file is malformed."""


@dataclass(frozen=True)
class Violation:
    """One violation of the layout policy."""

    rule: str
    path: str
    line: int
    key: str
    value: int
    message: str


@dataclass
class Aggregate:
    """All violations that share a key."""

    key: str
    value: int = 0
    first_line: int = 0
    message: str = ""
    path: str = ""
    rule: str = ""
    lines: list[int] = field(default_factory=list)


def aggregate(violations: list[Violation]) -> dict[str, Aggregate]:
    """Groups violations by key, summing their values."""
    out: dict[str, Aggregate] = {}
    for v in violations:
        agg = out.get(v.key)
        if agg is None:
            agg = out[v.key] = Aggregate(
                key=v.key,
                first_line=v.line,
                message=v.message,
                path=v.path,
                rule=v.rule,
            )
        agg.value += v.value
        agg.lines.append(v.line)
    return out


def _is_counted_key(key: str) -> bool:
    """Tells whether the value of a key is a count that is rendered explicitly."""
    return key.startswith(("R1 ", "R4 "))


def render(key: str, value: int) -> str:
    """Renders a key and its value as a baseline line."""
    if _is_counted_key(key):
        return f"{key} {value}"
    parts = key.split(" ", 3)
    if parts[0] == "R2" and len(parts) > 3 and parts[2] == "mod":
        return f"{key} tests={value}"
    return f"{key} x{value}" if value > 1 else key


_R2_MOD = re.compile(r"^(R2 \S+ mod \S+) tests=(\d+)$")
_COUNTED = re.compile(r"^((?:R1|R4) \S+) (\d+)$")
_MULTIPLE = re.compile(r"^(.*\S) x(\d+)$")
_KEYED = re.compile(r"^R[1-6] \S+ \S+")


def parse_line(line: str) -> tuple[str, int]:
    """Parses one baseline line into its key and value."""
    m = _R2_MOD.match(line)
    if m:
        return m.group(1), int(m.group(2))
    m = _COUNTED.match(line)
    if m:
        return m.group(1), int(m.group(2))
    if not _KEYED.match(line):
        raise BaselineError(f"unrecognized entry: {line!r}")
    m = _MULTIPLE.match(line)
    if m and not line.startswith(("R1 ", "R4 ")):
        return m.group(1), int(m.group(2))
    return line, 1


def load(path: Path) -> dict[str, int]:
    """Loads a baseline or allowlist file; a missing file is empty."""
    entries: dict[str, int] = {}
    if not path.is_file():
        return entries
    for number, raw in enumerate(
        path.read_text(encoding="utf-8").splitlines(), start=1
    ):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        try:
            key, value = parse_line(line)
        except BaselineError as err:
            raise BaselineError(f"{path}:{number}: {err}") from err
        if key in entries:
            raise BaselineError(f"{path}:{number}: duplicate entry: {key!r}")
        entries[key] = value
    return entries


def _sort_key(line: str) -> tuple[str, str]:
    return line.split(" ", 2)[1], line


def dump(entries: dict[str, int], header: list[str]) -> str:
    """Serializes entries sorted by path, one per line, below a comment header."""
    lines = sorted((render(k, v) for k, v in entries.items()), key=_sort_key)
    out = [f"# {h}".rstrip() for h in header]
    out.extend(lines)
    return "\n".join(out) + "\n"
