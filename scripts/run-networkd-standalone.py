#!/usr/bin/env python3

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Runs a standalone UserVM against a decoupled ``networkd`` process.

This launcher wires up the decoupled network daemon end-to-end: it starts the
``networkd`` host binary listening on its own socket, then boots a standalone
UserVM configured to forward guest socket system calls to that separate
process (``-allow-host-networking -networkd-addr ...``). The two processes are
fully independent and communicate only over the given socket, mirroring the
target architecture in which ``networkd`` may eventually run on a different
machine.

The guest program's exit code is the smoke-test verdict: standalone mode
drains and discards guest stdout, so a network test binary that returns
``Ok(())`` on success (exit code 0) and an error otherwise (non-zero) is the
recommended payload (e.g. ``./bin/test-rust-network.initrd``).

Usage:
    python scripts/run-networkd-standalone.py <kernel> <guest> <timeout> \
        [--networkd-addr <addr>] [--networkd-socket-type unix|tcp] \
        [--allow-host <cidr> ...] [--block-host <cidr> ...]

Arguments:
    kernel   Path to the kernel ELF binary.
    guest    Path to the guest program image loaded as the initrd.
    timeout  Timeout in seconds for the guest run.

Options:
    --networkd-addr <addr>          Address networkd binds to and the UserVM
                                    dials. Defaults to a temporary Unix socket.
    --networkd-socket-type <type>   ``unix`` (default) or ``tcp``.
    --allow-host <cidr>             Add an egress allowlist entry to networkd
                                    (repeatable). Mutually exclusive with
                                    --block-host.
    --block-host <cidr>             Add an egress blocklist entry to networkd
                                    (repeatable). Mutually exclusive with
                                    --allow-host.
    --kernel-args <args>            Kernel arguments passed via -kernel-args.

Environment:
    USERVM    Path to the uservm binary   (default: ./bin/uservm.elf).
    NETWORKD  Path to the networkd binary (default: ./bin/networkd.elf).

Note:
    Building the guest kernel and network test image requires the Nanvix cross
    toolchain (see doc/setup.md). Build everything with ``./z build -- all``
    (DEPLOYMENT_MODE=standalone) before running this launcher.
"""

import argparse
import os
import subprocess
import sys
import tempfile
import time


def parse_args() -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser(
        description="Run a standalone UserVM against a decoupled networkd process."
    )
    parser.add_argument("kernel", help="Path to the kernel ELF binary.")
    parser.add_argument("guest", help="Path to the guest program image (initrd).")
    parser.add_argument("timeout", type=int, help="Guest run timeout in seconds.")
    parser.add_argument(
        "--networkd-addr",
        dest="networkd_addr",
        default="",
        help="Address networkd binds to and the UserVM dials (default: temp Unix socket).",
    )
    parser.add_argument(
        "--networkd-socket-type",
        dest="networkd_socket_type",
        default="unix",
        choices=["unix", "tcp"],
        help="Socket type used to reach networkd (default: unix).",
    )
    parser.add_argument(
        "--allow-host",
        dest="allow_hosts",
        action="append",
        default=[],
        help="Egress allowlist entry for networkd (repeatable).",
    )
    parser.add_argument(
        "--block-host",
        dest="block_hosts",
        action="append",
        default=[],
        help="Egress blocklist entry for networkd (repeatable).",
    )
    parser.add_argument(
        "--kernel-args",
        dest="kernel_args",
        default="",
        help="Kernel arguments passed via -kernel-args to the UserVM.",
    )
    return parser.parse_args()


def validate(args: argparse.Namespace, uservm: str, networkd: str) -> None:
    """Validate inputs; exit on failure."""
    for label, path in (("Kernel", args.kernel), ("Guest", args.guest)):
        if not os.path.isfile(path):
            print(f"[ERROR] {label} file not found: {path}", file=sys.stderr)
            sys.exit(1)
    for label, path in (("UserVM", uservm), ("networkd", networkd)):
        if not os.path.isfile(path):
            print(f"[ERROR] {label} binary not found: {path}", file=sys.stderr)
            sys.exit(1)
    if args.timeout <= 0:
        print(f"[ERROR] Timeout must be positive, got: {args.timeout}", file=sys.stderr)
        sys.exit(1)
    if args.allow_hosts and args.block_hosts:
        print(
            "[ERROR] --allow-host and --block-host are mutually exclusive.",
            file=sys.stderr,
        )
        sys.exit(1)


# Substring networkd logs once it is bound and listening, immediately before it accepts the
# single user VM connection (see src/daemons/networkd/src/main.rs). Watching for this marker is
# non-intrusive: unlike a probing connect(), it does not consume networkd's one-and-only accept
# slot, which the real UserVM must be the one to claim.
NETWORKD_READY_MARKER = "listening for the user VM"


def read_log(log_path: str) -> str:
    """Return the current contents of the networkd log file (best effort)."""
    try:
        with open(log_path, "r", errors="replace") as log:
            return log.read()
    except OSError:
        return ""


def wait_for_networkd(proc: subprocess.Popen, log_path: str, deadline: float) -> bool:
    """Wait until networkd reports it is listening, without connecting to it.

    Connecting to probe readiness would be claimed by networkd as its single accepted user VM
    connection, so the readiness signal is taken from its log instead.
    """
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            return False
        if NETWORKD_READY_MARKER in read_log(log_path):
            return True
        time.sleep(0.1)
    return False


def start_networkd(
    networkd: str, addr: str, socket_type: str, allow_hosts, block_hosts, log_file
) -> subprocess.Popen:
    """Launch networkd, streaming its output to ``log_file``, and return the process handle."""
    cmd = [
        networkd,
        "-user-vm-bind-addr",
        addr,
        "-user-vm-bind-socket-type",
        socket_type,
    ]
    for entry in allow_hosts:
        cmd.extend(["-allow-host", entry])
    for entry in block_hosts:
        cmd.extend(["-block-host", entry])

    print(f"[INFO] Starting networkd: {' '.join(cmd)}")
    env = dict(os.environ)
    env.setdefault("RUST_LOG", "info")
    return subprocess.Popen(
        cmd,
        stdout=log_file,
        stderr=subprocess.STDOUT,
        env=env,
    )


def run_uservm(
    uservm: str,
    kernel: str,
    guest: str,
    addr: str,
    socket_type: str,
    kernel_args: str,
    timeout: int,
) -> subprocess.CompletedProcess:
    """Boot the standalone UserVM connected to the decoupled networkd."""
    cmd = [
        uservm,
        "-standalone",
        "-kernel",
        kernel,
        "-initrd",
        guest,
        "-allow-host-networking",
        "-networkd-addr",
        addr,
        "-networkd-socket-type",
        socket_type,
    ]
    if kernel_args:
        cmd.extend(["-kernel-args", kernel_args])

    print(f"[INFO] Running UserVM: {' '.join(cmd)}")
    return subprocess.run(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        timeout=timeout,
    )


def stop_networkd(proc: subprocess.Popen, addr: str, socket_type: str) -> None:
    """Terminate networkd and clean up its socket."""
    if proc.poll() is None:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()
    if socket_type == "unix" and os.path.exists(addr):
        try:
            os.unlink(addr)
        except OSError:
            pass


def main() -> None:
    """Entry point."""
    args = parse_args()
    uservm = os.environ.get("USERVM", os.path.join(".", "bin", "uservm.elf"))
    networkd = os.environ.get("NETWORKD", os.path.join(".", "bin", "networkd.elf"))
    validate(args, uservm, networkd)

    addr = args.networkd_addr
    if not addr:
        if args.networkd_socket_type != "unix":
            print(
                "[ERROR] --networkd-addr is required for non-Unix socket types.",
                file=sys.stderr,
            )
            sys.exit(1)
        addr = os.path.join(tempfile.gettempdir(), f"networkd-{os.getpid()}.sock")
        if os.path.exists(addr):
            os.unlink(addr)

    print("=" * 69)
    print(f"KERNEL    = {args.kernel}")
    print(f"GUEST     = {args.guest}")
    print(f"NETWORKD  = {networkd} @ {addr} ({args.networkd_socket_type})")
    print(f"USERVM    = {uservm}")
    print(f"TIMEOUT   = {args.timeout}")
    print("=" * 69)

    networkd_log_fd, networkd_log_path = tempfile.mkstemp(
        prefix="networkd-", suffix=".log"
    )
    networkd_log_file = os.fdopen(networkd_log_fd, "w")
    networkd_proc = start_networkd(
        networkd,
        addr,
        args.networkd_socket_type,
        args.allow_hosts,
        args.block_hosts,
        networkd_log_file,
    )

    exit_code = 1
    try:
        deadline = time.monotonic() + 10
        if not wait_for_networkd(networkd_proc, networkd_log_path, deadline):
            print("[ERROR] networkd did not start listening in time.")
            print(read_log(networkd_log_path))
            sys.exit(1)
        print("[INFO] networkd is listening; booting the guest...")

        try:
            result = run_uservm(
                uservm,
                args.kernel,
                args.guest,
                addr,
                args.networkd_socket_type,
                args.kernel_args,
                args.timeout,
            )
        except subprocess.TimeoutExpired as exc:
            print(f"[ERROR] UserVM timed out after {args.timeout}s.")
            if exc.stdout:
                sys.stdout.buffer.write(exc.stdout)
            print("--- networkd log ---")
            print(read_log(networkd_log_path))
            sys.exit(124)

        output = result.stdout.decode(errors="replace")
        exit_code = result.returncode
        if exit_code == 0:
            print(
                "[SUCCESS] Guest exited cleanly; decoupled networkd smoke test passed."
            )
        else:
            print(f"[ERROR] Guest exited with code {exit_code}.")
            print(output)
            print("--- networkd log ---")
            print(read_log(networkd_log_path))
    finally:
        stop_networkd(networkd_proc, addr, args.networkd_socket_type)
        networkd_log_file.close()
        try:
            os.unlink(networkd_log_path)
        except OSError:
            pass

    sys.exit(exit_code)


if __name__ == "__main__":
    main()
