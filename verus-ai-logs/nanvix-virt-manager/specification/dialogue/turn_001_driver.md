## Turn 1: Full checklist walkthrough — `mm::virt::manager` specification phase (independent strict re-review)

> Re-verified from scratch with tool output. Confirmed current state: **7**
> manager-own `external_body` fns (not 9 — `try_unmap_upage` and `ctrl_upage` are
> translated, non-`external_body` bodies). Every determination below is backed by
> commands I ran myself this turn.

### Progress
- Done (PASS with evidence): items 1–17 (ALL).
- Current: final verdict.
- Remaining: none.

**Verdict: RESOLVED** — every checklist item PASSes.

---

### Commands run this turn (evidence)
- `make verify-kernel` → exit 0; `note: verifying module mm::virt::manager`;
  `verification: cached (no recompilation)`. Cheating line (whole crate):
  `assume=0 external_body=64 admit=0 trusted=0 no_decreases=0`. Coverage 40/1069.
  (`status: CHEATING_DETECTED` is the script's whole-crate `external_body` heuristic,
  NOT a verification failure — exit code is 0 and the module verified.)
- `make` (full standalone build) → exit 0, "Standalone images built successfully."
- `fn_coverage.py manager.rs manager.rs` → 16 matched, 0 missing, 0 extra, 0 spec-only.
- `spec_drift.py git-diff manager.rs --before 7fedc70cc (caller-analysis START =
  spec-phase baseline) --after HEAD` → **Ensures removed: 0**; functions removed: 0;
  9 fns with requires/ensures *added* (net-new contract from a no-contract baseline).
- `grep -E 'assume_specification|admit|assume|trusted' manager.{rs,spec.rs,proof.rs}`
  → only `external_body` attrs + comments; zero `assume_specification`/`admit`/`assume`/`trusted`.
- Read in full: manager.rs (all contract blocks + bodies), manager.spec.rs,
  manager.proof.rs, caller_analysis.md, bugs.md, cheating-detail.txt.

---

### Item-by-item verification

**1. Every in-scope exec fn has requires/ensures — PASS.** The 9 in-scope fns
(`new_vmem`:245, `link_user_pages`:321, `try_resolve_cow_fault`:578,
`try_unmap_upage`:653, `alloc_upages`:699, `ctrl_upage`:865, `alloc_kpage`:901,
`alloc_kpages`:944, `load_elf`:1000) each carry a `#[verus_spec(ret => requires …
ensures …)]` — confirmed by reading each block. Out-of-scope fns (`init`, `get`,
`get_mut`, `new`, `link_one_user_page`, `rollback_linked_pages`,
`make_uninitialized_array`) carry none — matches `caller_analysis.md §Scope`.

**2. Caller coverage — PASS.** Cross-checked every `caller_analysis.md §Caller
Expectations` entry against the contract:
- `new_vmem`: Ok ⇒ `new.inv()`, `new@.kernel==vmem@.kernel`, `new@.user==empty`,
  `new@.pgdir!=vmem@.pgdir`; `vmem:&Vmem` immutable ⇒ untouched. ✔
- `link_user_pages`: requires `link_user_pages_pre`; Ok ⇒ `links_child_cow`+inv. The
  doc's "full rollback on Err" is intentionally **softened** to
  `final(parent).inv() && final(child).inv()` — recorded in `bugs.md` as a sound
  honest weakening (rollback is best-effort by design; fork discards child, keeps a
  valid parent). Not an over-promise. ✔
- `try_resolve_cow_fault`: Ok(false) disjunction encodes "wrong bits / non-user /
  not-CoW ⇒ Ok(false) not Err"; Ok(true) mirrors verified `Vmem::resolve_cow_at`. ✔
- `try_unmap_upage`: Ok(true)=mapped+`spec_unmap`; Ok(false)=`!user_mapped`,unchanged
  (idempotent). ✔
- `alloc_upages`/`alloc_kpages`: empty-buffer precondition, full rollback
  `final==old`, buffer drained on every path. ✔
- `ctrl_upage`: requires `user_mapped`; Ok ⇒ `spec_uctrl`. ✔
- `alloc_kpage`: Ok ⇒ aligned+physical page. ✔
- `load_elf`: domain growth (`subset_of`), kernel+pgdir preserved, user-addr
  entry/args. ✔

**3. View consistency — PASS.** Specs reference only `VmemView` fields (`.user`,
`.kernel`, `.pgdir`, `UserPageView.{frame,perms,cow}`) + inherited spec fns
(`user_mapped`, `spec_unmap`, `spec_resolve_cow`, `spec_uctrl`, `addr_nat`,
`perms_view`). Manager View is the documented unit marker
(`VirtMemoryManagerView`, `inv()==internal_inv()==true`). Every mutating Ok-arm
re-asserts `final(vmem).inv()`.

**4. No tautological ensures — PASS (challenged).** Exactly two `Err(_) => true`
arms: `new_vmem`:257 and `alloc_kpage`:910. I challenged both. `new_vmem` takes
`&self`(unit)+`vmem:&Vmem`(immutable) and returns an owned `Vmem` absent on Err;
`alloc_kpage` takes `&mut self`(unit) and returns an owned `KernelPage` absent on
Err. Neither has any view-modeled mutable state reachable on Err (the global frame
pool is deliberately outside any View). `true` is therefore **complete, not lazy** —
there is nothing in the model to constrain. Principled, not a violation. All other
Err arms are substantive (`final==old` or `inv()`).

**5. No subsumed ensures — PASS.** `final(vmem).inv()` is kept alongside the
state-equality clauses (not derivable from them without a per-transition inv lemma;
callers need it explicit). Disjoint Ok(true)/Ok(false) arms are mutually independent.

**6. Error paths meaningful — PASS.** `Ok=>…, Err=>…` match style throughout. Err
arms: `final==old` (`try_unmap_upage`,`try_resolve_cow_fault`,`ctrl_upage`),
`final==old ∧ inv ∧ buf drained` (`alloc_upages`), `len==0` (`alloc_kpages`),
`inv()` (`link_user_pages`,`load_elf`); only the two state-free fns use `true` (item 4).

**7. No assume_specification for workspace-internal code — PASS.** grep over all
three manager files → zero `assume_specification`.

**8. vstd searched before assume_specification — PASS (vacuous).** None present.

**9. Specs written for the caller — PASS.** Contracts are over
`old/final(...)@` snapshots + named composite predicates (`maps_user_run_with`,
`links_child_cow`, `link_user_pages_pre`, `spec_is_cow_write_fault`) re-asserting
`inv()`. Caller proofs (fork/exec/do_mmap/mctrl/munmap) usable without opening bodies.

**10. Trait obligations — PASS (none).** `caller_analysis.md §Trait Obligations`:
none; all in-scope fns are inherent `pub fn`. Only `View` is implemented; its
`view()`/`inv()` match module convention.

**11. Spec completeness (advisory) — PASS.** Intentional nondeterminism matching
callers: `alloc_kpage(s)` don't pin "cleared"/contiguity; `alloc_upages` new frames
existential; `try_resolve_cow_fault` new frame existential; `load_elf` exposes only
domain-growth + entry/args validity. All endorsed by `caller_analysis.md` "caller
doesn't care …".

**12. Loop invariants — PASS.** The two translated (non-external_body) fns
`try_unmap_upage` (`Ok(vmem.unmap(vaddr)?.is_some())`) and `ctrl_upage`
(`vmem.uctrl(...)`) contain no loops. The loops in
`link_user_pages`/`alloc_upages` live inside `external_body` bodies not submitted
to Verus. No outstanding `invariant` obligation.

**13. No cheating on module's own functions — PASS (full per-function report).**
admit=0, assume=0, trusted=0. `external_body` on module-own in-scope fns = **7**,
each challenged individually and found to be a genuine trusted boundary
(external unverified dependency or Verus front-end limitation), appropriate for the
spec phase:
- `new_vmem`(263): body uses `PhysMemoryManager::alloc_kernel_frame`, `KernelPage::new`,
  `Vmem::clone`. `Vmem::clone`'s contract does not yet expose pgdir-freshness/kernel-eq
  (per bugs.md), so the `Ok` clauses cannot be discharged today → trusted boundary.
- `link_user_pages`(346): body passes a closure to `for_each_user_mapping` capturing
  `count`/`buf`/`child` by `&mut` — Verus front-end forbids `&mut` closure capture.
  Genuine limitation (verus-unsupported.md).
- `try_resolve_cow_fault`(611): relates `error_code.is_present/write/user()` (unverified
  `arch::cpu::excp` const fns) to uninterp `ec_*` projections; uses `align_down`,
  `PageAligned::from_raw_value` (no contracts). Delegates to verified `Vmem::resolve_cow_at`.
- `alloc_upages`(726): `Vec::drain`/`Vec::capacity` + std iterators unmodeled by vstd; loops.
- `alloc_kpage`(915): `phys`/`kpage` deps unverified.
- `alloc_kpages`(965): `iter_mut().try_for_each` std combinator unmodeled.
- `load_elf`(1020): forwards wholly to unverified `elf::elf32_load` (external_body ≡
  assume_specification on that dep).
3 `external_type_specification` structs (`ExExcpErrorCode`,`ExKernelFrame`,`ExElf32Fhdr`)
wrap opaque external dep types — allowed. Corroboration: the completed predecessor
`mm::virt::vmem` itself ships 38+ own `external_body` fns with trusted contracts.

**14. No specs weakened — PASS.** `spec_drift git-diff` (before=7fedc70cc spec-phase
START): **Ensures removed: 0**, functions removed: 0. Flagged "requires added" are
net-new contract from a no-contract baseline, not weakenings — and each added
`requires` is caller-satisfiable per caller_analysis (`inv()` trivial/held;
empty-buffer scratch; `ctrl_upage`'s `user_mapped` — callers ELF-loader/`mctrl`
operate on mapped pages; `link_user_pages_pre` — fork passes a fresh non-overlapping
child). No vacuous narrowing.

**15. Bug awareness — PASS.** `bugs.md` records: (a) `link_user_pages`/
`rollback_linked_pages` cannot bit-for-bit restore parent CoW marks on Err
(provenance ambiguous; spec soundly weakens Err arm to `inv()` — not unsound);
(b) two fixed spec gaps (`ctrl_upage` precondition; `new_vmem` pgdir distinctness).
All consistent with current code/spec.

**16. Cross-module regression — PASS.** `make verify-kernel` verifies the whole
kernel crate → exit 0; no module reported failing (manager cached, others unaffected).

**17. Verification + build — PASS.** `make verify-kernel` → exit 0, 0 verification
errors. `make` → exit 0, "Standalone images built successfully."

---

### Fix Request
None. All 17 checklist items PASS with the tool/code evidence above. Writing
`STOP = RESOLVED`.
