#!/usr/bin/env python3

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Run the Nanvix standalone test suite against the OpenVMM-based ``nanvixd-vmm``.

This harness reads Nanvix's own ``test/test-standalone.toml`` and executes the
``terminal`` executor test cases against the ``nanvixd-vmm`` binary, which boots
the Nanvix guest on the OpenVMM virtualization stack while reusing the real
Nanvix host-side daemons (``hostfsd`` and ``networkd``).

Because it reuses the real host-side daemons, this runner also supports the
host filesystem mount (``-mount``) and host networking (``-allow-host-networking``)
cases, so those are executed rather than skipped.

Each case's input is bridged to the guest's stdin, the guest's stdout is
captured, and the result (and/or exit code) is compared against the expected
value. Cases requiring the HTTP executor or snapshots remain out of scope and
are skipped.

Usage:
    run-standalone-tests.py --nanvix-dir <path-to-nanvix-repo> \\
        [--vmm <path-to-nanvixd-vmm>] [--timeout <seconds>] [--verbose]
"""

from __future__ import annotations

import argparse
import subprocess
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path

# Executors this standalone runner does not implement.
UNSUPPORTED_EXECUTORS = ("http", "snapshot-restore", "snapshot-save-exit")


@dataclass
class Result:
    name: str
    status: str  # "pass", "fail", "skip"
    detail: str = ""


def build_initrd_args(
    program_args: str, program_env: str, padding_len: int
) -> str | None:
    """Builds the guest command-line tail (``"<args>;<env>"``)."""
    args = program_args
    if padding_len:
        # The cmdline-length test pads the argument string to a fixed size.
        pad = padding_len - len(args)
        if pad > 0:
            args = args + ("a" * pad if not args else " " + "a" * (pad - 1))
    if program_env:
        return f"{args};{program_env}"
    if args:
        return args
    return None


def run_case(
    vmm: Path, nanvix_dir: Path, test: dict, timeout: int, verbose: bool
) -> Result:
    name = test.get("name") or test["program"]
    executor = test.get("executor", "")

    if executor in UNSUPPORTED_EXECUTORS:
        return Result(name, "skip", f"executor '{executor}' not supported")

    program_args = test.get("program_args", "")
    program_env = test.get("program_env", "")
    # Literal ';' in args/env needs an escaping scheme this harness does not model.
    if ";" in program_args or ";" in program_env:
        return Result(name, "skip", "escaped-semicolon argument case")

    program = nanvix_dir / test["program"].lstrip("./")
    if not program.exists():
        return Result(name, "skip", f"missing program {program}")

    initrd_args = build_initrd_args(
        program_args, program_env, test.get("program_args_padding_len", 0)
    )

    # Run from the Nanvix directory so the relative `./bin/...` paths in the
    # test's `extra_nanvixd_args` (e.g. `-mount ./bin/mount-test-data`) resolve
    # exactly as the native test runner expects.
    cmd = [str(vmm), "-bin-dir", "./bin"]
    extra = test.get("extra_nanvixd_args", "")
    if extra:
        cmd += extra.split()
    cmd += ["--", "./" + test["program"].lstrip("./")]
    if initrd_args is not None:
        cmd.append(initrd_args)

    # The terminal executor feeds `input` to the guest's stdin.
    stdin_data = test.get("input", "")
    if isinstance(stdin_data, list):
        stdin_data = ""

    if verbose:
        print(f"    $ {' '.join(cmd)}", file=sys.stderr)

    try:
        proc = subprocess.run(
            cmd,
            input=stdin_data.encode(),
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            timeout=timeout,
            cwd=str(nanvix_dir),
        )
    except subprocess.TimeoutExpired:
        return Result(name, "fail", f"timed out after {timeout}s")

    stdout = proc.stdout.decode(errors="replace")
    exit_code = proc.returncode

    if "expected_exit_code" in test and exit_code != test["expected_exit_code"]:
        return Result(
            name, "fail", f"exit {exit_code} != expected {test['expected_exit_code']}"
        )

    if test.get("expect_empty_output"):
        if stdout.strip():
            return Result(name, "fail", f"expected empty output, got {stdout!r}")
    elif "expected_output" in test:
        if stdout.strip() != test["expected_output"].strip():
            return Result(
                name,
                "fail",
                f"output {stdout.strip()!r} != expected "
                f"{test['expected_output'].strip()!r}",
            )

    return Result(name, "pass", f"exit={exit_code}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--nanvix-dir",
        type=Path,
        default=None,
        help="Path to the Nanvix repository (with built bin/).",
    )
    parser.add_argument(
        "--vmm", type=Path, default=None, help="Path to the nanvixd-vmm binary."
    )
    parser.add_argument(
        "--timeout", type=int, default=60, help="Per-test timeout in seconds."
    )
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()

    # Default the Nanvix directory to the workspace root that contains this crate.
    nanvix_dir = (args.nanvix_dir or Path(__file__).resolve().parents[3]).resolve()
    kernel = nanvix_dir / "bin" / "kernel.elf"
    if not kernel.exists():
        print(
            f"error: kernel not found at {kernel}; build Nanvix first", file=sys.stderr
        )
        return 2

    vmm = (args.vmm or (nanvix_dir / "target" / "debug" / "nanvixd-vmm")).resolve()
    if not vmm.exists():
        print(
            f"error: nanvixd-vmm not found at {vmm}; run "
            f"`cargo build -p nanvixd-vmm`",
            file=sys.stderr,
        )
        return 2

    toml_path = nanvix_dir / "test" / "test-standalone.toml"
    with toml_path.open("rb") as f:
        config = tomllib.load(f)
    tests = config.get("tests", [])

    print("=== standalone deployment via nanvixd-vmm (reusing hostfsd/networkd) ===")
    results: list[Result] = []
    for test in tests:
        if test.get("executor") != "terminal":
            continue
        result = run_case(vmm, nanvix_dir, test, args.timeout, args.verbose)
        results.append(result)
        symbol = {"pass": "PASS", "fail": "FAIL", "skip": "SKIP"}[result.status]
        print(f"[{symbol}] {result.name}  ({result.detail})")

    passed = sum(1 for r in results if r.status == "pass")
    failed = sum(1 for r in results if r.status == "fail")
    skipped = sum(1 for r in results if r.status == "skip")
    print(f"\nnanvixd-vmm: {passed} passed, {failed} failed, {skipped} skipped")

    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
