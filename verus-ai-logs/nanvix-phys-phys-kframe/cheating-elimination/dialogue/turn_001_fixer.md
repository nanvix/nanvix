## Response to Turn 1

### What I Changed

The reviewer flagged that the trusted contract
`assume_specification[ KernelFrame::map_frame ]` (introduced when the `external_body`
on `KernelFrame::new` was eliminated) was **not** tracked in
`verus-ai-logs/tcb-allowed.md`, and that the stale `KernelFrame::new` `external_body`
entry still lingered there. I took the **sanctioned path (option B)** — ledger the
trusted contract at the genuine `mm::virt` boundary — after first proving option A
(re-inlining the side effect into the verified `new`) is impossible (evidence below).

**1. `verus-ai-logs/tcb-allowed.md`**
- **Removed** the stale ledger entry
  `src/kernel/src/mm/phys/kframe.rs::KernelFrame::new` (it no longer carries
  `external_body`; its `kf@ == base@` / `kf.inv()` postconditions are now
  machine-verified).
- **Added** a ledger entry for
  `src/kernel/src/mm/phys/kframe.rs::KernelFrame::map_frame`, documenting that it is an
  `assume_specification` with an **empty** contract whose sole effect is calling
  `mm::virt::identity_map_page`, whose precondition `identity_map_view().inv()` is an
  `uninterp spec fn` in the **private, non-re-exported** `mod identity_map` — therefore
  not nameable or dischargeable from `mm::phys`. Removed when `mm::virt` is verified.
- Renamed the section header from "Cross-module dependencies marked `external_body`
  (eliminated when their module is verified)" to "Cross-module dependencies trusted
  until their module is verified (`external_body` / `assume_specification`)" so it
  correctly hosts both trust kinds.

**2. `src/kernel/src/mm/phys/kframe.rs` (map_frame extraction site, ~line 90)**
- Added a `// VERUS REWRITE:` comment block at the `KernelFrame::map_frame` helper
  explaining why the side effect was extracted out of the verified `new`, and why
  re-inlining is impossible (the three blockers below). It points at the trusted
  contract in `kframe.spec.rs` and the ledger entry in `tcb-allowed.md`.

No exec semantics changed: `new` still performs `map_frame(base)?; Ok(Self { base })`,
and `map_frame` still runs the identical `PageAligned::from_raw_value` +
`identity_map_page` calls with the same error logging. Time/space complexity unchanged.

### Why option A (re-inline into verified `new`) is impossible — captured errors

I attempted to delete `map_frame` + its `assume_specification` and call the side effect
directly from the verified `new`. Three independent Verus blockers, each reproduced:

**(a) `error!` logging macro fails to verify:**
```
error: Unsupported constant type
   --> src/kernel/src/macros.rs:188:12
   ::: src/kernel/src/mm/phys/kframe.rs:83:17
 83 |   error!("frame base is not page-aligned: {e:?}");
```
→ would require cfg-gating the logging (cfg-gated exec = a cheating indicator).

**(b) `PageAligned::from_raw_value` is external (no spec):**
```
error: cannot use function `...address::aligned::page::PageAligned::from_raw_value`
which is ignored because it is either declared outside the verus! macro or it is
marked as `external`.
  --> src/kernel/src/mm/phys/kframe.rs:82:13
  = help: The following declaration may resolve this error:
    pub assume_specification<T> [<... PageAligned<T> as sys::mm::Address>::from_raw_value] ...
```
→ re-inlining needs *another* trusted `assume_specification`, increasing trust surface.

**(c) `identity_map_page`'s precondition cannot be named or discharged:**
`identity_map_page` carries `requires identity_map_view().inv()`.
`identity_map_view` is `pub uninterp spec fn` (identity_map.spec.rs:36) inside the
**private** `mod identity_map`, and `mm/virt/mod.rs` re-exports only
`identity_map_page`, `memcpy`, `sync_kernel_pdes` — **not** `identity_map_view`:
```
$ grep identity_map_view src/kernel/src/mm/virt/mod.rs
(no output — not re-exported)
```
→ The precondition cannot even be *named* from `mm::phys`, so it cannot be threaded as
a `requires` on `new` nor discharged. (Threading it would also regress verified
`manager.rs` callers of `new`.)

Conclusion: option A converts ONE clean, empty trusted contract into TWO trusted
contracts plus cfg-gated exec, and STILL cannot discharge the precondition. The
extracted `map_frame` (option B) trusts strictly **less** — only the cross-module
page-table side effect, exactly at the `mm::virt` boundary — and is now ledgered.

### Verification

`make verify-kernel MODULE=mm::phys::kframe`:
```
=== Results ===
  3 verified
  0 errors
  Exit code : 0

=== Cheating Pattern Check ===
  ✅ No cheating detected in module mm::phys::kframe.

=== Summary ===
  verification: 3 verified, 0 errors (exit 0)
  cheating: assume=0 external_body=15 admit=7 trusted=0 no_decreases=0 cfg_gate=12
  coverage: 3/7 exec functions have contracts
  status: CLEAN
```
(The global non-zero `external_body`/`admit`/`cfg_gate` counts are all in OTHER
bottom-up modules and ledgered in `tcb-allowed.md`; `grep -i kframe cheating-detail.txt`
returns nothing — zero cheating items live in this module.)

Full kernel `make verify-kernel`: exit 0 (cached, no regression).

Ledger checks:
```
$ grep -c 'KernelFrame::new'       verus-ai-logs/tcb-allowed.md   # stale ledger entry
0   (the single remaining hit is descriptive text inside the map_frame entry)
$ grep -c 'KernelFrame::map_frame' verus-ai-logs/tcb-allowed.md
2
```

`make check-kernel`: the only error is the **pre-existing** `manager.rs:245`
`unused variable: i` warning (treated as error under `-D warnings`). It is byte-identical
on base branch `verus-ai-prove-bottom-up`
(`git show verus-ai-prove-bottom-up:src/kernel/src/mm/phys/manager.rs | sed -n '245p'`
→ `for i in 0..count {`), is in a file I did not touch
(`git diff --name-only | grep manager.rs` → 0), and is out of scope for this task.

### Result: FIXED

The untracked trusted contract is now ledgered in `tcb-allowed.md` (option B, the
sanctioned path), the stale `KernelFrame::new` entry is removed, a `// VERUS REWRITE:`
comment documents the extraction at the source site, and option A was proven impossible
with the three exact Verus errors above. The `mm::phys::kframe` module remains CLEAN
(3 verified, 0 errors, no cheating).
