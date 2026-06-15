# Verus-unsupported constructs — `mm::virt::vmem`

## `while let Some(..) = list.pop_front() { .. }`

- **Error:** `The verifier does not yet support the following Rust feature: let expressions`
- **Locations (original):** `Vmem::new` (×2), `impl Drop for Vmem` (×3).
- **Trigger (minimal):**
  ```rust
  while let Some(x) = list.pop_front() { drop(x); }
  ```
- **Resolution:** Verus cannot parse/translate `while let`, which blocks the
  verifier from running on the whole crate. Because the verifier cannot process
  the function at all, an `admit()` placeholder is insufficient (it only skips a
  proof obligation, not a front-end parse failure). The loops were rewritten into
  the exactly-equivalent `loop { let n = list.pop_front(); if n.is_none() { break; }
  let x = n.unwrap(); .. }` form. This preserves runtime semantics bit-for-bit
  (same drain order, same effects), so it does not introduce verified/runtime
  divergence — it only removes a front-end syntax the verifier does not yet accept.

# Verus-unsupported constructs — `mm::virt::manager`

These constructs force the affected `VirtMemoryManager` methods to keep their
`#[verus_verify(external_body)]` annotation: the rich `#[verus_spec]` contract on
each is the trusted boundary (the body is not translated), exactly as for the rest
of the codebase. The contracts are still fully checked at every call site.

## Closures capturing `&mut` locals

- **Error:** `Verus does not currently support closures capturing a mutable reference`
- **Locations:** `VirtMemoryManager::link_user_pages` and its helper
  `rollback_linked_pages` pass closures that capture `&mut count`, `&mut buf`, and
  `&mut child` into `Vmem::for_each_user_mapping`.
- **Resolution:** This is a Verus front-end limitation (no `admit()` workaround,
  since it is a translation failure, not a proof obligation). The function keeps
  `external_body` with its full contract.

## `std` iterator adapters not modeled by `vstd`

- **Error:** `cannot use function ... which is ignored` for `Vec::drain`,
  `Vec::capacity`, `<[_]>::iter_mut`, `Iterator::try_for_each`.
- **Locations:** `VirtMemoryManager::alloc_upages` (`uframes.drain(..)`,
  `.capacity()`); `VirtMemoryManager::alloc_kpages`
  (`kframes.iter_mut().try_for_each(..)`).
- **Resolution:** `vstd` does not provide specifications for these `std`
  iterator/`Vec` APIs, so their bodies cannot be translated. The functions keep
  `external_body` with their full contracts.

## Delegation to not-yet-verified modules

- `new_vmem`, `try_resolve_cow_fault`, `alloc_kpage`, and `load_elf` call into the
  `phys`, `kpage`, `elf`, `arch`, `sys`, and `hal` modules, which do not yet carry
  Verus contracts. Per the verification plan these are `external_body` for now and
  will have their bodies verified once those modules are specified. Their contracts
  here remain the trusted callee boundary.
