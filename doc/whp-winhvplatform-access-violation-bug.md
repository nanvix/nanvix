# WHP `winhvplatform.dll+0x12f1d` ACCESS_VIOLATION Bug

## Summary

An intermittent ACCESS_VIOLATION (Windows exception code `0xC0000005`)
occurs inside `winhvplatform.dll` at offset `0x12f1d`. The fault is a
**read from address `0xFFFFFFFFFFFFFFFF`** (an invalid pointer / −1
sentinel), which crashes the host process with exit code −1073741819.

The bug is triggered by calling `WHvCancelRunVirtualProcessor` from a
background thread while `WHvRunVirtualProcessor` is executing on the VMM
loop thread. It is **not** a Nanvix bug — the crash is inside a
Microsoft system DLL — but it is reproducible and must be worked around
at the VMM level.

| Field               | Value                                      |
|---------------------|--------------------------------------------|
| Faulting module      | `C:\Windows\SYSTEM32\winhvplatform.dll`    |
| Faulting offset      | `0x12f1d`                                  |
| Exception code       | `0xC0000005` (ACCESS_VIOLATION)            |
| Access type          | READ                                       |
| Fault address        | `0xFFFFFFFFFFFFFFFF`                       |
| Process exit code    | `−1073741819` (`0xC0000005` as signed i32) |
| Affected component   | Nanvix UserVM — WHP VMM backend            |
| Affected OS          | Windows 11 with Windows Hypervisor Platform |
| Nanvix fix commit    | `f21110c1a`                                |

---

## Reproduction

### Preconditions

- Windows 11 with WHP (Windows Hypervisor Platform) enabled.
- Nanvix built for standalone/microvm deployment on Windows.
- LAPIC emulation enabled in XApic mode (partition property
  `WHvPartitionPropertyCodeLocalApicEmulationMode = 1`).

### Trigger

The crash is triggered by concurrent calls to WHP APIs from multiple
threads:

- **Thread A (VMM loop):** Calls `WHvRunVirtualProcessor` in a tight
  loop, blocking until the guest triggers a VM exit (I/O port access,
  HLT with LAPIC, MMIO fault, etc.).
- **Thread B (clock-refresh):** Calls `WHvCancelRunVirtualProcessor`
  every ~1 ms to force a VM exit so the VMM loop can update the
  pvclock page and check for IKC/shutdown.

When thread B's `WHvCancelRunVirtualProcessor` call races with thread
A's `WHvRunVirtualProcessor`, the WHP runtime inside `winhvplatform.dll`
dereferences an invalid pointer (`0xFFFFFFFFFFFFFFFF`), raising a
structured exception that terminates the process.

### Failure rate

| Scenario                   | Crash rate | Notes                              |
|----------------------------|------------|------------------------------------|
| Original code (with clock-refresh thread) | ~5–20 %    | Varies by guest workload; I/O-heavy programs (echo) crash more often |
| LAPIC emulation disabled (HLT exits)      | 100 %      | Tight VMM spin creates deterministic crash during partition teardown  |
| Clock-refresh thread removed (fix)        | 0 %        | 100/100 runs verified (50 hello-c + 50 echo-c)                      |

### Minimal reproduction steps

```
1.  Build Nanvix for Windows:  .\z.ps1 build -- all
2.  Revert commit f21110c1a to restore the clock-refresh thread.
3.  Run:  bin\nanvixd.exe -- bin\hello-c.elf
4.  Repeat ~20 times. Expect 1–4 crashes (exit code −1073741819).
```

---

## Investigation Details

### Diagnostic approach

A Vectored Exception Handler (VEH) was installed via raw FFI
(`AddVectoredExceptionHandler`) to capture crash details before the
default handler terminated the process. The VEH resolved the faulting
RIP to a module + offset using `GetModuleHandleExW` with the
`GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS` flag, then logged:

```
[VEH] ACCESS_VIOLATION: rip=0x00007ff929202f1d fault_addr=0xffffffffffffffff
      type=READ module=C:\Windows\SYSTEM32\winhvplatform.dll offset=0x12f1d
```

The offset `0x12f1d` was consistent across all observed crashes, even
as the DLL base address varied due to ASLR. The fault address
`0xFFFFFFFFFFFFFFFF` strongly suggests an uninitialized or cleared
pointer field inside WHP's internal state (−1 is a common sentinel for
"invalid handle" on Windows).

### Hypotheses tested

| # | Hypothesis | Result |
|---|-----------|--------|
| 1 | Crash is stdin-related (echo programs only) | **Disproved** — hello-c (no stdin) also crashes at ~7 % |
| 2 | Crash is caused by concurrent `WHvCancelRunVirtualProcessor` | **Confirmed** — removing the clock-refresh thread eliminates the crash |
| 3 | Crash can be avoided by disabling LAPIC emulation | **Disproved** — crash rate increases to 100 % (different code path in `winhvplatform.dll` for HLT exits without LAPIC) |
| 4 | `WHvRequestInterrupt` (timer thread) also triggers the crash | **Disproved** — timer thread survives 100/100 runs; only `WHvCancelRunVP` is problematic |
| 5 | Crash is during partition teardown (`WHvDeletePartition`) | **Partially confirmed** — with LAPIC disabled, crash is deterministic during teardown; with LAPIC enabled, crash occurs during the VMM loop |

### Key finding

`WHvCancelRunVirtualProcessor` is **not safe to call concurrently** with
`WHvRunVirtualProcessor` on the same partition, despite the API being
documented as the mechanism to interrupt a running vCPU. The crash is
inside Microsoft's `winhvplatform.dll` and cannot be caught or recovered
from at the application level (it is a structured exception, not a Rust
panic).

In contrast, `WHvRequestInterrupt` — which injects an interrupt through
the WHP LAPIC emulator — is safe for concurrent use. The LAPIC emulator
serializes interrupt delivery internally, avoiding the race condition.

---

## Fix

### Approach

Eliminate all calls to `WHvCancelRunVirtualProcessor` from background
threads. The VMM loop handles pvclock updates, IKC delivery, and
shutdown checks inline on each iteration, which runs on the same thread
as `WHvRunVirtualProcessor`.

### Changes

#### 1. Remove clock-refresh thread (`mod.rs`)

The background thread that called `WHvCancelRunVirtualProcessor` every
1 ms was removed entirely. Pvclock (`system_time`) is now updated at the
top of the VMM loop before each `WHvRunVirtualProcessor` call. Since the
timer thread delivers interrupts via `WHvRequestInterrupt` every ~1 ms,
the guest wakes from HLT at least once per timer period, ensuring
pvclock stays fresh.

**Before (removed):**
```rust
let clock_refresh_thread = std::thread::spawn(move || {
    unsafe { timeBeginPeriod(1) };
    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(1));
        unsafe {
            WHvCancelRunVirtualProcessor(partition, 0, 0);  // BUG TRIGGER
        }
    }
    unsafe { timeEndPeriod(1) };
});
```

**After:**
```rust
// Pvclock updated inline at top of VMM loop on every iteration.
// No background thread, no WHvCancelRunVirtualProcessor.
```

#### 2. Simplify IKC notifier (`mod.rs`)

The `IkcNotifier` previously called `WHvCancelRunVirtualProcessor` to
wake the vCPU when IKC credits were written by the host. This was
replaced with a simple atomic flag. The VMM loop polls the flag on each
iteration; IKC delivery latency is bounded by the timer period (~1 ms).

**Before (removed):**
```rust
pub fn notify(&self) -> Result<()> {
    self.pending.swap(true, Ordering::AcqRel);
    unsafe {
        WHvCancelRunVirtualProcessor(self.partition, self.vp_index, 0);  // BUG TRIGGER
    }
    Ok(())
}
```

**After:**
```rust
pub fn notify(&self) -> Result<()> {
    self.pending.store(true, Ordering::Release);  // Flag only, no WHP API
    Ok(())
}
```

#### 3. Duplicate MMIO mapping guard (`vmem.rs`)

Added a `HashSet<u64>` (`mapped_gpas`) to track page-aligned GPAs that
have already been lazily mapped. This prevents `WHvMapGpaRange` from
failing on a duplicate mapping if the guest triggers MMIO exits for
the same page more than once (e.g., LAPIC probe during boot).

#### 4. Enable echo tests (`test-standalone-windows.toml`)

With the crash eliminated, the previously-disabled echo-c, echo-cpp,
and echo-rust-nostd tests were re-enabled.

### Architecture after fix

```
┌──────────────────────────────────────────────────┐
│  VMM Loop Thread (single thread)                 │
│                                                  │
│  loop {                                          │
│      update_pvclock()          // inline          │
│      check ikc_notifier flag   // atomic read     │
│      WHvRunVirtualProcessor()  // blocks          │
│      handle VM exit            // PMIO, MMIO, ... │
│  }                                               │
└──────────────────────────────────────────────────┘
         ▲                          
         │ (no concurrent WHP calls)
         │                          
┌────────┴─────────────────────────────────────────┐
│  Timer Thread (background)                       │
│                                                  │
│  loop {                                          │
│      sleep(period)                               │
│      WHvRequestInterrupt(...)  // LAPIC-safe      │
│  }                                               │
└──────────────────────────────────────────────────┘
         ▲
         │ (atomic flag only)
         │
┌────────┴─────────────────────────────────────────┐
│  Host / Orchestrator Thread                      │
│                                                  │
│  ikc_notifier.notify()         // sets AtomicBool │
│  (no WHP API calls)                              │
└──────────────────────────────────────────────────┘
```

### Validation

- **hello-c.elf:** 50/50 passes (previously ~7 % crash rate).
- **echo-c.elf:** 50/50 passes (previously ~10–20 % crash rate).
- **Full test suite (11 tests):** 5/5 consecutive complete runs.

---

## Remaining WHP-related issues

The following tests remain disabled due to separate WHP/guest kernel
timing issues (not related to the `winhvplatform.dll` crash):

| Test                 | Exit code | Root cause                          |
|----------------------|-----------|-------------------------------------|
| thread-rust.elf      | 116       | condvar/timer errors during boot    |
| stress-rust.elf      | 116       | same as above                       |
| thread-c.elf         | 116       | same as above                       |
| misc-c.elf           | 116       | same as above                       |
| memory-c.elf         | 116       | same as above                       |
| dlfcn-c.elf          | N/A       | missing standalone-rootfs.img       |
| dlfcn-pie-c.elf      | N/A       | missing standalone-rootfs.img       |

---

## Recommendations

1. **Never call `WHvCancelRunVirtualProcessor` from a background
   thread.** Use `WHvRequestInterrupt` (through the LAPIC emulator)
   instead for waking the vCPU.

2. **If a future feature requires waking the vCPU from a non-VMM
   thread,** inject an interrupt via `WHvRequestInterrupt` and handle
   the resulting VM exit in the VMM loop.

3. **Monitor Microsoft updates** to `winhvplatform.dll` — the bug may
   be fixed in a future Windows update, which could allow re-enabling
   `WHvCancelRunVirtualProcessor` if needed for lower-latency IKC
   delivery.

4. **Keep the VEH diagnostic handler available** (currently removed
   from production) as a debugging aid. It can be re-added behind a
   `#[cfg(debug_assertions)]` gate if intermittent crashes resurface.
