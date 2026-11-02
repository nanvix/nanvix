# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Violation keys and the allowlist file.

A violation is identified by a canonical *key* and carries a *value* (a count). Keys look like
`R2 src/a.rs mod tests`:

    R1 <path>                          test attributes outside test files
    R2 <path> mod <name>               inline test-gated module
    R2 <path> modfile <name>           test-gated `mod x;` that does not resolve to a test file
    R2 <path> file gate                file-level test gate in a non-test file
    R3 <path> <kind> <name>            test-only seam
    R4 <path>                          executable doc-test blocks
    R5 <path> <kind>                   test file outside its designated location
    R6 <path> <kind>                   end-to-end manifest outside `test/`

Only seams (R3) can be allowlisted. An allowlist line is the key, followed by `x<N>` when the seam
occurs N times.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path


class AllowlistError(Exception):
    """Raised when an allowlist file is malformed."""


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


def render(key: str, value: int) -> str:
    """Renders an allowlist entry."""
    return f"{key} x{value}" if value > 1 else key


_MULTIPLE = re.compile(r"^(.*\S) x(\d+)$")
_SEAM = re.compile(r"^R3 \S+ \S+")


def parse_line(line: str) -> tuple[str, int]:
    """Parses one allowlist line into its key and value."""
    if not _SEAM.match(line):
        raise AllowlistError(
            f"unrecognized entry: {line!r} (only `R3 <path> <kind> <name> [xN]` can be allowlisted)"
        )
    m = _MULTIPLE.match(line)
    if m:
        return m.group(1), int(m.group(2))
    return line, 1


def parse_text(text: str, label: str = "allowlist") -> dict[str, int]:
    """Parses the text of an allowlist file; `label` is used in error messages."""
    entries: dict[str, int] = {}
    for number, raw in enumerate(text.splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        try:
            key, value = parse_line(line)
        except AllowlistError as err:
            raise AllowlistError(f"{label}:{number}: {err}") from err
        if key in entries:
            raise AllowlistError(f"{label}:{number}: duplicate entry: {key!r}")
        entries[key] = value
    return entries


def load(path: Path) -> dict[str, int]:
    """Loads an allowlist file; a missing file is empty."""
    if not path.is_file():
        return {}
    return parse_text(path.read_text(encoding="utf-8"), str(path))
