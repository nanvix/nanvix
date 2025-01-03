# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

# ======================================================================================================================
# Imports
# ======================================================================================================================

import subprocess
import argparse
from typing import List, Tuple

# ======================================================================================================================
# Constants
# ======================================================================================================================

# List of supported target machines.
TARGET_MACHINES: List[str] = ["qemu-isapc",
                              "qemu-pc", "qemu-baremetal", "microvm", "hyperlight"]

# List of supported target architectures.
TARGET_ARCHS: List[str] = ["x86"]

# List of supported log levels.
LOG_LEVELS: List[str] = ["trace", "debug", "info", "warning", "error"]


# ======================================================================================================================
# Standalone Functions
# ======================================================================================================================


def run_command(command: List[str], stdout_log_file: str, stderr_log_file: str) -> int:
    """
    Runs a command and tees the output to separate files for stdout and stderr.

    Args:
        command (List[str]): Command to run.
        stdout_log_file (str): Path to the stdout log file.
        stderr_log_file (str): Path to the stderr log file.

    Returns:
        int: Command return code.
    """

    # Echo command.
    print(f"Running command: {' '.join(command)}")

    with open(stdout_log_file, 'w') as stdout_file, open(stderr_log_file, 'w') as stderr_file:
        process = subprocess.Popen(
            command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

        # Poll stdout and stderr
        while True:
            stdout = process.stdout.readline()
            stderr = process.stderr.readline()

            if process.poll() is not None and stdout == '' and stderr == '':
                break

            if stdout:
                print(stdout.strip())
                stdout_file.write(stdout)

            if stderr:
                print(stderr.strip())
                stderr_file.write(stderr)

        return_code = process.poll()

    return return_code


def make(target: str, machine: str, arch: str, release: bool, toolchain_dir: str = None, log_level: str = None, verbose: bool = False, timeout: int = None) -> None:
    """
    Runs make command.

    Args:
        target (str): Make target.
        machine (str): Target machine.
        arch (str): Target architecture.
        release (bool): Release build.
        toolchain_dir (str, optional): Toolchain directory. Defaults to None.
        log_level (str, optional): Log level. Defaults to None.
        verbose (bool, optional): Verbose build. Defaults to False.
        timeout (int, optional): Timeout. Defaults to None.
    """

    command = ["make", target, f"MACHINE={machine}", f"TARGET={arch}"]

    if log_level:
        command.append(f"LOG_LEVEL={log_level}")

    if toolchain_dir:
        command.append(f"TOOLCHAIN_DIR={toolchain_dir}")

    if verbose:
        command.append("VERBOSE=yes")

    if release:
        command.append("RELEASE=yes")

    if timeout:
        command.append(f"TIMEOUT={timeout}")

    return_code = run_command(
        command, f"{target}-stdout.log", f"{target}-stderr.log")

    if return_code != 0:
        print("Make failed.")
        exit(1)


def lint(machine: str, arch: str, release: bool, toolchain_dir: str = None, log_level: str = None, verbose: bool = False) -> None:
    """
    Lints Nanvix source code.

    Args:
        machine (str): Target machine.
        arch (str): Target architecture.
        release (bool): Release build.
        toolchain_dir (str, optional): Toolchain directory. Defaults to None.
        log_level (str, optional): Log level. Defaults to None.
        verbose (bool, optional): Verbose build. Defaults to False.
    """

    make("clippy", machine, arch, release, toolchain_dir, log_level, verbose)


def build(machine: str, arch: str, release: bool, toolchain_dir: str = None, log_level: str = None, verbose: bool = False) -> None:
    """
    Builds Nanvix for a target machine and architecture.

    Args:
        machine (str): Target machine.
        arch (str): Target architecture.
        release (bool): Release build.
        toolchain_dir (str, optional): Toolchain directory. Defaults to None.
        log_level (str, optional): Log level. Defaults to None.
        verbose (bool, optional): Verbose build. Defaults to False.
    """

    make("all", machine, arch, release, toolchain_dir, log_level, verbose)


def test(machine: str, arch: str, release: bool, toolchain_dir: str = None, log_level: str = None, verbose: bool = False, timeout: int = None) -> None:
    """
    Tests Nanvix for a target machine and architecture.

    Args:
        machine (str): Target machine.
        arch (str): Target architecture.
        release (bool): Release build.
        toolchain_dir (str, optional): Toolchain directory. Defaults to None.
        log_level (str, optional): Log level. Defaults to None.
        verbose (bool, optional): Verbose build. Defaults to False.
        timeout (int, optional): Timeout. Defaults to None.
    """

    make("run", machine, arch, release,
         toolchain_dir, log_level, verbose, timeout)

    # Check if last line of "test-stdout.log" contains magic string "hello, world!".
    with open("run-stdout.log", "r") as file:
        lines = file.readlines()
        last_line = lines[-1]

        # Check if last line contains magic string.
        if "hello, world!" not in last_line:
            print("last line:", last_line)
            print("Test failed.")
            exit(1)


def parse_args() -> argparse.Namespace:
    """
    Parses command-line arguments.

    Returns:
        argparse.Namespace: The parsed arguments.
    """

    parser = argparse.ArgumentParser(
        description="A simple CLI program.", allow_abbrev=False)

    # Required arguments.
    parser.add_argument("--target-machine", type=str,
                        help=f"Set target machine {TARGET_MACHINES}", required=True)
    parser.add_argument("--target-arch", type=str,
                        help=f"Set target architecture {TARGET_ARCHS}", required=True)

    # Optional arguments.
    parser.add_argument("--release", action="store_true",
                        help="Build in release mode", default=False)
    parser.add_argument("--toolchain-dir", type=str,
                        help="Set toolchain directory")
    parser.add_argument("--log-level", type=str,
                        help=f"Set log level {LOG_LEVELS}", default="trace")
    parser.add_argument("--verbose", action="store_true",
                        help="Enable verbose build", default=True)
    parser.add_argument("--timeout", type=int, help="Set test timeout")
    parser.add_argument("--lint", action="store_true",
                        help="Lint Nanvix source code", default=False)
    parser.add_argument("--build", action="store_true",
                        help="Build Nanvix", default=False)
    parser.add_argument("--test", action="store_true",
                        help="Test Nanvix (implies --build)", default=False)

    return parser.parse_args()


def main() -> None:
    args = parse_args()

    # Print arguments.
    print("Arguments:")
    print(f"  - Target machine: {args.target_machine}")
    print(f"  - Target architecture: {args.target_arch}")
    print(f"  - Toolchain directory: {args.toolchain_dir}")
    print(f"  - Log level: {args.log_level}")
    print(f"  - Release: {args.release}")
    print(f"  - Lint: {args.lint}")
    print(f"  - Build: {args.build}")
    print(f"  - Verbose: {args.verbose}")
    print(f"  - Timeout: {args.timeout}")

    # Lint source code.
    if args.lint:
        lint(args.target_machine, args.target_arch, args.release, args.toolchain_dir,
             args.log_level, args.verbose)

    # Build source code.
    if args.build or args.test:
        build(args.target_machine, args.target_arch, args.release,
              args.toolchain_dir, args.log_level, args.verbose)

    # Test Nanvix.
    if args.test:
        test(args.target_machine, args.target_arch, args.release,
             args.toolchain_dir, args.log_level, args.verbose, args.timeout)


if __name__ == "__main__":
    main()
