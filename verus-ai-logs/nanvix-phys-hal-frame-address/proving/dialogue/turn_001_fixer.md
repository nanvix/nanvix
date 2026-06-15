## Response to Turn 1

### What I Changed

The reviewer's item #4 required **deleting the intra-crate `assume_specification`** for
`<PhysicalAddress as ::sys::mm::Address>::from_raw_value` from `frame.spec.rs` (an
unsanctioned, kernel-internal trust hole — not listed in `tcb-allowed.md`).

I applied **Option B** (the TCB-sanctioned option), not Option A. Rationale:

- **Option A** (give the real callee a verified `#[verus_spec]` and verify
  `from_raw_value` in-body) requires modifying
  `<PhysicalAddress as Address>::from_raw_value` **and** `from_virtual_address` at
  `phys.rs:193`/`phys.rs:49`. Both live in the **separate, already-completed
  `hal::mem::types::address::phys` module** and are **unlisted functions** for this task.
  My hard rules forbid touching unlisted functions. (It would also require a new
  `assume_specification` for the `sys`-crate inherent `VirtualAddress::from_raw_value`,
  i.e. relocating trust rather than eliminating it.)
- **Option B** stays entirely within my module's files, touches only my listed target
  function, and is explicitly approved in `tcb-allowed.md:137-138`.

Changes:

1. **`src/kernel/src/hal/mem/types/address/frame.rs`** (`FrameAddress::from_raw_value`,
   ~line 88–101): added `#[verus_verify(external_body)]` above the existing
   `#[verus_spec]`. The **strengthened ensures is preserved verbatim**
   (`Ok(fa) => fa.inv() && fa@ == raw_addr as int`) — the contract is unchanged, only its
   discharge moves from in-body proof to a TCB-listed trusted boundary. Updated the doc
   comment to record the TCB rationale.

2. **`src/kernel/src/hal/mem/types/address/frame.spec.rs`** (lines 11–29): removed the
   `pub assume_specification[ <PhysicalAddress as ::sys::mm::Address>::from_raw_value ]`
   block (and its surrounding `verus! { }` wrapper) entirely, replacing it with a comment
   explaining the removal. The `use crate::hal::mem::types::address::phys::{...}` import is
   retained (those spec helpers are still used by `from_frame_number`/`into_frame_number`
   ensures and `frame.proof.rs`).

Post-change grep:
```
$ grep -rn 'assume_specification\|assume(' src/kernel/src/hal/mem/types/address/frame*.rs
frame.rs:40:      // ... assume_specification[PAGE_SIZE] ...   (prose in comment)
frame.spec.rs:11: // The former assume_specification ...      (prose in comment)
```
No `assume_specification` declarations and no `assume(` calls remain — only comment prose.

### Verification

**Module verification (non-cached):**
```
$ touch src/kernel/src/hal/mem/types/address/frame.rs
$ make verify-kernel MODULE=hal::mem::types::address::frame
verification results:: 5 verified, 0 errors (partial verification with `--verify-*`)
=== Results ===
  5 verified
  0 errors
  Exit code : 0
=== Summary ===
  cheating: assume=0 external_body=20 admit=12 trusted=0 no_decreases=0 cfg_gate=19
```
(5 verified instead of 6: `from_raw_value` is now a trusted `external_body` boundary rather
than a verified-in-body function. `external_body` 19→20 is this one function; `admit`
unchanged at 12, all in other modules.)

**Full kernel verification (cross-module regression):**
```
$ make verify-kernel
=== Results ===  Exit code : 0
=== Summary ===  verification: exit 0; assume=0 external_body=20 admit=12
```
No regressions.

**Normal build:**
```
$ ./z build -- all-kernel
   Compiling kernel v0.16.17
    Finished `dev` profile [optimized + debuginfo] target(s) in 10.88s
[OK]    Build complete.
```

**Spec-drift (no weakening) — proving-start `1f6205c56`:**
```
frame.rs       → ✅ No contract drift detected (ensures removed: 0, requires added: 0)
frame.spec.rs  → ✅ No contract drift detected
frame.proof.rs → ✅ No contract drift detected
```
The `from_raw_value` postcondition `Ok(fa) => fa.inv() && fa@ == raw_addr as int` is
preserved verbatim; upstream callers keep the full guarantee.

### Result: FIXED

The unsanctioned intra-crate `assume_specification` is removed. The trust is now carried by
the TCB-sanctioned `#[verus_verify(external_body)]` on `FrameAddress::from_raw_value`
(`tcb-allowed.md:137-138`), with its strengthened contract intact and no spec weakening.
Module verifies (5 verified, 0 errors), full kernel verification passes (exit 0), and the
normal build compiles clean.
