# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.


# ======================================================================================================================
# Imports
# ======================================================================================================================

import sys
import subprocess
import time
import requests
import os
import signal
import argparse
import argparse
import signal
from datetime import timedelta

# ======================================================================================================================
# Constants
# ======================================================================================================================

# Default timeout for a request. If a timeout is reached, execution will be aborted.
TIMEOUT = 1

# Default number of requests to send.
NREQUESTS = 1000

# Default core affinity for nanvixd.
CORE_AFFINITY = "0-3"

# ======================================================================================================================
# Standalone Functions
# ======================================================================================================================


def parse_args() -> argparse.Namespace:
    """
    Parses command-line arguments.

    Returns:
        argparse.Namespace: The parsed arguments.
    """

    parser = argparse.ArgumentParser(
        description="CLI for benchmarking Nanvix.", allow_abbrev=False)

    # Required arguments.
    parser.add_argument("--nanvixd-sockaddr", type=str,
                        help="Set socket address for nanvixd", required=True)
    parser.add_argument("--linuxd-sockaddr", type=str,
                        help="Set socket address for linuxd", required=True)
    parser.add_argument("--sandbox-sockaddr", type=str,
                        help="Set socket address for sandbox", required=True)
    parser.add_argument("--program-name", type=str,
                        help="Set program name", required=True)
    parser.add_argument("--program-args", type=str,
                        help="Set program args", required=True)

    # Optional arguments.
    parser.add_argument("--timeout", type=int,
                        help="Set test request timeout", default=TIMEOUT)
    parser.add_argument("--nrequests", type=int,
                        help="Set number of requests to send", default=NREQUESTS)
    parser.add_argument("--core-affinity", type=str,
                        help="Set core affinity for nanvixd", default=CORE_AFFINITY)

    return parser.parse_args()


def cleanup_socket_files(nanvixd_sockaddr: str, linuxd_sockaddr: str, sandbox_sockaddr: str) -> None:
    """
    Removes socket files.

    Args:
        nanvixd_sockaddr (str): Nanvixd socket address.
        linuxd_sockaddr (str): Linuxd socket address.
        sandbox_sockaddr (str): Sandbox socket address.
    """

    subprocess.run(["sudo", "rm", "-f", f"/tmp/{nanvixd_sockaddr}*.socket"])
    subprocess.run(["sudo", "rm", "-f", f"/tmp/{linuxd_sockaddr}*.socket"])
    subprocess.run(["sudo", "rm", "-f", f"/tmp/{sandbox_sockaddr}*.socket"])


def kill_nanvixd(nanvixd_process: subprocess.Popen) -> None:
    """
    Kills nanvixd process.

    Args:
        nanvixd_process (subprocess.Popen): Nanvixd process.
    """

    print(f"Killing nanvixd (pid={nanvixd_process.pid})")
    subprocess.run(["sudo", "/usr/bin/kill", "-s",
                   "SIGINT", f"{nanvixd_process.pid}"])
    print(f"Killed nanvixd (pid={nanvixd_process.pid})")


def print_progress_bar(iteration: int, total: int, prefix: str = "", length: int = 50, fill: str = "#") -> None:
    """
    Prints a progress bar to the console.

    Args:
        iteration (int): Current iteration.
        total (int): Total iterations.
        prefix (str, optional): Prefix string. Defaults to "".
        length (int, optional): Character length of bar. Defaults to 50.
        fill (str, optional): Bar fill character. Defaults to "█".
    """

    percent = ("{0:.1f}").format(100 * (iteration / float(total)))
    filled_length = int(length * iteration // total)
    bar = fill * filled_length + "-" * (length - filled_length)
    print(f"\r{prefix} |{bar}| {percent}%", end="\r")
    if iteration == total:
        print()


def abort_execution(nanvixd_process: subprocess.Popen = None, nanvixd_sockaddr: str = "", linuxd_sockaddr: str = "", sandbox_sockaddr: str = "") -> None:
    """
    Aborts execution.

    Args:
        nanvixd_process (subprocess.Popen, optional): Nanvixd process.
        nanvixd_sockaddr (str, optional): Nanvixd socket address.
        linuxd_sockaddr (str, optional): Linuxd socket address.
        sandbox_sockaddr (str, optional): Sandbox socket address.
    """
    if nanvixd_process:
        kill_nanvixd(nanvixd_process)
    if nanvixd_sockaddr or linuxd_sockaddr or sandbox_sockaddr:
        cleanup_socket_files(
            nanvixd_sockaddr, linuxd_sockaddr, sandbox_sockaddr)
    sys.exit(1)


def main() -> None:
    args = parse_args()

    # Print arguments.
    print("Arguments:")
    print(f"  - Nanvixd socket address: {args.nanvixd_sockaddr}")
    print(f"  - Linuxd socket address: {args.linuxd_sockaddr}")
    print(f"  - Sandbox socket address: {args.sandbox_sockaddr}")
    print(f"  - Program name: {args.program_name}")
    print(f"  - Program args: {args.program_args}")
    print(f"  - Timeout: {args.timeout}")
    print(f"  - Number of Requests: {args.nrequests}")
    print(f"  - Core Affinity: {args.core_affinity}")

    # Extract arguments.
    nanvixd_sockaddr = args.nanvixd_sockaddr
    linuxd_sockaddr = args.linuxd_sockaddr
    sandbox_sockaddr = args.sandbox_sockaddr
    program_name = args.program_name
    program_args = args.program_args
    nanvixd_port_number = nanvixd_sockaddr.split(":")[1]
    timeout = args.timeout
    nrequests = args.nrequests
    core_affinity = args.core_affinity

    # Check if program exists.
    if not os.path.exists(program_name):
        print(f"Program not found at {program_name}")
        abort_execution()

    # Request sudo permissions.
    os.system("sudo -v")

    nanvixd_path = os.path.join(os.getcwd(), "bin/nanvixd.elf")

    # Install signal handler for cleaning up state.
    def signal_handler(sig, frame):
        print(f"Caught signal {sig}")
        abort_execution(nanvixd_process, nanvixd_sockaddr,
                        linuxd_sockaddr, sandbox_sockaddr)

    signal.signal(signal.SIGINT, signal_handler)

    # Check if nanvixd exists.
    if not os.path.exists(nanvixd_path):
        print(f"nanvixd not found at {nanvixd_path}")
        abort_execution()

    command = [
        "sudo", "-E",
        "nice", "-n", "-20",
        "taskset", "-a", "-c", core_affinity,
        nanvixd_path,
        "-http-addr", nanvixd_sockaddr,
        "-linuxd-addr", linuxd_sockaddr,
        "-sandbox-addr", sandbox_sockaddr,
        "-keep-alive", "0"
    ]

    # Run nanvixd. Note we set process group to be able to kill all processes
    # that are spawned by nanvixd itself.
    print(f"Running: {' '.join(command)}")
    nanvixd_process = subprocess.Popen(
        command,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        preexec_fn=os.setpgrp,
    )

    # Wait for nanvixd to start.
    time.sleep(0.5)

    elapsed_times = []

    for i in range(nrequests):
        print_progress_bar(i + 1, nrequests, prefix="Progress:")
        # Run a client.
        try:
            response = requests.post(
                f"http://localhost:{nanvixd_port_number}",
                headers={"Content-Type": "application/json"},
                json={"clientid": 1, "program": program_name,
                      "args": program_args.split()},
                timeout=timeout
            )
        except requests.Timeout:
            print("Request timed out, aborting execution.")
            abort_execution(nanvixd_process, nanvixd_sockaddr,
                            linuxd_sockaddr, sandbox_sockaddr)

        # Check if response is successful.
        if not response.ok:
            print(f"Request failed with status code {response.status_code}")
            abort_execution(nanvixd_process, nanvixd_sockaddr,
                            linuxd_sockaddr, sandbox_sockaddr)
        else:
            elapsed_times.append(response.elapsed / timedelta(microseconds=1))

    # Exclude the first entry from elapsed times.
    first_elapsed_time = elapsed_times[0]
    elapsed_times = elapsed_times[1:]

    # Calculate p50, p90, and p99.
    elapsed_times.sort()
    p50 = elapsed_times[int(0.50 * len(elapsed_times))]
    p90 = elapsed_times[int(0.90 * len(elapsed_times))]
    p99 = elapsed_times[int(0.99 * len(elapsed_times))]

    # Print results.
    print(f"1st elapsed time: {first_elapsed_time} us")
    print(f"p50 elapsed time: {p50} us")
    print(f"p90 elapsed time: {p90} us")
    print(f"p99 elapsed time: {p99} us")

    kill_nanvixd(nanvixd_process)
    cleanup_socket_files(
        nanvixd_sockaddr, linuxd_sockaddr, sandbox_sockaddr)


if __name__ == "__main__":
    main()
