# Verification TODOs — mm::phys (phys-mod phase)

Honest hand-off of genuinely-stuck proofs. Each item is a real `admit()` that the cheating
gate flags; none can be soundly eliminated within the `mm::phys` scope with the current
toolchain and spec design. They are recorded here with the precise Verus error / code
pattern that blocks them. **This file does not make the phase pass** — these admits still
trip the cheating gate, so the phase is a BLOCKER.

Build state: `make verify-kernel MODULE=mm::phys` → 39 verified, 0 errors (the admits below
keep it green; removing them produces the errors quoted).

================================================================================
## Group A — 8 `Inner` methods (`src/kernel/src/mm/phys/frame.rs`)
`alloc` (136), `alloc_contiguous` (213), `free` (298), `share` (379), `refcount` (442),
`book` (497), `is_covered` (535), `alloc_range` (583).

### A1. Totality gap on `into_frame_number` (blocks all 8)
- **Verus error (reproduced by removing the admits):**
  `error: precondition not satisfied --> frame.rs:298:35 ::: hal/.../frame.rs:142:13`
  (and the same at the `phys_addr.into_frame_number()` / `frame.into_frame_number()` call
  in each method).
- **Cause:** `FrameAddress::into_frame_number` (`hal/mem/types/address/frame.rs:138`)
  `requires self.inv() && spec_frame_number(self@) <= spec_max_frame_number()`. But
  `FrameAddress::inv()` = `self@ % spec_page_size() == 0` (**alignment only**); the
  frame-number bound lives only in `PhysicalAddress::inv()`. The `Inner` methods receive
  only `frame.inv()` / `phys_addr.inv()` (alignment), so the bound is unavailable.
- **Why not fixable in-scope:**
  - Strengthening `FrameAddress::inv` to include the bound is a **HAL-layer** change
    (`src/kernel/src/hal/...`, a different phase) that cascades to every `FrameAddress`
    constructor across the kernel.
  - Adding the bound to each `Inner` precondition breaks the currently-verified
    `frame::share` / `frame::refcount` / `frame::is_covered` callers (they hold only
    alignment), forcing 3 more verified wrappers into TCB `external_body` — enlarging the
    trusted base for no phase-level gain.
- **Resolution path:** verify the HAL address layer so `FrameAddress::inv` carries the
  frame-number bound (making `into_frame_number` total at the type level), then the
  `Inner` methods inherit it.

### A2. Closed-view set-transition postconditions (blocks all 8, in addition to A1)
- **Verus error:** `error: postcondition not satisfied --> frame.rs:<view-clause>`.
- **Cause:** each method's `ensures` constrains the **closed** `View for Inner`
  (`allocated_frames`/`free_frames` = `Set::new(|addr| exists i: bitmap.set_bits ... &&
  addr == frame_addr_of(i))`, `refcounts: Map`). Discharging it needs set-extensionality +
  injectivity of `frame_addr_of(i) = i*spec_page_size()` (ps > 0) + the bitmap contracts
  (`alloc` ⇒ `set_bits.insert`, `clear` ⇒ `remove`, `test` ⇒ `is_bit_set`):
  - `alloc` / `book` ⇒ `allocated_frames' == allocated_frames.insert(frame_addr_of(idx))`,
    `refcounts' == refcounts.insert(.., 1)`.
  - `free` ⇒ `remove`; `share` ⇒ `refcounts[..] += 1`; `refcount` ⇒ pure read equality;
    `is_covered` ⇒ `(frame_number < num_bits) <==> covers(phys_addr@)`.
  - `alloc_contiguous` (213) and `alloc_range` (583) additionally need **full loop
    invariants** replacing the `#[cfg_attr(verus_keep_ghost, verus_spec(invariant false))]`
    placeholders.
- **Resolution path:** write per-method set-extensionality proofs (and loop invariants for
  the two range methods). Provable in principle but substantial; depends on A1 first.

================================================================================
## Group B — 7 lemmas (`src/kernel/src/mm/phys/manager.proof.rs`)
`lemma_manager_attached` (12), `lemma_free_count_bounded` (21), `lemma_kernel_alloc_one`
(36), `lemma_kernel_alloc_contiguous` (49), `lemma_user_bulk_ok` (94),
`lemma_user_bulk_err_restored` (115), `lemma_kernel_bulk_err_restored` (125).

- **Status: FALSE as standalone proof functions — unprovable and unsound to `admit()`.**
  Each asserts an equality/membership over universally-quantified arbitrary inputs not
  implied by its (weak or absent) `requires`:
  - `lemma_manager_attached(m) ensures m@ == phys_view().frames` — both opaque; false for
    arbitrary `m`.
  - `lemma_free_count_bounded() ensures phys_view().frames.free_count() <= usize::MAX` — no
    `requires`; `phys_view()` is `uninterp`, `free_frames` unconstrained.
  - `lemma_kernel_alloc_one(pre, post, addr) requires pre.wf() ensures
    pre.free_frames.contains(addr) && post == pre.alloc_one(addr)` — false for arbitrary
    `post`/`addr` (counterexample: empty `pre`, `addr = 5`).
  - `lemma_kernel_alloc_contiguous` / `lemma_user_bulk_ok`: `ensures post == pre.book_all(..)`
    for arbitrary `post`.
  - `lemma_user_bulk_err_restored` / `lemma_kernel_bulk_err_restored`: `ensures m@ == pre`
    for arbitrary `m`/`pre`.
- **Cause:** they are the **§8 ghost-token attachment** axioms (`view_design.md §8`,
  explicitly *deferred to the proving phase*), meant to be discharged by a token over the
  `frame::INSTANCE` / `PhysMemoryManager` singletons. That token was never implemented.
- **Why no workaround:** the statements are *false* without the token, so no proof exists;
  `admit()` is unsound; `external_body` is forbidden on proof functions; `verus!`-style
  axiomatization would just relabel the same unsoundness.
- **Resolution path:** implement the §8 global ghost-token attachment (a new proof-phase
  artifact), then re-state these lemmas with the token in their `requires`/owned state so
  they become true and provable. Out of scope for the phys-mod cheating-elimination phase.

================================================================================
## Already resolved (not TODOs)
- `manager.proof.rs::lemma_contig_no_overflow` — proved via
  `vstd::arithmetic::mul::lemma_mul_inequality`.
- `frame.rs` free-fn wrappers `alloc`, `alloc_contiguous`, `free_count`, `free`, `book`,
  `alloc_range` — converted from `admit()` placeholders to design-sanctioned TCB
  `external_body` (all listed in `verus-ai-logs/tcb-allowed.md`).
- `mod.rs::book_physical_memory_regions`, `mod.rs::book_mmio_regions` — TCB `external_body`
  (no vstd `LinkedList` model; orphan rule E0117 blocks an in-crate iterator model).

## Out of scope for this phase (other modules' admits, seen only because the gate compiles the whole kernel bin)
- `hal/mem/types/address/frame.proof.rs:13,22`, `hal/.../phys.proof.rs:6,16` (4 admits).
- `mm/virt/identity_map.proof.rs:14,23,32,45,53` and `mm/virt/identity_map.rs:533,627,718`
  (8 admits).
