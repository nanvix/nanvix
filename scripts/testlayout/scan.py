# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Scanner that finds test code embedded in Rust mainline source files.

The scanner reports three kinds of facts about a source file:

- test-only gated modules (`#[cfg(test)] mod x { ... }` and `#[cfg(test)] mod x;`),
- other test-only gated code ("seams": helpers, mocks, accessors, statements, fields, ...), and
- test attributes (`#[test]` and friends) that appear outside any test-only region.

Everything inside a test-only region is considered test code and is not inspected any further.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field

from .cfgexpr import Pred, is_test_only, parse_cfg_args
from .rustlex import IDENT, PUNCT, STRING, Tokens, tokenize

# Last path segment of attributes that turn a function into a test or a benchmark.
TEST_ATTR_LAST = frozenset({"test", "test_case", "rstest", "bench"})

# Items whose gated declaration is wiring rather than test code.
WIRING_KINDS = frozenset({"use", "extern_crate"})

# Item kinds that end at a `;` or at the close of a `{}` body.
_BODY_ITEMS = frozenset(
    {
        "mod",
        "fn",
        "impl",
        "struct",
        "enum",
        "union",
        "trait",
        "extern_block",
        "macro_rules",
    }
)

# Plain keywords that introduce an item; `const`, `unsafe`, `extern` and `macro_rules` need context.
_PLAIN_KEYWORDS = frozenset(
    {"mod", "fn", "impl", "struct", "enum", "union", "trait", "static", "type", "use"}
)

_SNIPPET_LENGTH = 48


@dataclass
class Attr:
    """One parsed outer or inner attribute."""

    path: tuple[str, ...]
    line_token: int
    pred: Pred | None = None
    value: str | None = None


@dataclass
class GatedMod:
    """A test-only gated module declaration."""

    name: str
    line: int
    inline: bool
    tests: int
    path_attr: str | None
    # Names of the inline modules that enclose the declaration, outermost first.
    chain: tuple[str, ...] = ()
    # Offsets into the source text: start of the first attribute, start of the item itself (after
    # its attributes), and the `{` / `}` of an inline body (both -1 for out-of-line modules).
    start_off: int = -1
    unit_off: int = -1
    body_open: int = -1
    body_close: int = -1


@dataclass
class Seam:
    """Test-only gated code that is not a module."""

    kind: str
    name: str
    line: int


@dataclass
class ScanResult:
    """Everything the scanner found in one file."""

    file_gate_line: int = 0
    mods: list[GatedMod] = field(default_factory=list)
    seams: list[Seam] = field(default_factory=list)
    test_attr_lines: list[int] = field(default_factory=list)
    doctest_lines: list[int] = field(default_factory=list)


def _parse_attr(toks: Tokens, hash_index: int, open_bracket: int) -> Attr:
    """Parses the attribute whose `[` is at `open_bracket`."""
    close = toks.partner[open_bracket]
    i = open_bracket + 1
    path: list[str] = []
    while i < close and toks.kinds[i] == IDENT:
        path.append(toks.src(i))
        if toks.is_punct(i + 1, ":") and toks.is_punct(i + 2, ":"):
            i += 3
        else:
            i += 1
            break
    attr = Attr(path=tuple(path), line_token=hash_index)
    if tuple(path) == ("cfg",) and toks.is_punct(i, "("):
        attr.pred = parse_cfg_args(toks, i)
    elif tuple(path) == ("path",) and toks.is_punct(i, "=") and i + 1 < close:
        if toks.kinds[i + 1] == STRING:
            attr.value = toks.src(i + 1).strip('"')
    return attr


def _skip_visibility(toks: Tokens, k: int) -> int:
    if toks.is_ident(k, "pub"):
        k += 1
        if toks.is_punct(k, "("):
            k = toks.partner[k] + 1
    return k


def _find_keyword(toks: Tokens, k: int) -> tuple[str, int] | None:
    """Skips item modifiers and returns the item keyword (normalized) and its token index."""
    n = len(toks)
    while k < n and toks.kinds[k] == IDENT:
        word = toks.src(k)
        nxt = toks.src(k + 1) if toks.is_ident(k + 1) else None
        if word in ("async", "default", "safe"):
            k += 1
        elif word == "unsafe":
            if nxt in (
                "fn",
                "impl",
                "trait",
                "extern",
                "mod",
                "static",
                "async",
                "const",
            ):
                k += 1
            else:
                return None
        elif word == "const":
            if nxt in ("fn", "unsafe", "async", "extern"):
                k += 1
            else:
                return "const", k
        elif word == "extern":
            if nxt == "crate":
                return "extern_crate", k
            k2 = k + 1
            if k2 < n and toks.kinds[k2] == STRING:
                k2 += 1
            if toks.is_ident(k2, "fn"):
                k = k2
            elif toks.is_punct(k2, "{"):
                return "extern_block", k
            else:
                return None
        elif word == "macro_rules" and toks.is_punct(k + 1, "!"):
            return "macro_rules", k
        elif word in _PLAIN_KEYWORDS:
            return word, k
        else:
            return None
    return None


def _skip_to_semicolon(toks: Tokens, k: int) -> int:
    """Returns the index of the next `;` at the current nesting level (or the last token)."""
    n = len(toks)
    while k < n:
        if toks.is_punct(k, ";"):
            return k
        if toks.kinds[k] == PUNCT and toks.partner[k] > k:
            k = toks.partner[k]
        elif toks.kinds[k] == PUNCT and toks.partner[k] != -1:
            # An unmatched closer ends the enclosing scope.
            return k - 1
        k += 1
    return n - 1


def _skip_to_body_end(toks: Tokens, k: int) -> int:
    """Returns the index that ends an item with an optional `{}` body."""
    n = len(toks)
    while k < n:
        if toks.is_punct(k, ";"):
            return k
        if toks.is_punct(k, "{"):
            return toks.partner[k]
        if toks.kinds[k] == PUNCT and toks.partner[k] > k:
            k = toks.partner[k]
        elif toks.kinds[k] == PUNCT and toks.partner[k] != -1:
            return k - 1
        k += 1
    return n - 1


def _skip_expression(toks: Tokens, k: int) -> int:
    """Returns the index that ends a statement, expression, field or match arm starting at `k`."""
    n = len(toks)
    while k < n:
        if toks.is_punct(k, ";") or toks.is_punct(k, ","):
            return k
        if toks.kinds[k] == PUNCT and toks.partner[k] != -1:
            if toks.partner[k] < k:
                return k - 1
            close = toks.partner[k]
            if toks.is_punct(k, "{"):
                nxt = close + 1
                if toks.is_ident(nxt, "else"):
                    k = nxt + 1
                    continue
                if any(toks.is_punct(nxt, c) for c in ".?,;"):
                    k = nxt
                    continue
                return close
            k = close + 1
            continue
        k += 1
    return n - 1


def _normalize(text: str) -> str:
    return " ".join(text.split())


def _header(toks: Tokens, brace: int) -> tuple[str, str]:
    """Returns the kind and name of the item whose `{}` body opens at `brace`.

    Returns `("", "")` when the block does not belong to a named item (`if`, `match`, closures).
    """
    m = brace - 1
    while m >= 0:
        if toks.is_punct(m, ";") or toks.is_punct(m, "}") or toks.is_punct(m, "{"):
            break
        if toks.kinds[m] == PUNCT and toks.partner[m] != -1:
            if toks.partner[m] > m:
                break
            m = toks.partner[m]
        m -= 1
    k = m + 1
    while k < brace:
        if toks.is_punct(k, "#") and toks.is_punct(k + 1, "["):
            k = toks.partner[k + 1] + 1
        else:
            break
    k = _skip_visibility(toks, k)
    found = _find_keyword(toks, k)
    if found is None:
        return "", ""
    kind, kw = found
    if kind == "impl":
        end = kw + 1
        while end < brace and not toks.is_ident(end, "where"):
            end += 1
        if end <= kw + 1:
            return "", ""
        return kind, _normalize(toks.span(kw + 1, end - 1))
    if kind in ("mod", "fn", "struct", "enum", "union", "trait") and toks.is_ident(
        kw + 1
    ):
        return kind, toks.src(kw + 1)
    return "", ""


def _enclosing(toks: Tokens, index: int) -> str:
    """Returns the name of the innermost named item enclosing token `index`."""
    opener = toks.parent[index] if index < len(toks) else -1
    while opener != -1:
        if toks.is_punct(opener, "{"):
            name = _header(toks, opener)[1]
            if name:
                return name
        opener = toks.parent[opener]
    return ""


def _mod_chain(toks: Tokens, index: int) -> tuple[str, ...]:
    """Returns the names of the inline modules enclosing token `index`, outermost first."""
    chain: list[str] = []
    opener = toks.parent[index] if index < len(toks) else -1
    while opener != -1:
        if toks.is_punct(opener, "{"):
            kind, name = _header(toks, opener)
            if kind == "mod":
                chain.append(name)
        opener = toks.parent[opener]
    return tuple(reversed(chain))


def _item_name(toks: Tokens, kind: str, kw: int, brace_limit: int) -> str:
    """Returns the display name of an item whose keyword is at `kw`."""
    if kind == "impl":
        end = kw + 1
        while (
            end <= brace_limit
            and not toks.is_ident(end, "where")
            and not toks.is_punct(end, "{")
        ):
            end += 1
        return _normalize(toks.span(kw + 1, end - 1)) if end > kw + 1 else "impl"
    if kind in ("extern_block",):
        return "extern"
    k = kw + 1
    if kind == "macro_rules":
        k = kw + 2
    elif kind == "static" and toks.is_ident(k, "mut"):
        k += 1
    if k < len(toks) and (toks.kinds[k] == IDENT or toks.is_punct(k, "_")):
        return toks.src(k)
    return "?"


def _count_tests(toks: Tokens, first: int, last: int) -> int:
    """Counts test attributes in the token range [first, last]."""
    count = 0
    i = first
    while i <= last:
        if (
            toks.is_punct(i, "#")
            and toks.is_punct(i + 1, "[")
            and toks.partner[i + 1] <= last
        ):
            j = i + 2
            path: list[str] = []
            while j < toks.partner[i + 1] and toks.kinds[j] == IDENT:
                path.append(toks.src(j))
                if toks.is_punct(j + 1, ":") and toks.is_punct(j + 2, ":"):
                    j += 3
                else:
                    break
            if path and path[-1] in TEST_ATTR_LAST:
                count += 1
        i += 1
    return count


def _test_only(attrs: list[Attr]) -> bool:
    return any(a.pred is not None and is_test_only(a.pred) for a in attrs)


def scan_rust(text: str) -> ScanResult:
    """Scans Rust source text for test code embedded in mainline code."""
    toks = tokenize(text)
    res = ScanResult()
    n = len(toks)
    i = 0
    while i < n:
        if not toks.is_punct(i, "#"):
            i += 1
            continue
        if toks.is_punct(i + 1, "!") and toks.is_punct(i + 2, "["):
            attr = _parse_attr(toks, i, i + 2)
            if (
                toks.parent[i] == -1
                and attr.pred is not None
                and is_test_only(attr.pred)
                and not res.file_gate_line
            ):
                res.file_gate_line = toks.line_of(i)
            i = toks.partner[i + 2] + 1
            continue
        if not toks.is_punct(i + 1, "["):
            i += 1
            continue
        attrs: list[Attr] = []
        j = i
        while toks.is_punct(j, "#") and toks.is_punct(j + 1, "["):
            attrs.append(_parse_attr(toks, j, j + 1))
            j = toks.partner[j + 1] + 1
        if not _test_only(attrs):
            for attr in attrs:
                if attr.path and attr.path[-1] in TEST_ATTR_LAST:
                    res.test_attr_lines.append(toks.line_of(attr.line_token))
            i = j
            continue
        i = _record_gated(toks, res, attrs, i, j) + 1
    res.doctest_lines = scan_doctests(toks)
    return res


@dataclass(frozen=True)
class Unit:
    """The extent and identity of one item, statement, field or match arm."""

    # Index of the last token of the unit.
    end: int
    # Kind of the item the unit declares, or `""` when it is not an item.
    kind: str
    # Display name: the item name, or a normalized snippet for units that are not items.
    name: str


def measure_unit(toks: Tokens, unit: int) -> Unit:
    """Measures the unit that starts at token `unit` (the first token after its attributes)."""
    n = len(toks)
    found = _find_keyword(toks, _skip_visibility(toks, unit))
    if found is None:
        end = max(_skip_expression(toks, unit), unit)
        snippet = _normalize(toks.span(unit, end))[:_SNIPPET_LENGTH].rstrip()
        return Unit(end, "", snippet)
    kind, kw = found
    if kind in _BODY_ITEMS:
        end = _skip_to_body_end(toks, kw + 1)
    else:
        end = _skip_to_semicolon(toks, kw + 1)
    return Unit(max(end, unit), kind, _item_name(toks, kind, kw, n - 1))


def read_attrs(toks: Tokens, index: int) -> tuple[list[Attr], int]:
    """Reads the outer attributes that start at `index`; returns them and the next token index."""
    attrs: list[Attr] = []
    while toks.is_punct(index, "#") and toks.is_punct(index + 1, "["):
        attrs.append(_parse_attr(toks, index, index + 1))
        index = toks.partner[index + 1] + 1
    return attrs, index


def is_test_gated(attrs: list[Attr]) -> bool:
    """Tells whether any attribute compiles its item only in test builds."""
    return _test_only(attrs)


def is_test_marked(attrs: list[Attr]) -> bool:
    """Tells whether any attribute turns its function into a test or a benchmark."""
    return any(a.path and a.path[-1] in TEST_ATTR_LAST for a in attrs)


def has_test_file_gate(toks: Tokens) -> bool:
    """Tells whether the file starts with an inner attribute that makes it test-only."""
    i = 0
    while (
        toks.is_punct(i, "#")
        and toks.is_punct(i + 1, "!")
        and toks.is_punct(i + 2, "[")
    ):
        attr = _parse_attr(toks, i, i + 2)
        if attr.pred is not None and is_test_only(attr.pred):
            return True
        i = toks.partner[i + 2] + 1
    return False


def enclosing_items(toks: Tokens, index: int) -> tuple[tuple[str, str], ...]:
    """Returns the `(kind, name)` of the named items that enclose token `index`, outermost first."""
    chain: list[tuple[str, str]] = []
    opener = toks.parent[index] if index < len(toks) else -1
    while opener != -1:
        if toks.is_punct(opener, "{"):
            kind, name = _header(toks, opener)
            if name:
                chain.append((kind, name))
        opener = toks.parent[opener]
    return tuple(reversed(chain))


def _record_gated(
    toks: Tokens, res: ScanResult, attrs: list[Attr], start: int, unit: int
) -> int:
    """Records one test-only gated unit and returns the index of its last token."""
    n = len(toks)
    line = toks.line_of(start)
    if unit >= n:
        return n - 1
    measured = measure_unit(toks, unit)
    end, kind, name = measured.end, measured.kind, measured.name
    context = _enclosing(toks, start)

    def qualified(name: str) -> str:
        return f"{context}::{name}" if context else name

    if kind == "":
        res.seams.append(Seam("code", qualified(name), line))
        return end
    if kind == "mod":
        inline = toks.is_punct(end, "}")
        path_attr = next((a.value for a in attrs if a.value is not None), None)
        res.mods.append(
            GatedMod(
                name=name,
                line=line,
                inline=inline,
                tests=_count_tests(toks, unit, end) if inline else 0,
                path_attr=path_attr,
                chain=_mod_chain(toks, start),
                start_off=toks.starts[start],
                unit_off=toks.starts[unit],
                body_open=toks.starts[toks.partner[end]] if inline else -1,
                body_close=toks.starts[end] if inline else -1,
            )
        )
    elif kind not in WIRING_KINDS:
        res.seams.append(Seam(kind, qualified(name), line))
    return max(end, unit)


# ---------------------------------------------------------------------------------------------
# Documentation tests
# ---------------------------------------------------------------------------------------------

_FENCE = re.compile(r"^ {0,3}(`{3,}|~{3,})\s*(.*?)\s*$")
_RUST_FENCE_TOKENS = frozenset(
    {
        "rust",
        "should_panic",
        "no_run",
        "compile_fail",
        "test_harness",
        "standalone_crate",
        "standalone",
    }
)
_ERROR_CODE = re.compile(r"^E\d{4}$")


def _doc_lines(comment: str) -> list[str]:
    """Returns the Markdown lines of one documentation comment."""
    if comment.startswith("//"):
        body = comment[3:]
        return [body[1:] if body.startswith(" ") else body]
    inner = comment[3:-2] if comment.endswith("*/") else comment[3:]
    lines = []
    for raw in inner.split("\n"):
        stripped = raw.lstrip()
        if stripped.startswith("*"):
            stripped = stripped[1:]
        lines.append(stripped[1:] if stripped.startswith(" ") else stripped)
    return lines


def is_executable_fence(info: str) -> bool:
    """Tells whether a fenced code block is compiled and run (or checked) as a Rust doc-test."""
    tags = [t for t in re.split(r"[,\s]+", info.strip()) if t]
    for tag in tags:
        if tag in _RUST_FENCE_TOKENS or _ERROR_CODE.match(tag):
            continue
        if tag.startswith("edition") or tag.startswith("ignore"):
            continue
        return False
    return not any(t.startswith("ignore") for t in tags)


def scan_doctests(toks: Tokens) -> list[int]:
    """Returns the lines of fenced code blocks in doc comments that rustdoc would execute."""
    found: list[int] = []
    text = toks.text
    previous_end = -1
    fence: tuple[str, int] | None = None
    for start, end in toks.doc_comments:
        if previous_end >= 0 and text[previous_end:start].strip():
            fence = None
        previous_end = end
        base_line = text.count("\n", 0, start) + 1
        for offset, line in enumerate(_doc_lines(text[start:end])):
            m = _FENCE.match(line)
            if m is None:
                continue
            marker, info = m.group(1), m.group(2)
            if fence is None:
                fence = (marker[0], len(marker))
                if is_executable_fence(info):
                    found.append(base_line + offset)
            elif marker[0] == fence[0] and len(marker) >= fence[1] and not info:
                fence = None
    return found
