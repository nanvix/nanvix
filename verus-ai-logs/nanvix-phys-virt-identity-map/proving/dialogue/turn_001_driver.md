## Turn 1: Full checklist walkthrough + Fix request (admit elimination)

### Progress
- Done (PASS this turn):
  - [PASS] No specs weakened (spec-drift)
  - [PASS] Zero assume/assume_specification beyond allowed external-bottom boundaries
  - [PASS] Zero external_body exec functions in scope (only the documented external-type registration)
  - [PASS] Cheating audit performed (exact counts + locations below)
  - [PASS] Exec rewrites minimal / `// VERUS REWRITE` review (none present)
  - [PASS] Cross-module regression (`make verify-kernel`, Module `<all>`, exit 0)
- Current (FAIL — fix requested this turn):
  - [FAIL] **Zero remaining admit()** — 8 `admit()` in scope
- Remaining (FAIL/blocked by the admits — re-checked after admits are gone):
  - [FAIL] No cfg-gated exec code — 5 `#[cfg(not(verus_keep_ghost))]` exec gates
  - [FAIL] Claimed Verus limitation needs isolated reproducer — none of the 8 admits is yet justified or discharged
  - [FAIL] Verification 0 errors / 0 warnings — `make verify-kernel` is green **only** because of the admits (status: `CHEATING_DETECTED`)

---

### Verification (commands run + findings)

`make verify-kernel` → exit 0, but:
```
=== Cheating Pattern Check ===
  cheating: assume=0 external_body=20 admit=12 trusted=0 no_decreases=0 cfg_gate=19
  status: CHEATING_DETECTED
```
The verify is green only because every in-scope exec body / lemma is `admit()`-ed.

**Cheating audit — in-scope (the three `identity_map.*` files) exact counts + locations:**

`admit()` = **8**
- `identity_map.proof.rs:18` `lemma_install_page_maps`
- `identity_map.proof.rs:27` `lemma_install_page_monotone`
- `identity_map.proof.rs:40` `lemma_install_page_preserves_inv`
- `identity_map.proof.rs:49` `lemma_map_page_accessible`
- `identity_map.proof.rs:60` `lemma_map_page_preserves_inv`
- `identity_map.rs:534` `ensure_pt`  (`proof! { admit(); }`)
- `identity_map.rs:632` `ensure_pte` (`proof! { admit(); }`)
- `identity_map.rs:719` `identity_map_page` (`proof! { admit(); }`)

`external_body` (exec functions) = **0** in scope.
- Only `identity_map.spec.rs:143` `ExPageTableBss` — a `#[verifier::external_type_specification]` + `#[verifier::external_body]` *type* registration for the unverified `page_table_allocator::PageTableBss`. This is the standard opaque-external-type mechanism (same class as `ExLinkedList`), not an exec-function `external_body`. NOTE: it is not currently listed in `verus-ai-logs/tcb-allowed.md`; add a one-line entry there for traceability (same as `ExLinkedList`). Not a blocker.

`assume` (proof `assume(...)`) = **0** in scope.
- `assume_specification` = 2, both allowed external-bottom boundaries:
  - `identity_map.spec.rs:179` `<[T]>::as_ptr` — std slice primitive.
  - `identity_map.spec.rs:183` `FixedSizeBumpAllocator::<N,A,S>::new` — external `bump_allocator` crate constructor.

`cfg`-gated exec = **5** (`#[cfg(not(verus_keep_ghost))]` over `error!(...)` exec statements):
- `identity_map.rs:537`, `:553`, `:565` (inside `ensure_pt`)
- `identity_map.rs:635`, `:648` (inside `ensure_pte`)
- (`:24`/`:26` `#[cfg(verus_keep_ghost)] include!` of spec/proof and `:744` `#[cfg(feature = "test")]` test module are not in-scope exec gates.)

**Spec-drift** (`spec_drift.py git-diff … --before 725c7655` = proving phase-start SHA) on all three files → **exit 0, no contract drift**. PASS.

**Cross-module** — `make verify-kernel` runs Module `<all>`; exit 0, no module fails verification. PASS.

---

### Fix Request (address THIS first — Zero remaining admit())

Justification is not a fix. Each of the 8 `admit()`s below must be **discharged with a real proof** (or, if and only if a genuine Verus limitation is hit, replaced by an isolated reproducer + documented trust boundary — but these are plain Set/arithmetic facts, so a real proof is expected). Address every function individually:

**Proof lemmas (`identity_map.proof.rs`) — these are pure `Set`/`int` facts and should prove directly:**

1. `lemma_install_page_maps:18` — goal `v.spec_install_page(page).mapped.contains(page)`.
   `spec_install_page` is `mapped.insert(page)`. Replace `admit();` with the proof body (typically empty / `assert(v.mapped.insert(page).contains(page));`). Verify it goes through.

2. `lemma_install_page_monotone:27` — goal `v.mapped.subset_of(v.spec_install_page(page).mapped)`.
   `insert` only adds; prove via `assert forall|x| v.mapped.contains(x) implies v.mapped.insert(page).contains(x)`. Remove `admit();`.

3. `lemma_install_page_preserves_inv:40` — `requires v.inv(), v.initialized, spec_is_page_aligned(page)`, `ensures v.spec_install_page(page).inv()`.
   Show each `inv()` conjunct survives `insert(page)`: aligned-membership (use the `page` aligned hypothesis for the new element, existing members from `v.inv()`), and the `!initialized ==> empty` clause holds vacuously since `v.initialized`. Remove `admit();`.

4. `lemma_map_page_accessible:45` — `ensures v.spec_map_page(page).accessible(page)`.
   Case-split on `v.initialized`: initialized → `mapped.insert(page).contains(page)`; not initialized → `accessible` is `!initialized || …` = true. Remove `admit();`.

5. `lemma_map_page_preserves_inv:53` — `requires v.inv(), spec_is_page_aligned(page)`, `ensures v.spec_map_page(page).inv()`.
   Case-split on `v.initialized`: initialized → reuse the lemma_install_page_preserves_inv reasoning; not initialized → `spec_map_page` returns `self`, so `inv()` is unchanged. Remove `admit();`.

**Exec functions (`identity_map.rs`) — remove `proof! { admit(); }` and discharge the `#[verus_spec]` postconditions using the dependency contracts (`Table::read/write`, `pd_index/pt_index`, PDE/PTE constructors) and the lemmas above:**

6. `ensure_pt:534` — drop `proof! { admit(); }`. Prove both `Ok`/`Err` arms: `inv()` preserved on every path, and `Ok(pt_paddr) => spec_is_page_aligned(pt_paddr as int)` (page-table address is page-aligned — derive from the BSS `alloc_as` alignment contract). `ensure_pt` installs only empty PTs so `mapped` is untouched — make that explicit so `inv()` carries through.

7. `ensure_pte:632` — drop `proof! { admit(); }`. Prove `inv()` preserved, and the result arms: `Ok(_) => mapped.contains(spec_page_base(phys_addr as int))` (present-fast-path and freshly-installed path both, via `lemma_install_page_maps`), `Err(_) => !mapped.contains(spec_page_base(phys_addr as int))`. This is the leaf step that mutates the abstract `mapped`; thread the `v -> v'` page-table token transition rather than admitting it.

8. `identity_map_page:719` — drop `proof! { admit(); }`. Prove `inv()` preserved and `Ok(_) => accessible(phys_addr@)` / `Err(_) => !accessible(phys_addr@)`. Pre-init branch (`pd_paddr == 0` → `Ok(())`) is the no-op case: use `lemma_map_page_accessible` for the `accessible` fact; compose `ensure_pt` + `ensure_pte` postconditions for the post-init path.

**How to verify:** run `make verify-kernel`. Required end-state for this item: the `=== Cheating Pattern Check ===` line must show `admit=0` for the in-scope module (and the per-file detail in `verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt` must contain **no** `mm/virt/identity_map*` `admit` entries), with exit code 0. Do not replace an `admit()` with `assume(...)`, an `external_body`, a weakened spec, or a cfg-gate — any of those is a new FAIL.

After admits are gone I will re-check: the cfg-gated exec (5×), the limitation/reproducer item, and the final 0-error/0-warning verification.

### Do NOT proceed to STOP
Multiple items are FAIL/unverified. No `STOP` file written.
