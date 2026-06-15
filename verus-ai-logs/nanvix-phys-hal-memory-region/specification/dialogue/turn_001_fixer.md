## Response to Turn 1

### What I Changed

The reviewer's literal suggestion — an `assume_specification` on the generic
`<T as Clone>::clone` — does **not compile** under Verus (it is structurally
rejected; see the failed attempts below). Their cited precedent
`<PageAligned<T> as Address>::into_raw_value` works only because its `Self` is a
*concrete type constructor* (`PageAligned<T>`); for `clone` the receiver is a
*bare type parameter* `T`, which Verus forbids.

I therefore removed the `admit()` by eliminating the unprovable obligation at its
root rather than trusting it. The accessor returns `self.start.clone()`; the
obligation `spec_addr(&clone) == spec_addr(&self.start)` only existed because the
value was *cloned* (and `Clone::clone` carries no value guarantee in vstd). Every
`Address` implementor (`VirtualAddress`, `PhysicalAddress`, `PageAligned<T>`,
`PageTableAligned<T>`) already derives `Copy`, so I:

1. **`src/libs/sys/src/sys/mm/address/mod.rs:33`** — added `Copy` to the
   `Address` supertrait bound:
   ```rust
   Self: core::fmt::Debug + Clone + Copy + PartialEq + Eq + PartialOrd + Ord,
   ```
   This is satisfied by all four existing impls (the two generic ones derive
   `Copy` conditioned on `T: Copy`, which the new supertrait supplies). No `dyn
   Address` exists in the tree, so object-safety is unaffected.

2. **`src/kernel/src/hal/mem/types/region.rs:220-221`** — changed
   `MemoryRegion::start`'s body from `self.start.clone()` to a `Copy` field read
   `self.start`. With `T: Copy` a field read is **identity** in Verus
   (`result == self.start`), so `spec_addr(&result) == spec_addr(&self.start) ==
   self@.start` discharges by congruence — **no clone law, no `admit()`**. The
   `ensures` (`spec_addr(&result) == self@.start`) is unchanged and equally
   strong. `TruncatedMemoryRegion::start` delegates to this and inherits the
   guarantee.

3. **`src/kernel/src/hal/mem/types/region.spec.rs`** — removed the broken
   clone-spec experiments; the file holds only the View/inv/helpers.

### Verification

`make verify-kernel MODULE=hal::mem::types::region`:
```
verification: 5 verified, 0 errors (exit 0)
cheating: assume=0 external_body=25 admit=0 trusted=0 no_decreases=0 cfg_gate=7
coverage: 4/28 exec functions have contracts
status: CLEAN
```
`grep -nE "region\.(rs|spec\.rs|proof\.rs)" verus-logs/cheating-detail.txt` → **no
output** (own-module `admit` count = 0).

`make verify` (all modules) → **exit 0**, `admit=0`. The 25 `external_body`
entries are pre-existing TCB-allowed boundaries in *other* modules (unchanged
baseline); the `sys` crate re-verified clean after the supertrait change.

`./z build -- all` → **`[OK] Build complete.`** (dual compilation intact; the
`Copy` read is valid in the normal build because `Address: Copy` now holds).

### Result: FIXED

The `admit()` is gone, the `ensures` is byte-for-byte as strong, and both
`make verify-kernel` and `make verify` exit 0 with `admit=0` for this module.

---

#### Appendix — every approach tried for the reviewer's literal mechanism (all rejected by Verus)

1. `pub assume_specification<T: Address>[ <T as Clone>::clone ](addr: &T) -> (result: T) ensures spec_addr(&result) == spec_addr(addr);`
   ```
   error: assume_specification cannot be used to specify generic specifications of trait methods; consider using external_trait_specification instead
   ```

2. `external_trait_specification` proxy `ExAddressClone: Sized { type ExternalTraitSpecificationFor: Address; fn clone(&self) -> (result: Self) ensures spec_addr(&result) == spec_addr(self); }`
   ```
   error[E0277]: the trait bound `Self: sys::mm::Address` is not satisfied
   note: required by a bound in `...::spec_addr`
   ```
   (No proxy mechanism grants `Self: Address`, which `spec_addr<T: Address>` requires.)

3. Same proxy but `ExAddressClone: Address`:
   ```
   error: external_trait_specification trait bound mismatch
   help: external_trait_specification requires trait bounds to match exactly ...
   ```

4. `external_trait_extension(CloneAddrSpec via ...)` re-declaring `clone` on `Clone`:
   ```
   error: duplicate method core::clone::Clone::clone `core::clone::Clone`
   ```
   (vstd already owns `Clone`'s spec via `ExClone`, with no value guarantee, and
   it cannot be overridden or strengthened for the `Address` subfamily.)

These confirm a value-preservation guarantee cannot be attached to
`Clone::clone` for a generic `T: Address`. Avoiding the clone entirely (the
`Copy` field read) is the sound resolution and is what the final fix uses.
