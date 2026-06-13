## Response to Turn 1

### What I Changed

Root cause (item 13 FAIL): `MemoryRegion::start` discharged
`self.start.clone()@ == self.start@` with `proof! { admit(); }` because the bare
`Clone` supertrait of `Address` carries no Verus contract. Fix per the review
request: add a view-preserving clone contract to the `Address` trait, implement
it (provably) in every concrete impl, and use it in `start()` instead of `admit()`.

1. **`src/libs/sys/src/sys/mm/address/mod.rs`** (trait `Address`, after
   `into_raw_value`, ~line 56) — added a spec'd method:
   ```rust
   #[verus_spec(result =>
       ensures
           result@ == self@,
   )]
   fn clone_address(&self) -> Self;
   ```

2. **Implemented `clone_address` in all four `Address` impls** (provably, without
   relying on the unspecified `Clone::clone`):
   - `src/libs/sys/src/sys/mm/address/virt.rs` (`impl Address for VirtualAddress`):
     `fn clone_address(&self) -> Self { VirtualAddress(self.0) }` — `self.0: usize`
     is `Copy`; view is `self.0 as int`, so `result@ == self@`.
   - `src/kernel/src/hal/mem/types/address/phys.rs` (`impl Address for PhysicalAddress`):
     `fn clone_address(&self) -> Self { PhysicalAddress(self.0) }` — `self.0:
     VirtualAddress` is `Copy`; view is `self.0@`.
   - `src/kernel/src/hal/mem/types/address/aligned/page.rs`
     (`impl<T: Address> Address for PageAligned<T>`):
     `fn clone_address(&self) -> Self { PageAligned(self.0.clone_address()) }` —
     recurses through the inner `T`'s contract (`T` need not be `Copy`); view is
     `self.0@`, so `result@ == self.0.clone_address()@ == self.0@ == self@`.
   - `src/kernel/src/hal/mem/types/address/aligned/pgtab.rs`
     (`impl<T: Address> Address for PageTableAligned<T>`):
     `fn clone_address(&self) -> Self { PageTableAligned(self.0.clone_address()) }` —
     same recursive reasoning.

3. **`src/kernel/src/hal/mem/types/region.rs`** (`MemoryRegion::start`, ~line 209) —
   removed the `proof! { admit(); }` and the deferral comment; body is now:
   ```rust
   #[verus_spec(result =>
       ensures
           result@ == self@.start,
   )]
   pub fn start(&self) -> T {
       self.start.clone_address()
   }
   ```
   `TruncatedMemoryRegion::start` delegates to `self.0.start()` and needed no change.

I deliberately did **not** use the literal `self.clone()` body suggested in the
request: `Clone::clone` is `external_derive` (no Verus spec), so `self.clone()@ ==
self@` is not provable and would have re-introduced an `admit()`. Reconstructing
from the (Copy) field / recursing through the inner `Address` contract is the
provable form of the same view-preserving clone. No `assume_specification` on
`Clone` was used (respects item 7).

### Verification

`make verify-kernel` (run without `MODULE` so the changed `Address` trait and all
address impls are re-checked alongside `region`):

```
MAKE EXIT=0
verification results:: 4 verified, 0 errors
verification results:: 0 verified, 0 errors
verification results:: 70 verified, 0 errors
verification results:: 35 verified, 0 errors
verification results:: 59 verified, 0 errors
  verification: 59 verified, 0 errors (exit 0)
  cheating: assume=0 external_body=14 admit=26 trusted=0 no_decreases=0 cfg_gate=15
  status: CHEATING_DETECTED
```

Item-13 evidence (the previously-flagged admit is gone):
- `grep -n "admit" src/kernel/src/hal/mem/types/region.rs` → **NO MATCHES**.
- `verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt` no longer lists
  `hal/mem/types/region.rs:... start: admit`; only the two pre-existing
  `mm/phys/mod.rs` `external_body` entries remain (out of scope).
- Global `admit` count dropped 27 → 26 (exactly the one removed).

Cross-module / cross-crate regression check — full `make verify` (all crates):
```
MAKE EXIT=0
  Exit code : 0   (×5 crates)
  verification: cached (no recompilation), — (exit 0)
```
All previously-verified modules (including `hal::mem::types::address::*` after the
trait change) still pass with 0 errors.

Dual compilation — `./z build`:
```
BUILD EXIT=0
[OK]    Build complete.
```

### Result: FIXED

Item 13 cleared: no `admit()` (or any cheat) remains on this module's own
functions. The `MemoryRegion::start` postcondition `result@ == self@.start` is now
discharged through the new `Address::clone_address` contract, which is itself
proven in every concrete address impl. `make verify-kernel`, full `make verify`,
and `./z build` all exit 0 with no regressions.
