## Turn 1: Specification — in-scope function coverage (`Address::into_raw_value`)

### Progress
- Done (PASS):
  - No tautological ensures — current ensures are concrete equalities, none `=> true`.
  - No subsumed ensures — `result@ == value as int` is not derivable from `inv()` (== true).
  - Error paths meaningful — N/A: all three in-scope fns are total/infallible (no `Result`/`Option` in scope).
  - No `assume_specification` for workspace-internal code — none present.
  - vstd searched before `assume_specification` — N/A: no `assume_specification` used.
  - Trait obligations (partial) — `new`/inherent `from_raw_value` match caller contract.
  - Loop invariants — N/A: no loops in module.
  - No cheating on module's own functions — grep in `virt.rs`/`virt.spec.rs`/`virt.proof.rs`:
    `admit=0 assume=0 external_body=0 trusted=0 no_decreases=0` (verify-sys cheating check: all 0;
    `cfg_gate=1` is a crate-wide platform `#[cfg(target_pointer_width=...)]`, not in virt files, not a cheat).
  - No specs weakened — specs were only added; nothing pre-existing was relaxed.
  - Bug awareness — no fundamentally incorrect code in the three in-scope fns; all are pure identity
    wraps/projection. (Out-of-scope `Add`/`AddAssign` use unchecked `+`, but not in scope; no bug recorded.)
  - Cross-module regression — `make verify-sys` exit 0, module `sys::mm::address::virt` verified, cached.
- Current: **Item 1 — every in-scope exec function has requires/ensures** → **FAIL**.
- Remaining (blocked on the FAIL fix, re-check after):
  - Caller coverage (into_raw_value inverse/round-trip expectation).
  - View consistency for `into_raw_value`.
  - Spec completeness (advisory).
  - Verification + build re-run after the fix.

### Verification
- Scope (verification-plan.json, module `sys::sys::mm::address::virt`): the type `VirtualAddress`,
  `VirtualAddress::new`, `VirtualAddress::into_raw_value`, `VirtualAddress::from_raw_value` (inherent).
- `make verify-sys`: exit 0, but **coverage = 2/255**; `coverage-unverified.txt` lists `into_raw_value`
  as unverified (line 170).
- `grep -n verus_spec src/.../virt.rs` → only lines 49 (`new`) and 66 (inherent `from_raw_value`).
  `into_raw_value` (line 254) has **no** `#[verus_spec]`.
- The `impl Address for VirtualAddress` block (line 168) is **not** annotated `#[verus_verify]`
  (unlike the inherent `impl` at line 47 which is).
- view_design.md §4 specifies the required contract:
  `Address::into_raw_value(self) -> usize` → `ensures result as int == self@`
  (total projection, exact inverse of construction; underwrites the round-trip identity that
  `pm/sync.rs` and `mm/mmio.rs` callers rely on per caller_analysis.md).

### Fix Request
`Address::into_raw_value` is an in-scope verification target but carries no `ensures`, so the
caller-required inverse/round-trip property (`from_raw_value(x).into_raw_value() == x`) is unproven.
A justification will not be accepted — add the spec.

Do exactly this in `src/libs/sys/src/sys/mm/address/virt.rs`:

1. Annotate the trait impl block so Verus processes it. Change line 168
   `impl Address for VirtualAddress {` to:
   ```rust
   #[verus_verify]
   impl Address for VirtualAddress {
   ```
2. Add the ensures to `into_raw_value` (currently line 254). Replace:
   ```rust
   fn into_raw_value(self) -> usize {
       self.0
   }
   ```
   with:
   ```rust
   #[verus_spec(result =>
       ensures
           result as int == self@,
   )]
   fn into_raw_value(self) -> usize {
       self.0
   }
   ```
   This matches view_design.md §4 (`result as int == self@`) and references the `closed` View `self@`.

Verify after the change:
- Run `make verify-sys`. Required outcome: exit 0 AND the Function Coverage line increases from
  `2/255` to `3/255` (i.e. `into_raw_value` no longer appears in `coverage-unverified.txt`).
- Confirm the cheating check still reports `assume=0 external_body=0 admit=0 trusted=0`.
- Run the build (`./z build` / the project build target) to confirm exec compilation is unaffected.

Do NOT add a spec to the out-of-scope trait `from_raw_value (-> Result)` or the alignment/`checked_*`
methods in this phase — keep the change limited to `into_raw_value`.
