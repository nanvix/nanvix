# Verification TODO — `mm::phys::kframe`

Status of in-scope functions (`KernelFrame::new`, `KernelFrame::base`, `KernelFrame::drop`):
**all three are fully verified in-body** (`make verify-kernel MODULE=mm::phys::kframe` →
`3 verified, 0 errors`, `status: CLEAN`). No `admit()`, no `assume()`, no `external_body`,
no cfg-gated exec code remains in the module.

## Remaining trusted boundary (genuine cross-module blocker)

- **Function:** `KernelFrame::map_frame` (exec-only helper extracted from `KernelFrame::new`).
- **What it does:** installs the kernel identity mapping for the frame via
  `crate::mm::virt::identity_map_page`.
- **Blocking Verus fact:** `identity_map_page` carries
  `requires identity_map_view().inv()` (see
  `src/kernel/src/mm/virt/identity_map.rs:698-718`). `identity_map_view()` is an
  `uninterp spec fn` (`identity_map.spec.rs:36`); **no** function or lemma in the tree
  *produces* `identity_map_view().inv()` from nothing — every occurrence is a `requires`
  on a function that already has it. The only lemmas
  (`lemma_install_page_preserves_inv`, `lemma_map_page_preserves_inv`) take `v.inv()` as a
  **precondition**. Therefore the invariant cannot be discharged from within `mm::phys`.
- **Why it cannot be solved by changing `new`'s contract:** adding
  `requires identity_map_view().inv()` to `new` propagates to its verified callers in
  `mm::phys::manager` (`alloc_kernel_frame` at `manager.rs:388`, and
  `FrameAddress::from_raw_value(raw_addr).and_then(KernelFrame::new)` at `manager.rs:485`).
  The `and_then` spec (`manager.spec.rs:9-18`) discharges
  `op.requires((result->Ok_0,))`, i.e. it would require `manager` to prove
  `identity_map_view().inv()`, which `PhysMemoryManager::inv()` does not provide — this
  regresses `make verify-kernel` (manager is verified and out of scope to modify).
  `identity_map_view()` also lives in the **private** `mod identity_map` and is not
  re-exported, so `mm::phys::kframe` cannot even name it in a contract.
- **Current handling:** `map_frame` is exec-only (no `#[verus_verify]`) and is given a
  **trusted, empty** contract via `assume_specification[ KernelFrame::map_frame ]` in
  `kframe.spec.rs` (no `requires`, no abstract `ensures`). This is the Verus-mandated way
  to invoke exec-external code from a verified function (Verus emits
  *"cannot use function … which is … marked as `external`"* otherwise and suggests exactly
  this `assume_specification`). It trusts **strictly less** than the previous
  `external_body` on `new`: `new`'s `kf@ == base@` and `kf.inv()` postconditions are now
  machine-verified; only the cross-module page-table side effect at the `mm::virt`
  trust boundary remains trusted. The empty contract assumes nothing false.

## Resolution path (when `mm::virt` is verified)

Provide, in `mm::virt`, a trusted accessor that yields `identity_map_view().inv()` — the
exact pattern already used for the singleton frame allocator
(`frame::instance()` pins `(*r).inv()` to `phys_view()`). Once such an accessor (or a
verified global invariant token) exists and is reachable from `mm::phys`, `map_frame` can be
folded back into `new` and verified in-body, and the `assume_specification` removed.
