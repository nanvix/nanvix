## Turn 1: Full checklist sweep — RESOLVED

### Progress
- Done (PASS): **all 17 checklist items** — verified independently this turn with
  fresh tool runs against the committed `specification prover END: vmem` state
  (`HEAD = d6db4ec1a`, working tree clean).
- Current: full-checklist verification (this turn).
- Remaining: none.

### Verification (commands I ran myself)

**`make verify-kernel`** → `Exit code : 0`, module `mm::virt::vmem` verified,
`cached (no recompilation)`. Guardrail line:
`cheating: assume=0 external_body=66 admit=0 trusted=0 no_decreases=0 cfg_gate=63`,
coverage `40/1069` crate-wide. (`status: CHEATING_DETECTED` is driven solely by the
crate-wide `external_body` count; see item 13 for the per-function disposition in
this module.)

**`./z build -- all`** → `[OK]  Build complete.` — the `#[cfg(verus_keep_ghost)]`
spec/proof includes are inert in the production build; no breakage.

**`fn_coverage.py /tmp/vmem_prespec.rs(=9e6b50c3c) src/.../vmem.rs`** →
Source exec fns 35, Verus exec fns 35, Matched 35, Missing 0, Extra 0.

**Cheating greps (vmem.rs + vmem.spec.rs + vmem.proof.rs only):**
`admit=0  assume(=0  assume_specification=0  trusted=0`,
`verus_verify(external_body)=35` (every in-scope exec fn), `no_unwind=1`,
`opens_invariants=1` (both on `Drop`).

**`spec_drift.py git-diff … --before a6761174d(first prover END) --after HEAD`** →
`Ensures removed: 0`, `Requires added: 32` (all of them `self.inv()`),
`Functions removed: 0`. No guarantee dropped.

I also read in full: `vmem.spec.rs` (442 L), every `#[verus_spec]` block in
`vmem.rs`, `caller_analysis.md`, `view_design.md`, `bugs.md`.

### Per-item disposition

1. **Coverage — PASS.** `fn_coverage.py` = 35/35 matched. 30 entry points carry full
   `requires`/`ensures`; `drop` carries `opens_invariants none` + `no_unwind` (no
   pre/post needed). The 4 uncontracted fns — `allocate_kernel_page_table` (L412),
   `allocate_user_page_table` (L436), `lookup_user_page_table` (L691),
   `lookup_kernel_page_table` (L739) — are representation-only helpers returning
   concrete `PageTable`/`&mut PageTable`/`Rc<RefCell<…>>` values that the View
   abstracts away (`internal_inv()`). Each carries the required deferral comment
   (verified at L408-410, L432-434, L687-689, L735-737). All their callers are
   themselves `external_body` this phase, so the missing contracts create **no**
   verification gap now; they are a proving-phase obligation. Accepted.

2. **Caller coverage — PASS.** Walked every entry in `caller_analysis.md` against its
   contract: `new`/`clone` (empty user half, kernel carried, fresh pgdir),
   `map`/`map_kpage` (frame-pinned insert via `spec_map`/`spec_map_kpage`),
   `unmap` (returns `old@.user[v].frame`, `spec_unmap`), CoW family
   (`mark`/`unmark`/`resolve_cow_at`/`resolve_cow_for_region` round-trip + idempotent
   `Ok(false)`), copy paths (user/kernel/physical-region error predicates),
   `kctrl`/`uctrl` (dry-run⇒commit, `spec_kctrl`/`spec_uctrl`). All match.

3. **View consistency / inv — PASS.** Contracts reference `VmemView` fields and the
   `spec_*` transitions; every mutator (`map`, `map_kpage`, `unmap`, `mark`/`unmark`,
   `resolve_cow_*`, `uctrl`, `kctrl`, `memset`, copy*) ensures `final(self).inv()` on
   `Ok`. `Vmem::inv == self@.inv() && internal_inv()` (spec.rs L430-439).

4. **No tautological ensures — PASS.** `Err(_) => true` appears only on constructors
   (`new` L121, `clone` L219) and non-mutating `&self` queries (`is_user_page_mapped`
   L545, `find_user_frame` L801, `try_find_user_frame` L855, `try_find_user_pte` L906,
   `for_each_user_mapping` L966) — no post-state to deny, and the `Ok` arm carries the
   real content in each. Examined individually; none vacuous on a mutator.

5. **No subsumed ensures — PASS.** `map` pins `spec_map(v, frame, perms)` (not an
   existential); `map_kpage` uses full `spec_map_kpage`; `unmap` pins the returned
   frame to `old@.user[v].frame`. No clause is derivable-and-redundant from `inv()`.

6. **Error paths meaningful — PASS.** Every mutator's `Err` arm asserts
   `final(self)@ == old(self)@` (`map` L460, `map_kpage` L321, `mark` L1031,
   `unmark` L1075, `replace` L1128, `resolve_cow_at` L1194, `resolve_cow_for_region`
   L1280, `unmap` L1886, `uctrl` L1981, `kctrl` L2068). The `&self`/`&mut self`
   content-only copy paths assert `final(self)@ == old(self)@` unconditionally.

7. **No assume_specification for internal code — PASS.** `assume_specification` grep =
   0 across all three files.

8. **vstd searched before assume_specification — PASS (vacuous).** No
   `assume_specification` exists.

9. **Specs caller-usable — PASS.** Written in `old(self)@` / `final(self)@` form over
   the public `VmemView`/`spec_*` vocabulary — directly usable in caller proofs.

10. **Trait obligations — PASS.** `impl Drop for Vmem` (L2150-2187) carries
    `opens_invariants none` + `no_unwind`; ownership-release semantics documented in
    `caller_analysis.md` (Trait Obligations) and `external_body`-deferred for proving.

11. **Spec completeness (advisory) — PASS.** The only intentional nondeterminism is
    `resolve_cow_at`'s existential fresh frame (L1182-1186, with
    `is_page_aligned(f) && spec_is_physical_region(f, page_size())` side-conditions),
    which matches the caller expectation ("don't care about frame allocation details").

12. **Loop invariants — PASS.** Every in-scope exec fn is `external_body`, so Verus
    does not check loop bodies (confirmed: loops in `clone`, `for_each_user_mapping`
    L975-999, `resolve_cow_for_region`, `drop` emit no missing-invariant error).
    `vmem.proof.rs` is empty and `vmem.spec.rs` contains no loops. No gap now;
    invariants are a proving-phase obligation.

13. **No cheating on own functions — PASS (accepted spec-phase boundary).**
    `admit=0  assume=0  trusted=0` in this module. The 35 `external_body` markers are
    the deliberate spec-phase trust boundary installed by the
    `specification strip-external-body: vmem (35)` tooling step — each carries either a
    real contract or (for the 4 helpers) a documented deferral. Bodies are proven in
    the *next* phase. No `assume_specification` on workspace-internal code.

14. **No specs weakened — PASS.** `spec_drift git-diff` first-prover-END→HEAD:
    `Ensures removed: 0`. The 32 `Requires added` are uniformly `self.inv()`
    well-formedness preconditions that every caller already maintains (the View's
    mirrored invariant) and that each `Ok` arm re-establishes — not a real weakening.

15. **Bug awareness — PASS.** `bugs.md` records BUG-001 (ghost `vmem_view` placeholder
    via `Ghost::assume_new()` — proving-phase fix), SB-1 (`kctrl` MMIO identity-map vs
    `spec_kctrl`/TYPE-5), SB-2 (`map`/`map_kpage` empty-page-table leak on late error),
    SB-3a/b (`copy_to_user_unaligned_unchecked` dry-run/commit asymmetry vs the
    `self@==old@` scaffold). All flagged for the proving phase with concrete plans.

16. **Cross-module regression — PASS.** `make verify-kernel` (whole `kernel` crate)
    exits 0; all modules report cached-pass, not just `mm::virt::vmem`.

17. **Verification — PASS.** `make verify-kernel` exit 0, 0 verification errors;
    `./z build -- all` `[OK]  Build complete.`

### Fix Request
None. Every checklist item is PASS with concrete tool/code evidence and the working
tree is clean. The spec-phase trust boundary (all-`external_body`, 4 documented helper
deferrals, recorded SB-/BUG- items) is the correct, intended terminal state for the
specification phase; the open items are explicitly scoped to the proving phase.

### Conclusion
RESOLVED. Creating `STOP` = `RESOLVED`. No rollback needed — the view-design held
throughout; all spec work is in-phase and internally consistent.
