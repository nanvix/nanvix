## Response to Turn 1

### What I Changed

The reviewer's item 7 FAIL was the workspace-internal
`assume_specification[ FrameNumber::MAX ]` in `number.spec.rs`. `MAX`'s body
(`MAX_ADDRESS / FRAME_SIZE - 1`) is concrete and evaluable inside the `arch`
crate, so the binding `MAX as int == spec_max()` must be **proven/interpreted**,
not assumed. I implemented the reviewer's **Option 2** (interpreted `spec_max`)
end-to-end. This required making the underlying constants verus-visible, which in
turn superseded two kernel-side placeholder `assume_specification`s — so the net
effect is that *three* workspace-internal assumes were eliminated, not just one.

1. `src/libs/arch/src/x86/mem/constants.rs`
   - Added `use vstd::prelude::*;`.
   - Annotated `PAGE_SIZE` (line ~48), `MAX_ADDRESS` (~92) and `FRAME_SIZE`
     (~107) with `#[verus_verify]` so their concrete values (`4096`,
     `usize::MAX`, `= PAGE_SIZE`) are visible to Verus.

2. `src/libs/arch/src/x86/mem/paging/frame/number.spec.rs`
   - **Removed** `assume_specification[ FrameNumber::MAX ]`.
   - Changed `spec_max()` from `uninterp` to an interpreted
     `pub open spec fn spec_max() -> nat { (mem::MAX_ADDRESS / mem::FRAME_SIZE - 1) as nat }`.
     The `MAX as int == spec_max()` binding is now discharged by verification.

3. `src/libs/arch/src/x86/mem/paging/frame/number.rs`
   - Moved `pub const MAX` out of the plain (external) impl into the
     `#[verus_verify] impl FrameNumber` block, so its body is verified against
     `spec_max()` instead of being assumed.

4. `src/kernel/src/hal/mem/types/address/frame.rs`
   - Removed the now-duplicate placeholder `assume_specification[ ::arch::mem::PAGE_SIZE ]`.
   - Changed `spec_page_size()` from `uninterp spec fn` to
     `pub open spec fn spec_page_size() -> int { ::arch::mem::PAGE_SIZE as int }`,
     delegating to arch's verified constant (the value the proofs already relied on).

5. `src/kernel/src/mm/phys/frame.spec.rs`
   - Removed the now-duplicate placeholder `assume_specification[ ::arch::mem::FRAME_SIZE ]`.
     Callers relying on `FRAME_SIZE == spec_page_size()` still get it: both reduce
     to the verified constant `4096`.

### Verification

`make verify-arch` (arch crate) — CLEAN, no assumes:

```
verification results:: 8 verified, 0 errors
  ✅ No cheating detected.
  cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0
  status: CLEAN
```

`make verify` (full workspace) — kernel passes, no regression:

```
Crate     : arch
  ✅ No cheating detected.
  status: CLEAN
Crate     : kernel
verification results:: 66 verified, 0 errors
  verification: 66 verified, 0 errors (exit 0)
```

(The remaining `external_body`/`admit`/`cfg_gate` counters on kernel and the
other crates are the pre-existing baseline from unrelated modules; this change
*reduced* the workspace assume count by 3 — arch went from `assume=1` to
`assume=0`, and two kernel placeholder assumes were removed.)

`./z build -- all` (dual compilation) — succeeds:

```
[OK]    Build complete.
```

### Result: FIXED

The workspace-internal `assume_specification[ FrameNumber::MAX ]` is gone.
`FrameNumber::spec_max()` is now interpreted as `MAX_ADDRESS / FRAME_SIZE - 1`,
and the exec constant `MAX` is verified against it rather than assumed. Two
additional kernel placeholder assumes (`PAGE_SIZE`, `FRAME_SIZE`) were superseded
by arch's now-verified constants, with `spec_page_size()` delegating to the real
`arch::mem::PAGE_SIZE`. All of `make verify-arch`, `make verify`, and
`./z build -- all` pass with 0 errors.
