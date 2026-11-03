# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Minimal C/C++ tokenizer used by the PR flagger.

Like the Rust tokenizer it only has to be precise enough to match brackets and slice top-level
declarations without being fooled by comments, strings or character literals. Preprocessor
directives are returned as single tokens of kind `DIRECTIVE`.
"""

from __future__ import annotations

import re

from .rustlex import CHAR, IDENT, NUMBER, PUNCT, STRING, LexError, Tokens

DIRECTIVE = "directive"

_OPENERS = {"(": ")", "[": "]", "{": "}"}
_CLOSERS = {")": "(", "]": "[", "}": "{"}

_TOKEN = re.compile(
    r"""
      (?P<ws>\s+)
    | (?P<line>//[^\n]*)
    | (?P<block>/\*)
    | (?P<raw>(?:u8|u|U|L)?R"(?P<delim>[^()\\\s]{0,16})\(.*?\)(?P=delim)")
    | (?P<string>(?:u8|u|U|L)?"(?:[^"\\\n]|\\.)*")
    | (?P<char>(?:u8|u|U|L)?'(?:[^'\\\n]|\\.)*')
    | (?P<ident>[^\W\d]\w*)
    | (?P<number>\.?\d(?:[eEpP][+-]|[\w.]|'(?=\w))*)
    | (?P<punct>.)
    """,
    re.VERBOSE | re.DOTALL,
)

_KIND_OF_GROUP = {
    "raw": STRING,
    "string": STRING,
    "char": CHAR,
    "ident": IDENT,
    "number": NUMBER,
    "punct": PUNCT,
}

_LINE_CONTINUATION = re.compile(r"\\\r?\n")


def _block_comment_end(text: str, start: int) -> int:
    """Returns the offset just after the block comment starting at `start`."""
    end = text.find("*/", start + 2)
    if end == -1:
        raise LexError("unterminated block comment")
    return end + 2


def _directive_end(text: str, start: int) -> int:
    """Returns the offset of the newline that ends the preprocessor directive at `start`."""
    i = start
    n = len(text)
    while i < n:
        ch = text[i]
        if ch == "\n":
            return i
        if ch == "\\" and text.startswith("\n", i + 1):
            i += 2
        elif ch == "\\" and text.startswith("\r\n", i + 1):
            i += 3
        elif text.startswith("/*", i):
            i = _block_comment_end(text, i)
        elif text.startswith("//", i):
            end = text.find("\n", i)
            return n if end == -1 else end
        elif ch == '"':
            end = i + 1
            while end < n and text[end] not in '"\n':
                end += 2 if text[end] == "\\" else 1
            i = end + 1
        else:
            i += 1
    return n


def _starts_line(text: str, pos: int) -> bool:
    """Tells whether only whitespace precedes `pos` on its line."""
    line_start = text.rfind("\n", 0, pos) + 1
    return not text[line_start:pos].strip()


def tokenize_c(text: str) -> Tokens:
    """Tokenizes C or C++ source text, dropping whitespace and comments."""
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
        if kind == "ws" or kind == "line":
            pos = end
            continue
        if kind == "block":
            pos = _block_comment_end(text, pos)
            continue
        index = len(toks.kinds)
        if kind == "punct" and text[pos] == "#" and _starts_line(text, pos):
            tkind = DIRECTIVE
            end = _directive_end(text, pos)
        else:
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
                toks.parent[index] = toks.parent[opener]
        pos = end
    if stack:
        line = text.count("\n", 0, toks.starts[stack[-1]]) + 1
        raise LexError(
            f"unclosed '{text[toks.starts[stack[-1]]]}' opened at line {line}"
        )
    return toks


def directive_words(text: str) -> list[str]:
    """Returns the words of one preprocessor directive, without comments or continuations."""
    body = _LINE_CONTINUATION.sub(" ", text).lstrip()[1:]
    try:
        inner = tokenize_c(body)
    except LexError:
        return body.split()
    return ["#"] + [inner.src(i) for i in range(len(inner))]


def normalized_c_span(toks: Tokens, first: int, last: int) -> list[str]:
    """Returns the token texts of `toks[first..last]`, ignoring comments and layout."""
    words: list[str] = []
    for i in range(first, last + 1):
        if toks.kinds[i] == DIRECTIVE:
            words.extend(directive_words(toks.src(i)))
        else:
            words.append(toks.src(i))
    return words
