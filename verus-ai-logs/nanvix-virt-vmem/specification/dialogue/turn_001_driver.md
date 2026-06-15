## Turn 1: Full checklist sweep (independent re-verification) — RESOLVED

### Progress
- Done (PASS/accepted): **all 17 checklist items**, each re-verified this turn with
  fresh tool runs against the current working tree (`HEAD = 3437511d9`).
- Current: full-checklist verification (this turn).
- Remaining: none.

### Verification (commands I ran myself)

- **`make verify-kernel`** → `Exit code : 0`; module `mm::virt::vmem` verified
  (`cached (no recompilation)`). Guardrail line:
  `cheating: assume=0 external_body=64 admit=0 trusted=0 no_decreases=0 cfg_gate=61`,
  coverage `40/1069` crate-wide. `status: CHEATING_DETECTED` is driven solely by the
  crate-wide `external_body` count — see item 13 for the per-function disposition.
- **`./z build -- all`** → `[OK]  Build complete.` (non-Verus dual build; the
  `#[cfg(verus_keep_ghost)]` spec/proof includes are inert in production).
- **`fn_coverage.py /tmp/vmem_prespec.rs(=9e6b50c3c) src/.../vmem.rs`** →
  Source exec fns 35, Verus exec fns 35, **Matched 35, Missing 0, Extra 0**.
- **`spec_drift.py git-diff vmem.rs --before a6761174d --after HEAD`** →
  **Ensures removed: 0**, Requires added: 32 (all `self.inv()`), Functions removed: 0,
  Functions added: 2 (`is_kernel_addr`/`is_kernel_region` gained contracts).
- **Cheating greps (the three module files only):**
  `admit=0  assume(=0  assume_specification=0  trusted=0`;
  `verus_verify(external_body)=35` (every in-scope exec fn); `external_body` in
  `vmem.spec.rs = 20` (all `external_type_specification` on external-dependency
  types); two `Ghost::assume_new()` (`new` L206, `clone` L270 — BUG-001).
- Read in full: `vmem.spec.rs` (458 L), every `#[verus_spec]` block in `vmem.rs`,
  `vmem.proof.rs` (empty), `caller_analysis.md`, `view_design.md`, `bugs.md`,
  `cheating-detail.txt`.

### Per-item disposition

1. **Coverage — PASS.** `fn_coverage` = 35/35 matched. 30 entry points + 1 helper
   (`try_find_user_frame`) carry full `requires`/`ensures` (31 contracted); `drop`
   carries `opens_invariants none` + `no_unwind`. The 4 uncontracted fns —
   `allocate_kernel_page_table` (L412), `allocate_user_page_table` (L436),
   `lookup_user_page_table` (L691), `lookup_kernel_page_table` (L739) — are
   representation-only helpers returning concrete `PageTable`/`&mut`/`Rc<RefCell<…>>`
   values abstracted away by `internal_inv()`; each carries the required deferral
   comment (verified at L408-411, L432-435, L687-690, L735-738). All callers are
   themselves `external_body` this phase → no verification gap; proving-phase
   obligation. Accepted.

2. **Caller coverage — PASS.** Walked every entry in `caller_analysis.md` against its
   contract: `new`/`clone` (empty user half, `v@.kernel == from@.kernel`, fresh pgdir);
   `map`/`map_kpage` (frame-pinned `spec_map`/`spec_map_kpage`, BadAddress⇒user-addr
   postcond); `unmap` (returns `old@.user[v].frame`, `spec_unmap`, `Ok(None)` on
   absent); CoW family (`mark`/`unmark`/`replace`/`resolve_cow_at`/`resolve_cow_for_region`
   round-trip + idempotent `Ok(false)`); copy paths (user/kernel/physical region
   predicates + `region_cow_resolved` on the kernel→user write paths); `uctrl`/`kctrl`
   (dry-run⇒commit; `spec_kctrl` vs `spec_kctrl_create` for present/absent). All match.

3. **View consistency / inv — PASS.** `view_design.md`'s `VmemView`
   (`user`/`kernel`/`pgdir`, `UserPageView`, `KernelPageView`, `PagePerms`) matches
   `vmem.spec.rs` exactly. Contracts reference these fields + the `spec_*` transitions;
   every mutator ensures `final(self).inv()` on `Ok`. `Vmem::inv == self@.inv() &&
   internal_inv()` (spec.rs L446-455).

4. **No tautological ensures — PASS.** `Err(_) => true` appears only on constructors
   (`new` L121, `clone` L219) and non-mutating `&self` queries (`is_user_page_mapped`
   L545, `find_user_frame` L801, `try_find_user_frame` L855, `try_find_user_pte` L906,
   `for_each_user_mapping` L966) — no post-state to deny, and each `Ok` arm carries the
   real content. Examined individually; none vacuous on a mutator.

5. **No subsumed ensures — PASS.** `map` pins `spec_map(v, frame, perms)` (frame not
   existential); `map_kpage` uses full `spec_map_kpage`; `unmap`/`replace`/`resolve_cow_at`
   pin the returned/old frame to `old@.user[v].frame`. No clause is derivable-and-
   redundant from `inv()`.

6. **Error paths meaningful — PASS.** Every mutator's `Err` arm asserts
   `final(self)@ == old(self)@` (`map_kpage` L321, `map` L460, `mark` L1031,
   `unmark` L1075, `replace` L1128, `resolve_cow_at` L1194, `resolve_cow_for_region`
   L1280, `copy_to_user*` L1521/L1693, `memset` L1854 (unconditional), `unmap` L1903,
   `uctrl` L1998, `kctrl` L2092). `&self` content-copy paths constrain regions on `Ok`.

7. **No assume_specification for internal code — PASS.** grep = 0 across all 3 files.

8. **vstd searched before assume_specification — PASS (vacuous).** No
   `assume_specification` exists.

9. **Specs caller-usable — PASS.** Written in `old(self)@` / `final(self)@` form over
   the public `VmemView`/`spec_*` vocabulary — directly usable in caller proofs.

10. **Trait obligations — PASS.** `impl Drop for Vmem` (L2176-2181) carries
    `opens_invariants none` + `no_unwind`; ownership-release semantics documented in
    `caller_analysis.md` (Trait Obligations), `external_body`-deferred for proving.

11. **Spec completeness (advisory) — PASS.** The only intentional nondeterminism is
    `resolve_cow_at`'s existential fresh frame (L1182-1186, gated by
    `is_page_aligned(f) && spec_is_physical_region(f, page_size())`), matching the
    caller expectation ("don't care about frame-allocation details").

12. **Loop invariants — PASS (deferred-correct).** Every in-scope exec fn is
    `external_body`, so Verus does not translate/check loop bodies (loops in `clone`,
    `for_each_user_mapping`, `resolve_cow_for_region`, `find_user_frame`,
    `try_find_user_frame`, `drop`). `make verify-kernel` exits 0 with no
    missing-invariant error; `vmem.proof.rs` is empty and `vmem.spec.rs` has no loops.
    Loop invariants are a proving-phase obligation once bodies are translated. No gap now.

13. **No cheating on own functions — PASS (accepted spec-phase boundary).** Module-local
    greps: `admit=0  assume=0  trusted=0  assume_specification=0`. The 35
    `verus_verify(external_body)` markers are the deliberate spec-phase trust boundary
    installed by the `specification strip-external-body: vmem (35)` tooling step — each
    fn carries a real contract (or, for the 4 representation helpers, a documented
    deferral). The 20 `external_body` in `vmem.spec.rs` are all
    `external_type_specification` on external-dependency types (`sys`/`hal`/`mm::phys`/
    `core`/`alloc`) — explicitly allowed. Two `Ghost::assume_new()` placeholders
    (`new`/`clone`) are recorded as BUG-001 (ghost-field compile workaround) for the
    proving phase. Bodies are proven in the next phase. No cheating beyond the
    intended/recorded boundary.

14. **No specs weakened — PASS.** `spec_drift git-diff a6761174d→HEAD`:
    **Ensures removed: 0**, Functions removed: 0. The 32 `Requires added` are uniformly
    `self.inv()` well-formedness preconditions that every caller already maintains (the
    View's mirrored invariant) and that each `Ok` arm re-establishes — not a real
    weakening. The 2 added functions (`is_kernel_addr`/`is_kernel_region`) gained
    contracts (strengthening).

15. **Bug awareness — PASS.** `bugs.md` records BUG-001 (ghost `vmem_view` placeholder
    via `Ghost::assume_new()` — proving-phase fix), SB-1 (`kctrl` MMIO identity-map vs
    `spec_kctrl`; FIXED this phase via `spec_kctrl_create` + relaxed TYPE-5), SB-2
    (`map`/`map_kpage` empty-page-table leak on late error — recorded), SB-3a
    (`copy_to_user_unaligned_unchecked` dry-run does not validate destination —
    recorded), SB-3b (commit mutates View — FIXED via `region_cow_resolved` ensures).
    All flagged with concrete proving-phase plans.

16. **Cross-module regression — PASS.** `make verify-kernel` (whole `kernel` crate)
    exits 0; all modules cached-pass, not just `mm::virt::vmem`.

17. **Verification — PASS.** `make verify-kernel` exit 0, 0 verification errors;
    `./z build -- all` `[OK]  Build complete.`

### Fix Request
None. Every checklist item is PASS (or an accepted, documented spec-phase boundary)
with concrete tool/code evidence. The terminal spec-phase state — all-`external_body`
own functions, 4 documented helper deferrals, `external_type_specification` on external
deps, `Ghost::assume_new()` recorded as BUG-001, SB-1/2/3 recorded — is the correct,
intended end of the specification phase; the open items are explicitly scoped to the
proving phase.

### Conclusion
RESOLVED. Creating `STOP` = `RESOLVED`. Independent re-verification confirms the prior
turns' disposition with fresh tool output; no rollback needed — the View design held
throughout and all spec work is in-phase and internally consistent.
