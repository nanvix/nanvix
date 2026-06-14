## Turn 1: Full checklist walk-through — FAIL on `assume_specification` for workspace-internal code

### Progress
- **PASS (13):**
  1. In-scope exec functions have requires/ensures
  2. Caller coverage
  3. View consistency
  4. No tautological ensures
  5. No subsumed ensures (the `into_raw_value` `0<=self@<=spec_max()` clause is required for callers, see below)
  6. Error paths have meaningful ensures
  8. vstd searched before assume_specification (no vstd alternative for `MAX`)
  9. Specs written for the caller
  10. Trait obligations satisfied (none — only `Debug/Clone/Copy`)
  11. Spec completeness (advisory)
  12. Loop invariants (no loops — N/A)
  13. No cheating on module's own functions (admit/assume/external_body/trusted = 0)
  14. No specs weakened (spec-drift: kernel placeholders *superseded*, not weakened)
  15. Bug awareness (no bugs)
  16. Cross-module regression (`make verify` exit 0; kernel cheating counts unchanged from baseline)
  17. Verification (`make verify-arch` exit 0, CLEAN)
- **Current FAIL (1):** Item 7 — *No assume_specification for workspace-internal code*
- **Remaining:** none unverified; only the FAIL above must be fixed.

### Verification

**Tools run:**
- `make verify-arch` → exit 0, status CLEAN, cheating: assume=0 external_body=0 admit=0 trusted=0.
- `make verify` (all crates + kernel) → exit 0. Kernel cheating counts (admit=28 external_body=11 cfg_gate=15) are **pre-existing** and identical to the pre-phase baseline (commit `7445e2b4c`), so this phase introduced none.
- `fn_coverage.py number.rs` → 4/4 exec fns matched; in-scope `from_raw_value`, `into_raw_value` both carry contracts. Tests are out of scope.
- `spec_drift` (manual git diff `860ea19a..HEAD`): kernel `phys.spec.rs` / `frame.spec.rs` changed only to **remove now-superseded placeholders** (`assume_specification[from_raw_value/into_raw_value]`, `ExFrameNumber`) and turn `spec_frame_raw_value`/`spec_max_frame_number` from `uninterp` into `open` delegations to the arch quantities. Arch ensures match the removed kernel ensures verbatim. No weakening.
- `grep assume_specification src/libs/**` → every other use is on an **external/std** item (`Error::new`, `<*mut T>::is_null`, `<usize>::div_ceil`, pointer cmp). `FrameNumber::MAX` is the **only** assume_specification on workspace-internal code.

**The failing item — `number.spec.rs:47`:**
```rust
pub assume_specification[ FrameNumber::MAX ] -> (result: usize)
    ensures
        result as int == FrameNumber::spec_max() as int,
;
```
`FrameNumber::MAX` is defined in **this same file** (`number.rs:39`):
```rust
pub const MAX: usize = mem::MAX_ADDRESS / mem::FRAME_SIZE - 1;
```
The source comment and `view_design.md` justify the assume_specification by claiming `MAX`'s body "bottoms out at build-time page-size constants that Verus treats as external." I checked `constants.rs`:
- `MAX_ADDRESS: usize = usize::MAX`  (constants.rs:90)
- `FRAME_SIZE: usize = PAGE_SIZE`    (constants.rs:104)
- `PAGE_SIZE: usize = 4096`          (constants.rs:47)

These are **concrete literals**, not external symbols. `usize::MAX / 4096 - 1` is fully evaluable by Verus. The "external boundary" framing only holds *cross-crate* (as in the kernel, where `arch` is an unverified dependency) — but here we are verifying the `arch` crate itself, so `MAX` is internal and concrete. Therefore the assume_specification is on workspace-internal code with a value that is not actually external. **FAIL.**

### Fix Request

Eliminate the `assume_specification[ FrameNumber::MAX ]` (workspace-internal). Choose ONE:

1. **Prove the binding instead of assuming it.** Give `FrameNumber::MAX` a real verified contract tying its exec value to `spec_max()` (e.g. move it into the `#[verus_verify]` impl with `#[verus_spec(ensures result as int == Self::spec_max() as int)]`, discharged from the concrete const body), and delete the `assume_specification`. Keep `spec_max()` as the abstract bound but make the connection a *proof*, not an assumption.

2. **Make `spec_max()` interpreted.** Define it as an `open`/`closed spec fn` computing the concrete bound (mirroring `MAX_ADDRESS / FRAME_SIZE - 1`) so no assumption is needed, then prove `from_raw_value`/`into_raw_value` against it. (Verify the kernel's `spec_max_frame_number()` delegation still holds — it must remain equally strong.)

3. **Only if neither works:** produce concrete Verus tool output proving that `FrameNumber::MAX` genuinely cannot be interpreted within the `arch` crate (i.e. it truly resolves to an external symbol). A prose justification is **not** acceptable — show the error/output.

**Verify after fixing:**
- `grep -rn "assume_specification" src/libs/arch/src/x86/mem/paging/frame/` → must return **no** workspace-internal target.
- `make verify-arch` → exit 0, CLEAN.
- `make verify` → exit 0 (kernel must still pass; `spec_max_frame_number()` / `spec_frame_raw_value()` delegations unchanged in strength).

Do not weaken `from_raw_value`/`into_raw_value` ensures while fixing.
