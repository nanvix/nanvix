# Bugs — Nanvix phys manager

## Observations (record-only; validated in the proving phase)

### OBS-1: `alloc_many_kernel_frames` lacks the `count == 0` fast-path guard

`alloc_many_user_frames` early-returns `Ok(())` when `count == 0`, but
`alloc_many_kernel_frames` has no such guard and unconditionally calls
`frame::alloc_contiguous(count)`. `Inner::alloc_contiguous` (and therefore the
`frame::alloc_contiguous` wrapper) `requires count > 0`, so a `count == 0` call
would violate that precondition.

- **Spec decision**: added `requires count > 0` to
  `PhysMemoryManager::alloc_many_kernel_frames`, matching the contiguous
  allocator it delegates to. The sole caller (`mm::virt::manager::alloc_kpages`,
  not yet verified) must establish `count > 0`; this becomes an obligation when
  that module is verified.
- **Not auto-fixed**: adding a `count == 0` early return would change exec
  behavior of an unverified caller path. Recorded for the proving phase to
  decide whether to add the guard (code fix) or keep the precondition.
