# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Unit tests for the benchmark runner (scripts/benchmark.py)."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_REPO_ROOT / "scripts"))

import benchmark  # noqa: E402

# Output of `nanvix-bench -benchmark syscall-bench`, trimmed to a few system calls.
SYSCALL_BENCH_STDOUT = """
Batch sizes (calls per sample):
  getppid                   32768
  getpgrp                    1024
  open+close                  256

Kernel                  Samples   p50 (ns)   p95 (ns)   p99 (ns)
--------------------------------------------------------------
getppid                      20        328        345        345

Daemons                 Samples   p50 (ns)   p95 (ns)   p99 (ns)
--------------------------------------------------------------
getpgrp                      20      14306      18498      18498
open+close                   20      96062     130889     130889
"""


class FilterBenchmarkStdoutTest(unittest.TestCase):
    """Tests for the conversion of benchmark output into CSV results."""

    def test_syscall_bench_reports_one_row_per_system_call(self) -> None:
        csv_text = benchmark.filter_benchmark_stdout(
            "syscall-bench", SYSCALL_BENCH_STDOUT, "abc123"
        )

        self.assertEqual(
            csv_text.splitlines(),
            [
                "commit,section,operation,samples,p50,p95,p99",
                "abc123,Kernel,getppid,20,328,345,345",
                "abc123,Daemons,getpgrp,20,14306,18498,18498",
                "abc123,Daemons,open+close,20,96062,130889,130889",
            ],
        )


if __name__ == "__main__":
    unittest.main()
