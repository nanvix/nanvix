# Verification TODOs: phys-manager (`src/kernel/src/mm/phys/manager.rs`)

## Proof gaps (verifiable but not yet proven)

None. There are **no `admit()` / `assume()`** anywhere in `manager.rs`,
`manager.spec.rs`, or `manager.proof.rs`. The proof lemmas
(`lemma_watermark_monotone`, `lemma_contiguous_run_distinct`) are discharged with
real proofs.

## Trust-boundary `external_body` (not proof gaps — cannot be eliminated under
## the task constraints; whitelisted in `verus-ai-logs/tcb-allowed.md`)

All six in-scope target methods of `PhysMemoryManager` remain `external_body`.
These are NOT proof gaps: they cannot be body-verified without violating the hard
rule "do not touch unlisted functions", because the facts their `ensures`
clauses assert can only be produced by *unlisted* callees that carry no Verus
`ensures`, and/or by constructs Verus cannot compile. Empirically confirmed
(see fix_report.md): removing `external_body` from `check_user_watermark`
produces `error: Unsupported constant type` from the `error!`/`write!` macro
expansion ("verus did not run", 0 verified) — and even after gating that macro
the postcondition is undischarge-able for the structural reasons below.

| Function | Blocking construct / missing upstream spec |
|----------|--------------------------------------------|
| `init` | Writes `static mut PHYS_MEMORY_MANAGER` (`MaybeUninit::write`) + flips `AtomicBool`; raw global state with no ghost model. The lifecycle flag has no abstract model in the do-not-modify `PhysMemView`. |
| `alloc_user_frame` | Calls unlisted `Upool::alloc` (no `ensures`); cannot prove `allocated_frames.contains(frame@)` / `frame@ % spec_page_size() == 0`. Also calls `check_user_watermark`. |
| `check_user_watermark` | `error!` macro → "Unsupported constant type"; `config::kernel::KERNEL_WATERMARK` not linked to `uninterp spec_kernel_watermark()`; unlisted `frame::free_count()` (no spec) not linked to `phys_view().frames.free_frames.len()`. |
| `alloc_many_user_frames` | Loop over unlisted `Upool::alloc` (no `ensures`) into `&mut Vec`; `error!` macro; `clear()`-on-error rollback. |
| `alloc_kernel_frame` | Unlisted `frame::alloc` / `frame::free` / `KernelFrame::new` (no `ensures`); `warn!` macro; `inspect_err` closure. |
| `alloc_many_kernel_frames` | Unlisted `frame::alloc_contiguous` / `frame::free` / `KernelFrame::new` (no `ensures`); `warn!` macro; two-phase rollback; contiguity guarantee depends on un-specced base address. |

### What would be required to eliminate them (out of scope here)
Add `#[verus_spec] ensures` contracts to the unlisted upstream primitives
(`Upool::alloc`, `frame::alloc`, `frame::alloc_contiguous`, `frame::free`,
`frame::free_count`, `KernelFrame::new`) tying their results to `phys_view()`,
and link `config::kernel::KERNEL_WATERMARK` to `spec_kernel_watermark()`. All of
these touch functions outside this module's scope and are therefore disallowed by
the current task's hard rules. Until then `external_body` is the correct,
sanctioned trust boundary (documented in `tcb-allowed.md`).
