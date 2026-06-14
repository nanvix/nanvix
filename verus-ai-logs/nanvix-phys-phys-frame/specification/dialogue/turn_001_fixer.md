## Response to Turn 1

### What I Changed

Replaced the tautological `Err(_) => true` arms on the four shims the reviewer
flagged with the meaningful error-path facts each one's `Inner::*` counterpart
already establishes. All edits are in
`src/kernel/src/mm/phys/frame.rs` (shim `#[verus_spec]` Err arms only — no Ok arm,
no `requires`, and no `Inner::*` / view definition was touched).

1. **`alloc` shim (was line 748).** Out-of-memory witness from `Inner::alloc`
   (`old(self)@.free_frames.is_empty()`):
   ```rust
   Err(_) => phys_view().frames.free_frames.is_empty(),
   ```

2. **`book` shim (was line 890).** "Allocator unchanged / frame was not free" from
   `Inner::book` Err (`!old(self)@.free_frames.contains(phys_addr@)`):
   ```rust
   Err(_) => !phys_view().frames.free_frames.contains(phys_addr@),
   ```

3. **`alloc_range` shim (was line 914).** "Region was not fully free" from
   `Inner::alloc_range` Err (`!frames.subset_of(old(self)@.free_frames)`), reusing
   the shim's existing `region_frames` vocabulary:
   ```rust
   Err(_) => !PhysMemView::region_frames(region@.start, region@.size)
       .subset_of(phys_view().frames.free_frames),
   ```

4. **`share` shim (was line 941).** "Frame not allocated OR refcount would
   overflow 255" from `Inner::share` Err:
   ```rust
   Err(_) => !phys_view().frames.allocated_frames.contains(frame@)
       || (phys_view().frames.refcounts.contains_key(frame@)
           && phys_view().frames.refcounts[frame@] >= 255),
   ```

#### The two deliberately-retained `Err(_) => true` (with quoted `Inner` Err arms)

- **`alloc_contiguous` shim (line 792).** `Inner::alloc_contiguous` Err arm states
  *only* state preservation, nothing else:
  ```rust
  Err(_) => {
      final(self)@ == old(self)@
  }
  ```
  The sole fact is `final(self)@ == old(self)@`. Over the fixed uninterpreted
  `phys_view()` there is no `old(phys_view())` to diff against, so this fact is
  inexpressible at the shim level — every observable Err postcondition reduces to
  `phys_view().inv()`, which the shim already states above the `match`. Hence
  `Err(_) => true` is the strongest expressible arm. (Contrast `alloc`, whose
  `Inner` Err arm carries the *additional* `old(self)@.free_frames.is_empty()`
  witness, which IS expressible because it is a property of the unchanged
  pre-state = current `phys_view()`.)

- **`init` (line 686).** `init`'s own contract Err arm has no abstract post-state
  beyond the already-stated `phys_view().inv()`:
  ```rust
  ensures
      phys_view().inv(),
      match result {
          Ok(()) => phys_view().initialized,
          Err(_) => true,
      },
  ```
  On Err, `init` did not establish the singleton, so `initialized` need not hold
  and there is no allocator state to describe; `phys_view().inv()` (stated
  unconditionally) is the only guarantee. `init` is additionally a Skip/exclude
  TCB trust boundary per `tcb-allowed.md`.

`grep -n "Err(_) => true" src/kernel/src/mm/phys/frame.rs` → `686`, `792` only,
exactly as the reviewer's acceptance criterion permits.

### Verification

`make verify-kernel MODULE=mm::phys VERUS_EXECUTABLE_DIR=~/toolchain/verus`:

```
note: verifying module mm::phys
note: verifying module mm::phys::frame
note: verifying module mm::phys::kframe
note: verifying module mm::phys::manager
note: verifying module mm::phys::upool

verification results:: 31 verified, 0 errors (partial verification with `--verify-*`)
    Finished `dev` profile [optimized + debuginfo] target(s) in 2.20s

=== Results ===
  31 verified
  0 errors
  Exit code : 0
```

The `admit()` in each mutating shim discharges the proof obligation at this phase;
no other arm was weakened. Cheating counts unchanged from the green baseline
(`external_body=20`/`admit=7` within `mm::phys`; the strengthened Err arms add no
new trust boundary).

`make all-kernel` (normal dual compilation):

```
   Compiling kernel v0.16.17 (/home/ruize/nanvix-phy/src/kernel)
    Finished `dev` profile [optimized + debuginfo] target(s) in 10.78s
cp -f --preserve .../kernel.elf /home/ruize/nanvix-phy/bin/kernel.elf
```

### Result: FIXED
