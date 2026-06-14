## Turn 1: Full checklist sweep (specification phase, `virt-identity-map`)

In-scope functions (caller_analysis.md): `identity_map_page`, `ensure_pt`,
`ensure_pte`. Source/spec/proof:
- `src/kernel/src/mm/virt/identity_map.rs`
- `src/kernel/src/mm/virt/identity_map.spec.rs`
- `src/kernel/src/mm/virt/identity_map.proof.rs`

### Progress
- Done (PASS): #1 fn coverage, #3 view consistency, #5 no-subsumed, #8 vstd-searched,
  #9 caller-usable, #10 trait-obligations, #12 loop-invariants, #14 no-spec-drift,
  #15 bug-awareness, #16 cross-module regression, #17 verification.
- FAIL (fix required): **#4 tautological ensures**, **#6 meaningless error paths**,
  **#2 caller coverage (Err side)**, **#13 admit() on the module's own functions**.
- CONCERN (track, not blocking spec-phase): #7 assume_specification on workspace-internal
  crates, #11 spec-completeness (Err nondeterminism).
- Current: driving the **#4 / #6** failure (tautological `Err(_) => true`).

### Verification performed

Commands run from repo root:

- `make verify-kernel` → **exit 0**, `note: verifying module mm::virt::identity_map`,
  results "cached (no recompilation)". Cheating check:
  `assume=0 external_body=18 admit=36 trusted=0 no_decreases=0 cfg_gate=15`,
  status `CHEATING_DETECTED`. (#16, #17 PASS; #13 evidence.)
- `fn_coverage.py src/kernel/src/mm/virt/identity_map.rs <same>` → 14/14 matched, 0 missing.
  All three in-scope functions carry `#[verus_spec(... requires ... ensures ...)]`
  (`identity_map.rs:509`, `:600`, `:684`). (#1 PASS.)
- `spec_drift.py check virt-identity-map` → "Functions with changes: 0; Contract drift: 0;
  No contract drift detected." Baseline had empty stubs, so only additions. (#14 PASS.)
- `cheating-detail.txt` (per-function):
  - admit on **module's own functions**: `ensure_pt` (`identity_map.rs:523`),
    `ensure_pte` (`:613`), `identity_map_page` (`:698`); proof lemmas
    `lemma_install_page_maps` (`:14`), `lemma_install_page_monotone` (`:23`),
    `lemma_install_page_preserves_inv` (`:32`), `lemma_map_page_accessible` (`:45`),
    `lemma_map_page_preserves_inv` (`:53`). **admit = 8 in this module.**
  - `external_type_specification` entries (`ExTable`, `ExTableIndex`, `ExPageDirectoryEntry`,
    `ExPageTableEntry`, `ExPageDirectoryEntryFlags`, `ExPageTableEntryFlags`, `ExPageTableBss`)
    are on **foreign** types, not the module's own exec functions. `external_body` on the
    module's own functions = **0**; `assume`/`trusted` = **0**.
- vstd search for `<[T]>::as_ptr` in
  `/home/ruize/toolchain/verus/vstd/{slice.rs,std_specs/slice.rs}` → **no spec exists**, so
  the `assume_specification<T>[ <[T]>::as_ptr ]` placeholder is justified (#8 PASS).
- Sibling-module precedent: `mm::phys` uses the **same** parameterless global accessor
  `phys_view()` and still gives **meaningful** `Err` arms, e.g.
  `frame.rs:754 Err(_) => phys_view().frames.free_frames.is_empty()`,
  `:865 Err(_) => !phys_view().frames.free_frames.contains(phys_addr@)`,
  `:926 Err(_) => !phys_view().frames.allocated_frames.contains(frame@)`,
  `manager.rs:101 Err(_) => phys_view().manager_ready`. This proves the parameterless
  `identity_map_view()` model is **not** an excuse for `Err(_) => true`.

### Item-by-item verdicts

| # | Item | Verdict | Evidence |
|---|------|---------|----------|
| 1 | In-scope exec fns have requires/ensures | PASS | all 3 annotated |
| 2 | Caller coverage (caller_analysis.md) | **FAIL** | Ok side covered; every `Err` expectation ("frame NOT accessible / caller must not deref") is dropped to `true` |
| 3 | View consistency (view_design.md) | PASS | specs use `inv()`, `mapped`, `accessible`, `spec_map_page`; `Set<int>` vs `Set<usize>` deviation is documented + justified in `identity_map.spec.rs:14-25` |
| 4 | No tautological ensures | **FAIL** | `Err(_) => true` at `identity_map.rs:520`, `:610`, `:695` — the exact forbidden pattern |
| 5 | No subsumed ensures | PASS | Ok arms add `accessible` / `mapped.contains` / `pt_paddr` alignment beyond `inv()` |
| 6 | Error paths have meaningful ensures | **FAIL** | same three `Err(_) => true` |
| 7 | No assume_specification for workspace-internal code | CONCERN | `arch` and `bump_allocator` are workspace crates (`src/libs/...`); `identity_map.spec.rs:179-251` assume-specs `pd_index/pt_index/invlpg`, `Table::{from_address,read,write}`, PDE/PTE ctors, `FixedSizeBumpAllocator::new`. Temp-allowed per the "external deps" note + sibling precedent, but must be tracked |
| 8 | vstd searched before assume_specification | PASS | `as_ptr` absent from vstd; arch/HAL not modeled by vstd |
| 9 | Specs usable by caller | PASS | `KernelFrame::new` only needs `Ok => accessible(phys_addr@)`, which is provided |
| 10 | Trait obligations satisfied | PASS | none for in-scope fns (caller_analysis.md) |
| 11 | Spec completeness (advisory) | CONCERN | `Err` arms fully nondeterministic — overlaps #4 |
| 12 | Loop invariants | PASS | no loops in the three in-scope fns (`init`'s loop is out of scope) |
| 13 | No cheating on module's own functions | **FAIL** | admit = 8 (3 exec bodies + 5 proof lemmas), enumerated above |
| 14 | No specs weakened | PASS | spec_drift = 0 |
| 15 | Bug awareness | PASS | no fundamentally incorrect code found in the three fns (pre-init guard, idempotent present-checks, BSS-pool source all consistent with caller_analysis) |
| 16 | Cross-module regression | PASS | `kernel::all` exit 0 |
| 17 | Verification | PASS | `make verify-kernel` exit 0 (build-std build succeeded); cheating status reported under #13 |

### Fix Request (address #4 + #6 first; they also clear #2)

**Problem.** All three in-scope functions end their `ensures` match with
`Err(_) => true` (`identity_map.rs:520`, `:610`, `:695`). This is the canonical
tautological/no-information error arm the checklist forbids, and it drops every
`Err`-side caller expectation from caller_analysis.md (on failure the frame is *not*
accessible and `KernelFrame::new` must not construct the frame). The sibling
`mm::phys` module — same parameterless `phys_view()` global model — demonstrates that
meaningful, sound `Err` arms are both expected and achievable here.

**What to change.** In `src/kernel/src/mm/virt/identity_map.rs`, replace each
`Err(_) => true` with a meaningful, **sound** post-condition stated over
`identity_map_view()`, mirroring the `mm::phys` convention. Concretely:

1. `identity_map_page` (`:684-696`): the headline caller guarantee on failure is
   "the page was *not* made newly accessible; treat as unreachable." Express the
   failure-state relationship to the abstract map rather than `true`. If a sound
   post-state-only predicate cannot be written without referring to the prior state
   (i.e. you need "state unchanged"), then thread the abstract transition explicitly
   — view_design.md states global-state threading is a specification-phase concern,
   and the "How each in-scope function uses the View" table already prescribes
   `self@ == old@.spec_map_page(base(phys_addr@))` with `On Err, self@ == old@`.
   Realize that `On Err, self@ == old@` (no page silently added) so the `Err` arm is
   non-tautological.
2. `ensure_pt` (`:509-521`): `ensure_pt` adds nothing to `mapped` on either path, so
   give the `Err` arm a real statement (e.g. the abstract map is unchanged /
   `identity_map_view().mapped` is unaffected) instead of `true`.
3. `ensure_pte` (`:600-611`): on failure the leaf entry was not installed; state the
   corresponding failure-state fact over `identity_map_view().mapped` rather than `true`.

**Do not** justify the tautology verbally. Either (a) write a meaningful sound `Err`
predicate, or (b) introduce the `old()`/transition threading and assert
`self@ == old@` on the error path. A verbal "nothing meaningful can be said" is not
accepted unless accompanied by verifier output proving the alternative predicate is
unsound.

**How to verify the fix:**
- `make verify-kernel` must still exit 0 with `mm::virt::identity_map` verifying.
- `python3 /home/ruize/verus-ai-exp/verus-ai/scripts/spec_drift.py check virt-identity-map`
  must report no *weakening* (ensures may only be strengthened).
- Re-inspect `identity_map.rs:520/:610/:695`: no `Err(_) => true` may remain.

### Remaining after #4/#6
- #2 caller coverage is cleared by the same fix (Err side becomes meaningful).
- #13 (admit on own functions): the 3 exec-body `proof!{ admit(); }` and the 5
  `admit()` proof lemmas are the specification-phase scaffold; they must be discharged
  in the proving phase. Tracked, not closed.
- #7 / #11: tracked concerns (workspace-internal assume_specifications must move to the
  real modules' `#[verus_spec]` or be documented in `tcb-allowed.md`; Err
  nondeterminism resolved by #6).

**STOP not created** — checklist items #2, #4, #6, #13 are FAIL.
