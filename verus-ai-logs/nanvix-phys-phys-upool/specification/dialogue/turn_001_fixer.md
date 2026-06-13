## Response to Turn 1

### What I Changed

Removed the tautological `Err(_) => true` from `UserFrame::share` and its trusted
delegate `frame::share`, replacing it with the soundly-expressible snapshot Err
fact derived from `Inner::share` (frame.rs:358–366: `Err` iff out-of-bounds /
`refcount==0` ⟹ not in `allocated_frames`, or overflow ⟹ `refcount==255`).

1. **`src/kernel/src/mm/phys/frame.rs`** — trusted `external_body` wrapper
   `frame::share` (ensures block ~line 811), Err arm:
   ```rust
   ensures
       match result {
           Ok(())  => crate::mm::phys::phys_view().frames.allocated_frames.contains(frame@),
           Err(_)  => !crate::mm::phys::phys_view().frames.allocated_frames.contains(frame@)
               || crate::mm::phys::phys_view().frames.refcounts[frame@] >= 255,
       },
   ```

2. **`src/kernel/src/mm/phys/upool.rs`** — `UserFrame::share` (ensures block
   ~line 157), Err arm (mirrors the delegate; `self.addr@ == self@` transfers the
   `frame@`-fact through `frame::share(self.addr)?`):
   ```rust
   Err(_) => {
       ||| !crate::mm::phys::phys_view().frames.allocated_frames.contains(self@)
       ||| crate::mm::phys::phys_view().frames.refcounts[self@] >= 255
   },
   ```

No `old(phys_view())` / `F' == F.add_ref(...)` transition was added (per the
explicit instruction and the §2 upstream limitation).

### Verification

`make verify-kernel MODULE=mm::phys::upool` tail:
```
note: verifying module mm::phys::upool
6 verified
0 errors
  Exit code : 0
  ✅ All 8 exec functions have contracts.
=== Summary ===
  verification: 6 verified, 0 errors (exit 0)
  coverage: 8/8 exec functions have contracts
```
The new upool Err arm discharges directly from the strengthened `frame::share`
Err arm (no admit, no bridge lemma).

`make verify-kernel MODULE=mm::phys::frame` → Exit code : 0 (frame layer still
verifies; `share` is `external_body`, body untouched, and the strengthened
ensures is sound vs. `Inner::share`).

`./z build` (normal dual compilation, Verus constructs erased) → `[OK] Build
complete.`

`UserFrame::share` spec block (post-change, confirms no `Err(_) => true`):
```rust
ensures
    match result {
        Ok(uf) => {
            &&& uf@ == self@
            &&& uf.inv()
            &&& crate::mm::phys::phys_view().frames.allocated_frames.contains(self@)
        },
        Err(_) => {
            ||| !crate::mm::phys::phys_view().frames.allocated_frames.contains(self@)
            ||| crate::mm::phys::phys_view().frames.refcounts[self@] >= 255
        },
    },
```
`grep "Err(_) => true"` over `share` in both files → none (the remaining
matches at frame.rs:707/734 belong to unrelated functions outside this request's
scope).

### Result: FIXED
