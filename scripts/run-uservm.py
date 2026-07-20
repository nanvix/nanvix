#!/usr/bin/env python3

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Runs Nanvix kernel tests via the standalone UserVM.

Boots the standalone UserVM with the given kernel binary, enforces a timeout,
and optionally verifies that a magic string appears in the output.

Usage:
    python scripts/run-uservm.py <kernel> <timeout> --wait-for-string <string>

Arguments:
    kernel   Path to the kernel ELF binary.
    timeout  Timeout in seconds.

Options:
    --wait-for-string <s>  Verify that <s> appears in the captured output.

Environment:
    USERVM  Path to uservm binary (default: ./bin/uservm.elf on Linux,
            .\\bin\\uservm.exe on Windows).
"""

import argparse
import os
import platform
import subprocess
import sys


def default_uservm_path() -> str:
    """Return the platform-appropriate default UserVM binary path."""
    if platform.system() == "Windows":
        return os.path.join(".", "bin", "uservm.exe")
    return os.path.join(".", "bin", "uservm.elf")


def parse_args() -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser(
        description="Run Nanvix kernel tests via the standalone UserVM."
    )
    parser.add_argument("kernel", help="Path to the kernel ELF binary.")
    parser.add_argument("timeout", type=int, help="Timeout in seconds.")
    parser.add_argument(
        "--wait-for-string",
        dest="wait_for_string",
        default="",
        help="Monitor output for this string and verify it appears.",
    )
    parser.add_argument(
        "--kernel-args",
        dest="kernel_args",
        default="",
        help="Kernel arguments passed via -kernel-args to the UserVM.",
    )
    return parser.parse_args()


def validate(kernel: str, uservm: str, timeout: int) -> None:
    """Validate inputs; exit on failure."""
    if not os.path.isfile(kernel):
        print(f"[ERROR] Kernel file not found: {kernel}", file=sys.stderr)
        sys.exit(1)
    if not os.path.isfile(uservm):
        print(f"[ERROR] UserVM binary not found: {uservm}", file=sys.stderr)
        sys.exit(1)
    if timeout <= 0:
        print(
            f"[ERROR] Timeout must be a positive integer, got: {timeout}",
            file=sys.stderr,
        )
        sys.exit(1)


def run_uservm(
    kernel: str,
    uservm: str,
    timeout: int,
    wait_for_string: str,
    kernel_args: str,
) -> None:
    """Boot the standalone UserVM and check results."""
    cmd = [uservm, "-kernel", kernel]
    if kernel_args:
        cmd.extend(["-kernel-args", kernel_args])

    print("=" * 69)
    print(f"KERNEL   = {kernel}")
    print(f"USERVM   = {uservm}")
    print(f"TIMEOUT  = {timeout}")
    print(f"WAIT_FOR = {wait_for_string}")
    print("=" * 69)

    print(f"[INFO] Running: {' '.join(cmd)}")
    if wait_for_string:
        print(
            f"[INFO] Waiting for '{wait_for_string}' in output (timeout: {timeout}s)..."
        )

    try:
        result = subprocess.run(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as exc:
        print(f"[ERROR] Command timed out after {timeout}s.")
        if exc.stdout:
            sys.stdout.buffer.write(exc.stdout)
        sys.exit(124)

    output = result.stdout.decode(errors="replace")

    if result.returncode != 0:
        print(f"[ERROR] Command exited with code {result.returncode}.")
        print(output)
        sys.exit(result.returncode)

    if wait_for_string:
        if wait_for_string not in output:
            print(
                f"[ERROR] Expected output to contain '{wait_for_string}', "
                "but it was not found."
            )
            print(output)
            sys.exit(1)
        print(f"[SUCCESS] Output contains '{wait_for_string}'. Tests passed.")


def main() -> None:
    """Entry point."""
    args = parse_args()
    uservm = os.environ.get("USERVM", default_uservm_path())
    validate(args.kernel, uservm, args.timeout)
    run_uservm(
        args.kernel, uservm, args.timeout, args.wait_for_string, args.kernel_args
    )


if __name__ == "__main__":
    main()
