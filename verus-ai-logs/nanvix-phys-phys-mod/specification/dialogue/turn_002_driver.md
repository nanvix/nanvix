## Turn 2: Verify fixer turn 1 — full checklist re-walk

### Progress
- Done (PASS/FIXED, all 18): #1–#18 below.
- Current: final verification of the fixer's cluster A/B/C changes.
- Remaining: none.

### Verification (commands run this turn)
- `make verify-kernel` → **exit 0**, `cheating: assume=0 external_body=17 admit=0 trusted=0`.
  (admit was 7 in turn 1 → now **0**.)
- `./z build` → `[OK] Build complete.` (dual compilation).
- `grep mm/phys cheating-detail.txt | grep -E "admit|assume|trusted"` → **no matches**
  (no in-scope admit/assume/trusted).
- `spec_drift.py check nanvix-phys-phys-mod` → **0 contract drift**, ensures removed 0.
- Read mod.rs (init + both `book_*`), mod.proof.rs (all 7 lemma bodies), mod.spec.rs
  (`phys_view`), frame.rs free-function wrappers.

I did not trust the fixer's claims — I re-ran every tool and read every changed file.

### Soundness audit of the `phys_view()` model (critical, done by hand)
`phys_view()` is a 0-arg `uninterp spec fn` whose value is pinned by the external_body
wrappers' `ensures`. I verified the axiom set is **satisfiable** (e.g. `initialized:true`,
`allocated_frames = all aligned addrs`, `free_frames = {}`, `refcounts: all↦1` satisfies
`wf()` and every wrapper ensures simultaneously), so verification is **not vacuous** —
`init` proves a real postcondition, not `false`. The model is weak (a constant cannot
express the pre→post transition), but that binding to the global singleton is exactly the
item the view-design phase explicitly deferred; the idiom (uninterp handle pinned by
external_body ensures) is the accepted way to talk about an unviewable `static mut`. No
rollback warranted.

### Per-item verdicts

| # | Item | Verdict | Evidence |
|---|------|---------|----------|
| 1 | In-scope fns have requires/ensures | **FIXED** | `init` (mod.rs:163), `book_physical_memory_regions` (:71), `book_mmio_regions` (:106) all carry `#[verus_spec]`. fn_coverage: 3 targets matched. |
| 2 | Caller coverage | **FIXED** | `init` Ok-ensures gives the exact guarantees `kernel_vas::init` consumes: `phys_view().initialized` + `phys_view().inv()` (⇒ every later `frame::*` may assume `wf`) + `allocated.disjoint(free)` (reserved frames never handed out). Per-frame enumeration is abstracted (LinkedList contents unviewable) and captured at lemma level (`lemma_init_establishes_and_reserves`); the caller does not consume per-frame facts. |
| 3 | View consistency | **FIXED** | Specs reference `phys_view().inv()`, `.initialized`, `.frames.allocated_frames/free_frames`, `covered()`, `region_frames()`; `inv()` preserved on every path. |
| 4 | No tautological ensures | **FIXED** | `init`/`book_*` hoist `phys_view().inv()` (and `book` `initialized`) **outside** the `match`, so the Err path guarantees inv-preservation — not `true`. The `Err(_) => true` inside the match correctly encodes the caller-documented "failure is terminal, no partial-state guarantee consumed." |
| 5 | No subsumed ensures | **PASS (note)** | `init`'s `Ok ⇒ disjoint` is derivable from `inv()+initialized`, but it is the caller_analysis headline invariant ("reserved frames disjoint from free") stated explicitly so callers need not unfold the `open` `wf()`. Intentional caller-convenience (serves #9); accepted. |
| 6 | Error paths meaningful | **FIXED** | `inv()` guaranteed on all paths incl. Err; matches caller contract. |
| 7 | No assume_specification for internal code | **PASS** | `assume=0`. Only `external_type_specification` on foreign std `LinkedList`. |
| 8 | vstd searched before assume_specification | **PASS** | No assume_specification added; LinkedList gap documented. |
| 9 | Specs written for the caller | **FIXED** | `init` ensures usable directly in `kernel_vas::init` (no longer `ensures true`). |
| 10 | Trait obligations | **PASS** | No traits. |
| 11 | Spec completeness (advisory) | **PASS** | Exec contract abstracts per-frame booking (LinkedList limit); full semantics in proven lemmas. Intentional, matches caller needs. |
| 12 | Loop invariants | **PASS** | Only loops are inside `book_*`/`frame::init`, all `external_body` (bodies unverified → no invariant required); `init` has no loop. |
| 13 | No cheating on own functions | **FIXED** | **admit=0** (was 7); all 7 lemmas in mod.proof.rs have real proof bodies (verified by reading + `make verify-kernel`). assume=0, trusted=0. In-scope `external_body` = the two `book_*` only, each justified (LinkedList orphan rule, verus-unsupported.md) **and** now carrying full contracts. frame.rs wrappers are the pre-existing frame-layer boundary to the `static mut` singleton, now spec'd. |
| 14 | No specs weakened | **PASS** | spec_drift: 0 contract drift, 0 ensures removed. Added `requires` (`spec_page_size()>0` on a lemma; `initialized/inv` on previously-contract-less `book_*`/wrappers) introduce no weakening of any pre-existing delivered contract. |
| 15 | Bug awareness | **PASS** | bugs.md present; no code bug; tooling limitation recorded. |
| 16 | Cross-module regression | **PASS** | `make verify-kernel` verified mm::phys, frame, manager, upool — exit 0. |
| 17 | Verification + build | **PASS** | `make verify-kernel` exit 0, admit=0; `./z build` `[OK] Build complete.` The global `CHEATING_DETECTED` flag is driven by `external_body` count across the whole kernel (pre-existing frame/manager/upool design), not by any in-scope admit/assume/trusted. |

### Lemma proof spot-check (mod.proof.rs)
- `lemma_spec_book_frames_preserves_inv` (:35) — full `wf()` proof, all 6 conjuncts by
  Set/Map reasoning. Real, non-trivial.
- `lemma_spec_book_frame_preserves_inv` (:126) — singleton case via `=~=` extensionality.
- `lemma_book_region_reserves_region_frames` (:165) — alignment via `nonlinear_arith` with
  `spec_page_size()>0` (a true global fact; strengthening-only).
- `lemma_book_mmio_skip_untracked`/`books_tracked` (:204/:218), `lemma_spec_initialize…`
  (:19), `lemma_init_establishes_and_reserves` (:238) — all discharged by composition.
All accepted by Verus (exit 0, admit=0).

### Fix Request
None. All 18 checklist items are PASS or FIXED with tool-verified evidence.

### STOP status
Creating STOP = RESOLVED.
