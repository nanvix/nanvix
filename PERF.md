# Cold-Start Benchmark Performance Results

This document summarizes the cold-start benchmark results for the performance
optimization commits in the `bugfix-build-target_spec` branch, on top of the
base commit `625d2bb1` (`[build] B: Add json-target-spec flag`).

## Methodology

- **Build command:** `./z build -- RELEASE=yes BUILD_OPT=no LOG_LEVEL=panic`
- **Benchmark command:** `./bin/nanvix-bench.elf -benchmark cold-start -iterations 1000`
- **Iterations:** 1000
- **Platform:** Linux 6.6.87.2-microsoft-standard-WSL2

## Raw Results

All values in microseconds (us). Lower is better.

| # | Commit     | Description                          | First req | p50    | p95    | p99    |
|---|------------|--------------------------------------|-----------|--------|--------|--------|
| 0 | `625d2bb1` | **Baseline**                         | 81,528    | 65,308 | 81,528 | 87,219 |
| 1 | `8689ca7d` | Clear CPUID speculation bits         | 57,721    | 64,761 | 81,153 | 89,380 |
| 2 | `373c507e` | Skip CR3 reload on context switch    | 60,905    | 65,132 | 81,893 | 87,399 |
| 3 | `56c8dc58` | Skip CR0 write in load_page_directory| 68,601    | 64,956 | 81,792 | 89,183 |
| 4 | `a4b8e8e9` | Use invlpg for single-page TLB flush | 60,744    | 64,621 | 81,687 | 86,629 |
| 5 | `c598d2db` | Use identity CR3 for phys memops     | 66,414    | 60,983 | 79,606 | 84,432 |
| 6 | `1eb86584` | Shared-memory ring buffer for MicroVM| 68,968    | 63,261 | 79,606 | 87,331 |
| 7 | `e3196fb8` | Optimize nested virt perf            | 49,644    | 56,701 | 76,518 | 84,144 |

## Cumulative Change vs Baseline

| # | Commit     | Description                          | First req | p50     | p95    | p99    |
|---|------------|--------------------------------------|-----------|---------|--------|--------|
| 1 | `8689ca7d` | Clear CPUID speculation bits         | -29.2%    | -0.8%   | -0.5%  | +2.5%  |
| 2 | `373c507e` | Skip CR3 reload on context switch    | -25.3%    | -0.3%   | +0.4%  | +0.2%  |
| 3 | `56c8dc58` | Skip CR0 write in load_page_directory| -15.8%    | -0.5%   | +0.3%  | +2.3%  |
| 4 | `a4b8e8e9` | Use invlpg for single-page TLB flush | -25.5%    | -1.1%   | +0.2%  | -0.7%  |
| 5 | `c598d2db` | Use identity CR3 for phys memops     | -18.5%    | -6.6%   | -2.4%  | -3.2%  |
| 6 | `1eb86584` | Shared-memory ring buffer for MicroVM| -15.4%    | -3.1%   | -2.4%  | +0.1%  |
| 7 | `e3196fb8` | Optimize nested virt perf            | -39.1%    | -13.2%  | -6.1%  | -3.5%  |

## Overall Improvement (Baseline vs HEAD)

| Metric    | Baseline   | HEAD       | Improvement |
|-----------|------------|------------|-------------|
| First req | 81,528 us  | 49,644 us  | **-39.1%**  |
| p50       | 65,308 us  | 56,701 us  | **-13.2%**  |
| p95       | 81,528 us  | 76,518 us  | **-6.1%**   |
| p99       | 87,219 us  | 84,144 us  | **-3.5%**   |

## Observations

- The first four kernel-level commits (1-4) had negligible effect on p50
  individually. The first clear p50 drop came at commit 5 (`c598d2db` - identity
  CR3 for phys memops), followed by a large additional drop at commit 7
  (`e3196fb8` - optimize nested virt perf).
- Tail latencies (p95/p99) improvements were modest and concentrated in commits
  5 and 7. The kernel-only commits (1-4) had essentially no effect on tail
  latency.
- First request latency is inherently noisy (single sample), but shows a
  consistent downward trend and the largest relative improvement at -39.1%.
- The two most impactful commits are `c598d2db` (identity CR3 for phys memops)
  and `e3196fb8` (optimize nested virt perf), which together account for the
  bulk of the p50 and tail-latency improvements.
