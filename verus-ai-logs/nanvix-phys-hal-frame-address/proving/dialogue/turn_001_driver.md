## Turn 1: Full checklist walkthrough — FAIL on `assume_specification`

### Progress
- Done (PASS): #1 spec-drift, #2 admit, #3 external_body, #5 cfg-gated exec,
  #6 cheating audit, #7 Verus-limitation reproducer, #8 exec rewrites,
  #9 cross-module regression, #10 verification/build.
- Current (FAIL): #4 Zero assume/assume_specification.
- Remaining: none — every item verified this turn.

### Verification

Files reviewed (no changes during proving — `git diff 1f6205c56 HEAD` on
`frame*.rs` is empty; the module was already proven at proving-start):
- `src/kernel/src/hal/mem/types/address/frame.rs`
- `src/kernel/src/hal/mem/types/address/frame.spec.rs`
- `src/kernel/src/hal/mem/types/address/frame.proof.rs`

Commands run:
- `make verify-kernel` → exit 0, `note: verifying module hal::mem::types::address::frame`,
  no warnings/errors in the log. Last non-cached run (commit `1d1951da5`) reported
  "6 verified, 0 errors".
- `spec_drift.py git-diff <each frame file> --before 1f6205c56` (proving-start =
  specification-END) → **0 contract drift** on all three files.
- Per-file grep for `admit` / `external_body` / `assume` / `cfg(` (frame.rs, frame.spec.rs, frame.proof.rs).

Per-item verdicts:

1. **No specs weakened — PASS.** `spec_drift.py` reports 0 ensures-removed,
   0 requires-added, 0 proof-body drift on all three frame files vs proving-start.
   No frame file changed during proving.

2. **Zero admit() — PASS.** `grep admit` = 0 hits in all three frame files.
   `cheating-detail.txt` has no `hal/mem/types/address/frame` entries (all 12
   admits are in `mm/phys/manager.proof.rs` and `mm/virt/identity_map.*` — other modules).

3. **Zero external_body (unless TCB) — PASS.** `grep external_body` = 0 hits in
   all three frame files. The struct uses `#[verus_verify(external_derive)]`
   (derive-macro handling), not `external_body`. NOTE: `tcb-allowed.md:137-140`
   still lists `FrameAddress::from_raw_value`/`into_raw_value` as `external_body`,
   but the current source verifies them in-body — the TCB entries are stale, not a violation.

4. **Zero assume/assume_specification — FAIL.** See Fix Request.

5. **No cfg-gated exec code — PASS.** The 3 `#[cfg(verus_keep_ghost)]`
   (frame.rs:9, 11, 36) gate only ghost content: `include!("frame.spec.rs")`,
   `include!("frame.proof.rs")`, and the `verus! { ... }` spec block
   (`spec_page_size`, `View` impl, `inv`). The struct and all exec fns are
   unconditional. No exec branch/expr/match-arm is cfg-gated.

6. **Cheating audit (frame module) — reported.**
   - `admit` = 0
   - `external_body` = 0
   - `assume_specification` = **1** → `frame.spec.rs:20`
     (`<PhysicalAddress as ::sys::mm::Address>::from_raw_value`)
   - cfg-gated exec = 0 (3 ghost-only `cfg(verus_keep_ghost)` attrs)
   - The `assume` token at `frame.rs:40` is prose in a comment, not code.
   - WARNING: the automated `Cheating Pattern Check` printed `assume=0` because
     it matches `assume(` calls only; it does **not** count `assume_specification`.
     This violation is invisible to the script and must be tracked manually.

7. **Claimed Verus limitation has isolated reproducer — PASS (vacuous).** The
   frame module makes no Verus-limitation claim: no `// VERUS REWRITE` comment
   and no limitation-justified `external_body` in any frame file.

8. **Exec rewrites minimal & semantically equivalent — PASS (vacuous).** No
   `// VERUS REWRITE` in frame files. Exec bodies are unchanged; only `proof!`
   blocks were added inside `from_frame_number`/`into_frame_number` (ghost code,
   no exec-semantics change).

9. **Cross-module regression (`make verify-kernel`, all modules) — PASS.**
   Exit 0; frame module verified. The pre-existing `external_body=19`/`admit=12`
   in `mm/phys/*` and `mm/virt/*` are other modules' tracked debt, out of scope here.

10. **Verification + build, 0 errors/0 warnings — PASS.** `make verify-kernel`
    exit 0, clean log, cargo "Finished dev profile". (Latest run was cached; re-run
    non-cached after the #4 fix to reconfirm.)

### Fix Request (item #4)

**Problem.** `frame.spec.rs:20` declares:

```rust
pub assume_specification[ <PhysicalAddress as ::sys::mm::Address>::from_raw_value ](value: usize)
    -> (result: Result<PhysicalAddress, ::sys::error::Error>)
    ensures match result { Ok(pa) => pa@ == value as int, Err(_) => true };
```

`PhysicalAddress` lives in `crate::hal::mem::types::address::phys` — an
**intra-crate** kernel type, not a std/external crate. The checklist allows
`assume_specification` **only** for external-bottom std/external boundaries. This
one supplies a *trusted, unverified* contract to a kernel-internal callee:
I confirmed `<PhysicalAddress as Address>::from_raw_value` at `phys.rs:193` carries
**no** `#[verus_spec]`, and the `nanvix-phys-hal-phys-address` pipeline already
finished (`completed_phases: [proving, cheating-elimination]`) **without** speccing
it. It is also **not** present in `tcb-allowed.md`'s intra-crate assume list
(lines 180-190 list `PhysicalAddress::into_frame_number`, the `PageAligned` ones,
and `FrameAddress::from/into_frame_number` — but **not** `from_raw_value`). So this
is an unsanctioned intra-crate trust hole.

**Required change — delete the `assume_specification` from `frame.spec.rs`.** Pick one:

- **Option A (preferred — removes the trust):** Give the real callee a verified
  spec. Add to `<PhysicalAddress as ::sys::mm::Address>::from_raw_value`
  (`src/kernel/src/hal/mem/types/address/phys.rs:193`):
  ```rust
  #[verus_spec(result =>
      ensures match result { Ok(pa) => pa@ == value as int, Err(_) => true },
  )]
  ```
  and verify it in-body (it delegates to `from_virtual_address(VirtualAddress::from_raw_value(value))`).
  Then remove `frame.spec.rs:20-27`. No trust remains.

- **Option B (TCB-sanctioned fallback):** Restore `FrameAddress::from_raw_value`
  (`frame.rs:99`) to `#[verus_verify(external_body)]` keeping its existing
  `#[verus_spec]` ensures — this exact function is already approved in
  `tcb-allowed.md:137-138` — and remove `frame.spec.rs:20-27`. This trades an
  unsanctioned intra-crate `assume_specification` for a TCB-listed `external_body`.

Do **not** keep the `assume_specification` and merely re-justify it in a comment —
justification is not a fix; the intra-crate trusted contract must be removed.

**Verify after the change:**
1. `grep -rn 'assume_specification\|assume(' src/kernel/src/hal/mem/types/address/frame*.rs`
   → must return nothing.
2. `make verify-kernel` → exit 0, module `frame` verified, 0 errors, 0 warnings
   (run non-cached, e.g. touch a frame file or clear the cache).
3. `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/spec_drift.py git-diff
   src/kernel/src/hal/mem/types/address/frame.spec.rs --before 1f6205c56`
   (Option A must show no *weakening*; removing an intra-crate assume that is
   replaced by an equally-strong real spec is acceptable — confirm the `Ok`
   postcondition `pa@ == value as int` is preserved verbatim on the real callee).
