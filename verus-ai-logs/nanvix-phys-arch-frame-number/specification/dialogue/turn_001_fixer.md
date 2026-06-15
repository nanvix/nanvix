## Response to Turn 1

### What I Changed

The reviewer's blocking item (item 1) had two parts: (a) the crate did not compile
(duplicate `use crate::mem;`), and (b) the two in-scope exec functions carried no
`#[verus_spec]` contracts. Both are fixed, plus the downstream items 2,3,6,9,14,16,17
they blocked. Fixing them required Verus-enabling the `arch` crate, which in turn
exposed a full-`make verify` regression in the kernel (it held placeholder
`assume_specification`s for the now-natively-specified `arch` symbols); that was
migrated as well so the ULTIMATE GOAL (`make verify` 0 errors) holds.

**`src/libs/arch/src/x86/mem/paging/frame/number.spec.rs`**
- Removed the duplicate `use crate::mem;` (it is already imported by `number.rs`,
  into which this file is `include!`d) — fixes the E0252 compile error (item 1/16/17).
- View/inv unchanged in intent: `impl View for FrameNumber { type V = int; closed
  view = self.0 as int }`, and `inv()` as a `#[verifier::type_invariant]`
  (`0 <= self@ <= spec_max_frame_number()`). Spec constants `spec_frame_size()` and
  `spec_max_frame_number()` mirror the exec `FrameNumber::MAX` and the names trusted
  in `phys.spec.rs` / `tcb-allowed.md` (items 3,14).

**`src/libs/arch/src/x86/mem/paging/frame/number.rs`**
- `struct FrameNumber` (line 28): added `#[verus_verify(external_derive)]` (line 26)
  so the `#[verifier::type_invariant]` registers while keeping the derived
  `Debug/Clone/Copy`.
- `FrameNumber::MAX` (line 37): added `#[verus_verify]` (line 36) so the associated
  const is usable in exec/spec position.
- `into_raw_value` (line 60): added `#[verus_spec(result => ...)]` (line 55) ensuring
  `result as int == self@` and `0 <= result as int <= spec_max_frame_number()`, with
  `proof! { use_type_invariant(&self); }` (line 61) discharging the range from the
  type invariant — unconditional, matching the upstream tcb-allowed contract
  (items 2,5,14).
- `from_raw_value` (line 93): moved into a `#[verus_verify] impl` block (line 66,
  self-less associated fn per the sibling `frame.rs` pattern) and added
  `#[verus_spec(result => ...)]` (line 88) ensuring `Some(f) <==> value as int <=
  spec_max_frame_number()`, index preservation `f@ == value as int`, `f.inv()`, and
  the `None => value as int > spec_max_frame_number()` error path (items 2,6,14).

**`src/libs/arch/src/x86/mem/constants.rs`**
- Added `use vstd::prelude::*;` and `#[verus_verify]` on `PAGE_SIZE`, `MAX_ADDRESS`,
  `FRAME_SIZE` (referenced by the arch spec constants).

**`src/libs/arch/Cargo.toml`** — added `[package.metadata.verus] verify = true` and the
`vstd` workspace dependency. **`Makefile`** (line 301) — added `arch` to `VERUS_CRATES`.

**Kernel migration (consequence of arch now owning these specs natively; required for
`make verify`):** removed the 4 placeholder duplicates and grounded the kernel's
abstractions in arch's native specs (redefine, never weaken):
- `frame.spec.rs`: `spec_page_size()` `uninterp` → `open { ::arch::mem::PAGE_SIZE as int }`;
  removed `assume_specification[ ::arch::mem::PAGE_SIZE ]`.
- `phys.spec.rs`: removed the `ExFrameNumber` `external_type_specification` (arch now
  registers `FrameNumber` as a datatype); `spec_frame_raw_value(frame)` `uninterp` →
  `open { frame@ }` (arch's `View`); removed the duplicate
  `assume_specification`s for `::arch::mem::FRAME_SIZE`, `FrameNumber::into_raw_value`,
  and `FrameNumber::from_raw_value`. Kept the `FRAME_SHIFT` and `VirtualAddress`
  boundaries (not owned by arch). The kernel's `spec_max_frame_number()` is unchanged
  and provably equal to arch's by unfolding the `open` bodies (both reduce to
  `usize::MAX / 4096 - 1`).

### Verification

`make verify-arch VERUS_EXECUTABLE_DIR=/home/ruize/toolchain/verus-pinned-0531`:
```
  cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0
  coverage: 2/525 exec functions have contracts
  status: CLEAN
```
(6 verified, 0 errors — both in-scope functions now carry contracts; no cheating.)

Full `make verify VERUS_EXECUTABLE_DIR=/home/ruize/toolchain/verus-pinned-0531` → exit 0,
every crate 0 errors:
```
  arch::all      → CLEAN (0 errors)
  kernel::all    → 47 verified, 0 errors
  sys / bitmap / nanvix-slab / bump-allocator → 0 errors (cached)
make verify exit: 0
```
The `CHEATING_DETECTED` tags on kernel/bitmap/nanvix-slab/bump-allocator are
pre-existing TCB (`external_body`/`cfg_gate`) not introduced here — the kernel change
only *removed* `assume_specification`s and an `external_body` type spec (no
`external_body`/`admit`/`assume` lines were added; verified via `git diff`).

`./z build -- all` → exit 0 (`[OK] Build complete.`) — dual compilation intact.

### Result: FIXED
