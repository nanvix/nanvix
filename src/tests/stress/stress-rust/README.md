# stress-rust

## Overview
`stress-rust` isolates the high-load thread and synchronization workloads that used to
live inside `thread-rust`. The goal is to keep the basic conformance tests green while
we continue to engineer and debug these heavier scenarios.

## Workloads
- **Thread fan-out:** repeatedly spawns short-lived workers that yield heavily before
  returning tagged values.
- **Mutex churn:** multiple threads contend on a single pthread mutex, combining KCALL
  lock/unlock cycles with opportunistic yields.
- **Parallel spawners:** nested creation of child threads that each allocate their own
  stacks and synchronize on return tags.
- **KCALL hammer:** mixes `lock_mutex()`, `unlock_mutex()`, `gettime()`, and
  `sched_yield()` to provoke concurrent kernel call traffic.
- **Debug console spam:** repeatedly issues `debug()` kernel calls with varied payloads to
  saturate the console path without any intermediate buffering.
- **Event registration churn:** acquires `capctl()` privileges and loops through
  `evctrl()` register/unregister cycles for multiple exception events.
- **Memory mapping storm:** toggles `mmap()`, `mprotect()`, and `munmap()` sequences across
  a sliding window of pages to stress the memory-management kernel calls directly.

## Current Status (January 14, 2026)
- The stress tests currently fail *before* the VM is brought up.
- `nanvixd` reports repeated "failed to connect to gateway socket" errors, suggesting
  a sandbox or runner startup problem rather than an in-guest crash.
- Guest logs (when available) show `condvar::wait()` alarms expiring immediately at
  `SystemTime { seconds: 0, nanoseconds: 0 }`, after which the dispatcher reports
  `OperationTimedOut` while trying to sleep.
- We have not yet determined whether the regressions originate from the stress tests
  themselves or from a kernel/runtime limitation exposed by the heavier load.

## Next Steps
1. Reproduce the failure with logging cranked up (`LOG_LEVEL=trace`) to capture the
   control-plane handshake with the sandbox gateway.
2. Compare behavior against a reduced workload (e.g., disable `test_kcall_hammer()`)
   to see whether a single scenario is triggering the regression.
3. Inspect `nanvixd` and `uservm` logs for socket lifecycle issues when the gateway
   directory is cleaned between test runs.

> This file should be kept up to date so we can quickly resume the investigation when
> returning to the stress suite.
