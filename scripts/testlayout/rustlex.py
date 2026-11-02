# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Minimal Rust tokenizer used by the test-layout tooling.

The tokenizer is not a Rust parser. It only has to be precise enough to find attributes, match
brackets, and slice item bodies without being fooled by comments, strings, raw strings, character
literals or lifetimes.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field

# Token kinds.
IDENT = "ident"
LIFETIME = "lifetime"
NUMBER = "number"
STRING = "string"
CHAR = "char"
PUNCT = "punct"

_OPENERS = {"(": ")", "[": "]", "{": "}"}
_CLOSERS = {")": "(", "]": "[", "}": "{"}

_TOKEN = re.compile(
    r"""
      (?P<ws>\s+)
    | (?P<line>//[^\n]*)
    | (?P<block>/\*)
    | (?P<raw>(?:b|c)?r(?P<hashes>\#*)".*?"(?P=hashes))
    | (?P<string>(?:b|c)?"(?:[^"\\]|\\.)*")
    | (?P<char>b?'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^\\'\n])')
    | (?P<lifetime>'[^\W\d]\w*)
    | (?P<ident>(?:r\#)?[^\W\d]\w*)
    | (?P<number>\d\w*(?:\.\d\w*)?)
    | (?P<punct>.)
    """,
    re.VERBOSE | re.DOTALL,
)

_KIND_OF_GROUP = {
    "raw": STRING,
    "string": STRING,
    "char": CHAR,
    "lifetime": LIFETIME,
    "ident": IDENT,
    "number": NUMBER,
    "punct": PUNCT,
}


class LexError(Exception):
    """Raised when the source cannot be tokenized consistently."""


@dataclass
class Tokens:
    """Token stream of a Rust source file with bracket matching information."""

    text: str
    kinds: list[str] = field(default_factory=list)
    starts: list[int] = field(default_factory=list)
    ends: list[int] = field(default_factory=list)
    # For every bracket token, index of its partner token (-1 for non-brackets).
    partner: list[int] = field(default_factory=list)
    # For every token, index of the innermost enclosing opening bracket (-1 at top level).
    parent: list[int] = field(default_factory=list)
    # Doc comments as (start, end) offsets into `text`.
    doc_comments: list[tuple[int, int]] = field(default_factory=list)

    def __len__(self) -> int:
        return len(self.kinds)

    def src(self, index: int) -> str:
        """Returns the source text of one token."""
        return self.text[self.starts[index] : self.ends[index]]

    def is_punct(self, index: int, char: str) -> bool:
        """Tells whether a token is the given punctuation character."""
        return (
            0 <= index < len(self.kinds)
            and self.kinds[index] == PUNCT
            and self.text[self.starts[index]] == char
        )

    def is_ident(self, index: int, word: str | None = None) -> bool:
        """Tells whether a token is an identifier (optionally a specific one)."""
        if not 0 <= index < len(self.kinds) or self.kinds[index] != IDENT:
            return False
        return word is None or self.src(index) == word

    def span(self, first: int, last: int) -> str:
        """Returns the source text from the start of `first` to the end of `last`."""
        return self.text[self.starts[first] : self.ends[last]]

    def line_of(self, index: int) -> int:
        """Returns the 1-based line number of a token."""
        return self.text.count("\n", 0, self.starts[index]) + 1


def _skip_block_comment(text: str, start: int) -> int:
    """Returns the offset just after a (possibly nested) block comment starting at `start`."""
    depth = 0
    i = start
    n = len(text)
    while i < n:
        if text.startswith("/*", i):
            depth += 1
            i += 2
        elif text.startswith("*/", i):
            depth -= 1
            i += 2
            if depth == 0:
                return i
        else:
            i += 1
    raise LexError("unterminated block comment")


def _is_doc_comment(text: str, start: int, block: bool) -> bool:
    """Tells whether a comment is an outer or inner documentation comment."""
    if block:
        if text.startswith("/**/", start):
            return False
        return (
            text.startswith("/**", start) and not text.startswith("/***", start)
        ) or (text.startswith("/*!", start))
    return (text.startswith("///", start) and not text.startswith("////", start)) or (
        text.startswith("//!", start)
    )


def tokenize(text: str) -> Tokens:
    """Tokenizes Rust source text, dropping whitespace and comments."""
    toks = Tokens(text=text)
    stack: list[int] = []
    pos = 0
    n = len(text)
    match = _TOKEN.match
    while pos < n:
        m = match(text, pos)
        assert m is not None  # The final alternative matches any character.
        kind = m.lastgroup
        end = m.end()
        if kind == "ws":
            pos = end
            continue
        if kind == "line":
            if _is_doc_comment(text, pos, block=False):
                toks.doc_comments.append((pos, end))
            pos = end
            continue
        if kind == "block":
            end = _skip_block_comment(text, pos)
            if _is_doc_comment(text, pos, block=True):
                toks.doc_comments.append((pos, end))
            pos = end
            continue
        index = len(toks.kinds)
        tkind = _KIND_OF_GROUP[kind]
        toks.kinds.append(tkind)
        toks.starts.append(pos)
        toks.ends.append(end)
        toks.partner.append(-1)
        toks.parent.append(stack[-1] if stack else -1)
        if tkind == PUNCT:
            ch = text[pos]
            if ch in _OPENERS:
                stack.append(index)
            elif ch in _CLOSERS:
                if not stack or text[toks.starts[stack[-1]]] != _CLOSERS[ch]:
                    line = text.count("\n", 0, pos) + 1
                    raise LexError(f"unbalanced '{ch}' at line {line}")
                opener = stack.pop()
                toks.partner[opener] = index
                toks.partner[index] = opener
                # A closing bracket belongs to the scope of its opener's parent.
                toks.parent[index] = toks.parent[opener]
        pos = end
    if stack:
        line = text.count("\n", 0, toks.starts[stack[-1]]) + 1
        raise LexError(
            f"unclosed '{text[toks.starts[stack[-1]]]}' opened at line {line}"
        )
    return toks
