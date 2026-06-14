## Response to Turn 1

### What I Changed

Removed the `admit()` from `PageDirectoryEntry::frame_address` and discharged its
two `ensures` clauses with a real proof. No `ensures` was weakened; no new
`assume`/`admit`/`external_body` introduced.

**1. `src/libs/arch/src/x86/mem/paging/pde.rs` (`frame_address`, ~lines 414–418)**

Before:
```rust
pub fn frame_address(&self) -> usize {
    proof! { admit(); }
    self.frame.into_raw_value() << crate::mem::FRAME_SHIFT
}
```
After:
```rust
pub fn frame_address(&self) -> usize {
    let raw: usize = self.frame.into_raw_value();
    proof! { lemma_frame_address(raw); }
    raw << crate::mem::FRAME_SHIFT
}
```
The raw frame index is bound once so the proof relates it to the `<<` below.
`FrameNumber::into_raw_value` already supplies `raw as int == self.frame@` and
`0 <= self.frame@ <= FrameNumber::spec_max()` (number.rs:79–83); the view
(`self@.frame == self.frame@`) bridges to the `ensures`.

**2. `src/libs/arch/src/x86/mem/paging/pde.proof.rs` (was empty `verus! { }`)**

Added the reusable arithmetic lemma the reviewer suggested:
```rust
pub proof fn lemma_frame_address(raw: usize)
    requires
        0 <= raw as int <= FrameNumber::spec_max(),
    ensures
        (raw << crate::mem::FRAME_SHIFT) as int == raw as int * (crate::mem::FRAME_SIZE as int),
        (raw << crate::mem::FRAME_SHIFT) as int % (crate::mem::FRAME_SIZE as int) == 0,
```
Proof outline (no admit):
- `vstd::arithmetic::power2::lemma2_to64()` ⇒ `pow2(FRAME_SHIFT) == FRAME_SIZE`
  (`pow2(12) == 4096`).
- Overflow bound `raw * FRAME_SIZE <= usize::MAX`: from
  `raw <= MAX_ADDRESS / FRAME_SIZE - 1` (the `FrameNumber` type-invariant bound,
  `MAX_ADDRESS == usize::MAX`) via `lemma_mod_bound` +
  `lemma_fundamental_div_mod` + `lemma_mul_inequality`, closed by a small
  `by (nonlinear_arith)` step.
- `vstd::bits::lemma_usize_shl_is_mul(raw, FRAME_SHIFT)` ⇒
  `(raw << FRAME_SHIFT) == raw * pow2(FRAME_SHIFT) == raw * FRAME_SIZE`
  (proves ensures #1).
- `vstd::arithmetic::div_mod::lemma_mod_multiples_basic(raw, FRAME_SIZE)` ⇒
  `(raw * FRAME_SIZE) % FRAME_SIZE == 0` (proves ensures #2).

The `ensures` clauses were **not** changed — they remain
`result as int == self@.frame * FRAME_SIZE` and `result % FRAME_SIZE == 0`,
matching caller invariants 2 and 4.

### Verification

`make verify-arch` → exit 0:
```
verification results:: 41 verified, 0 errors
  41 verified
  0 errors
  Exit code : 0
  cheating: assume=0 external_body=2 admit=1 trusted=0 no_decreases=0 cfg_gate=2
```
- `admit` dropped from 2 → 1; the remaining admit is `table.proof.rs:8`
  (`lemma_entry_roundtrip`, the unrelated `table` module — out of scope per the
  review note). The two `external_body` are also `table` (read/write).
- `grep -n "admit" src/libs/arch/src/x86/mem/paging/pde.rs` → **no matches**.
- `grep -iE "pde" cheating-detail.txt` → **no pde cheats**.

`./z build -- all-kernel` → exit 0, `[OK] Build complete.` (dual compilation,
non-Verus path, still builds `kernel.elf`).

`make verify` (whole workspace, regression check) → exit 0 on both crates:
```
verification results:: 41 verified, 0 errors   (arch)
verification results:: 76 verified, 0 errors   (kernel)
```
No kernel regression — `frame_address`'s `ensures` is unchanged, so downstream
consumers see the identical contract.

### Result: FIXED

The `admit()` in `PageDirectoryEntry::frame_address` is removed and its
postcondition is now proven from vstd lemmas and the `FrameNumber` type
invariant. `make verify-arch` reports `admit=0` for `pde.rs` (the only remaining
arch admit belongs to the out-of-scope `table` module).
