# Bug Awareness — `sys::mm::address` (`Address` trait), specification phase

No fundamentally incorrect code found.

Checked:
- `from_raw_value` Ok arm (`a@ == raw_addr as int`) is satisfiable by every
  implementor (VirtualAddress, PhysicalAddress, PageAligned, PageTableAligned).
- PhysicalAddress's stricter (sparse) validity only narrows the **Err** arm,
  which the trait spec leaves as `e.code == BadAddress` — consistent, no
  contradiction.
- `into_raw_value` lossless projection and `is_aligned` alignment predicate
  match all implementor behaviors and caller/test expectations.
- Ordering/equality agree with `@` via the `int` view; no observed violation.

Status: clean.
