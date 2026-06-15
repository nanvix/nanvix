## Turn 2: Item 7 (assume_specification registration) — verified FIXED → all items RESOLVED

### Progress
- Done (PASS / FIXED with evidence): all 17 items.
  - PASS (turn 1): 1–6, 8–12, 14–17.
  - FIXED (turn 2): 7.
  - Deferred-acceptable (flagged for proving phase, not a spec-phase blocker): 13
    (single `admit` = bridge lemma).
- Current: re-verification of item 7 after fixer's turn-1 change.
- Remaining: none.

### Verification

Fixer claim: added a `tcb-allowed.md` section registering the two new
`frame.spec.rs` `assume_specification`s; no source changes. Verified each claim:

1. **TCB registration present and complete** — confirmed by reading
   `verus-ai-logs/tcb-allowed.md:272–309`. New section
   `## Allowed assume_specification — hal::mem::types::address::frame library edge`
   registers BOTH boundaries by fully-qualified path:
   - `<crate::hal::mem::types::address::PhysicalAddress as sys::mm::Address>::from_raw_value`
     — states ensures (`Ok(r) => r@ == value as int && spec_frame_number(r@) <= spec_max_frame_number(); Err(_) => true`),
     cites the `usize as *const u8` Verus-unsupported blocker at `phys.rs:230`,
     the value-free-Err rationale, the `r@`-vs-`spec_addr` cycle avoidance,
     precedent (`kframe.spec.rs`/`page.spec.rs`), and the removal note.
   - `<crate::hal::mem::PageAligned<T> as core::ops::Deref>::deref`
     — states ensures (`spec_addr(result) == addr@`), core-trait boundary
     rationale, precedent `page.spec.rs`, removal note.
   This matches the documented discipline of the sibling sections (181, 217, 252).

2. **No source weakening** — `git status --short` shows only dialogue-log files
   dirty; `frame.rs` / `frame.spec.rs` / `frame.proof.rs` unchanged (the
   `assume_specification` contracts are byte-identical to turn 1). Confirmed
   docs-only fix.

3. **Re-ran `make verify-kernel` myself** → exit 0.
   Cheating scan: `assume=0 external_body=24 admit=1 trusted=0`. Coverage 51/1022.

4. **Frame-module cheating markers** — `grep frame.(rs|spec|proof)`
   cheating-detail.txt shows the module's ONLY marker is
   `frame.proof.rs:28 lemma_phys_view_is_spec_addr: admit` (item 13, deferred).
   The 24 `external_body`s are all in `mm/phys/*` (downstream proof-target
   module, pre-existing, out of this module's spec scope); none in `frame.rs`.
   `into_raw_value` remains body-verified (no `external_body`), as in turn 1.

The reported `status: CHEATING_DETECTED` is driven solely by the deferred
bridge-lemma `admit` (item 13) and the unrelated pre-existing `mm/phys`
`external_body`s — neither is a specification-phase defect for `frame`.

### Fix Request
None. Item 7 is FIXED (TCB registration added, verification exit 0, build exit 0
from turn 1 build, no spec weakening). All 17 checklist items are PASS/FIXED with
the single, agreed deferral of the proving-phase bridge-lemma `admit` (item 13).

Specification phase for `hal-frame-address` is RESOLVED.
