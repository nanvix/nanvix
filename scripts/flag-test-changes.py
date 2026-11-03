#!/usr/bin/env python3
# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""
Flags pull requests that change existing tests.

Adding tests is never flagged. Changing, removing or renaming an existing test, or growing the
test layout allowlist, is reported so that a reviewer looks at it carefully. The check
only reads git objects; it never builds or runs anything from the pull request. It is advisory, so
the exit status is 0 whether or not anything was flagged (see `doc/test.md`).

Usage:
    flag-test-changes.py --base REV [--head REV] [--root DIR]
                         [--json [FILE]] [--markdown [FILE]] [--step-summary]
                         [--github-output]
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from testlayout import flagging, gitdiff  # noqa: E402


def _write(target: str, content: str) -> None:
    """Writes `content` to a file, or to the standard output when `target` is `-`."""
    if target == "-":
        sys.stdout.write(content)
        return
    with open(target, "w", encoding="utf-8", newline="\n") as handle:
        handle.write(content)


def _append_env_file(variable: str, content: str) -> None:
    """Appends to the file a GitHub Actions environment variable points to, if it is set."""
    path = os.environ.get(variable)
    if not path:
        return
    with open(path, "a", encoding="utf-8", newline="\n") as handle:
        handle.write(content)


def main(argv: list[str] | None = None) -> int:
    """Entry point."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parent.parent
    )
    parser.add_argument(
        "--base", required=True, help="revision the pull request targets"
    )
    parser.add_argument(
        "--head", default="HEAD", help="revision of the pull request (default: HEAD)"
    )
    parser.add_argument(
        "--json",
        nargs="?",
        const="-",
        metavar="FILE",
        help="write the report as JSON (to the standard output without FILE)",
    )
    parser.add_argument(
        "--markdown",
        nargs="?",
        const="-",
        metavar="FILE",
        help="write the pull request comment (to the standard output without FILE)",
    )
    parser.add_argument(
        "--step-summary",
        action="store_true",
        help="append the full report to the file named by $GITHUB_STEP_SUMMARY",
    )
    parser.add_argument(
        "--github-output",
        action="store_true",
        help="append `flagged` and `count` to the file named by $GITHUB_OUTPUT",
    )
    args = parser.parse_args(argv)

    try:
        report = flagging.analyze(args.root.resolve(), args.base, args.head)
    except gitdiff.GitError as err:
        print(f"error: {err}", file=sys.stderr)
        return 2

    if args.json is not None:
        _write(args.json, json.dumps(report.to_json(), indent=2) + "\n")
    if args.markdown is not None:
        _write(args.markdown, flagging.render_markdown(report))
    if args.json is None and args.markdown is None:
        sys.stdout.write(flagging.render_text(report))
    if args.step_summary:
        _append_env_file(
            "GITHUB_STEP_SUMMARY",
            flagging.render_markdown(report, limit=None, marker=False),
        )
    if args.github_output:
        _append_env_file(
            "GITHUB_OUTPUT",
            f"flagged={'true' if report.flagged else 'false'}\n"
            f"count={len(report.findings)}\n",
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
