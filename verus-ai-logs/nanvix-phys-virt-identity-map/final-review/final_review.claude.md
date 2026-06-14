# Final Review (claude-opus-4.8): virt-identity-map

Independent strict verification of `identity_map_page`, `ensure_pt`, `ensure_pte` in
`src/kernel/src/mm/virt/identity_map.rs`. All files read directly; no summary trusted.

## Spec Quality

The API contracts are well-written *prose-wise* but are **vacuous in substance** because of
two compounding facts:

- `identity_map_view()` is `uninterp` (spec.rs:36) AND `IdentityMapView::internal_inv()` is the
  literal placeholder `true` (spec.rs:60-62). The implementation-consistency invariant that is
  supposed to tie the abstract `mapped` set to the real PDE/PTE present-bits is therefore *not
  encoded at all*. The View is completely disconnected from the concrete page tables.
- The three exec bodies are entirely `admit()`-ed (see below), so none of the ensures are
  proven against the code.

Detailed criteria:
- **No tautological ensures:** mostly OK. `ensure_pt`'s `Err(_) => identity_map_view().inv()`
  (spec.rs:530) is not literally `true` (inv() carries the alignment + pre-init-empty
  constraints), but it is a *weak* failure postcondition — it says nothing about `mapped`.
  `ensure_pte`'s `Err(_) => !mapped.contains(page_base)` (rs:624) and `identity_map_page`'s
  `Err(_) => !accessible(phys_addr@)` (rs:715) are genuinely meaningful, non-tautological
  error paths. Good.
- **Mathematical types:** addresses modeled as `int`, `mapped: Set<int>` — appropriate.
- **inv() encodes real constraints:** partial. The two outer conjuncts (alignment of every
  mapped page; `!initialized ==> mapped == empty`) are real. But `internal_inv() == true`
  removes the *only* invariant that would make the spec non-trivial, so `inv()` does not
  constrain the abstract state to match reality.
- **Subsumption:** on `identity_map_page` Ok, `accessible(phys_addr@)` is trivially `true`
  whenever `!initialized` (accessible returns `!initialized || ...`). That is intended no-op
  semantics, acceptable.
- **Permissions / TLB:** the "writable, supervisor" and "TLB-consistent" guarantees callers
  rely on are only *documentation* (View doc-comments at spec.rs:22-25), not checkable
  predicates. invlpg carries an empty contract, so TLB consistency is not expressed in spec.

The proof.rs transition lemmas (5 lemmas) ARE proven cleanly without admit and are internally
consistent — but they only relate `spec_install_page`/`spec_map_page` to each other; they never
touch the exec bodies, which are admitted.

## Caller Coverage  (Covered ~5/8, Missing list)

From caller_analysis.md, the `identity_map_page` Ok-expectations:
- [Covered] V==P / page reachable → `accessible(phys_addr@)`.
- [Covered] Idempotence / no-op safe → `spec_install_page` uses `Set::insert` (idempotent).
- [Covered] Pre-init no-op → `!initialized` branch of `spec_map_page` + `accessible`.
- [Partial] Supervisor + read/write permissions → only documented via View comment, not an
  explicit predicate.
- [Missing] TLB consistency after fresh map → not expressed (invlpg empty contract).
- [Missing] No frame-allocator recursion (BSS pool only) → no requires/ensures models this
  structural guarantee; the View has no frame-allocator notion.
- [Covered] Err → not accessible / caller must not deref → `Err(_) => !accessible`.

`ensure_pt` Ok-expectations: page-alignment of `pt_paddr` and inv() are covered, but "PDE
present after the call" and "`pt_paddr` is the PT's frame address" are **not** expressed; the
Err arm is only inv()-preservation (weak). `ensure_pte` Ok/Err coverage of the V==P leaf step
is adequate.

**Crucially, coverage is moot:** every covered expectation is asserted only via an `admit()`-ed
body, so nothing is actually guaranteed to callers.

## Proof Completeness (admit count + locations, external_body count)

- **admit() in-scope: 3 — BLOCKER.**
  - `identity_map.rs:534` — `ensure_pt` body, `proof! { admit(); }`
  - `identity_map.rs:632` — `ensure_pte` body, `proof! { admit(); }`
  - `identity_map.rs:719` — `identity_map_page` body, `proof! { admit(); }`
  (Verus cheating-detail reports these as lines 533/627/718 — the attribute/fn line; same three.)
- external_body in-scope: **0** (the line-151 hit is a comment about invlpg in another module).
- proof.rs: 0 admit (lemmas fully proven).

## TCB Compliance

No `external_body` is declared in the in-scope files, so there is nothing requiring a
tcb-allowed.md entry here. The dependencies the bodies lean on (`Table::read`/`write`,
`invlpg`, `bump_allocator::alloc`/`alloc_as`) are external_body in *other* files and are all
already listed in tcb-allowed.md. The three in-scope functions are correctly NOT marked
external_body — but they substitute `admit()` instead, which is worse (zero proof, and not even
a recorded trust boundary). Compliant on the letter of the TCB list; defeated by admit().

## Guardrails Compliance (admit/assume/external_body/assume_specification/cfg-gated counts)

In-scope files only:
- `admit`: **3** (rs:534, rs:632, rs:719) — **BLOCKER (admit > 0)**.
- `assume` (proof statement): 0.
- `external_body`: 0.
- `assume_specification`: 2 — spec.rs:178 (`<[T]>::as_ptr`), spec.rs:182
  (`FixedSizeBumpAllocator::new`). Both are std/not-yet-verified-callee placeholders, not in
  tcb-allowed.md (that list governs external_body, not assume_specification). Not a hard blocker
  per the admit/assume rule, but they are unproven trusted contracts.
- `external_type_specification`: 1 — spec.rs:141-142 (`ExPageTableBss(PageTableBss)`).
- cfg-gated exec: 5 — `#[cfg(not(verus_keep_ghost))]` around `error!` logging on the error
  paths (rs:537, 553, 565, 635, 648). They strip logging from the verified build; benign in
  logic but technically exec hidden from verification.

## AST Consistency (PASS)

No `// VERUS REWRITE` comments exist in any in-scope file (grep returned none). No rewrite
pairs to compare → PASS by absence. (Minor note: the 5 `cfg(not(verus_keep_ghost))` logging
gates mean the verified text differs from production by removed `error!` calls — a logging
side-effect only, not a logic divergence.)

## Verification (FAIL)

Ground-truth `make verify-kernel`: exit 0 but **status = CHEATING_DETECTED**, crate-wide
`external_body=11 admit=31 ...`. cheating-detail.txt explicitly enumerates the three in-scope
admits (confirmed by direct grep). A run that is CHEATING_DETECTED is not a clean verification.
The in-scope functions contribute 3 of those admits. **FAIL.**

## Bug Summary

`bugs.md` does **not** exist (confirmed) — consistent with "no functional code bugs were logged."
I found no obvious functional defect in the exec logic of the three functions (the read→present
fast-path→allocate→install→invlpg flow is sound on inspection). **However**, because all three
bodies are `admit()`-ed, *none* of that logic is actually verified: the present-bit/idempotence,
the BSS-vs-frame-allocator separation, the page-alignment of `pt_paddr`, and the V==P leaf
install are all unproven assertions. The unresolved "verification failure" is precisely the
substitution of proof obligations with `admit()` rather than discharging them. So: no logged
code bug, but a wholesale unproven module.

## Issues (highest priority first)

1. **BLOCKER — 3 `admit()`s** at identity_map.rs:534/632/719 stub out the entire proof of all
   three in-scope functions. Verification is CHEATING_DETECTED.
2. **BLOCKER — verification is not clean** (CHEATING_DETECTED), directly caused by (1).
3. **Major — spec is vacuous against the implementation:** `identity_map_view()` is `uninterp`
   and `internal_inv() == true`, so the abstract `mapped`/`accessible` model is never tied to
   the real page tables; even if the admits were removed, the contracts encode little about
   actual page-table state.
4. **Minor — missing caller coverage:** no spec captures (a) no-frame-allocator-recursion,
   (b) TLB consistency, (c) explicit writable/supervisor permission; `ensure_pt` Ok omits
   "PDE present / pt_paddr is the PT frame address."
5. **Minor — unproven trusted contracts:** 2 `assume_specification` + 1
   `external_type_specification` in spec.rs (not blockers, but trusted and not on the TCB list).

## Result: FAIL
