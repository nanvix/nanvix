## Turn 1: Zero admit() remaining

### Progress
- Done (PASS this turn):
  - **Zero assume()** — PASS. Counter `assume=0`; `grep "assume("` over the three
    files returns nothing. (`assume_specification` entries exist but are trusted
    external contracts, tracked under the external_body item, not `assume()`.)
  - **Zero trusted functions** — PASS. Counter `trusted=0`.
  - **Zero exec_allows_no_decreases_clause** — PASS. Counter `no_decreases=0`;
    `grep no_decreases` returns nothing.
  - **Zero cfg-gated exec code** — PASS. All `cfg` in `identity_map.rs` are
    allowed kinds:
      - L24/L26 `#[cfg(verus_keep_ghost)] include!(...spec/proof.rs)` → imports.
      - L537/L553/L565/L635/L648 `#[cfg(not(verus_keep_ghost))]` each wrap a single
        `error!(...)` → logging.
      - L744 `#[cfg(feature = "test")]` → test module gating.
    No exec *logic* is cfg-gated. PASS.
- Current: **Zero admit() remaining** — FAIL (this turn's fix request).
- Remaining (not yet adjudicated; blocked behind admit removal):
  - Zero external_body unless listed in tcb-allowed (ExPageTableBss + 2
    assume_specifications need adjudication — see Notes).
  - AST consistency (ast-consistency skill).
  - All exec rewrites have VERUS REWRITE comment + minimal reproducer.
  - Surviving external_body each listed in tcb-allowed.
  - No specs weakened (spec-drift-check).
  - Cross-module regression (`make verify-kernel` all modules).
  - Verification + build: 0 errors, 0 warnings.

### Verification
Ran `make verify-kernel` (exit 0, **status: CHEATING_DETECTED**). Cheating detail
filtered to this module (`verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt`):

```
- mm/virt/identity_map.rs:533 ensure_pt: admit
- mm/virt/identity_map.rs:627 ensure_pte: admit
- mm/virt/identity_map.rs:718 identity_map_page: admit
- mm/virt/identity_map.spec.rs:143 ExPageTableBss (struct): external_type_spec
```

Read the three exec bodies directly. Each opens with `proof! { admit(); }`:

- `ensure_pt` — `src/kernel/src/mm/virt/identity_map.rs:534`
- `ensure_pte` — `src/kernel/src/mm/virt/identity_map.rs:632`
- `identity_map_page` — `src/kernel/src/mm/virt/identity_map.rs:719`

The spec.rs comment (L106) openly states: *"The intra-call obligations they would
discharge are currently `admit()`-ed in the exec bodies."* That is exactly the
cheating this phase must eliminate. The `#[verus_spec]` postconditions on all three
functions are therefore currently **unproven** — `admit()` discharges every proof
goal unconditionally.

### Fix Request
**Remove all three `admit()` calls and prove each function's contract for real.**
Address each function individually — do not batch.

1. **`ensure_pt` (delete `proof! { admit(); }` at line 534).**
   Contract requires: on `Ok(pt_paddr)` → `identity_map_view().inv()` and
   `spec_is_page_aligned(pt_paddr as int)`; on `Err` → `identity_map_view().inv()`.
   - Page-alignment of `pt_paddr` must come from the real arch/bump-allocator
     contracts (`Table`/`alloc_as`/`as_ptr`), not an assumption. If the current
     dependency `ensures` are too weak to prove `spec_is_page_aligned(pt_paddr)`,
     strengthen the proof using the genuine contracts or a proof lemma — do not
     re-introduce `admit()`.
   - `inv()` preservation: `ensure_pt` installs only an *empty* page table, so
     `mapped` is unchanged; invoke the proof.rs transition lemmas as needed.

2. **`ensure_pte` (delete `proof! { admit(); }` at line 632).**
   Contract requires: `identity_map_view().inv()` always; on `Ok` →
   `mapped.contains(spec_page_base(phys_addr as int))`; on `Err` →
   `!mapped.contains(spec_page_base(phys_addr as int))`.
   - The `Ok` membership fact is the V==P leaf step — discharge it via
     `lemma_install_page_maps` / `lemma_map_page_*` in `identity_map.proof.rs`
     against the real page-table write contract, not `admit()`.
   - Prove the idempotent fast path (`pte.is_present()` → `Ok`) and the failure
     path (`!contains`) consistently with the View transition.

3. **`identity_map_page` (delete `proof! { admit(); }` at line 719).**
   Contract requires: `inv()` always; on `Ok` → `accessible(phys_addr@)`; on `Err`
   → `!accessible(phys_addr@)`.
   - Compose the now-proven `ensure_pt` and `ensure_pte` postconditions plus
     `lemma_map_page_accessible` / `lemma_map_page_preserves_inv` to discharge the
     pre-init no-op branch (L724–726) and the post-init mapping branch.

**Constraint:** Justification is not a fix. Do not replace `admit()` with `assume()`,
a weakened (`true`/trivial) postcondition, or a new unlisted `external_body`. The
function signatures and `#[verus_spec]` contracts must remain as strong as they are
now (see spec-drift item).

**Verify:** Run `make verify-kernel` and confirm the summary line reports
`admit=0` for this module (no `identity_map.rs:*: admit` lines in
`cheating-detail.txt`) with `Exit code : 0` and no new `assume`/`external_body`/
`no_decreases` entries.

### Notes (deferred items flagged, to adjudicate after admit removal)
- `ExPageTableBss` (spec.rs:143, `external_type_specification` + `external_body`)
  is **not** present in `verus-ai-logs/tcb-allowed.md`. Once admits are gone this
  must either be listed with a written rationale or eliminated.
- `assume_specification` for `<[T]>::as_ptr` (spec.rs:179) and
  `FixedSizeBumpAllocator::<N,A,S>::new` (spec.rs:183): only `alloc`/`alloc_as` are
  in tcb-allowed — `new` and `as_ptr` are not. These are trusted external contracts
  the real proofs will depend on; they must be justified/listed or replaced by the
  real arch/bump contracts.
