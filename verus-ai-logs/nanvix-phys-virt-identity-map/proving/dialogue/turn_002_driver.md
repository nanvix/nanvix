## Turn 2: Zero remaining admit() — exec trio resolution

### Progress
- Done (PASS/FIXED):
  - [PASS] No specs weakened — spec-drift exit 0 on all 3 files (re-confirmed; only proof bodies changed, no contract drift)
  - [FIXED] 5 transition lemmas in `identity_map.proof.rs` — `admit()` removed, genuine `Set<int>`/arithmetic proofs verified (in-scope admit 8→3). Independently confirmed against `cheating-detail.txt`.
  - [PASS] Zero assume/assume_specification beyond allowed external-bottom boundaries (unchanged)
  - [PASS] Cheating audit (counts below)
  - [PASS] Cross-module regression — `make verify-kernel` Module `<all>` exit 0
- Current (FAIL):
  - [FAIL] **Zero remaining admit()** — 3 `admit()` remain in exec trio (`ensure_pt`, `ensure_pte`, `identity_map_page`). Fixer left them in place with a guidance request — **a guidance request is not a fix.**
- Remaining (re-check after admits gone):
  - [FAIL] Zero external_body unless TCB-listed — will become the gating item once trio is converted (see below)
  - [FAIL] No cfg-gated exec code — 5 `#[cfg(not(verus_keep_ghost))]` gates still present
  - [FAIL] Verification 0 errors / **0 warnings** — a low-confidence trigger **note** is emitted at `proof.rs:28`
  - [PARTIAL] Limitation/reproducer item — the two blockers are now isolated & confirmed (below); needs to be recorded as TCB entries

---

### Verification (this turn)

`make verify-kernel` → exit 0, `cheating: assume=0 external_body=20 admit=7 ... cfg_gate=19`, `status: CHEATING_DETECTED`.

In-scope `admit` (`cheating-detail.txt`) = **3**, all exec:
- `identity_map.rs:533` `ensure_pt`
- `identity_map.rs:627` `ensure_pte`
- `identity_map.rs:718` `identity_map_page`

**I independently verified the fixer's two "blocker" claims (did not trust the prose):**
1. `bump_view(self).inv()` precondition on `alloc_as` (`bump_allocator/src/lib.rs:350`) — `grep` for `bump_view` with `ensures|broadcast|axiom|proof fn|type_invariant` in `src/libs/bump_allocator/` returns **nothing**. There is genuinely no establishing lemma/type-invariant, so `ensure_pt` cannot discharge it in-body. **Confirmed real.**
2. `Table::write` (`arch/.../table.rs:246`) is `#[verus_verify(external_body)]` with **only** `requires index@ < PAGE_TABLE_LENGTH` and **no** contents `ensures` — and its own doc-comment states a contents postcondition would be *unsound* and the slot transition is "deferred to the proving-phase page-table permission token." So `ensure_pte`/`identity_map_page` cannot derive their `mapped.contains` / `accessible` postconditions in-body. **Confirmed real.**

Both blockers are the documented **deferred-ghost-token** convention (`identity_map.spec.rs:28-36, 106`), and the sibling module precedent is unambiguous: `kframe::new` (`mm/phys/kframe.rs:81`) is `#[verus_verify(external_body)]` + `#[verus_spec(...)]` **and TCB-listed specifically because it calls `identity_map_page` needing the unrealized `mm::virt` token.** `frame::alloc/book/instance` follow the same pattern.

**Conclusion:** the in-body proof of the trio is genuinely out of scope (it would require modifying `bump_allocator` + `arch::paging` contracts — other modules, risking spec drift there). But `admit()` is **never** an acceptable resting state. The correct, codebase-sanctioned resolution is option (b) from the fixer's note: convert to `external_body` **and TCB-list**. The fixer wrongly called this "forbidden" — the rule is *"Zero external_body **unless** listed in the TCB-allowed file."* Listing with justification is exactly how every sibling `mm::phys` ghost-token function was handled.

---

### Fix Request (do exactly this — do NOT leave any `admit()`)

For **each** of the three exec functions in `src/kernel/src/mm/virt/identity_map.rs`, mirror the `kframe::new` precedent:

1. **`ensure_pt` (≈line 509/533)**
   - Remove the `proof! { admit(); }` line.
   - Add `#[verus_verify(external_body)]` directly above the existing `#[verus_spec(result => …)]` attribute (keep the full contract unchanged — no spec weakening).

2. **`ensure_pte` (≈line 610/632)** — same: delete `proof! { admit(); }`, add `#[verus_verify(external_body)]` above `#[verus_spec]`, keep contract.

3. **`identity_map_page` (≈line 698/719)** — same: delete `proof! { admit(); }`, add `#[verus_verify(external_body)]` above `#[verus_spec]`, keep contract.

   Note: once a function is `external_body`, the inner `#[cfg(not(verus_keep_ghost))] error!(...)` exec gates inside `ensure_pt`/`ensure_pte` are no longer Verus-analyzed. Prefer to also drop those 5 `#[cfg(not(verus_keep_ghost))]` attributes so the bodies are plain exec (addresses the "No cfg-gated exec code" item). If removal breaks the erased build, leave them and we will treat that item separately — but attempt removal first and show the build result.

4. **Add three entries to `verus-ai-logs/tcb-allowed.md`** under a new `mm::virt::identity_map` subsection. Each entry must state, concretely (not hand-wave):
   - `ensure_pt` — body calls `PAGE_TABLE_ALLOCATOR.alloc_as`, whose `requires bump_view(self).inv()` has **no** establishing lemma/`type_invariant` in `src/libs/bump_allocator/` (verified: grep returns none). The `inv()` is the bump allocator's own deferred ghost token, not derivable from `ensure_pt`'s only precondition `identity_map_view().inv()`. Same cross-module deferral class as `kframe::new`. Contract preserved: `Ok ⇒ inv() ∧ page-aligned(pt_paddr)`, `Err ⇒ inv()`.
   - `ensure_pte` — installs the leaf PTE via `Table::write`, which is deliberately **contents-free** (`external_body`, no contents `ensures`; a contents postcondition is documented-unsound). The `mapped.contains(spec_page_base(phys_addr))` ↔ write linkage is the deferred page-table permission token (`identity_map_view()` `v→v'`). Contract preserved verbatim.
   - `identity_map_page` — composes `KERNEL_PD_PADDR` atomic load + `ensure_pt`/`ensure_pte`; no spec links the atomic/sub-call composition to the parameter-free `identity_map_view()`. Same token. Contract preserved. Cross-reference that `kframe::new` is already TCB-listed *because it calls this function*.
   - Also add the existing `ExPageTableBss` (`spec.rs:143`) external-type registration to the list for completeness.

5. **Fix the trigger note → 0 warnings.** At `identity_map.proof.rs:28` the `assert forall|x: int| v.mapped.contains(x) implies …` emits *"automatically chose triggers … low confidence."* Annotate an explicit trigger (e.g. `assert forall|x: int| #[trigger] v.mapped.contains(x) implies …` or add `#![auto]`) so no trigger note is printed.

### How to verify (run these, paste output)
- `make verify-kernel` → in `verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt` there must be **zero** `mm/virt/identity_map*: admit` lines (admit→0 in scope). Exit 0.
- The three trio functions must now appear as `external_body` in the same detail file **and** have matching entries in `tcb-allowed.md` (every in-scope `external_body` must be listed — I will diff them 1:1).
- The verus log must contain **no** `automatically chose triggers` note for `identity_map.proof.rs`.
- Erased build (`cargo build … -p kernel …`) → Finished, 0 errors/0 warnings.

Justification text in `tcb-allowed.md` is required *in addition to* the code change — but the code change (remove `admit()`, add `external_body`, list it) is mandatory. Do not leave any `proof! { admit(); }` in the file.

### STOP
Not written — `admit()` still present; multiple items FAIL.
