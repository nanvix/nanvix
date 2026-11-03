# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Parsing and evaluation of Rust `cfg` predicates.

The only question the test-layout policy asks about a predicate is whether it is *test-only*, that
is, whether it can only hold when compiling tests (`test`) or the in-kernel test suite
(`feature = "test"`). It is answered by enumerating truth assignments of the atoms involved.
"""

from __future__ import annotations

import itertools
from dataclasses import dataclass

from .rustlex import IDENT, STRING, Tokens

# Atoms that denote a test build.
TEST_ATOMS = ("test", 'feature="test"')

# Upper bound on distinct atoms enumerated; larger predicates are conservatively not test-only.
_MAX_ATOMS = 14


@dataclass(frozen=True)
class Pred:
    """A parsed `cfg` predicate.

    `op` is one of `atom`, `all`, `any` or `not`. Atoms carry a normalized `key` such as `test` or
    `feature="std"`; the other operators carry their operands in `args`.
    """

    op: str
    key: str = ""
    args: tuple["Pred", ...] = ()


class CfgParseError(Exception):
    """Raised when a `cfg` predicate cannot be parsed."""


def _parse(toks: Tokens, pos: int, end: int) -> tuple[Pred, int]:
    """Parses one predicate starting at `pos` (exclusive `end`); returns it and the next index."""
    if pos >= end or toks.kinds[pos] != IDENT:
        raise CfgParseError("expected a predicate")
    name = toks.src(pos)
    nxt = pos + 1
    if nxt < end and toks.is_punct(nxt, "("):
        close = toks.partner[nxt]
        if name not in ("all", "any", "not"):
            raise CfgParseError(f"unknown predicate '{name}'")
        args: list[Pred] = []
        cursor = nxt + 1
        while cursor < close:
            arg, cursor = _parse(toks, cursor, close)
            args.append(arg)
            if cursor < close:
                if not toks.is_punct(cursor, ","):
                    raise CfgParseError("expected ','")
                cursor += 1
        if name == "not" and len(args) != 1:
            raise CfgParseError("not() takes one operand")
        return Pred(op=name, args=tuple(args)), close + 1
    if nxt < end and toks.is_punct(nxt, "="):
        if nxt + 1 >= end or toks.kinds[nxt + 1] != STRING:
            raise CfgParseError("expected a string value")
        value = toks.src(nxt + 1)
        return Pred(op="atom", key=f"{name}={value}".replace(" ", "")), nxt + 2
    return Pred(op="atom", key=name), nxt


def parse_cfg_args(toks: Tokens, open_paren: int) -> Pred:
    """Parses the predicate inside the parenthesis group opening at `open_paren`."""
    close = toks.partner[open_paren]
    pred, cursor = _parse(toks, open_paren + 1, close)
    if cursor < close and toks.is_punct(cursor, ","):
        cursor += 1
    if cursor != close:
        raise CfgParseError("trailing tokens in cfg predicate")
    return pred


def _atoms(pred: Pred, into: set[str]) -> None:
    if pred.op == "atom":
        into.add(pred.key)
    else:
        for arg in pred.args:
            _atoms(arg, into)


def evaluate(pred: Pred, assignment: dict[str, bool]) -> bool:
    """Evaluates a predicate under a truth assignment of its atoms."""
    if pred.op == "atom":
        return assignment[pred.key]
    if pred.op == "all":
        return all(evaluate(arg, assignment) for arg in pred.args)
    if pred.op == "any":
        return any(evaluate(arg, assignment) for arg in pred.args)
    return not evaluate(pred.args[0], assignment)


def is_test_only(pred: Pred) -> bool:
    """Tells whether a predicate can only hold in a test build.

    A predicate is test-only when every assignment that satisfies it also sets `test` or
    `feature = "test"`. For example `all(test, feature = "std")` is test-only, whereas
    `any(feature = "syscall", test)` and `not(test)` are not.
    """
    atoms: set[str] = set()
    _atoms(pred, atoms)
    atoms.update(TEST_ATOMS)
    ordered = sorted(atoms)
    if len(ordered) > _MAX_ATOMS:
        return False
    for values in itertools.product((False, True), repeat=len(ordered)):
        assignment = dict(zip(ordered, values))
        if evaluate(pred, assignment) and not any(assignment[a] for a in TEST_ATOMS):
            return False
    return True
