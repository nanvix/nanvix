#!/usr/bin/env python3
# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""
Unified smoke-test driver for nanvixd (Windows + Linux).

Boots a system image under nanvixd and verifies correct behavior. In debug
mode it validates both the kernel magic string and the process exit code; in
release mode it validates the process exit code only (debug-level console
output, including the magic string, is compiled out of release builds).

On Linux, nanvixd is launched against cloud-hypervisor via -clh-bin-path and
the kernel console is wired to nanvixd's stdout via `-console-file /dev/stdout`.
On Windows, nanvixd uses the Windows Hypervisor Platform (WHP) backend directly
(no clh-bin-path) and the kernel console is written to a file that is tailed
to our stdout.

Usage:
    run-smoke-test.py <machine> <image> [--timeout N]
                      [--magic-string STR] [--expected-exit-code N]
                      [--release]
"""

from __future__ import annotations

import argparse
import os
import signal
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import IO


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Unified nanvixd smoke-test driver.")
    p.add_argument("machine", help="Machine type (microvm).")
    p.add_argument("image", help="Path to the multibin system image.")
    p.add_argument(
        "--timeout", type=int, default=120, help="Timeout in seconds (default: 120)."
    )
    p.add_argument(
        "--magic-string",
        default="hello, world!",
        help="Kernel magic string to look for in debug mode.",
    )
    p.add_argument(
        "--expected-exit-code",
        type=int,
        default=4,
        help="Exit code expected from nanvixd in release mode.",
    )
    p.add_argument(
        "--release",
        action="store_true",
        help="Release mode: validate exit code rather than magic string.",
    )
    p.add_argument(
        "--nanvixd", help="Path to nanvixd binary (default: ./bin/nanvixd[.exe|.elf])."
    )
    p.add_argument(
        "--clh-bin-path",
        help="Path to cloud-hypervisor binary directory (Linux only). "
        "Defaults to ./toolchain/bin (matches CLH_DIR in the Makefile).",
    )
    p.add_argument(
        "--log-dir", default="logs", help="Log output directory (default: ./logs)."
    )
    return p.parse_args()


def default_nanvixd(is_windows: bool) -> Path:
    ext = "exe" if is_windows else "elf"
    return Path("bin") / f"nanvixd.{ext}"


def banner(
    args: argparse.Namespace, nanvixd: Path, console_arg: str, is_windows: bool
) -> None:
    print("=" * 69)
    print(f"PLATFORM         = {'windows' if is_windows else 'linux'}")
    print(f"MACHINE          = {args.machine}")
    print(f"IMAGE            = {args.image}")
    print(f"NANVIXD          = {nanvixd}")
    print(f"CONSOLE          = {console_arg}")
    print(f"TIMEOUT          = {args.timeout}")
    if args.release:
        print(
            f"MODE             = release (expected exit code={args.expected_exit_code})"
        )
    else:
        print(
            f"MODE             = debug (magic string '{args.magic_string}', "
            f"expected exit code={args.expected_exit_code})"
        )
    print("=" * 69, flush=True)


def build_command(
    args: argparse.Namespace,
    nanvixd: Path,
    console_arg: str,
    log_dir: Path,
    is_windows: bool,
) -> list[str]:
    cmd: list[str] = [str(nanvixd), "-console-file", console_arg]
    # `-log-dir` is supported by nanvixd on both Linux and Windows; passing it
    # unconditionally keeps nanvixd's own logs under our chosen log directory
    # instead of leaking into the current working directory.
    cmd += ["-log-dir", str(log_dir)]
    if not is_windows:
        # cloud-hypervisor is only used on Linux; on Windows nanvixd uses WHP
        # directly. Default to the repo-local ./toolchain/bin (matching
        # CLH_DIR := $(ROOT_DIR)/toolchain in the Makefile) rather than
        # $HOME/toolchain/bin.
        clh_bin_path = args.clh_bin_path or os.path.join("toolchain", "bin")
        cmd += ["-clh-bin-path", clh_bin_path]
    cmd += ["--", args.image]
    return cmd


def dump_logs(files: list[Path]) -> None:
    for f in files:
        if not f.exists():
            continue
        print(f"--- {f} ---")
        try:
            sys.stdout.write(f.read_text(encoding="utf-8", errors="replace"))
            sys.stdout.flush()
        except OSError as e:
            print(f"(failed to read {f}: {e})")


def search_files(
    files: list[Path],
    needle: str,
    offsets: dict[Path, int],
    carry: dict[Path, str],
) -> bool:
    """Incrementally scan log files for `needle`.

    Reads only bytes appended since the previous call (per-file byte offsets
    are tracked in `offsets`), so repeated polling stays O(N) over the total
    bytes written rather than O(N²) over file size. A small per-file `carry`
    buffer (length `len(needle) - 1`) bridges chunk boundaries so the needle
    is still detected when it straddles two reads.
    """
    keep = max(0, len(needle) - 1)
    for f in files:
        if not f.exists():
            continue
        try:
            with f.open("rb") as fh:
                fh.seek(offsets.get(f, 0))
                chunk = fh.read()
                offsets[f] = fh.tell()
        except OSError:
            continue
        if not chunk:
            continue
        text = chunk.decode("utf-8", errors="replace")
        combined = carry.get(f, "") + text
        if needle in combined:
            return True
        carry[f] = combined[-keep:] if keep else ""
    return False


def _tee_stream(src: IO[bytes], sinks: list[IO[bytes]]) -> None:
    """Read bytes from src and mirror them to every sink until EOF."""
    try:
        while True:
            chunk = src.read(4096)
            if not chunk:
                break
            for sink in sinks:
                try:
                    sink.write(chunk)
                    sink.flush()
                except OSError:
                    pass
    except (OSError, ValueError):
        pass


def _tail_file(path: Path, sink: IO[bytes], stop_event: threading.Event) -> None:
    """Stream new bytes appended to `path` into `sink` until stop_event is set."""
    # Open lazily once the file appears.
    fh: IO[bytes] | None = None
    try:
        while not stop_event.is_set():
            if fh is None:
                if path.exists():
                    try:
                        fh = path.open("rb")
                    except OSError:
                        fh = None
                if fh is None:
                    time.sleep(0.1)
                    continue
            chunk = fh.read()
            if chunk:
                try:
                    sink.write(chunk)
                    sink.flush()
                except OSError:
                    pass
            else:
                time.sleep(0.1)
        # Final drain.
        if fh is not None:
            chunk = fh.read()
            if chunk:
                try:
                    sink.write(chunk)
                    sink.flush()
                except OSError:
                    pass
    finally:
        if fh is not None:
            try:
                fh.close()
            except OSError:
                pass


def terminate(proc: subprocess.Popen[bytes], is_windows: bool) -> None:
    """Terminate nanvixd and any child processes it spawned (e.g. CLH on Linux)."""
    if proc.poll() is not None:
        return

    # On Linux, nanvixd is launched in its own process group (start_new_session=True),
    # so we signal the whole group to also reap the cloud-hypervisor child.
    # `is_windows` is preferred over `os.name == "posix"` because MSYS/Cygwin
    # Python report `posix` even though nanvixd is a native Windows process
    # for which killpg is meaningless.
    try:
        if not is_windows:
            try:
                os.killpg(os.getpgid(proc.pid), signal.SIGTERM)  # type: ignore[attr-defined]
            except (ProcessLookupError, PermissionError):
                proc.terminate()
        else:
            proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            if not is_windows:
                try:
                    os.killpg(os.getpgid(proc.pid), signal.SIGKILL)  # type: ignore[attr-defined]
                except (ProcessLookupError, PermissionError):
                    proc.kill()
            else:
                proc.kill()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                pass
    except OSError:
        pass


def main() -> int:
    args = parse_args()
    # Robust Windows detection: catch native Python (`win32`) as well as MSYS
    # and Cygwin Python where `sys.platform` is `cygwin` / `msys`. All three
    # cases launch native `nanvixd.exe` and therefore want the Windows code
    # path. (`platform.system()` is unreliable here because it returns
    # `MSYS_NT-*` / `CYGWIN_NT-*` rather than `Windows`.)
    is_windows = sys.platform in {"win32", "cygwin", "msys"}

    if args.machine != "microvm":
        print(
            f"ERROR: unsupported machine type '{args.machine}'. Expected 'microvm'.",
            file=sys.stderr,
        )
        return 1

    nanvixd = Path(args.nanvixd) if args.nanvixd else default_nanvixd(is_windows)
    if not nanvixd.is_file():
        print(f"ERROR: nanvixd binary not found: {nanvixd}", file=sys.stderr)
        return 1

    image = Path(args.image)
    if not image.is_file():
        print(f"ERROR: image not found: {image}", file=sys.stderr)
        return 1

    if args.timeout <= 0:
        print(
            f"ERROR: timeout must be a positive integer, got {args.timeout}",
            file=sys.stderr,
        )
        return 1

    if not args.release and not args.magic_string.strip():
        print(
            "ERROR: --magic-string must be non-empty in debug mode.",
            file=sys.stderr,
        )
        return 1

    log_dir = Path(args.log_dir)
    log_dir.mkdir(parents=True, exist_ok=True)

    # On POSIX wire the kernel console into nanvixd's stdout so we can tee it
    # live. On Windows there is no /dev/stdout, so write the kernel console to
    # a file and tail it to our stdout from a helper thread.
    console_file = log_dir / "smoke-console.log"
    stdout_file = log_dir / "smoke-stdout.log"
    stderr_file = log_dir / "smoke-stderr.log"
    # Best-effort: remove stale log files from a prior run. On Windows the
    # unlink can raise PermissionError if another process still has the file
    # open; tolerate any OSError here and rely on the subsequent `"wb"` open
    # (which truncates) plus its dedicated error handling below to surface a
    # real problem.
    for f in (console_file, stdout_file, stderr_file):
        try:
            f.unlink()
        except FileNotFoundError:
            pass
        except OSError as e:
            print(f"WARNING: could not remove stale log {f}: {e}", file=sys.stderr)

    if is_windows:
        console_arg = str(console_file)
    else:
        console_arg = "/dev/stdout"

    banner(args, nanvixd, console_arg, is_windows)

    cmd = build_command(args, nanvixd, console_arg, log_dir, is_windows)
    print("Command:", " ".join(cmd), flush=True)

    # Search across whichever log files actually receive content.
    haystacks = [stdout_file, stderr_file, console_file]

    try:
        stdout_log = stdout_file.open("wb")
    except OSError as e:
        print(f"ERROR: failed to open {stdout_file}: {e}", file=sys.stderr)
        return 1
    try:
        stderr_log = stderr_file.open("wb")
    except OSError as e:
        print(f"ERROR: failed to open {stderr_file}: {e}", file=sys.stderr)
        stdout_log.close()
        return 1

    tail_stop = threading.Event()
    threads: list[threading.Thread] = []
    rc = 0

    # On Linux, run nanvixd in a new session so we can signal the whole process
    # group on timeout (nanvixd spawns cloud-hypervisor as a child on Linux).
    # Avoid this on Windows hosts (including MSYS/Cygwin Python) where nanvixd
    # is a native Windows process and `start_new_session` does not have the
    # same semantics.
    try:
        if is_windows:
            proc: subprocess.Popen[bytes] = subprocess.Popen(
                cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
        else:
            proc = subprocess.Popen(
                cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                start_new_session=True,
            )
    except OSError as e:
        print(f"ERROR: failed to launch nanvixd: {e}", file=sys.stderr)
        stdout_log.close()
        stderr_log.close()
        return 1

    try:
        assert proc.stdout is not None and proc.stderr is not None
        t_out = threading.Thread(
            target=_tee_stream,
            args=(proc.stdout, [sys.stdout.buffer, stdout_log]),
            daemon=True,
        )
        t_err = threading.Thread(
            target=_tee_stream,
            args=(proc.stderr, [sys.stderr.buffer, stderr_log]),
            daemon=True,
        )
        t_out.start()
        t_err.start()
        threads += [t_out, t_err]

        if is_windows:
            t_console = threading.Thread(
                target=_tail_file,
                args=(console_file, sys.stdout.buffer, tail_stop),
                daemon=True,
            )
            t_console.start()
            threads.append(t_console)

        if args.release:
            # Release mode: wait for process exit, validate exit code.
            try:
                proc.wait(timeout=args.timeout)
            except subprocess.TimeoutExpired:
                terminate(proc, is_windows)
                print(
                    f"ERROR: Smoke test failed: nanvixd did not exit within "
                    f"{args.timeout}s."
                )
                rc = 1
            else:
                if proc.returncode != args.expected_exit_code:
                    print(
                        f"ERROR: Smoke test failed: expected exit code "
                        f"{args.expected_exit_code}, got {proc.returncode}."
                    )
                    rc = 1
                else:
                    print(f"Smoke test passed (exit code={args.expected_exit_code}).")
        else:
            # Debug mode: validate BOTH the kernel magic string (proves the
            # kernel booted far enough to emit debug-level console output) AND
            # the process exit code (proves all guest tests passed; testd's
            # deliberate final page fault propagates as the expected status).
            # The magic string is emitted during shutdown, immediately before
            # the process exits, so we wait for natural exit rather than force-
            # terminating on first sighting. This catches guest-test regressions
            # (e.g. a test panicking early) that change the exit code without
            # affecting the boot-time magic string.
            deadline = time.monotonic() + args.timeout
            found = False
            timed_out = False
            offsets: dict[Path, int] = {}
            carry: dict[Path, str] = {}
            while True:
                if not found:
                    found = search_files(haystacks, args.magic_string, offsets, carry)
                if proc.poll() is not None:
                    break
                if time.monotonic() >= deadline:
                    timed_out = True
                    break
                time.sleep(1)

            # Stop the process (no-op if it already exited) and drain the tee
            # threads so any bytes still buffered in nanvixd's stdout/stderr
            # pipes are flushed into the on-disk log files before the final
            # magic-string scan. Without this, the magic string can be emitted
            # just before process exit and missed by `search_files` (false
            # negative).
            terminate(proc, is_windows)
            for t in (t_out, t_err):
                t.join(timeout=5)
            if not found:
                found = search_files(haystacks, args.magic_string, offsets, carry)

            if timed_out:
                print(
                    f"ERROR: Smoke test failed: nanvixd did not exit within "
                    f"{args.timeout}s."
                )
                rc = 1
            elif not found:
                print(
                    f"ERROR: Smoke test failed: magic string "
                    f"'{args.magic_string}' not found within {args.timeout}s."
                )
                rc = 1
            elif proc.returncode != args.expected_exit_code:
                print(
                    f"ERROR: Smoke test failed: expected exit code "
                    f"{args.expected_exit_code}, got {proc.returncode}."
                )
                rc = 1
            else:
                print(
                    f"Smoke test passed (magic string found, exit "
                    f"code={args.expected_exit_code})."
                )
    finally:
        terminate(proc, is_windows)
        tail_stop.set()
        for t in threads:
            t.join(timeout=2)
        try:
            stdout_log.close()
        except OSError:
            pass
        try:
            stderr_log.close()
        except OSError:
            pass

    # Only dump captured logs on failure (live output already streamed on success).
    if rc != 0:
        dump_logs(haystacks)
    return rc


if __name__ == "__main__":
    sys.exit(main())
