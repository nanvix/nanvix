## Turn 1: Full proving checklist walkthrough — `hal::mem::types::address::aligned::page` (`PageAligned<T>`)

### Progress
- Done (PASS): all 10 checklist items verified below.
- Current: full walkthrough complete.
- Remaining: none.

### Verification

Commands run this turn:
- `make verify-kernel` (cached) → exit 0, then forced fresh full-crate run
  (`touch page.rs && make verify-kernel`) → **33 verified, 0 errors, exit 0**
  (log `verus_2026-06-15_11-35-34.log`).
- Normal (non-ghost) kernel build with `--features microvm,trace` → exit 0,
  no warnings (only `Compiling kernel` / `Finished`).
- `spec_drift.py check hal-page-aligned` → exit 0 (no drift).
- `spec_drift.py git-diff page.rs --before 2a118cb` (specification END) → exit 0.
- Module-scoped cfg-gate scan + read of `page.rs` / `page.spec.rs` / `page.proof.rs`
  / `tcb-allowed.md` / `bugs.md`.
- `git diff 5320f77 (proving start) -> HEAD -- aligned/` → empty (no source change).

Proving phase made **no source changes** to the module (proof.rs is `verus! { }`);
the verification surface is the View/inv spec material plus two governed trust
boundaries to the unverified `sys::mm::Address` trait and `arch` constants.

#### Item-by-item

1. **No specs weakened** — PASS. `spec_drift.py` exit 0 against BOTH the proving
   start (`5320f77`) and the specification END (`2a118cb`). 0 functions changed,
   0 ensures removed, 0 requires added. `page.spec.rs` byte-identical since spec END.

2. **Zero admit()** — PASS. Cheating scan `admit=0` (whole crate). `page.proof.rs`
   is empty (`verus! { }`); no `admit` token anywhere in the module.

3. **Zero external_body unless in `tcb-allowed.md`** — PASS. The module's only
   `external_body` is `page.rs:65 PageAligned::from_address`. It is explicitly
   listed in `verus-ai-logs/tcb-allowed.md` (section "Allowed `external_body` —
   `hal::mem::PageAligned` (`page.rs`, proof target)", lines 197–215) with the
   `PAGE_ALIGNMENT`-unsupported / unspecced-`Address::is_aligned` rationale. Same
   accepted class as `FrameAddress::into_raw_value`.

4. **Zero assume/assume_specification (only external-bottom for std/external)** —
   PASS. The module's only `assume_specification` is `page.spec.rs:50`
   `<PageAligned<T> as Address>::into_raw_value`. `Address` is the **external
   `sys::mm::Address` trait** (an external crate edge), so this is a permitted
   external-bottom boundary. It is governed in `tcb-allowed.md` (section "Allowed
   `assume_specification` — `hal::mem::PageAligned` `Address::into_raw_value`",
   lines 217–234), mirroring the existing `from_raw_value` boundary in
   `kframe.spec.rs`. Scanner `assume=0` (assume_specification is not the forbidden
   in-body `assume`).

5. **No cfg-gated exec code** — PASS. The only `#[cfg(verus_keep_ghost)]` flagged
   by the scanner in this module is `page.rs:230`, which gates a `verus! { ... }`
   block containing **only** the `View` impl (`closed spec fn view`) and
   `pub open spec fn inv` — pure ghost/spec material, the correct pattern. The
   other gates are `include!("page.spec.rs")` / `include!("page.proof.rs")`
   (excluded). No exec branch/expression/match-arm is cfg-gated.

6. **Cheating audit (exact counts/locations)** — module scope:
   - `admit`: **0**.
   - `external_body`: **1** → `page.rs:65 from_address` (governed, TCB lines 197–215).
   - `assume`/`assume_specification`: **1** → `page.spec.rs:50 into_raw_value`
     (governed, TCB lines 217–234; external `sys::mm::Address` trait).
   - cfg-gated exec code: **0** (the one cfg gate is ghost spec material).
   Whole-crate scan for reference: `assume=0 external_body=24 admit=0 trusted=0
   no_decreases=0 cfg_gate=9` — every external_body in the cheating-detail list is
   a pre-existing governed TCB entry; none introduced by this module beyond the two above.

7. **Claimed Verus limitation has isolated reproducer for the specific construct** —
   PASS. Both claims name the **specific** unsupported construct with the exact
   Verus diagnostic, not a vague whole-function failure:
   - `into_raw_value` (assume_specification): the construct is the generic trait
     impl `impl<T: Address> Address for PageAligned<T>`. Marking it verified makes
     the Verus **front-end panic** — reproduced in the real build log
     `verus_2026-06-15_11-06-00.log:21-22`:
     `thread 'rustc' panicked at vir/src/traits.rs:511:13: assertion failed:
     !method_impls.contains(&p)` (exit 101). A front-end panic is conclusively a
     tool bug, not a maskable proof gap. Documented in `bugs.md`.
   - `from_address` (external_body): the construct is the `arch` `Alignment`
     constant; Verus emits `error: arch::x86::mem::constants::PAGE_ALIGNMENT is not
     supported` (deterministic translation failure of an external arch enum
     constant — same accepted class as `FrameAddress::into_raw_value`). Documented
     in `bugs.md`.

8. **Exec rewrites minimal & semantically equivalent (`// VERUS REWRITE`)** — PASS.
   `grep "VERUS REWRITE"` over the module returns nothing; the proving diff
   (`5320f77 -> HEAD`) shows no source changes. No exec was rewritten.

9. **Cross-module regression (`make verify-kernel`, all modules)** — PASS. Forced
   fresh full-crate run: **33 verified, 0 errors, exit 0** (Module: `<all>`).

10. **Verification + build (0 errors, 0 warnings)** — PASS. `make verify-kernel`
    exit 0; normal `cargo build` (kernel crate, microvm,trace) exit 0 with no
    warnings (`-D warnings` in effect). The earlier duplicate-`use` warning noted in
    `bugs.md` is already fixed.

### Fix Request
None. All 10 checklist items PASS with concrete tool evidence. The proving phase
left the module in a clean, fully-verified state: 0 admit, 0 in-body assume, no
cfg-gated exec code, no spec drift, and the only external_body /
assume_specification are the two governed `sys::mm::Address` / `arch` trust
boundaries documented in `tcb-allowed.md` and reproduced (front-end panic /
unsupported-constant error) in the verus logs.
