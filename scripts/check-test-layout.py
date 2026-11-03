#!/usr/bin/env python3
# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""
Checks that tests live in their own files instead of mainline source code.

The policy (rules R1-R6) is described in `doc/test.md`. Violations that predate the policy are
recorded in `scripts/test-layout-baseline.txt`; the check fails on new violations, on baselined
violations that grew, and on baseline entries that no longer apply, so the baseline can only shrink.

Usage:
    check-test-layout.py [--root DIR] [--format text|github]
    check-test-layout.py --update-baseline    # drop fixed entries from the baseline
    check-test-layout.py --print-baseline     # print every current violation as baseline lines
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from testlayout import baseline as bl  # noqa: E402
from testlayout import policy  # noqa: E402
from testlayout.paths import ALLOWLIST_FILE, BASELINE_FILE  # noqa: E402


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
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument(
        "--update-baseline", action="store_true", help="drop fixed baseline entries"
    )
    mode.add_argument(
        "--print-baseline", action="store_true", help="print current violations"
    )
    args = parser.parse_args(argv)
    root: Path = args.root.resolve()

    if args.print_baseline:
        current = bl.aggregate(policy.collect_violations(root))
        entries = {key: agg.value for key, agg in current.items()}
        sys.stdout.write(bl.dump(entries, policy.BASELINE_HEADER))
        return 0

    try:
        report = policy.check(root)
    except bl.BaselineError as err:
        print(f"error: {err}", file=sys.stderr)
        return 2

    if args.update_baseline:
        before, after = policy.shrink_baseline_file(root)
        print(f"{BASELINE_FILE}: {before} -> {after} entries")
        report = policy.check(root)

    _print_findings(report, args.format)
    if report.findings:
        print(
            f"\ntest-layout: {len(report.findings)} problem(s). Tests must live in their own files "
            f"(see doc/test.md); seams are listed in {ALLOWLIST_FILE}.",
            file=sys.stderr,
        )
        return 1
    print(
        f"test-layout: OK ({len(report.baseline)} baselined, {len(report.allowlist)} allowed)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
