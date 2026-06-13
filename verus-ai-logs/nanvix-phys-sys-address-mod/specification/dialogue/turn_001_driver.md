## Turn 1: Full checklist walk-through (specification phase, `sys::mm::address` / `Address` trait)

### Progress
- Done (PASS/FIXED, all with evidence below):
  1. Every in-scope exec function has requires/ensures — PASS
  2. Caller coverage — PASS
  3. View consistency — PASS
  4. No tautological ensures — PASS
  5. No subsumed ensures — PASS
  6. Error paths have meaningful ensures — PASS
  7. No assume_specification for workspace-internal code — PASS
  8. vstd searched before any assume_specification — PASS (N/A)
  9. Specs written for the caller — PASS
  10. Trait obligations satisfied — PASS
  11. Spec completeness (advisory) — PASS
  12. Loop invariants — PASS (N/A, no loops)
  13. No cheating on module's own functions — PASS
  14. No specs weakened — PASS
  15. Bug awareness — PASS (no bugs found)
  16. Cross-module regression — PASS
  17. Verification (`make verify-sys` + build) — PASS
- Current: none (all items resolved)
- Remaining: none

### Scope
In-scope = the `Address` trait method declarations in
`src/libs/sys/src/sys/mm/address/mod.rs`, specifically
**`from_raw_value`, `into_raw_value`, `is_aligned`** (per `caller_analysis.md`
line 152 and `view_design.md` line 185). `align_up`, `align_down`, `max_addr`,
`clone_address`, `as_ptr`, `as_mut_ptr` are out of scope (not modified).
`virt.rs` is a sibling module (`sys::mm::address::virt`), separately scoped.

### Verification (commands run, evidence)

**`make verify-sys`** → exit 0. Verified `sys::mm::address` and
`sys::mm::address::virt`. Cheating line:
`assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=1`.

**Item 1 — in-scope ensures.** `fn_coverage.py` reports 0 exec fns (mod.rs is a
trait declaration only; bodies live in implementors). Manual read of the trait:
- `from_raw_value`: `ensures match result { Ok(a) => a@ == raw_addr as int, Err(e) => e.code == ErrorCode::BadAddress }` ✔
- `into_raw_value`: `ensures result as int == self@` ✔
- `is_aligned`: `ensures result matches Ok(aligned) && aligned == spec_addr_is_aligned(self@, align)` ✔
All three in-scope methods carry ensures. PASS.

**Item 2 — caller coverage.** Cross-checked `caller_analysis.md`:
- round-trip `from_raw_value(r)@ == r` ✔ (Ok arm)
- `BadAddress` on failure (used by `?`-propagation in `PageAligned`/
  `PageTableAligned` blanket impls + kernel tests) ✔ (Err arm)
- `into_raw_value` lossless `result as int == self@` ✔
- `is_aligned == (self@ % align == 0)` ✔
Every caller expectation has a corresponding ensures. PASS.

**Item 3 — view consistency.** `view_design.md` fixes `View = int` (mandated by
`View<V = int>` supertrait); all three specs reference `self@`/`raw as int` and
the alignment predicate. `is_aligned` uses helper
`spec_addr_is_aligned(v,align) := v % spec_align_value(align) == 0`
(`mod.spec.rs`). The dropped `spec_max_addr`/`addr_wf` machinery is documented
(view_design.md §"Specification-phase update") and justified (PhysicalAddress
sparse validation makes `Err ⇔ raw > max_addr` untrue). No inv() method exists
because the view is the primitive `int`. Consistent. PASS.

**Item 4 — no tautological ensures.** No `Err(_) => true`. `from_raw_value` Err
arm pins `e.code == BadAddress`; `is_aligned` ensures forbid Err entirely
(conjunction `result matches Ok(aligned) && ...`). PASS.

**Item 5 — no subsumed ensures.** No inv()/other ensures to derive from; each
clause carries independent information. PASS.

**Item 6 — error paths meaningful.** `from_raw_value` Err pins the error code
(`BadAddress`), relied on by tests at test.rs:106–122. `is_aligned` proves
totality (never Err). PASS.

**Items 7–8 — assume_specification / vstd.** `grep` over mod.rs, mod.spec.rs,
mod.proof.rs → no `assume_specification`, `assume`, or `admit`. PASS (N/A).

**Item 9 — specs for the caller.** Contracts are stated purely over `self@` /
`raw as int` / alignment & error-code predicates — directly usable in
`PageAligned`/`MemoryRegion`/kernel-test proofs. PASS.

**Item 10 — trait obligations.** Trait-level contracts match the semantic
contract every implementor must honor (VirtualAddress always-Ok `is_aligned`;
round-trip `from_raw_value`; lossless `into_raw_value`). Documented in
caller_analysis §"Trait Obligations". PASS.

**Item 11 — spec completeness (advisory).** `from_raw_value`'s Err *condition*
is intentionally nondeterministic (does not pin WHEN it errs) because validity is
dynamic/per-platform (sparse physical memory). This matches caller expectations
("callers don't care how validity is computed", caller_analysis l.106–109).
Acceptable intentional nondeterminism. PASS.

**Item 12 — loop invariants.** No loops in mod.rs (trait declarations). PASS.

**Item 13 — no cheating.** `grep -E 'admit|assume|external_body|trusted|
assume_specification'` over the three mod files → NONE. Verifier cheating
counts: admit=0 external_body=0 trusted=0 assume=0. The `cfg_gate=1` is the
conventional `#[cfg(verus_keep_ghost)]` include guard (verify.sh
`count_cfg_gates` matches `verus_keep_ghost`), not one of the prohibited
patterns. PASS.

**Item 14 — no specs weakened.** `spec_drift.py git-diff … --before
192f966ee~1 --after 8f9b8dd4b`:
`Functions added: 1, Ensures removed: 0, Requires added: 0, Contract drift: 0`.
Read of the pre-phase trait confirms: `into_raw_value` and `clone_address`
unchanged; `is_aligned` rewritten via the open helper `spec_addr_is_aligned`
which unfolds to the identical predicate (`self@ % spec_align_value(align)==0`)
— equivalent, not weaker; `from_raw_value` strengthened from no-spec to a full
Ok/Err contract. PASS.

**Item 15 — bug awareness.** No fundamentally incorrect code. The trait `Ok`
arm `a@ == raw_addr` is satisfiable by all implementors; PhysicalAddress's
stricter validity only narrows the Err arm, which is consistent. Recorded in
`specification/bugs.md` (none found). PASS.

**Item 16 — cross-module regression.** `make verify-sys` (the project's verus
command for this crate) verifies all `sys` modules → exit 0, 0 errors. No
regression in sibling `virt`. PASS.

**Item 17 — verification + build.** `make verify-sys` → exit 0, 0 errors.
`./z build -- all` → `[OK] Build complete.` (confirms the dual non-Verus build,
including the incidental duplicate-`use` fix recorded in view_design.md). PASS.

### Fix Request
None. All 17 checklist items PASS with concrete tool evidence. No code changes
required this turn.

### Result
RESOLVED.
