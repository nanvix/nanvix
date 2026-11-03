# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Extraction of test items from source files, for the PR flagger.

A *test item* is the smallest unit of test code that the flagger tracks: a test function, a helper,
a mock, a constant, a macro, a manifest table, ... Every item has a display name, a line and a
*fingerprint*. The fingerprint depends only on the content of the item, so it is insensitive to
comments, formatting, and to where in the file (or in which file) the item lives. Two revisions of a
file therefore describe the same tests exactly when they have the same multiset of fingerprints.

Rust items are found with the tokenizer and the scanner of the test-layout checker. Python items
come from the `ast` module. C/C++ items are top-level declarations and `#define`s. Manifest items
are TOML tables.
"""

from __future__ import annotations

import ast
import hashlib
import re
from dataclasses import dataclass
from typing import Iterable

from . import paths
from .cfgexpr import CfgParseError
from .clex import DIRECTIVE, directive_words, normalized_c_span, tokenize_c
from .rustlex import IDENT, PUNCT, STRING, LexError, Tokens, normalized_span, tokenize
from .scan import (
    WIRING_KINDS,
    enclosing_items,
    has_test_file_gate,
    is_test_gated,
    is_test_marked,
    measure_unit,
    read_attrs,
)

RUST = "rust"
PYTHON = "python"
C = "c"
TOML = "toml"


class ExtractError(Exception):
    """Raised when a file cannot be analyzed."""


@dataclass(frozen=True)
class Item:
    """One piece of test code."""

    name: str
    line: int
    fingerprint: str


@dataclass(frozen=True)
class ModuleRef:
    """An out-of-line Rust module and the guards inherited by its source file."""

    name: str
    path: str | None
    chain: tuple[str, ...]
    context: tuple[str, ...]


def language(path: str) -> str | None:
    """Returns the language of a file the flagger tracks, or `None` for any other file."""
    path = paths.to_posix(path)
    if path.endswith(".rs"):
        return RUST
    if path.endswith(".py") and paths.is_python_test_file(path):
        return PYTHON
    if path.endswith(paths.C_EXTENSIONS) and paths.is_c_test_file(path):
        return C
    if paths.is_e2e_manifest(path):
        return TOML
    return None


def is_test_file(path: str) -> bool:
    """Tells whether a tracked file is a dedicated test file (as opposed to mainline source)."""
    lang = language(path)
    if lang == RUST:
        return paths.is_rust_test_file(path)
    return lang is not None


def extract(path: str, text: str, context: tuple[str, ...] = ()) -> list[Item]:
    """Returns the test items of a file; raises `ExtractError` if it cannot be analyzed."""
    lang = language(path)
    if lang is None:
        return []
    text = text.lstrip("\ufeff").replace("\r\n", "\n")
    if lang == RUST:
        try:
            return _rust_items(text, paths.is_rust_test_file(path), context)
        except CfgParseError as err:
            raise ExtractError(str(err)) from err
    if lang == PYTHON:
        return _python_items(text)
    if lang == C:
        return _c_items(text)
    return _toml_items(text)


def _digest(scope: Iterable[str], words: Iterable[str]) -> str:
    data = "\x1f".join([*scope, "\x00", *words])
    return hashlib.sha256(data.encode("utf-8", "surrogatepass")).hexdigest()[:24]


def _display(chain: Iterable[str], name: str) -> str:
    return "::".join([*chain, name])


# ---------------------------------------------------------------------------------------------
# Rust
# ---------------------------------------------------------------------------------------------

_MENTIONS_TEST = re.compile(r"test|bench")

# Items whose body holds further items that are tracked one by one.
_RUST_CONTAINERS = frozenset({"mod", "impl", "trait", "extern_block"})


def _rust_items(text: str, in_test_file: bool, context: tuple[str, ...]) -> list[Item]:
    if not in_test_file and not _MENTIONS_TEST.search(text):
        return []
    try:
        toks = tokenize(text)
    except LexError as err:
        raise ExtractError(str(err)) from err
    items: list[Item] = []
    walker = _RustWalker(toks, items)
    if in_test_file or has_test_file_gate(toks):
        walker.counted(0, len(toks), (), context)
    else:
        walker.scan(0, len(toks), context)
    return items


def rust_module_refs(text: str) -> list[ModuleRef]:
    """Extracts module references without reading or executing their source files."""
    try:
        toks = tokenize(text)
        walker = _RustWalker(toks, [])
        walker.scan(0, len(toks))
        return walker.modules
    except (LexError, CfgParseError) as err:
        raise ExtractError(str(err)) from err


class _RustWalker:
    """Collects the test items of one Rust token stream."""

    def __init__(self, toks: Tokens, items: list[Item]) -> None:
        self.toks = toks
        self.items = items
        self.modules: list[ModuleRef] = []

    def scan(self, lo: int, hi: int, context: tuple[str, ...] = ()) -> None:
        """Finds the test-only and test-marked units in mainline code."""
        toks = self.toks
        i = lo
        while i < hi:
            if not toks.is_punct(i, "#") and not (
                toks.is_ident(i) and toks.src(i) in ("mod", "impl", "trait", "extern")
            ):
                i += 1
                continue
            if toks.is_punct(i + 1, "!") and toks.is_punct(i + 2, "["):
                context += self._attributes(i, inner=True)
                i = toks.partner[i + 2] + 1
                continue
            attrs, unit = read_attrs(toks, i)
            if unit >= hi:
                break
            if is_test_gated(attrs) or is_test_marked(attrs):
                i = self._unit(i, unit, hi, enclosing_items(toks, i), context) + 1
                continue
            measured = measure_unit(toks, unit)
            if measured.kind == "mod" and toks.is_punct(measured.end, ";"):
                self._module(i, measured.name, enclosing_items(toks, i), context)
                i = measured.end + 1
                continue
            if measured.kind in _RUST_CONTAINERS and toks.is_punct(measured.end, "}"):
                self.scan(
                    toks.partner[measured.end] + 1,
                    measured.end,
                    context + self._attributes(i),
                )
                i = measured.end + 1
            else:
                i = max(unit, i + 1)

    def counted(
        self,
        lo: int,
        hi: int,
        chain: tuple[tuple[str, str], ...],
        context: tuple[str, ...] = (),
    ) -> None:
        """Records every unit of a range that is test code as a whole."""
        toks = self.toks
        i = lo
        while i < hi:
            if (
                toks.is_punct(i, "#")
                and toks.is_punct(i + 1, "!")
                and toks.is_punct(i + 2, "[")
            ):
                end = toks.partner[i + 2]
                self._add(i, end, chain, "#![..]", context)
                context += self._attributes(i, inner=True)
                i = end + 1
                continue
            _, unit = read_attrs(toks, i)
            if unit >= hi:
                self._add(i, hi - 1, chain, "#[..]", context)
                break
            i = self._unit(i, unit, hi, chain, context) + 1

    def _attributes(self, start: int, *, inner: bool = False) -> tuple[str, ...]:
        """Returns container attributes, except ordinary test-build wiring."""
        attrs, _ = read_attrs(self.toks, start, inner=inner)
        words: list[str] = []
        for attr in attrs[:1] if inner else attrs:
            if attr.path == ("path",):
                continue
            if (
                attr.pred is not None
                and attr.pred.op == "atom"
                and attr.pred.key in ("test", 'feature="test"')
            ):
                continue
            bracket = attr.line_token + (2 if inner else 1)
            words.extend(
                normalized_span(self.toks, attr.line_token, self.toks.partner[bracket])
            )
        return tuple(words)

    def _unit(
        self,
        start: int,
        unit: int,
        hi: int,
        chain: tuple[tuple[str, str], ...],
        context: tuple[str, ...],
    ) -> int:
        """Records the unit that starts at `unit`; returns the index of its last token."""
        toks = self.toks
        measured = measure_unit(toks, unit)
        end = min(measured.end, hi - 1)
        if measured.kind == "mod" and toks.is_punct(end, ";"):
            self._module(start, measured.name, chain, context)
        if measured.kind in WIRING_KINDS:
            return end
        if (
            measured.kind in _RUST_CONTAINERS
            and toks.is_punct(end, "}")
            and toks.partner[end] >= unit
        ):
            child = chain + ((measured.kind, measured.name),)
            self.counted(
                toks.partner[end] + 1, end, child, context + self._attributes(start)
            )
            return end
        if end == unit and (toks.is_punct(unit, ";") or toks.is_punct(unit, ",")):
            return end
        self._add(start, end, chain, measured.name, context)
        return end

    def _module(
        self,
        start: int,
        name: str,
        chain: tuple[tuple[str, str], ...],
        context: tuple[str, ...],
    ) -> None:
        attrs, _ = read_attrs(self.toks, start)
        self.modules.append(
            ModuleRef(
                name,
                next((a.value for a in attrs if a.path == ("path",)), None),
                tuple(item for kind, item in chain if kind == "mod"),
                context + self._attributes(start),
            )
        )

    def _add(
        self,
        start: int,
        end: int,
        chain: tuple[tuple[str, str], ...],
        name: str,
        context: tuple[str, ...],
    ) -> None:
        toks = self.toks
        scope = [f"{kind}:{item}" for kind, item in chain if kind != "mod"]
        words = normalized_span(toks, start, end)
        shown = [f"<{item}>" if kind == "impl" else item for kind, item in chain]
        self.items.append(
            Item(
                _display(shown, name),
                toks.line_of(start),
                _digest([*scope, *context], words),
            )
        )


# ---------------------------------------------------------------------------------------------
# Python
# ---------------------------------------------------------------------------------------------

_SCOPES = (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)


def _strip_docstrings(tree: ast.AST) -> None:
    """Drops docstrings, which are documentation rather than test behavior."""
    for node in ast.walk(tree):
        if not isinstance(node, _SCOPES):
            continue
        body = node.body
        first = body[0] if body else None
        if (
            isinstance(first, ast.Expr)
            and isinstance(first.value, ast.Constant)
            and isinstance(first.value.value, str)
        ):
            node.body = body[1:]
            if not node.body and not isinstance(node, ast.Module):
                node.body = [ast.copy_location(ast.Pass(), first)]


def _python_items(text: str) -> list[Item]:
    try:
        tree = ast.parse(text)
    except (SyntaxError, ValueError, RecursionError, MemoryError) as err:
        raise ExtractError(f"cannot parse Python: {err}") from err
    _strip_docstrings(tree)
    items: list[Item] = []
    _python_body(tree.body, (), items)
    return items


def _python_name(node: ast.stmt) -> str:
    if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
        return node.name
    targets: list[ast.expr] = []
    if isinstance(node, ast.Assign):
        targets = list(node.targets)
    elif isinstance(node, (ast.AnnAssign, ast.AugAssign)):
        targets = [node.target]
    names = [t.id for t in targets if isinstance(t, ast.Name)]
    return ",".join(names) if names else type(node).__name__.lower()


def _python_line(node: ast.stmt) -> int:
    decorators = getattr(node, "decorator_list", [])
    return min([node.lineno] + [d.lineno for d in decorators])


def _python_body(body: list[ast.stmt], chain: tuple[str, ...], out: list[Item]) -> None:
    for node in body:
        if isinstance(node, (ast.Import, ast.ImportFrom)):
            continue
        if isinstance(node, ast.ClassDef):
            header = [
                "class",
                node.name,
                *(ast.dump(n) for n in node.decorator_list),
                *(ast.dump(n) for n in node.bases),
                *(ast.dump(n) for n in node.keywords),
            ]
            out.append(
                Item(
                    _display(chain, node.name),
                    _python_line(node),
                    _digest(chain, header),
                )
            )
            _python_body(node.body, chain + (node.name,), out)
            continue
        out.append(
            Item(
                _display(chain, _python_name(node)),
                _python_line(node),
                _digest(chain, [ast.dump(node)]),
            )
        )


# ---------------------------------------------------------------------------------------------
# C and C++
# ---------------------------------------------------------------------------------------------

_C_QUALIFIERS = frozenset({"const", "noexcept", "override", "final", "volatile"})
_C_AGGREGATES = frozenset({"struct", "union", "enum", "class", "typedef"})
_C_NAME_LENGTH = 48


def _c_items(text: str) -> list[Item]:
    try:
        toks = tokenize_c(text)
    except LexError as err:
        raise ExtractError(str(err)) from err
    items: list[Item] = []
    _c_walk(toks, 0, len(toks), (), _c_conditions(toks), items)
    return items


def _c_conditions(toks: Tokens) -> list[tuple[str, ...]]:
    """Records the enclosing preprocessor conditions of each token."""
    contexts: list[tuple[str, ...]] = []
    guards: list[list[str]] = []
    for i in range(len(toks)):
        contexts.append(tuple(word for guard in guards for word in guard))
        if toks.kinds[i] != DIRECTIVE:
            continue
        words = directive_words(toks.src(i))
        if len(words) < 2:
            continue
        directive = words[1]
        if directive in ("if", "ifdef", "ifndef"):
            guards.append(words)
        elif directive in ("elif", "elifdef", "elifndef", "else", "endif"):
            if not guards:
                raise ExtractError(f"unmatched #{directive} at line {toks.line_of(i)}")
            if directive == "endif":
                guards.pop()
            else:
                guards[-1].extend(words)
    if guards:
        raise ExtractError("unclosed preprocessor conditional")
    return contexts


def _c_container(toks: Tokens, i: int, hi: int) -> tuple[int, str] | None:
    """Returns the `{` and the name of a `extern "C" {` or `namespace x {` block starting at `i`."""
    if (
        toks.is_ident(i, "extern")
        and i + 1 < hi
        and toks.kinds[i + 1] == STRING
        and toks.is_punct(i + 2, "{")
    ):
        return i + 2, "extern"
    if toks.is_ident(i, "namespace"):
        j = i + 1
        while j < hi and (toks.kinds[j] == IDENT or toks.is_punct(j, "::")):
            j += 1
        if toks.is_punct(j, "{"):
            return j, "namespace " + "".join(toks.src(k) for k in range(i + 1, j))
    return None


def _c_is_function_body(toks: Tokens, brace: int, start: int) -> bool:
    k = brace - 1
    while k >= start and toks.is_ident(k) and toks.src(k) in _C_QUALIFIERS:
        k -= 1
    return k >= start and toks.is_punct(k, ")")


def _c_statement_end(toks: Tokens, start: int, hi: int) -> int:
    """Returns the index that ends the declaration starting at `start`."""
    j = start
    while j < hi:
        if toks.is_punct(j, ";"):
            return j
        if toks.is_punct(j, "{"):
            close = toks.partner[j]
            if _c_is_function_body(toks, j, start):
                return close
            j = close + 1
        elif toks.kinds[j] == PUNCT and toks.partner[j] > j:
            j = toks.partner[j] + 1
        else:
            j += 1
    return hi - 1


def _c_name(toks: Tokens, start: int, end: int) -> str:
    """Names a declaration after its function, aggregate, variable or typedef."""
    typedef = toks.is_ident(start, "typedef")
    last = ""
    j = start
    while j <= end:
        if toks.is_punct(j, "("):
            if j == start + 1 and toks.is_ident(start):
                # A macro invocation such as `TEST(suite, name)` is named after its arguments.
                last = ""
                break
            if j > start and toks.is_ident(j - 1) and not typedef:
                return toks.src(j - 1)
            j = toks.partner[j] + 1
            continue
        if toks.kinds[j] == PUNCT and toks.partner[j] > j:
            if not typedef:
                break
            j = toks.partner[j] + 1
            continue
        if toks.is_punct(j, "="):
            break
        if toks.is_ident(j) and toks.src(j) not in _C_AGGREGATES:
            last = toks.src(j)
        j += 1
    return last or " ".join(toks.span(start, end).split())[:_C_NAME_LENGTH].rstrip()


def _c_walk(
    toks: Tokens,
    lo: int,
    hi: int,
    chain: tuple[str, ...],
    conditions: list[tuple[str, ...]],
    out: list[Item],
) -> None:
    i = lo
    while i < hi:
        if toks.kinds[i] == DIRECTIVE:
            words = directive_words(toks.src(i))
            if len(words) > 2 and words[1] in ("define", "undef"):
                out.append(
                    Item(
                        _display(chain, words[2]),
                        toks.line_of(i),
                        _digest(
                            (*chain, *conditions[i]), normalized_c_span(toks, i, i)
                        ),
                    )
                )
            i += 1
            continue
        if toks.is_punct(i, ";"):
            i += 1
            continue
        container = _c_container(toks, i, hi)
        if container is not None:
            brace, name = container
            _c_walk(
                toks, brace + 1, toks.partner[brace], chain + (name,), conditions, out
            )
            i = toks.partner[brace] + 1
            continue
        end = _c_statement_end(toks, i, hi)
        out.append(
            Item(
                _display(chain, _c_name(toks, i, end)),
                toks.line_of(i),
                _digest((*chain, *conditions[i]), normalized_c_span(toks, i, end)),
            )
        )
        i = end + 1


# ---------------------------------------------------------------------------------------------
# TOML (end-to-end test manifests)
# ---------------------------------------------------------------------------------------------

_TOML_LABEL = re.compile(r'^(name|program)\s*=\s*"(.*)"$')


def _toml_lines(text: str) -> list[tuple[int, str, bool]]:
    """Returns `(line number, normalized text, is table header)` for every significant line.

    Comments, blank lines and indentation are dropped. Lines inside multi-line strings are kept
    verbatim, and lines inside multi-line arrays or inline tables are never table headers.
    """
    lines: list[tuple[int, str, bool]] = []
    quote = ""  # The delimiter of the string being read (`"`, `'`, `"""` or `'''`).
    depth = 0
    for number, raw in enumerate(text.split("\n"), start=1):
        verbatim = quote in ('"""', "'''")
        clean_start = not quote and depth == 0
        out: list[str] = []
        i = 0
        while i < len(raw):
            ch = raw[i]
            if not quote:
                if ch == "#":
                    break
                for opener in ('"""', "'''", '"', "'"):
                    if raw.startswith(opener, i):
                        quote = opener
                        out.append(opener)
                        i += len(opener)
                        break
                else:
                    if ch in "[{":
                        depth += 1
                    elif ch in "]}":
                        depth = max(0, depth - 1)
                    out.append(ch)
                    i += 1
            elif ch == "\\" and quote[0] == '"':
                out.append(raw[i : i + 2])
                i += 2
            elif raw.startswith(quote, i):
                out.append(quote)
                i += len(quote)
                quote = ""
            else:
                out.append(ch)
                i += 1
        if quote in ('"', "'"):
            quote = ""
        text_line = "".join(out) if verbatim else "".join(out).strip()
        if not text_line.strip() and not verbatim:
            continue
        lines.append((number, text_line, clean_start and text_line.startswith("[")))
    return lines


def _toml_items(text: str) -> list[Item]:
    items: list[Item] = []
    current: list[tuple[int, str, bool]] = []

    def flush() -> None:
        if not current:
            return
        header = current[0][1] if current[0][2] else "(root)"
        label = ""
        for _, line, _ in current:
            match = _TOML_LABEL.match(line)
            if match:
                label = f' {match.group(1)}="{match.group(2)}"'
                break
        items.append(
            Item(
                header + label,
                current[0][0],
                _digest((), [line for _, line, _ in current]),
            )
        )

    for entry in _toml_lines(text):
        if entry[2]:
            flush()
            current = []
        current.append(entry)
    flush()
    return items
