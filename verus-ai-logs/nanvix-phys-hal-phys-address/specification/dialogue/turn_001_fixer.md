## Response to Turn 1

All three fix requests addressed with real proofs (no admits, no justifications).

### What I Changed

#### FIX-3 — Tautological error arm in `from_mmio_address` (`phys.rs:121-130`)
Replaced the `match { Ok(r) => ..., Err(_) => true }` ensures (the body `Ok(Self(addr))`
is infallible) with a success guarantee:
```rust
ensures
    result is Ok,
    result matches Ok(r) ==> r@ == addr@ && r.inv(),
```
The dead `Err` arm is gone; the spec now states the function always succeeds.

#### FIX-2 — `from_number` contract now usable by its caller (`phys.rs:147-204`)
Added the two facts the sole caller (`FrameAddress::from_frame_number` →
`PageAligned::from_address` + later `into_frame_number`) needs:
```rust
ensures
    result@ == spec_from_number(spec_frame_raw_value(frame)),
    result@ % spec_page_size() == 0,   // page-alignment for PageAligned::from_address
    result.inv(),                      // representable frame for into_frame_number
```
These are **proven**, not assumed. To make them derivable I tied the previously
uninterpreted bounds to their real arch definitions in `phys.spec.rs`:
- `spec_max_frame_number()` is now interpreted as `usize::MAX as int / spec_page_size() - 1`
  (the true `arch FrameNumber::MAX == MAX_ADDRESS / FRAME_SIZE - 1`).
- The `::arch::mem::FRAME_SIZE` assume_specification now also ensures `spec_page_size() > 0`
  (it is `4096`); the `::arch::mem::FRAME_SHIFT` one now ensures
  `pow2(result as nat) == spec_page_size()` (i.e. `2^FRAME_SHIFT == FRAME_SIZE`).
  Both are true facts about the arch constants, added at the existing arch trust boundary.

#### FIX-1 — Removed both `admit()`s and discharged the obligations

`from_number` (`phys.rs:155-204`): deleted `proof! { admit(); }`. Split the
`frame.into_raw_value() * mem::FRAME_SIZE` so the multiply has named ghost operands,
then proved no-overflow with `lemma_fundamental_div_mod` +
`lemma_mod_division_less_than_divisor` + a `nonlinear_arith` step
(`fr <= usize::MAX/p - 1 ∧ p>0 ⟹ fr*p <= usize::MAX`). Alignment and `inv()` follow
from `lemma_mod_multiples_basic` and `lemma_div_by_multiple`
(`(fr*p)%p == 0`, `(fr*p)/p == fr <= MAX`).

`into_frame_number` (`phys.rs:206-218`): deleted `proof! { admit(); }`. Bound
`shift = mem::FRAME_SHIFT` and applied `lemma_usize_shr_is_div`, so
`raw_addr >> shift == raw_addr / pow2(shift) == self@ / spec_page_size()`. With
`pow2(shift) == spec_page_size()` (FRAME_SHIFT spec) and `self.inv()`
(`self@ / spec_page_size() <= spec_max_frame_number()`), the index is representable,
so `FrameNumber::from_raw_value(..).unwrap()` is total and the ensures holds.

### Verification

`make verify-kernel MODULE=hal::mem::types::address::phys`:
```
verification results:: 4 verified, 0 errors
Global: assume=0 external_body=25 admit=0 trusted=0 cfg_gate=10
```
`admit=0` for this module (was `admit=2`). No `external_body` on any in-scope function.
The only remaining flag is the structural `#[cfg(verus_keep_ghost)]` gating of the
`include!`/`View` blocks — identical to every sibling module (`frame.rs`, etc.).

`make verify` (full regression): every crate exits 0
(bitmap, sys, nanvix-slab, bump-allocator, kernel) — no regressions.

`./z build -- all` (dual compilation, Verus erased): `[OK] Build complete.`

### Result: FIXED
