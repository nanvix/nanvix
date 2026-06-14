## Turn 1: Full Specification-Phase Checklist Review — `arch::x86::mem::paging` (`mod.rs`)

### Progress
- Done (PASS/verified): ALL 16 checklist items below.
- Current: Final verdict — RESOLVED.
- Remaining: none.

Module in scope: `src/libs/arch/src/x86/mem/paging/mod.rs`.
Single in-scope exec function: `invlpg` (per `verus-ai.toml` background and
`pipeline_state.json`). Sibling modules (`table`, `pde`, `pte`, `flags`,
`frame`) are out of scope here; their allow-listed TCB markers were approved in
earlier phases.

---

### Verification (commands actually run)

- `make verify-arch` → **exit 0**, verification *passed* (cached, no
  recompilation). Cheating-pattern summary for the whole `arch` crate:
  `assume=0 external_body=3 admit=1 trusted=0`. Per-marker detail
  (`verus-ai-logs/verify-arch/verus-logs/cheating-detail.txt`):
  - `x86/mem/paging/mod.rs:80 invlpg: external_body`  ← in scope
  - `x86/mem/paging/table.rs:209 read: external_body` ← sibling (table phase)
  - `x86/mem/paging/table.rs:246 write: external_body` ← sibling (table phase)
  - `x86/mem/paging/table.proof.rs:8 lemma_entry_roundtrip: admit` ← sibling
- `fn_coverage.py mod.rs mod.rs` → Source exec fns 1, Verus exec fns 1,
  Matched 1 (`invlpg`), Missing 0, Extra 0.
- `spec_drift.py check arch-paging-mod` → **✅ No contract drift detected**
  (ensures removed 0, requires added 0, functions removed 0).
- `cargo build -p arch` (kernel target, build-std) → **exit 0**, `Finished`.
- `git diff <spec-phase-start 4c73acbb>` → the only change is adding
  `#[verus_verify(external_body)]` + a trust-boundary comment to `invlpg`;
  `mod.spec.rs` / `mod.proof.rs` remain `verus! { }`.

---

### Per-item determinations

1. **Every in-scope exec function has requires/ensures** — **PASS.**
   `invlpg` is the only in-scope fn. It is an external-bottom hardware shim
   (`core::arch::asm! invlpg`). Per `caller_analysis.md` and `view_design.md`
   the *faithful* contract is empty: no `requires` (any `usize` accepted),
   trivial `ensures` (`()`, no Rust-visible effect). Writing an explicit
   `ensures true` would itself be a tautological-ensures violation (item 4), so
   the empty contract is the correct realization, not a gap. fn_coverage 1/1.

2. **Caller coverage** — **PASS.** `caller_analysis.md` enumerates all call
   sites (`identity_map.rs:668`, `page_table.rs:210/329/385/433/498`,
   `page_directory.rs:170`). Every caller relies only on: accepts any `usize`
   (⇢ no `requires`), infallible side-effect-only on TLB, preserves all
   caller-side invariants (⇢ trivial `ensures`, empty footprint). The empty
   contract satisfies each expectation; it also matches the inherited upstream
   `assume_specification[::arch::mem::paging::invlpg]` (no requires/ensures).

3. **View consistency** — **PASS.** `view_design.md` concludes *no non-trivial
   View* (empty `PagingView`, `inv() == true`) because the only effect is on
   unobservable hardware TLB state. The empty spec references no View fields,
   exactly as designed. No `inv()` obligation exists to maintain.

4. **No tautological ensures** — **PASS.** No `ensures` clause is present
   (verified: `mod.spec.rs`/`mod.proof.rs` empty; no `#[verus_spec]` on
   `invlpg`). Nothing tautological written.

5. **No subsumed ensures** — **PASS.** No `ensures` clauses at all.

6. **Error paths have meaningful ensures** — **PASS (N/A).** `invlpg -> ()` is
   infallible; there is no `Err` arm to specify.

7. **No assume_specification for workspace-internal code** — **PASS.**
   `grep` of `mod.rs`/`mod.spec.rs`/`mod.proof.rs` shows zero
   `assume_specification` in code (only a mention inside a doc comment).

8. **vstd searched before any assume_specification** — **PASS (N/A).** No
   `assume_specification` introduced in this module.

9. **Specs written for the caller** — **PASS.** The empty contract is directly
   usable in caller proofs and is identical in strength to the upstream
   `assume_specification` already relied on by `identity_map`.

10. **Trait obligations satisfied** — **PASS.** `caller_analysis.md`: `invlpg`
    is a free `unsafe fn`, member of no trait; no `Drop`/`GlobalAlloc`/
    `Iterator` dispatch. Nothing to satisfy.

11. **Spec completeness (advisory)** — **PASS (advisory).** The empty contract
    is *intentional* and is justified against every caller in
    `view_design.md` §Design Rationale / Rejected Alternatives (TLB modeling,
    flush-log, last_vaddr, status flag, ring-0 `requires` all rejected with
    reasons). Intentional non-determinism matches caller expectations.

12. **Loop invariants** — **PASS (N/A).** `invlpg`'s body is a single `asm!`
    block; no loops.

13. **No cheating on module's own functions** — **PASS (allow-listed).**
    In-scope counts for `mod.rs`: `external_body=1` (`invlpg`), `admit=0`,
    `assume=0`, `trusted=0`. The single `external_body` is a genuine
    external-bottom hardware trust boundary: Verus emits
    `error: ... does not yet support ... inline-asm expressions`
    (reproduced in `verus-unsupported.md`), so the body is unverifiable by any
    mechanism. `invlpg` is explicitly enrolled in `verus-ai-logs/tcb-allowed.md`
    (section "external_body introduced while speccing `arch::x86::mem::paging`
    (`mod.rs`)") with full rationale, and `view_design.md` pre-approved exactly
    this choice (confirming external_body over assume_specification only after
    tcb-allowed.md listing). Same class as `table::read/write` and
    `frame::instance`. Challenged and accepted on evidence. The `admit`/
    `external_body` in `table.*` are sibling-module markers out of scope for
    this review and already approved in the `arch-paging-table` phase.

14. **No specs weakened (spec_drift)** — **PASS.** `spec_drift.py check`
    reports no contract drift (0 ensures removed, 0 requires added). Strength
    is identical to the inherited upstream empty `assume_specification`; nothing
    weakened.

15. **Bug awareness** — **PASS.** `invlpg`'s body is the canonical
    `invlpg ({0})` AT&T-syntax flush with `nostack, preserves_flags`. No
    fundamentally incorrect code; no `bugs.md` required for this module (none
    exists, correctly).

16. **Cross-module regression** — **PASS.** `make verify-arch` verifies the
    entire `arch` crate; exit 0, all modules pass (cached). The
    `CHEATING_DETECTED` status string reflects only allow-listed markers
    (table external_body ×2, table.proof admit ×1, invlpg external_body),
    every one recorded in `tcb-allowed.md`. Verification itself succeeds.

17. **Verification + build** — **PASS.** `make verify-arch` exit 0 (0 errors);
    `cargo build -p arch` (kernel target) exit 0 (0 errors).

---

### Fix Request
None. Every checklist item is PASS with concrete tool evidence. The
specification-phase deliverable for `mod.rs` (mark `invlpg` `external_body` at
the documented inline-asm hardware trust boundary, empty faithful contract,
enrolled in `tcb-allowed.md`, empty `mod.spec.rs`/`mod.proof.rs`) is correct,
unweakened relative to the inherited upstream contract, and both verifies and
builds cleanly.

### Verdict: RESOLVED
