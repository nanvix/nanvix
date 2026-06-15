# Verification TODOs — phys-manager

## Approved trust-boundary assumes (genuinely-stuck proof gaps)

The four lemmas in `manager.proof.rs` discharge irreducible external-bottom
trust boundaries via single-line, gate-approved `limitation_assume`
(`// VERUS-AI LIMITATION: id=L60..L63`, registered in
`verus-ai-logs/approved-trust-boundaries.json`). They are recorded here as an
honest hand-off of proofs that cannot be completed until the frame
free-function layer is verified. Each has a reproducer under
`cheating-elimination/repros/`.

| id  | lemma                          | blocking Verus fact                                                        | root cause |
|-----|--------------------------------|----------------------------------------------------------------------------|------------|
| L60 | `lemma_manager_attached`       | `postcondition not satisfied: m@ == phys_view().frames`                     | `phys_view()` is a 0-arg `uninterp spec fn` (logic constant); `Upool::view` is `uninterp`. No in-module fact links them — a §8 ghost-token attachment over the `frame::INSTANCE` / `PhysMemoryManager` / `Upool` singletons. |
| L61 | `lemma_kernel_alloc_one`       | `postcondition not satisfied: post == pre.alloc_one(addr)` (and `contains`) | `frame::alloc()` is a free function (no `self`); the kernel path never mutates `self.upool`, so Verus sees `self@` unchanged. `post`/`addr` are unconstrained params carrying the runtime allocator's global-partition transition. |
| L62 | `lemma_kernel_alloc_contiguous`| `postcondition not satisfied: frames.len()==count`, `post == pre.book_all(...)` | Region analogue of L61: `frame::alloc_contiguous()` is a free function (no `self`); contiguity/length and the region transition come from the runtime allocator, unprovable in-module. |
| L63 | `lemma_user_bulk_err_restored` | `postcondition not satisfied: m@ == pre`                                    | Restoration relies on `Vec::clear()` → `Drop` → `frame::free()` side effects, which Verus exec semantics do not model. |

### Why these cannot be eliminated now

The escalation ladder was exhausted:
1. **vstd search** — no spec relates two distinct `uninterp` functions
   (`phys_view()` constant vs. `Upool::view`); no model of free-function or
   `Drop` global side effects exists.
2. **Isolated reproducer** — see `repros/L60.rs..L63.rs`: the minimal shape
   (uninterpreted constant vs. uninterpreted view; unconstrained `post`
   parameter; unmodeled `Drop`) reproduces `postcondition not satisfied`.
3. **Equivalent rewrite** — the kernel allocation path genuinely uses free
   functions with no `self`, and the bulk-error path genuinely relies on `Drop`;
   rewriting to thread `self` would change runtime semantics (forbidden by
   ast-consistency) and is impossible because the underlying `frame::*` layer is
   itself `external_body` (tcb-allowed, not yet verified).

**Unblock condition:** verify the `mm::phys::frame` free-function layer
(`frame::alloc` / `alloc_contiguous` / `free` / `book`) and the `Upool` /
`phys_view()` model with a ghost token tying the singleton instances to the
abstract partition. Once that token exists, L60–L63 become provable and the
`limitation_assume`s can be replaced with real proofs.

## Proof gaps (admit) — none

No `admit()` in any manager file.

## Out-of-scope (not this task)

- `mm::virt` carries 3 `admit()` (global scan) — separate module.
- Sibling files in `src/kernel/src/mm/phys/` (`frame.rs`, `mod.rs`, `upool.rs`,
  `kpool.rs`, `mod.spec.rs`) retain their own `tcb-allowed` `external_body` /
  `external_type_specification` — separate per-module verification targets.
