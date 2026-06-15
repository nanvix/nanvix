# Verus-Unsupported Constructs — `mm::virt::manager`

Genuine Verus front-end limitations encountered while specifying this module.
The affected functions keep their `#[verus_spec(...)]` contract as the trusted
boundary and are marked
`#[cfg_attr(verus_keep_ghost, verus_verify(external_body))]` so the module
compiles and verifies with 0 errors. Each contract is fully proven once the
construct gains vstd support and/or the dependency module is verified.

## `link_user_pages` — closure capturing a `&mut` reference

**Construct**: `parent.for_each_user_mapping(|vaddr, pte| { ... count += 1; ... })`
where the callback closure captures `count`, `buf`, and `child` by mutable
reference.

**Error**:

```
error: Verus does not currently support closures capturing a mutable reference
for variables of any mode
   --> src/kernel/src/mm/virt/manager.rs:374:21
    |
374 |                     count += 1;
    |                     ^^^^^^^^^^
```

**Minimal trigger**:

```rust
let mut count: usize = 0;
some_api(|_x| { count += 1; Ok(()) })?;
```

**Status**: `external_body`; contract is the trusted boundary. The callback-based
iteration API (`Vmem::for_each_user_mapping`) cannot be expressed without a
`&mut`-capturing closure. Do not rewrite exec code.

## `alloc_upages` — `Vec::drain(..)` / `Vec::capacity()`

`Vec::drain` returns a `Drain` iterator and `Vec::capacity` has no vstd spec
(see property_analysis.md "Assumed External Specs"). `external_body` until vstd
models these.

## `alloc_kpages` — `<[T]>::iter_mut().try_for_each(..)`

Slice `iter_mut` plus the `Iterator::try_for_each` combinator have no vstd spec.
`external_body` until vstd models these.

## `new_vmem`, `alloc_kpage`, `try_resolve_cow_fault`, `load_elf` — unverified dependencies

These delegate to functions in not-yet-verified modules (`mm::phys`,
`mm::virt::kpage`, `mm::elf`, `arch::cpu::excp::ErrorCode`, `sys::mm::align_down`,
`hal::PageAligned::from_raw_value`) that currently carry no Verus contracts. They
are `external_body` (contract = trusted boundary) and become provable once those
dependency modules are verified.
