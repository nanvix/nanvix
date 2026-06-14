# Verus-Unsupported Constructs — `arch::x86::mem::paging::table`

This file records language/library constructs in the `table` module that Verus
cannot translate or verify, together with the minimal reproduction and the
trust-boundary mitigation used.

## 1. Integer-to-pointer cast (`usize as *const T` / `usize as *mut T`)

### Where
- `Table::<E>::read` — `let ptr: *const PteWord = (self.base + offset) as *const PteWord;`
- `Table::<E>::write` — `let ptr: *mut PteWord = (self.base + offset) as *mut PteWord;`

Both functions materialize a raw pointer to a hardware page-table slot from the
integer base address stored in `self.base`, then perform a volatile load/store.

### Minimal reproduction
```rust
use vstd::prelude::*;
#[verus_spec]
pub fn load(base: usize) -> u32 {
    let ptr: *const u32 = base as *const u32;
    unsafe { ::core::ptr::read_volatile(ptr) }
}
```

### Exact error
```
error: Verus does not support this cast: `usize` to `*const u32`
 --> vt_cast.rs:5:27
  |
5 |     let ptr: *const u32 = base as *const u32;
  |                           ^^^^^^^^^^^^^^^^^^
```

### Why it is unsupported
Verus has no model for int-to-pointer provenance / materialization. A verified
load or store requires a `PointsTo`-style permission token witnessing ownership
of the target bytes; an `usize`-derived pointer to externally-owned, volatile
MMIO/page-table memory has no such token in scope.

### Mitigation (trust boundary)
`read` and `write` are marked `#[verus_verify(external_body)]` and recorded in
`verus-ai-logs/tcb-allowed.md`. This is the same int-to-pointer boundary already
used by `bump_allocator::alloc` (`usize as *mut`) and `frame::instance`. Their
`requires`/`ensures` describe the abstract contract; the bodies (the cast + the
volatile access) are trusted.

### Deferred work
`read` carries its **full** abstract contract: the global page-table-memory ghost
`spec_table_word` / `spec_table_read::<E>` models the slot contents
(parameter-free, like `phys_view()`), so `read` ensures
`result == spec_table_read::<E>(self@.addr, index@)`. `write` carries only the
sound `requires index@ < PAGE_TABLE_LENGTH` and **no** contents `ensures`: pinning
the *pure* `spec_table_word` cell to the caller-chosen `entry` in an assumed
(`external_body`) postcondition is unsound — two writes of distinct entries to the
same slot derive `spec_entry_raw(e1) == spec_entry_raw(e2)`, and with
`lemma_entry_roundtrip` that gives `e1 == e2`, i.e. `false` (Turn 2 review). The
genuine `old@ -> @` write transition (and its same-slot consistency) is therefore
deferred to a proving-phase ghost token over the page-table pages — the same
`phys_view()` / `identity_map_view()` placeholder rationale. Only the *body* (the
`usize`→pointer cast + the volatile access) is trusted via `external_body`. No exec
signature changed, so the out-of-scope `admit()` callers in `identity_map.rs`
(`ensure_pt`, `ensure_pte`, `identity_map_page`) do not cascade.
