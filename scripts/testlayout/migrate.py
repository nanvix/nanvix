# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Moves inline test modules out of mainline Rust source into their own files.

A mainline file such as

    #[cfg(all(test, feature = "std"))]
    mod tests {
        use super::*;
        ...
    }

becomes a declaration `#[cfg(all(test, feature = "std"))] mod tests;` next to a new `tests.rs`
holding the (dedented) body. The module keeps its name and its position in the module tree, so
test identifiers do not change. A module whose name does not mark it as a test file (for example
`ikc_tests` is fine but `errno_shim` is not) is written to a `tests/` directory and loaded with
`#[path = "..."]`.

Modules that cannot be moved mechanically are reported and left untouched.
"""

from __future__ import annotations

import posixpath
import re
import shutil
import subprocess
from dataclasses import dataclass, field
from pathlib import Path

from .cfgexpr import CfgParseError
from .paths import (
    crate_roots,
    is_rust_test_file,
    list_repo_files,
    module_directory,
    to_posix,
)
from .rustlex import IDENT, STRING, LexError, Tokens, normalized_tokens, tokenize
from .scan import GatedMod, scan_rust

LICENSE_HEADER = (
    "// Copyright(c) The Maintainers of Nanvix.\n// Licensed under the MIT License.\n"
)

# Macros whose result depends on where the file lives.
_LOCATION_MACROS = frozenset(
    {"include", "include_str", "include_bytes", "file", "line", "column"}
)

_FORMAT_CHUNK = 40


@dataclass
class Move:
    """One inline module that is moved to its own file."""

    name: str
    line: int
    target: str
    path_attr: str | None
    source: str


@dataclass
class Skip:
    """One inline module that was left in place."""

    name: str
    line: int
    reason: str


@dataclass
class FilePlan:
    """The migration of one mainline file."""

    path: str
    text: str
    moves: list[Move] = field(default_factory=list)
    skips: list[Skip] = field(default_factory=list)


def _protected_lines(body: str, toks: Tokens) -> set[int]:
    """Returns the indexes of body lines that start inside a multi-line string literal."""
    lines: set[int] = set()
    for i in range(len(toks)):
        if toks.kinds[i] != STRING:
            continue
        first = body.count("\n", 0, toks.starts[i])
        inner = body.count("\n", toks.starts[i], toks.ends[i])
        lines.update(range(first + 1, first + inner + 1))
    return lines


def dedent_body(body: str, toks: Tokens) -> str:
    """Removes one indentation level (4 spaces) without touching multi-line string contents."""
    protected = _protected_lines(body, toks)
    out: list[str] = []
    for index, line in enumerate(body.split("\n")):
        if index in protected:
            out.append(line)
        elif not line.strip():
            out.append("")
        elif line.startswith("    "):
            out.append(line[4:])
        else:
            out.append(line)
    return "\n".join(out)


def _body_problem(body: str, needs_path: bool) -> str | None:
    """Returns why a module body cannot be moved mechanically, if so."""
    try:
        toks = tokenize(body)
    except LexError as err:
        return f"its body cannot be tokenized ({err})"
    for i in range(len(toks)):
        if toks.kinds[i] != IDENT:
            continue
        word = toks.src(i)
        is_call = toks.is_punct(i + 1, "!") and any(
            toks.is_punct(i + 2, c) for c in "([{"
        )
        if word in _LOCATION_MACROS and is_call:
            return f"it uses `{word}!`, which depends on the location of the file"
        if (
            word == "path"
            and toks.is_punct(i - 1, "[")
            and toks.is_punct(i - 2, "#")
            and toks.is_punct(i + 1, "=")
        ):
            return "it contains a #[path] attribute, which is relative to the file"
        if (
            needs_path
            and word == "mod"
            and toks.kinds[i + 1 : i + 2] == [IDENT]
            and toks.is_punct(i + 2, ";")
        ):
            return "it declares child modules, which would resolve differently from a #[path] file"
    return None


def _target_for(path: str, mod: GatedMod, roots: set[str]) -> tuple[str, str | None]:
    """Returns the file a module moves to and the `#[path]` value needed to load it, if any."""
    directory = module_directory(path, roots)
    target = posixpath.join(directory, f"{mod.name}.rs")
    if is_rust_test_file(target):
        return target, None
    target = posixpath.join(directory, "tests", f"{mod.name}.rs")
    return target, posixpath.relpath(target, posixpath.dirname(path))


def _line_prefix(text: str, offset: int) -> str:
    return text[text.rfind("\n", 0, offset) + 1 : offset]


def plan_file(
    root: Path,
    path: str,
    text: str,
    known: set[str],
    roots: set[str],
    reserved: set[str] | None = None,
    allow_existing: bool = False,
) -> FilePlan:
    """Plans the migration of the inline test modules of one file.

    With `allow_existing`, target files that already exist are not a conflict; this is how a
    finished migration is compared against its original.
    """
    plan = FilePlan(path=path, text=text)
    taken = reserved if reserved is not None else set()
    mods = [m for m in scan_rust(text).mods if m.inline]
    for mod in sorted(mods, key=lambda m: m.start_off, reverse=True):
        problem = _problem_for(
            root, path, text, mod, known, roots, taken, allow_existing
        )
        if isinstance(problem, str):
            plan.skips.append(Skip(mod.name, mod.line, problem))
            continue
        target, path_attr = problem
        body = text[mod.body_open + 1 : mod.body_close]
        source = LICENSE_HEADER + "\n" + _render_body(body)
        taken.add(target)
        plan.moves.append(Move(mod.name, mod.line, target, path_attr, source))
        plan.text = _replace_block(plan.text, mod, path_attr)
    plan.moves.reverse()
    plan.skips.reverse()
    return plan


def _problem_for(
    root: Path,
    path: str,
    text: str,
    mod: GatedMod,
    known: set[str],
    roots: set[str],
    taken: set[str],
    allow_existing: bool,
) -> str | tuple[str, str | None]:
    """Returns a skip reason, or the (target, path attribute) pair when the module can move."""
    if mod.chain:
        return (
            f"it is nested in inline module '{'::'.join(mod.chain)}'; move it by hand"
        )
    if _line_prefix(text, mod.start_off):
        return "it is not declared at the start of a top-level line; move it by hand"
    if mod.path_attr is not None:
        return "it already has a #[path] attribute; move it by hand"
    target, path_attr = _target_for(path, mod, roots)
    exists = target in known or (root / target).exists()
    if target in taken or (exists and not allow_existing):
        return f"the target file {target} already exists"
    body = text[mod.body_open + 1 : mod.body_close]
    problem = _body_problem(body, path_attr is not None)
    if problem is not None:
        return problem
    return target, path_attr


def _render_body(body: str) -> str:
    """Returns the content of the new file after the license header."""
    toks = tokenize(body)
    lines = dedent_body(body, toks).split("\n")
    while lines and not lines[0].strip():
        lines.pop(0)
    while lines and not lines[-1].strip():
        lines.pop()
    return "\n".join(lines) + "\n" if lines else ""


def _replace_block(text: str, mod: GatedMod, path_attr: str | None) -> str:
    """Replaces `#[cfg(..)] mod name { .. }` with the out-of-line declaration."""
    attrs = text[mod.start_off : mod.unit_off]
    header = text[mod.unit_off : mod.body_open].rstrip()
    if path_attr is not None:
        separator = attrs[len(attrs.rstrip()) :] or " "
        attrs += f'#[path = "{path_attr}"]{separator}'
    return text[: mod.start_off] + attrs + header + ";" + text[mod.body_close + 1 :]


_EDITION = re.compile(r'^edition\s*=\s*"(\d+)"', re.MULTILINE)


def _declared_edition(manifest: Path) -> str | None:
    match = _EDITION.search(manifest.read_text(encoding="utf-8"))
    return match.group(1) if match else None


def rust_edition(root: Path, path: str) -> str:
    """Returns the edition of the crate owning `path`, as `cargo fmt` would use."""
    directory = posixpath.dirname(path)
    while directory and not (root / directory / "Cargo.toml").is_file():
        directory = posixpath.dirname(directory)
    manifest = root / directory / "Cargo.toml"
    edition = _declared_edition(manifest) if manifest.is_file() else None
    if edition is None and directory:
        edition = (
            _declared_edition(root / "Cargo.toml")
            if (root / "Cargo.toml").is_file()
            else None
        )
    return edition or "2021"


def format_files(root: Path, files: list[str]) -> bool:
    """Formats files with the repository's rustfmt configuration; returns False if unavailable."""
    if not files:
        return True
    if shutil.which("rustfmt") is None:
        return False
    by_edition: dict[str, list[str]] = {}
    for name in files:
        by_edition.setdefault(rust_edition(root, name), []).append(name)
    for edition, names in sorted(by_edition.items()):
        for first in range(0, len(names), _FORMAT_CHUNK):
            subprocess.run(
                [
                    "rustfmt",
                    "--edition",
                    edition,
                    "--config-path",
                    str(root / "rustfmt.toml"),
                    "--config",
                    "skip_children=true",
                    *names[first : first + _FORMAT_CHUNK],
                ],
                cwd=root,
                check=True,
            )
    return True


def select_files(root: Path, targets: list[str], files: list[str]) -> list[str]:
    """Returns the mainline Rust files under the given repository-relative paths."""
    prefixes = []
    for target in targets:
        candidate = Path(target)
        if candidate.is_absolute():
            candidate = candidate.resolve().relative_to(root)
        prefixes.append(to_posix(str(candidate)).strip("/"))
    selected = []
    for name in files:
        if not name.endswith(".rs") or is_rust_test_file(name):
            continue
        if any(
            p in ("", ".") or name == p or name.startswith(p + "/") for p in prefixes
        ):
            selected.append(name)
    return selected


@dataclass
class Result:
    """Outcome of one migration run."""

    plans: list[FilePlan] = field(default_factory=list)
    unformatted: bool = False

    @property
    def moved(self) -> int:
        """Number of modules moved."""
        return sum(len(p.moves) for p in self.plans)

    @property
    def skipped(self) -> int:
        """Number of modules left in place."""
        return sum(len(p.skips) for p in self.plans)


def migrate(
    root: Path,
    targets: list[str],
    apply: bool = True,
    format_output: bool = True,
) -> Result:
    """Migrates the inline test modules found under `targets`."""
    files = list_repo_files(root)
    known = set(files)
    roots = crate_roots(root, files)
    reserved: set[str] = set()
    result = Result()
    for path in select_files(root, targets, files):
        text = (root / path).read_text(encoding="utf-8")
        try:
            plan = plan_file(root, path, text, known, roots, reserved)
        except (LexError, CfgParseError) as err:
            plan = FilePlan(
                path=path, text=text, skips=[Skip("*", 1, f"cannot parse: {err}")]
            )
        if plan.moves or plan.skips:
            result.plans.append(plan)
    if not apply:
        return result
    written: list[str] = []
    for plan in result.plans:
        if not plan.moves:
            continue
        _write(root / plan.path, plan.text)
        written.append(plan.path)
        for move in plan.moves:
            (root / move.target).parent.mkdir(parents=True, exist_ok=True)
            _write(root / move.target, move.source)
            written.append(move.target)
    if format_output:
        result.unformatted = not format_files(root, written)
    return result


def _write(path: Path, content: str) -> None:
    with open(path, "w", encoding="utf-8", newline="\n") as handle:
        handle.write(content)


# ---------------------------------------------------------------------------------------------
# Verification against a revision
# ---------------------------------------------------------------------------------------------


def _git_show(root: Path, rev: str, path: str) -> str | None:
    proc = subprocess.run(
        ["git", "show", f"{rev}:{path}"], cwd=root, capture_output=True, check=False
    )
    if proc.returncode != 0:
        return None
    return proc.stdout.decode("utf-8").replace("\r\n", "\n")


def _changed_files(root: Path, rev: str) -> list[str]:
    tracked = subprocess.run(
        ["git", "diff", "--name-only", "-z", rev],
        cwd=root,
        capture_output=True,
        check=True,
    ).stdout.decode("utf-8")
    return sorted({to_posix(n) for n in tracked.split("\0") if n.endswith(".rs")})


def verify(root: Path, rev: str) -> list[str]:
    """Checks that the working tree is a mechanical migration of `rev`; returns the problems."""
    problems: list[str] = []
    roots = crate_roots(root, list_repo_files(root))
    for path in _changed_files(root, rev):
        if is_rust_test_file(path) or not (root / path).is_file():
            continue
        old = _git_show(root, rev, path)
        if old is None:
            continue
        try:
            plan = plan_file(root, path, old, set(), roots, allow_existing=True)
        except (LexError, CfgParseError) as err:
            problems.append(f"{path}: cannot parse the original: {err}")
            continue
        current = (root / path).read_text(encoding="utf-8")
        if normalized_tokens(current) != normalized_tokens(plan.text):
            problems.append(f"{path}: differs from the mechanical migration of {rev}")
        for move in plan.moves:
            target = root / move.target
            if not target.is_file():
                problems.append(
                    f"{move.target}: missing (expected from {path}::{move.name})"
                )
            elif normalized_tokens(
                target.read_text(encoding="utf-8")
            ) != normalized_tokens(move.source):
                problems.append(
                    f"{move.target}: body differs from {path}::{move.name} at {rev}"
                )
    return problems
