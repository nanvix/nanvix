## Turn 2: Re-verify fixer's tautology fixes (#2, #4, #6) + close remaining items

### Progress
- Done (PASS): #1, #3, #5, #7, #8, #9, #10, #11, #12, #13, #14, #15, #16, #17.
- FIXED this turn: **#2 caller coverage (Err side)**, **#4 no tautological ensures**,
  **#6 error paths meaningful**.
- Current: final sweep — all 17 items now PASS or FIXED.
- Remaining: none.

### Verification performed (independent — did not trust fixer claims)

Read the actual committed file (fixer changes are in commit `2439e5eda`), re-ran every tool.

**1. The three `Err(_) => true` arms are gone.** `grep -n "Err(_) =>" identity_map.rs`:
```
530:  Err(_) => identity_map_view().inv(),                                              # ensure_pt
624:  Err(_) => !identity_map_view().mapped.contains(spec_page_base(phys_addr as int)), # ensure_pte
715:  Err(_) => !identity_map_view().accessible(phys_addr@),                            # identity_map_page
```
No `Err(_) => true` anywhere in the three in-scope functions. (#4 FIXED.)

**2. Per-function soundness review of each new Err arm** (bodies `admit()` in spec phase,
so I checked soundness by hand, not just verifier acceptance):

- `identity_map_page` (`:698-716`): `Err(_) => !accessible(phys_addr@)`. Sound: the
  pre-init path returns `Ok` as a no-op (`:704-706`), so any `Err` is post-init
  (`initialized` true ⇒ `accessible(p) == mapped.contains(p)`); an already-present PTE
  returns `Ok` via the idempotent fast path, and a failed `ensure_pt`/`ensure_pte` does
  not add the page ⇒ `!accessible`. This is the exact caller_analysis Err expectation
  ("frame NOT accessible; `KernelFrame::new` must not deref"). Tight negation of the
  `Ok(_) => accessible` arm ⇒ non-tautological. (#2 Err side + #6 FIXED for this fn.)

- `ensure_pte` (`:610-625`): `Err(_) => !mapped.contains(spec_page_base(phys_addr))`.
  Sound: a present PTE returns `Ok` (idempotent fast path `:626-628`), so on `Err` the
  PTE was absent and the read/frame-number failure left it absent; one PTE per page ⇒
  page not in `mapped`. Exact negation of the `Ok` arm. Meaningful. (FIXED for this fn.)

- `ensure_pt` (`:509-531`): restructured to `Ok(pt_paddr) => inv() && aligned`,
  `Err(_) => inv()`. The reviewer's turn-1 suggestion ("`mapped` unaffected") is an
  `old@ == self@` delta that is **genuinely inexpressible** here: `ensure_pt` is a free
  function over the parameterless global accessor `identity_map_view()` — there is no
  `self`, hence no `old()` snapshot. I verified the established codebase convention:
  `mm::phys` free functions over the singleton `phys_view()` (same shape) state
  **post-state facts**, never `old()` deltas — e.g. `frame.rs:865`
  `Err(_) => !phys_view().frames.free_frames.contains(phys_addr@)`,
  `manager.rs:101 Err(_) => phys_view().manager_ready`; `grep 'old(' mm/phys/mod.rs` → none.
  `ensure_pt` adds nothing to `mapped` and has no page argument, so the strongest **sound,
  expressible** failure fact is invariant preservation (`inv()` = no corruption of the
  abstract map on failure). That is a real predicate, not `true`, and consistent with
  precedent. Accepted. (#6 FIXED for this fn.)

**3. No-subsumed recheck (#5 still holds).** For `ensure_pte`/`identity_map_page` the
top-level `inv()` plus `Ok => accessible/mapped.contains` and `Err => !accessible/
!mapped.contains` are not derivable from `inv()` (which never pins a specific page's
membership). For `ensure_pt`, the `aligned` conjunct is not implied by `inv()`. No arm
is subsumed.

**4. `make verify-kernel` (independent run)** → **exit 0**,
`note: verifying module mm::virt::identity_map`. Cheating line:
`assume=0 external_body=18 admit=36 trusted=0`. (#16, #17 PASS.)

**5. `spec_drift.py check virt-identity-map`** → "Functions with changes: 0; Contract
drift: 0; Ensures removed: 0; Requires added: 0; ✅ No contract drift detected."
Ensures were strengthened (Err arms gained content), never weakened. (#14 PASS.)

**6. `fn_coverage.py`** (unchanged from turn 1) → 14/14 matched; all 3 in-scope fns carry
`#[verus_spec(requires … ensures …)]`. (#1 PASS.)

**7. #13 — cheating on own functions.** `cheating-detail.txt` filtered to this module:
own-function cheating is **`admit` only** — 3 exec bodies (`identity_map.rs:533 ensure_pt`,
`:627 ensure_pte`, `:718 identity_map_page`) and 5 proof lemmas
(`identity_map.proof.rs:14/23/32/45/53`). **No `external_body`, `assume`, or `trusted` on
any of the module's own functions** (`assume=0 trusted=0`; the `external_body=18` are all
on foreign `Ex*` type wrappers / external-dep `assume_specification`s, not own exec fns).
These `admit()`s are the deliberate, enumerated **specification-phase scaffold**: the exec
bodies cannot be Verus-proved without proof annotations (that is the proving phase), and
the lemmas are signatures-only by design. For the **specification phase** bar this is
acceptable (counts reported, no illegitimate cheating form present); the proving phase
must discharge all 8 admits. PASS (spec-phase), tracked for proving.

**8. #7 — assume_specification on workspace crates.** `identity_map.spec.rs:179-251`
assume-specs `arch::mem::paging` (`pd_index`/`pt_index`/`invlpg`, `Table::{from_address,
read,write}`, PDE/PTE constructors) and `bump_allocator::FixedSizeBumpAllocator::new`.
Both are **not-yet-verified dependency boundaries**: `arch` is the trusted HAL/MMU
boundary (caller_analysis explicitly designates `Table::read/write` + `paging::invlpg` as
"the trusted HAL boundary"), and `bump_allocator::alloc/alloc_as` are already in
`tcb-allowed.md`. The spec.rs header documents the supersession plan ("when the underlying
modules are verified, their real `#[verus_spec]` contracts supersede them"). Per the
checklist's own carve-out ("assume_specification on external dependencies temporarily
allowed"), PASS (spec-phase); must be removed once `arch`/`bump_allocator` are verified.

**9. #11 — spec completeness (advisory).** The Err arms are now tight negations of the Ok
arms and match the caller_analysis Err expectations exactly; nondeterminism is intentional
and caller-aligned. PASS.

### Final verdicts (all 17)

| # | Item | Verdict |
|---|------|---------|
| 1 | In-scope fns have requires/ensures | PASS |
| 2 | Caller coverage (Err side) | **FIXED** |
| 3 | View consistency | PASS |
| 4 | No tautological ensures | **FIXED** |
| 5 | No subsumed ensures | PASS |
| 6 | Error paths meaningful | **FIXED** |
| 7 | No assume_spec for workspace-internal | PASS (temp-allowed HAL/bump_allocator, tracked) |
| 8 | vstd searched | PASS |
| 9 | Specs usable by caller | PASS |
| 10 | Trait obligations | PASS |
| 11 | Spec completeness (advisory) | PASS |
| 12 | Loop invariants | PASS |
| 13 | No cheating on own fns | PASS (admit-only spec scaffold, tracked for proving) |
| 14 | No specs weakened | PASS |
| 15 | Bug awareness | PASS |
| 16 | Cross-module regression | PASS |
| 17 | Verification | PASS |

### Fix Request
None. All previously-FAIL items (#2, #4, #6) are verified FIXED with sound, meaningful,
non-tautological Err postconditions; every other item is PASS.

### Carry-forward to the proving phase (not blocking spec phase)
- Discharge the 8 `admit()`s (3 exec bodies + 5 proof lemmas).
- Replace `arch`/`bump_allocator` `assume_specification`s with the real `#[verus_spec]`
  contracts once those modules are verified.

**STOP = RESOLVED** (all 17 checklist items PASS or FIXED with evidence).
