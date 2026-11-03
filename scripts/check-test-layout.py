#!/usr/bin/env python3
# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""
Checks that tests live in their own files instead of mainline source code.

The policy (rules R1-R6) is described in `doc/test.md`. Every violation fails the check. The only
exceptions are test-only seams (R3) that need private access to the code they test; they are listed,
with a reason, in `scripts/test-layout-allowlist.txt`, and an entry whose code is gone fails the
check too.

Usage:
    check-test-layout.py [--root DIR] [--format text|github]
    check-test-layout.py --print-allowlist    # print allowlist lines for the uncovered seams
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from testlayout import policy  # noqa: E402
from testlayout import violations as vl  # noqa: E402
from testlayout.paths import ALLOWLIST_FILE  # noqa: E402


def _escape(message: str) -> str:
    """Escapes a message for a GitHub Actions workflow command."""
    return message.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")


def _print_findings(report: policy.Report, fmt: str) -> None:
    for finding in report.findings:
        if fmt == "github":
            print(
                f"::error file={finding.path},line={finding.line},title=test-layout::"
                f"{_escape(finding.message)}"
            )
        else:
            print(f"{finding.path}:{finding.line}: {finding.message}")


def main(argv: list[str] | None = None) -> int:
    """Entry point."""
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parent.parent
    )
    parser.add_argument("--format", choices=("text", "github"), default="text")
    parser.add_argument(
        "--print-allowlist",
        action="store_true",
        help="print allowlist lines for the seams the allowlist does not cover",
    )
    args = parser.parse_args(argv)
    root: Path = args.root.resolve()

    try:
        report = policy.check(root)
    except vl.AllowlistError as err:
        print(f"error: {err}", file=sys.stderr)
        return 2

    if args.print_allowlist:
        for line in policy.uncovered_seams(report):
            print(line)
        return 0

    _print_findings(report, args.format)
    if report.findings:
        print(
            f"\ntest-layout: {len(report.findings)} problem(s). Tests must live in their own files "
            f"(see doc/test.md); seams are listed in {ALLOWLIST_FILE}.",
            file=sys.stderr,
        )
        return 1
    print(f"test-layout: OK ({len(report.allowlist)} allowed)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
