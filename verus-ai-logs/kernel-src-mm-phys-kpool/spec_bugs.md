# Spec Bugs — kpool

## SB-001: `alloc_range` Err postcondition too strong for non-empty addrs

**Location**: `kpool.rs`, `alloc_range` spec, Err arm

**Original postcondition** (commit 295f4fe0b):
```rust
Err(_) => {
    &&& count == 0 || forall|i: int| !old(self)@.range_free(i, count as int)
    &&& self@ == old(self)@
    &&& addrs@ == old(addrs)@
},
```

**Problem**: The original spec has `requires old(self).inv()` only (no precondition on `addrs`).
When `addrs` is non-empty, `count > 0`, and a free range exists, the function correctly
returns `Err` (at the `!addrs.is_empty()` guard), but the postcondition
`count == 0 || forall|i| !range_free(i, count)` is false — neither disjunct holds.
This makes the original spec **unprovable** for the non-empty addrs + free range case.

**Fix**: Added `old(addrs)@.len() > 0` as a disjunct to the Err postcondition:
```rust
Err(_) => {
    &&& old(addrs)@.len() > 0 || count == 0 || forall|i: int| !old(self)@.range_free(i, count as int)
    &&& self@ == old(self)@
    &&& addrs@ == old(addrs)@
},
```

This accurately describes all error conditions without weakening the spec for the
primary case (empty addrs). Callers still get the same guarantees when passing empty addrs.
