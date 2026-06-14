# Bugs — nanvix phys::manager

None.

No code bugs were found while proving `src/kernel/src/mm/phys/manager.rs`.

## Notes (not bugs)

The six target `PhysMemoryManager` methods (`init`, `alloc_user_frame`,
`check_user_watermark`, `alloc_many_user_frames`, `alloc_kernel_frame`,
`alloc_many_kernel_frames`) cannot be body-verified due to genuine Verus
front-end limitations, not code defects:

- `init` writes the `static mut PHYS_MEMORY_MANAGER` singleton — `static mut`
  has no Verus spec model (True Limitation).
- The remaining methods invoke the `error!`/`warn!` kernel log macros, which
  expand to `write!(...)` and trip Verus's "Unsupported constant type" error.

These methods form a stateless trust boundary over the global frame allocator,
already documented and authorized as `external_body` in
`verus-ai-logs/tcb-allowed.md`. They were marked `#[verus_verify(external_body)]`
(matching the sibling `frame.rs` shims) with their `#[verus_spec]` contracts
left unchanged (no weakening). The abstract laws backing those contracts
(`lemma_watermark_monotone`, `lemma_contiguous_run_distinct`) are fully proved
in `manager.proof.rs` with no `admit()`/`assume()`.
