## Turn 1: No cheating on module's own functions (`admit` in `frame_address`)

### Progress
- Done (PASS): every-exec-has-contracts, caller-coverage, view-consistency,
  no-tautological-ensures, error-path-ensures (N/A), no-assume_specification,
  vstd-searched (N/A), specs-for-caller, trait-obligations, loop-invariants (N/A),
  no-specs-weakened (spec-drift), bug-awareness, cross-module-regression, verification+build.
- Done (advisory / PASS-with-note): no-subsumed-ensures, spec-completeness.
- Current (FAIL): **No cheating on module's own functions** — `admit()` in `frame_address`.
- Remaining: none — every item has been verified this turn.

### Verification

In-scope functions (from `view_design.md` / `caller_analysis.md`):
`PageDirectoryEntryFlags::new`, `PageDirectoryEntry::new`,
`PageDirectoryEntry::is_present`, `PageDirectoryEntryFlags::is_present`,
`PageDirectoryEntry::frame_address`.

**1. Every in-scope exec fn has requires/ensures — PASS.**
All five carry `#[verus_spec(... ensures ...)]`:
- `PageDirectoryEntryFlags::new` → `result@ == spec_pde_flags_new(...8 args...)` (pde.rs:84–96)
- `PageDirectoryEntry::new` → `result@ == spec_pde_new(flags@, frame@)` + `result.inv()` (pde.rs:308–312)
- `PageDirectoryEntry::is_present` → `result == self@.flags.present` (pde.rs:380–382)
- `PageDirectoryEntryFlags::is_present` → `result == self@.present` (pde.rs:129–131)
- `PageDirectoryEntry::frame_address` → `result as int == self@.frame * FRAME_SIZE`,
  `result % FRAME_SIZE == 0` (pde.rs:409–413)

**2. Caller coverage — PASS.** Cross-checked each expectation in `caller_analysis.md`:
- Inv 1 (flags fidelity, 8 args) ↔ `spec_pde_flags_new` records all eight. ✓
- Inv 2 (entry pairs flags+frame) ↔ `spec_pde_new`. ✓
- Inv 2b (`frame_address == frame.into_raw_value() << FRAME_SHIFT`): spec gives
  `self@.frame * FRAME_SIZE`; `FrameNumber::into_raw_value` ensures `result as int == self@`
  (number.rs:79–83) and `FRAME_SIZE == 1 << FRAME_SHIFT`, so the spec form is the
  caller's form. ✓
- Inv 3 (presence delegation) ↔ `PageDirectoryEntry::is_present` ensures `self@.flags.present`,
  `PageDirectoryEntryFlags::is_present` ensures `self@.present`. ✓
- Inv 4 (frame-aligned) ↔ `result % FRAME_SIZE == 0`. ✓

**3. View consistency — PASS.** Specs reference View fields (`self@.present`,
`self@.flags.present`, `self@.frame`); `new` re-establishes `result.inv()`
(pde.spec.rs:145–147). Views are `closed` (encoding-independence, inv 6).

**4. No tautological ensures — PASS.** No `Err(_) => true` / vacuous clauses.

**5. No subsumed ensures — PASS (with note).** `frame_address`'s alignment clause
`result % FRAME_SIZE == 0` is derivable from `result == self@.frame * FRAME_SIZE`.
view_design.md:251–256 keeps it deliberately because `verify_kernel_mappings`
compares frame addresses; exposing the derived modular fact spares every caller a
re-derivation. Acceptable, not noise.

**6. Error-path ensures — PASS (N/A).** No in-scope fn returns `Result`/`Option`
(`is_present`→bool, `frame_address`→usize, `new`→Self).

**7. No assume_specification for workspace-internal code — PASS.** `grep` of
pde.rs/.spec.rs/.proof.rs finds none. (The placeholders in
`kernel/.../identity_map.spec.rs` are the downstream contract this module supersedes,
not code in this module.)

**8. vstd searched — PASS (N/A).** No `assume_specification` introduced here.

**9. Specs for the caller — PASS.** Abstract Views (`PdeFlagsView`/`PdeView`, `int`
frame) are directly usable in kernel proofs; no `PteWord`/`FrameNumber` internals leak.

**10. Trait obligations — PASS.** `TableEntry` methods (`from_raw`/`raw`) are
out of scope (view_design.md:12–16) and unspecced; in-scope specs match the
constructor/query semantics callers rely on.

**11. Spec completeness (advisory) — PASS-with-note.** In-scope contracts are
two-sided and total; the eight-bit flags View makes `new`'s spec reject an impl
that drops any argument. No nondeterminism.

**12. Loop invariants — PASS (N/A).** No loops in pde.rs.

**13. No cheating on module's own functions — FAIL.**
`make verify-arch` cheating report + `grep` confirm exactly one violation in this module:
```
cheating: assume=0 external_body=2 admit=2 trusted=0 ...
- x86/mem/paging/pde.rs:414 frame_address: admit      <-- THIS MODULE
- x86/mem/paging/table.proof.rs:8 lemma_entry_roundtrip: admit   (other module)
- x86/mem/paging/table.rs:209/246 read/write: external_body      (other module)
```
`grep -n "admit"` of pde.rs → line 415: `proof! { admit(); }` inside
`frame_address`. `admit` is NOT in the temporarily-allowed list (only
`assume_specification` on external deps is). The function's `ensures`
(`result as int == self@.frame * FRAME_SIZE`, `result % FRAME_SIZE == 0`) is
therefore unproven — it is asserted by fiat, not verified.

I confirmed the spec is **correct and provable without admit** (so this is a real
cheat to remove, not a spec defect):
- `self.frame.into_raw_value()` ensures `r as int == self.frame@ == self@.frame`
  and `0 <= r <= FrameNumber::spec_max()` (number.rs:79–83).
- `FRAME_SIZE == 1 << FRAME_SHIFT`; with `r <= MAX_ADDRESS/FRAME_SIZE - 1`,
  `r * FRAME_SIZE` cannot overflow `usize`, and `(r << FRAME_SHIFT) as int == r * FRAME_SIZE`.
- `inv()` (pde.spec.rs:145–147) already supplies the bound.

`pde.proof.rs` is currently empty (`verus! { }`), so the admit is masking the
missing proof rather than discharging it.

**14. No specs weakened (spec drift) — PASS.** Downstream placeholders in
`identity_map.spec.rs` were contract-free (signature-only `assume_specification`);
this module *adds* `ensures` (strengthens). `git diff --stat HEAD` shows no changes
to existing committed spec/proof contracts elsewhere. No spec fn weakened.

**15. Bug awareness — PASS.** `frame_address` body
(`into_raw_value() << FRAME_SHIFT`) is functionally correct; the admit hides a
*proof* gap, not a code bug. No fundamentally incorrect code found; bugs_file empty.

**16. Cross-module regression — PASS.** `make verify-arch` runs all arch modules,
Exit code 0, "cached (no recompilation)". No module fails verification. (Overall
status reads `CHEATING_DETECTED` solely because of the admit counts above.)

**17. Verification + build — PASS.** `make verify-arch` → exit 0, 0 verification
errors. `./z build` → exit 0, "[OK] Build complete." (full standalone image built).

### Fix Request

**Remove the `admit()` from `PageDirectoryEntry::frame_address` and discharge its
`ensures` with a real proof. Justification is not a fix — delete the admit and
make `make verify-arch` report `admit=0` for pde.rs.**

Concretely:

1. In `src/libs/arch/src/x86/mem/paging/pde.rs`, delete line 415
   `proof! { admit(); }` from `frame_address`.

2. Prove the two `ensures` clauses. Suggested approach — bind the raw value and use
   a shift-equals-multiply fact:
   ```rust
   pub fn frame_address(&self) -> usize {
       proof! { use_type_invariant(self); }            // gives self@.frame <= spec_max()
       let raw = self.frame.into_raw_value();           // raw as int == self@.frame, raw <= spec_max()
       // prove (raw << FRAME_SHIFT) as int == raw * FRAME_SIZE and no overflow here
       raw << crate::mem::FRAME_SHIFT
   }
   ```
   Put any reusable arithmetic in `pde.proof.rs` (currently empty) as a `proof fn`
   lemma, e.g. a lemma that for `0 <= x <= FrameNumber::spec_max()`,
   `FRAME_SIZE == 1 << FRAME_SHIFT` ⇒ `(x << FRAME_SHIFT) as int == x * FRAME_SIZE`
   and `x << FRAME_SHIFT <= usize::MAX`. You likely need `FRAME_SIZE`/`FRAME_SHIFT`
   spec facts (`mem::FRAME_SIZE == 1 << mem::FRAME_SHIFT`) and
   `vstd::bits`/`shift` lemmas — search vstd first; do not introduce a new
   `assume`/`admit`/`external_body`.

3. Verify:
   - `make verify-arch` → must show `cheating: assume=0 external_body=2 admit=1 ...`
     i.e. the pde.rs admit gone (only the unrelated `table.proof.rs` admit may
     remain), and exit code 0 with no new verification errors.
   - `grep -n "admit" src/libs/arch/src/x86/mem/paging/pde.rs` → no matches.
   - `./z build` → still exit 0.

Do not change the `ensures` to make it easier — the spec is already correct and
matches caller invariants 2 and 4. The required change is to *prove* it.

### Note (out of scope for this review, do not fix here)
`table.proof.rs:8` admit and `table.rs` read/write `external_body` belong to the
`table` module, not `pde`. They are flagged by the global counter but are not part
of the arch-x86-pde scope; leave them to their own module's review.
