## Response to Turn 1

### What I Changed

**Proven (kept from the proving pass — these are real, admit-free):**
- `src/kernel/src/mm/virt/identity_map.proof.rs` — all 5 transition lemmas have full
  proof bodies, **0 `admit()`**: `lemma_install_page_maps`, `lemma_install_page_monotone`,
  `lemma_install_page_preserves_inv`, `lemma_map_page_accessible`,
  `lemma_map_page_preserves_inv`. Module verifies **9 verified, 0 errors, 0 warnings**.

**Primary request (remove the 3 exec `admit()`s):** I removed each `admit()` and tried to
discharge the real contracts. Every approach failed on obligations that cannot be satisfied
without (a) weakening a spec, (b) `assume`/`external_body`, or (c) editing out-of-scope
functions — all forbidden. Exact errors below. The 3 `admit()`s in
`identity_map.rs` (`ensure_pt:534`, `ensure_pte:632`, `identity_map_page:719`) are therefore
**retained**, with an honest hand-off in `verification-todo.md`.

**Secondary items — investigated, no valid code change:**
- **Item 5 (cfg-gated `error!`)**: NOT a valid fix. Removing `#[cfg(not(verus_keep_ghost))]`
  from a single `error!` makes Verus fail to compile (exact error below). The
  `verus-constraints` skill explicitly permits cfg-gating logging macros (`error!`/`info!`).
  Gates kept.
- **Item 4 (`assume_specification`)**: both justified, nothing to remove.
  `<[T]>::as_ptr` is a std external-bottom boundary. `FixedSizeBumpAllocator::<N,A,S>::new`
  (lib.rs:243) carries **no** `#[verus_spec]` — `bump_allocator` is not verified, so the
  placeholder is still required.
- **Item 3 (`ExPageTableBss`)**: pure `external_type_specification` opaque-type registration
  for the BSS storage marker; carries no trusted exec function body.

### Verification

`make verify-kernel MODULE=mm::virt::identity_map` (current, clean state):
```
note: verifying module mm::virt::identity_map
verification results:: 9 verified, 0 errors (partial verification with `--verify-*`)
  0 errors
    ⚠️  admit: 3
    - identity_map.rs:533 ensure_pt: admit
    - identity_map.rs:627 ensure_pte: admit
    - identity_map.rs:718 identity_map_page: admit
  verification: 9 verified, 0 errors (exit 0)
```
`make all-kernel`: `Finished dev profile ... EXIT:0` (exec code compiles in the normal build).
`make build`: `Nothing to be done for 'build'` (no-op target), EXIT:0.

### Result: NEEDS_GUIDANCE

I tried, per function. Exact errors from every approach:

#### Attempt A — `ensure_pte` (`identity_map.rs:627`), admit removed
Minimal body (no admit). Verus output:
```
error: postcondition not satisfied
   --> src/kernel/src/mm/virt/identity_map.rs:618:13
    |
618 |             Ok(_) => identity_map_view().mapped.contains(spec_page_base(phys_addr as int)),
    |             ^^^^^ failed this postcondition
...
639 |         return Ok(());

error: precondition not satisfied
   --> src/kernel/src/mm/virt/identity_map.rs:631:40
    |
631 |     let pte: PageTableEntry = unsafe { pt.read(pte_idx) }.ok_or_else(|| {
    |                                        ^^^^^^^^^^^^^^^^
   ::: src/libs/arch/src/x86/mem/paging/table.rs:205:13   (requires index@ < PAGE_TABLE_LENGTH)
```
- **L618** is the headline blocker. `identity_map_view()` is `uninterp spec fn ... ->
  IdentityMapView` (spec.rs:36) — a **parameter-free, hence constant** function, so
  `identity_map_view().mapped` is a fixed unknown `Set`. The `Ok` arm must prove the page is
  in it and the `Err` arm must prove it is *not* — but the return path is unrelated to that
  fixed set. The existing lemmas only act on an arbitrary `v: IdentityMapView`; firing them
  needs the fact `identity_map_view() == old.spec_install_page(spec_page_base(phys_addr))`,
  which is exactly the **ghost-token attachment** the spec/TCB notes defer. It does not exist
  in the codebase, and asserting it without proof is an `assume` in disguise (forbidden).
  Confirming there is no spec hook: `PageTableEntry::is_present` (pte.rs:415) and
  `PageTableEntry::new` (pte.rs:317) have **no `#[verus_spec]`** — nothing ties the exec
  `pt.write(...)` to `identity_map_view().mapped`.

#### Attempt B — discharge the `read` precondition via the type invariant
Added `proof! { use_type_invariant(pte_idx); }`:
```
error: Verus Internal Error: missing type invariant function
   --> src/kernel/src/mm/virt/identity_map.rs:632:33
```
- `TableIndex`'s `#[verifier::type_invariant] inv(&self) -> bool { self@ < PAGE_TABLE_LENGTH }`
  lives in the **arch** crate (table.spec.rs:24); `use_type_invariant` on it from the
  **kernel** crate triggers a Verus internal error (cross-crate type invariants are not
  exported). This is a genuine Verus limitation (item 7). The only in-rules alternative —
  adding `requires pte_idx@ < PAGE_TABLE_LENGTH` to `ensure_pte` — is a **spec weakening**
  (adds a `requires`), which is forbidden.

#### Attempt C — `ensure_pt` (`identity_map.rs:533`), admit removed
```
error: precondition not satisfied
   --> src/kernel/src/mm/virt/identity_map.rs:534:44
534 |     let pde: PageDirectoryEntry = unsafe { pd.read(pde_idx) }.ok_or_else(|| {
   ::: src/libs/arch/src/x86/mem/paging/table.rs:205:13   (requires index@ < PAGE_TABLE_LENGTH)

error: precondition not satisfied
   --> src/kernel/src/mm/virt/identity_map.rs:549:9
549 | /         PAGE_TABLE_ALLOCATOR
550 | |             .alloc_as::<[PteWord; PAGE_TABLE_LENGTH]>()
    | |_______________________________________________________^   (requires bump_view(self).inv())
```
- `pd.read` precondition: same cross-crate type-invariant wall as Attempt B.
- `alloc_as` requires `bump_view(self).inv()` (lib.rs:350) for the module-static
  `PAGE_TABLE_ALLOCATOR`. There is no spec-readable invariant for that static — it would
  need a ghost attachment to the singleton, same deferral.
- The `Ok` alignment postcondition is **partly** provable (present-PDE path:
  `frame_address` ensures `% FRAME_SIZE == 0`, and `FRAME_SIZE == PAGE_SIZE ==
  spec_page_size()`), but the fresh-alloc path (`slot.as_ptr() as usize`) is not: the
  `<[T]>::as_ptr` `assume_specification` has an empty contract and is not linked to the
  allocator's `slot_ref_addr` alignment. Tightening `as_ptr` universally is unsound (a
  slice ptr is not generally page-aligned). Because a removed `admit()` must discharge the
  *whole* body, the unprovable preconditions above fail regardless.

#### Attempt D — `identity_map_page` (`identity_map.rs:718`), admit removed
```
error: postcondition not satisfied
   --> src/kernel/src/mm/virt/identity_map.rs:706:13
706 |             Ok(_) => identity_map_view().accessible(phys_addr@),
    |             ^^^^^ failed this postcondition          (3 instances: pre-init no-op arm,
                                                            ensure_pt? arm, ensure_pte tail)
```
- `accessible(p) := !initialized || mapped.contains(p)` over the same opaque constant view.
  No spec link from the exec `pd_paddr == 0` test to `identity_map_view().initialized`, nor
  from `ensure_pt`/`ensure_pte` to `mapped`. Same ghost-token deferral.

#### Attempt E — item 5, ungate one `error!`
Removed `#[cfg(not(verus_keep_ghost))]` above `error!("ensure_pt(): {reason}")`:
```
error: Unsupported constant type
   --> src/kernel/src/macros.rs:188:12
    = note: this error originates in the macro `write` ... expansion of the macro `error`
error: could not compile `kernel` (bin "kernel") ... (verus did not run)
```
- `error!` is **not** Verus-compilable; the cfg-gate is mandatory and skill-sanctioned.
  Reverted.

### What unblocking requires (for the driver)

All four primary failures reduce to one missing piece: `identity_map_view()` must stop being
an opaque parameter-free constant and become a concrete function of **spec-readable global
state** that the exec writes update. Building that within this task is not possible because:
1. It needs a spec-readable value of the `KERNEL_PD_PADDR` atomic. `KERNEL_PD_PADDR:
   AtomicUsize` is a plain `core` atomic with no Verus ghost/permission. Replacing it with a
   verified atomic changes the `load`/`store` sites in **`init`** and **`sync_kernel_pdes`**,
   which are **out of scope** ("do not touch unlisted functions").
2. It needs `arch::…::Table::write` to expose an `old@ -> @` slot-update postcondition;
   `tcb-allowed.md` records that this is **deliberately omitted** (a contents postcondition on
   that `external_body` write was shown unsound), and `table.rs` is out of scope.
3. Threading a `tracked` token through `ensure_pt`/`ensure_pte`/`identity_map_page` would
   change their signatures, breaking out-of-scope callers (`init`, `memcpy`, `memset`,
   `ensure_identity_mapped_range`).

Every sibling module (`mm/phys/*`) is in the identical deferred state (still carries exec
`admit()`s for the same opaque-view reason), so this is a cross-module framework decision,
not a local proof gap. Guidance needed: either (i) authorize the verified-atomic +
`Table::write` transition infrastructure (which spans out-of-scope modules/functions), or
(ii) accept these three exec obligations as proving-phase-deferred (tracked in
`verification-todo.md`) until that infrastructure lands. No spec was weakened and no
`assume`/`external_body`/`admit`-disguise was introduced to paper over the gap.
