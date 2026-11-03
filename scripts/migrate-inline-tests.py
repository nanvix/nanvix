#!/usr/bin/env python3
# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""
Moves inline test modules out of mainline Rust source into their own files.

    #[cfg(all(test, feature = "std"))]
    mod tests { ... }          ->          #[cfg(all(test, feature = "std"))] mod tests;

The body goes to `tests.rs` next to the declaring file (see doc/test.md, 'Test layout'). Modules
that cannot be moved mechanically are reported and left in place; fix those by hand.

Usage:
    migrate-inline-tests.py PATH [PATH ...]      # file or directory, relative to the repository
    migrate-inline-tests.py --dry-run PATH ...   # report what would move
    migrate-inline-tests.py --verify REV         # prove the working tree only moved tests

After moving, the tool formats the touched files and drops the fixed entries from
`scripts/test-layout-baseline.txt`.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from testlayout import migrate, policy  # noqa: E402
from testlayout.paths import BASELINE_FILE  # noqa: E402


def _print_result(result: migrate.Result, applied: bool) -> None:
    verb = "moved" if applied else "would move"
    for plan in result.plans:
        for move in plan.moves:
            via = f' (#[path = "{move.path_attr}"])' if move.path_attr else ""
            print(f"{verb}: {plan.path}: mod {move.name} -> {move.target}{via}")
        for skip in plan.skips:
            print(f"skipped: {plan.path}:{skip.line}: mod {skip.name}: {skip.reason}")


def main(argv: list[str] | None = None) -> int:
    """Entry point."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("paths", nargs="*", help="files or directories to migrate")
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parent.parent
    )
    parser.add_argument("--dry-run", action="store_true", help="report without writing")
    parser.add_argument("--no-format", action="store_true", help="do not run rustfmt")
    parser.add_argument(
        "--verify", metavar="REV", help="check the tree is a pure migration of REV"
    )
    args = parser.parse_args(argv)
    root: Path = args.root.resolve()

    if args.verify:
        problems = migrate.verify(root, args.verify)
        for problem in problems:
            print(problem)
        if problems:
            print(
                f"\nmigrate-inline-tests: {len(problems)} difference(s) from {args.verify}"
            )
            return 1
        print(
            f"migrate-inline-tests: the tree is a pure test migration of {args.verify}"
        )
        return 0

    if not args.paths:
        parser.error("give at least one path (use '.' for the whole repository)")

    result = migrate.migrate(
        root, args.paths, apply=not args.dry_run, format_output=not args.no_format
    )
    _print_result(result, applied=not args.dry_run)
    print(
        f"migrate-inline-tests: {result.moved} module(s) moved, {result.skipped} skipped"
    )
    if result.unformatted:
        print(
            "warning: rustfmt is not available; format the touched files before committing"
        )
    if result.moved and not args.dry_run:
        before, after = policy.shrink_baseline_file(root)
        print(f"{BASELINE_FILE}: {before} -> {after} entries")
    return 1 if result.skipped else 0


if __name__ == "__main__":
    sys.exit(main())
