## Turn 2: AST consistency / exec-rewrite documentation + final verification & build

### Progress
- Done (PASS/FIXED, all verified):
  - Zero admit() — PASS (turn 1; re-confirmed `cheating-detail.txt` has no `address/phys` entry).
  - Zero assume() — PASS.
  - Zero trusted functions — PASS (`trusted=0`).
  - Zero exec_allows_no_decreases_clause — PASS (`no_decreases=0`).
  - Zero cfg-gated exec — PASS (only `#[cfg(verus_keep_ghost)]` ghost `include!`s).
  - Zero in-scope external_body — PASS (no `address/phys` entry in detector detail).
  - Each surviving external_body in TCB list — PASS / N/A (none in scope; the 19
    crate-wide are all out-of-scope `mm/phys/*`, `arch/*`, `bump_allocator`).
  - **AST consistency — FIXED.** `into_frame_number` restored to `dev` baseline → now
    MATCH. Two divergences remain but are policy-compliant (documented; see below).
  - **All exec rewrites have VERUS REWRITE comment + reproducer — FIXED.**
  - No specs weakened — PASS (`spec_drift … check` → 0 drift, turn 1; no spec edits since).
  - Cross-module regression — PASS (`make verify-kernel` → 97 verified, 0 errors).
  - **Verification + build, 0 errors / 0 warnings — FIXED/PASS (verified this turn).**
- Current: none — all items closed.
- Remaining: none.

### Verification (independent, not trusting fixer claims)

**AST re-run** (`ast_consistency.py … summary`):
`Consistent: NO (matched=15 mismatched=1 missing=0 extra=1)` — improved from 2/1.
- `PhysicalAddress::into_frame_number` — was MISMATCH → now **MATCH**. Confirmed the
  `shift` binding was removed and exec is byte-identical to `dev`
  (`let frame_number: usize = raw_addr >> mem::FRAME_SHIFT;`, lemma called with
  `mem::FRAME_SHIFT` directly).
- `PhysicalAddress::from_number` — **MISMATCH, now documented** (`VERUS REWRITE` at
  phys.rs:142–148 + reproducer).
- `PhysicalAddress::clone_address` — **EXTRA_IN_VERUS, now documented** (`VERUS REWRITE`
  at phys.rs:277–285 + `verification_todo.md`).

**`from_number` rewrite — independently proven mandatory (did not accept the claim):**
1. Ran the committed reproducer through Verus myself:
   `verus repro/from_number.rs` → `2 verified, 1 errors`, error
   `possible arithmetic underflow/overflow` on the single-line `bad` form; the split
   `good` form verifies. The reproducer faithfully models the opaque cross-crate
   `FrameNumber` (bound exposed only via `into_raw_value()`'s postcondition).
2. **Authoritative test on the real file:** I temporarily restored the single-line form
   (`let addr: usize = frame.into_raw_value() * mem::FRAME_SIZE;`, keeping the `proof!`)
   and ran `make verify-kernel` → **`precondition not satisfied` on
   `lemma_from_number_no_overflow(frame)`, 96 verified, 1 errors** (matches fixer's
   Attempt A). Restored the split form afterward. The split is genuinely required by a
   real Verus ordering limitation; it is a semantically-equivalent rewrite (same value,
   same ops, same complexity). Compliant.

**`clone_address` — independently confirmed mandatory & out-of-scope:**
- `git show dev:src/libs/sys/src/sys/mm/address/mod.rs | grep -c "fn clone_address"` → 0;
  `git show verus-ai-prove:… | grep -c` → 1. The method was added to the `sys::mm::Address`
  trait during the pipeline and is **non-defaulted** (`fn clone_address(&self) -> Self;`
  with `ensures result@ == self@`). Since `PhysicalAddress: Address`, the impl is
  mandatory and cannot be removed without editing the out-of-scope `sys` crate. Carries a
  `VERUS REWRITE (interface addition)` comment + `verification_todo.md` entry. Compliant.

**Final verification & build (this turn):**
- `make verify-kernel` → `97 verified, 0 errors` (exit 0). In-module CLEAN; crate-wide
  `external_body=19 admit=14 cfg_gate=19` unchanged from base, all out-of-scope
  (`cheating-detail.txt` has zero `address/phys` entries).
- `./z build` → exit 0. Strict warning scan (`grep -iE "warning: "` excluding the benign
  `Sysroot directory … not found` environment note) → **no compiler warnings**.

### Result: ALL ITEMS PASS / FIXED
No fix request. Every checklist item is satisfied with tool-verified evidence. The two
surviving AST divergences are policy-compliant: one is a Verus-limitation rewrite proven
mandatory (reproducer + real-file failure reproduced by the reviewer) with a `VERUS
REWRITE` comment and committed reproducer; the other is a mandatory out-of-scope trait
method, documented with `VERUS REWRITE` + `verification_todo.md`. Creating STOP = RESOLVED.
