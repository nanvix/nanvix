## Turn 1: Full checklist walk-through (FAIL on "No cheating on module's own functions")

### Progress
- Done (PASS):
  1. Every in-scope exec fn has requires/ensures
  2. Caller coverage
  3. View consistency
  4. No tautological ensures
  5. No subsumed ensures
  6. Error-path ensures (N/A — getters return values, not `Result`)
  7. No `assume_specification` for workspace-internal code
  8. vstd searched before `assume_specification` (N/A — none used)
  9. Specs written for the caller
  10. Trait obligations satisfied
  11. Spec completeness (advisory)
  12. Loop invariants (N/A — no loops in scope)
  14. No specs weakened (`spec_drift` clean; `inv` strengthened)
  15. Bug awareness (no bug in the 4 in-scope getters)
  16. Cross-module regression (`make verify-kernel` exit 0, all modules pass)
  17. Verification + build (`make verify-kernel` exit 0, compiled)
- Current: **Item 13 — No cheating on module's own functions → FAIL**
- Remaining: none (all other items verified PASS)

### Verification

In-scope functions (per `caller_analysis.md` / `view_design.md`):
`MemoryRegion::{start,size}`, `TruncatedMemoryRegion::{start,size}`.

Commands run and evidence:

- `make verify-kernel` → Exit code 0. Module `hal::mem::types::region` verifies.
  Summary line: `cheating: assume=0 external_body=14 admit=27 ... status: CHEATING_DETECTED`,
  `coverage: 49/1023`. The verification itself passes (cross-module regression OK).
- `fn_coverage.py region.rs region.rs` → 17/17 exec fns matched; the 4 in-scope
  getters each carry a `#[verus_spec(... ensures ...)]`:
  - `MemoryRegion::start`  → `ensures result@ == self@.start`
  - `MemoryRegion::size`   → `ensures result as int == self@.size`
  - `TruncatedMemoryRegion::start` → `ensures result@ == self@.start`
  - `TruncatedMemoryRegion::size`  → `ensures result as int == self@.size`
  These match `view_design.md` §"Spec Method Specifications" verbatim and satisfy
  every caller expectation in `caller_analysis.md`. Not tautological, not subsumed
  by `inv()`. PASS for items 1–5, 9–11.
- `spec_drift.py git-diff region.rs --before 7eb7892da --after HEAD` →
  "✅ No contract drift detected" (0 ensures removed, 0 requires added).
  `TruncatedMemoryRegion::inv` was *strengthened* (added `self@.wf()` i.e. `size > 0`
  on top of the two `% spec_page_size() == 0` clauses); `MemoryRegion::inv` added.
  No guarantee weakened. PASS for item 14.
- Cheating grep on the module's own files:
  `grep -n "admit|assume|external_body|trusted|assume_specification"`
  on `region.rs` / `region.spec.rs` / `region.proof.rs` →
  **exactly one violation**: `region.rs:221  proof! { admit(); }` inside
  `MemoryRegion::start`. Confirmed by `cheating-detail.txt`:
  `- hal/mem/types/region.rs:214 start: admit`. `region.proof.rs` admit count = 0.
  `region.spec.rs` clean. No `external_body` / `trusted` / `assume_specification`
  in any module file.

So the ONLY checklist failure is item 13, localized to ONE function.

### FAIL — Item 13: `admit()` in `MemoryRegion::start` (region.rs:207–223)

```rust
#[verus_spec(result =>
    ensures
        result@ == self@.start,
)]
pub fn start(&self) -> T {
    // ... comment deferring to a future Address clone contract ...
    proof! { admit(); }      // <-- cheating on the module's own function
    self.start.clone()
}
```

This `admit()` discharges `self.start.clone()@ == self.start@` unconditionally.
That is exactly the prohibited pattern: a proof cheat on this module's own
in-scope function. The accompanying comment ("discharged once the address layer
exposes a clone contract; deferred here") is a **justification, not a fix** — per
the review rules, justification does not clear the item.

Root cause is real: `T: Address` requires `Clone` (supertrait) + `View<V = int>`,
but `core::clone::Clone::clone` has no Verus contract, so `clone(x)@ == x@` is
not derivable for a generic `T`. I confirmed:
- vstd offers only `vstd::pervasive::{strictly_cloned, cloned}`
  (`call_ensures(T::clone, (&a,), b)`); these yield nothing without a clone
  contract in scope.
- `assume_specification` cannot target the generic `<T as Clone>::clone`, and is
  in any case disallowed for workspace-internal code (item 7) — `Address` lives
  in `src/libs/sys`, which is workspace-internal.
- `Address::into_raw_value` (`ensures result as int == self@`) cannot relate the
  *cloned* value's view to the original's; `Eq`/`Ord` carry no view-equality spec.

Therefore the obligation must be discharged by giving the address layer a
**view-preserving clone contract**, then using it in `start()` — not by `admit()`.

### Fix Request (do exactly this)

1. **Add a view-preserving clone obligation to the `Address` trait**
   (`src/libs/sys/src/sys/mm/address/mod.rs`, the `#[verus_verify] pub trait Address`):
   add a spec'd method, e.g.

   ```rust
   #[verus_spec(result =>
       ensures
           result@ == self@,
   )]
   fn clone_address(&self) -> Self;
   ```

   Provide its (trivial) implementation for every `Address` impl
   (`PhysicalAddress`, `VirtualAddress`, `PageAligned<T>`, `FrameNumber`, and any
   other) as `self.clone()` — discharge each impl's `result@ == self@` against
   that type's existing view definition (these are concrete, non-generic, so the
   clone there is provable or at worst uses the type-local mechanism already in
   that module). Do NOT use `assume_specification` on `Clone` (item 7).

2. **Use the contract in `MemoryRegion::start`** and delete the cheat:

   ```rust
   #[verus_spec(result =>
       ensures
           result@ == self@.start,
   )]
   pub fn start(&self) -> T {
       self.start.clone_address()
   }
   ```

   Remove the `proof! { admit(); }` line and the deferral comment.

   (`TruncatedMemoryRegion::start` delegates to `self.0.start()`, so it is
   discharged automatically once `MemoryRegion::start` is clean — no change
   needed there.)

3. **Verify**:
   - `make verify-kernel` must exit 0 AND the Cheating Pattern Check must no
     longer list `hal/mem/types/region.rs:... start: admit`
     (check `verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt`).
   - `grep -n "admit" src/kernel/src/hal/mem/types/region.rs` → no matches.
   - Cross-module: the same `make verify-kernel` run must keep ALL previously
     verified modules passing (especially `hal::mem::types::address::*` after the
     trait change), not just `region`.

If — after implementing step 1 — you find the trait change genuinely cannot be
completed in this phase (e.g., it forces unprovable obligations in a concrete
address impl that itself depends on an unverified primitive), do NOT fall back to
`admit()`. Instead report the concrete blocking `make verify-kernel` error so we
can decide between a dependency-contract task and escalation. A clone contract on
`Address` is a dependency spec-interface extension (the same category the
view-design already anticipated for `max_addr`), so it belongs here, not in a
view-design rollback (view-design only controls the View struct / `inv()` /
transition functions, none of which can supply a clone contract).

### Status
- 16 of 17 checklist items PASS.
- 1 FAIL: item 13 (admit in `MemoryRegion::start`). Fix request issued above.
- STOP not created (open failure remains).
