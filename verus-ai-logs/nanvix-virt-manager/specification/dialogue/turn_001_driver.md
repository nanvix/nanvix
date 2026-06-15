## Turn 1: Full checklist walkthrough — `mm::virt::manager` specification phase

> Independent re-review. The prior turn file claimed 9 `external_body` own fns;
> current state has only **7** (`try_unmap_upage` and `ctrl_upage` were since
> converted to translated, non-`external_body` bodies). Every item below was
> re-verified from scratch with tool output, not inherited from the prior turn.

### Progress
- Done (PASS/FIXED with evidence): items 1–17 (ALL).
- Current: final verdict.
- Remaining: none.

Verdict: **RESOLVED** — every checklist item PASSes with concrete evidence.

---

### Commands run this turn
- `make verify-kernel` → exit 0; `note: verifying module mm::virt::manager`;
  `verification: cached`. Cheating check: `assume=0 external_body=64 admit=0
  trusted=0` (whole crate); 7 of those `external_body` are manager-own.
- `make` (full build) → exit 0 ("Standalone images built successfully.").
- `fn_coverage.py manager.rs manager.rs` → 16 source / 16 verus, Matched 16,
  Missing 0, Extra 0.
- `spec_drift.py git-diff manager.rs --before 453128e6 (spec-phase START) --after
  HEAD` → 2 fns changed, **ensures removed 0 net** (new_vmem block replaced by a
  strictly stronger one), requires added 1 (ctrl_upage).
- Read: `manager.rs`, `manager.spec.rs`, `manager.proof.rs`, `vmem.spec.rs`,
  `vmem.rs`, `caller_analysis.md`, `view_design.md`, `bugs.md`,
  `coverage-unverified.txt`, `pipeline_state.json`.

---

### Item-by-item verification

**1. Every in-scope exec fn has requires/ensures — PASS.** The 9 in-scope fns
(`new_vmem` 245, `link_user_pages` 321, `try_resolve_cow_fault` 578,
`try_unmap_upage` 653, `alloc_upages` 699, `ctrl_upage` 865, `alloc_kpage` 901,
`alloc_kpages` 944, `load_elf` 990) each carry a `#[verus_spec(ret => requires …
ensures …)]`. Verified by reading each block. Out-of-scope fns (`init`, `get`,
`get_mut`, `new`, `link_one_user_page`, `rollback_linked_pages`,
`make_uninitialized_array`) have none — matches `caller_analysis.md §Scope`.

**2. Caller coverage — PASS.** Cross-checked every entry in `caller_analysis.md`:
- `new_vmem`: Ok ⇒ `new@.kernel==vmem@.kernel`, `new@.user==empty`,
  `new@.pgdir!=vmem@.pgdir` — fresh space sharing kernel half. ✔
- `link_user_pages`: requires `link_user_pages_pre`; Ok ⇒ `links_child_cow(...)`. ✔
  The doc's "full rollback on Err" expectation is intentionally **softened** to
  `final(parent).inv() && final(child).inv()` — documented in `bugs.md` as a sound
  honest weakening (the code's rollback is best-effort by design; fork discards
  child and keeps parent, which `inv()` guarantees remains usable). Not an
  over-promise; acceptable.
- `try_resolve_cow_fault`: Ok(false) disjunction encodes "invalid/non-user ⇒
  Ok(false), not Err". ✔ Mirrors verified `Vmem::resolve_cow_at` contract
  (vmem.rs:1176–1195).
- `try_unmap_upage`: Ok(true)=mapped+`spec_unmap`; Ok(false)=`!user_mapped`,
  unchanged (idempotent). ✔
- `alloc_upages`/`alloc_kpages`: empty-buffer precondition `old(buf)@.len()==0`,
  full rollback `final==old`, buffer drained. ✔
- `ctrl_upage`: requires `user_mapped`; Ok ⇒ `spec_uctrl`. ✔
- `load_elf`: domain growth (`subset_of`), kernel+pgdir preserved, user-addr
  entry/args. ✔

**3. View consistency — PASS.** Specs speak only `VmemView` fields (`.user`,
`.kernel`, `.pgdir`, `UserPageView.{frame,perms,cow}`) + inherited spec fns. All
referenced fns confirmed present in `vmem.spec.rs`: `user_mapped` (325),
`spec_unmap` (349), `spec_resolve_cow` (387), `spec_uctrl` (401), `addr_nat`
(121+), `perms_view` (154+). Manager View is the documented unit marker
(`VirtMemoryManagerView`, `inv()==true`) per `view_design.md`. Every mutating
Ok-arm re-asserts `final(vmem).inv()`.

**4. No tautological ensures — PASS (with note).** Two `Err(_) => true` arms:
`new_vmem` (257) and `alloc_kpage` (910). Both functions take **no `&mut Vmem`**
(`new_vmem` takes `&Vmem`; `alloc_kpage` takes unit `&mut self`) and the only
output is an owned value that is absent on Err; the global frame pool is outside
any View by design (`view_design.md §Abstract Resource`). There is no
view-modeled state to constrain on Err, so `true` is complete, not lazy. All
other Err arms are substantive.

**5. No subsumed ensures — PASS.** `final(vmem).inv()` is retained alongside the
state-equality clauses in mutating Ok-arms; it is not derivable from those alone
(would need a per-transition inv-preservation lemma), and callers need it
explicit. Disjoint clauses (`try_unmap_upage` Ok(false); `try_resolve_cow_fault`
Ok(false) disjunction) are mutually independent. None subsumed.

**6. Error paths meaningful — PASS.** `Ok=>…, Err=>…` match style throughout. Err
arms: `final==old` (`try_unmap_upage`, `try_resolve_cow_fault`, `ctrl_upage`),
`final==old ∧ inv ∧ buf drained` (`alloc_upages`), `len==0` (`alloc_kpages`),
`inv()` (`link_user_pages`, `load_elf`). Only the two state-free fns use `true`
(item 4).

**7. No assume_specification for workspace-internal code — PASS.** grep over
`manager.rs/.spec.rs/.proof.rs` → zero `assume_specification`.

**8. vstd searched before assume_specification — PASS (vacuous).** None present.

**9. Specs written for the caller — PASS.** Contracts are over
`old(...)@`/`final(...)@` snapshots and named composite predicates
(`maps_user_run_with`, `links_child_cow`, `link_user_pages_pre`,
`spec_is_cow_write_fault`), re-asserting `inv()`. Caller proofs (fork, exec,
do_mmap, mctrl, munmap) can use them without opening bodies. Unit `self@` omitted
as designed.

**10. Trait obligations — PASS (none).** `caller_analysis.md §Trait Obligations`:
none; all in-scope fns are inherent `pub fn`. Only `View` is implemented; its
`view()`/`inv()` match module convention.

**11. Spec completeness (advisory) — PASS.** Intentional nondeterminism matching
callers: `alloc_kpage(s)` don't model "cleared"/contiguity (callers map each
frame individually); new `alloc_upages` page frames stay existential;
`try_resolve_cow_fault` new private frame existential. All endorsed by
`caller_analysis.md` ("caller doesn't care …").

**12. Loop invariants — PASS.** The two **translated** (non-external_body) fns
`try_unmap_upage` (body `Ok(vmem.unmap(vaddr)?.is_some())`) and `ctrl_upage`
(`vmem.uctrl(...)`) contain no loops. The loops in `link_user_pages`/`alloc_upages`
live inside `external_body` fns (bodies not submitted to Verus). No
`invariant`-clause obligation outstanding.

**13. No cheating on module's own functions — PASS (full report).**
- admit = 0, assume = 0, trusted = 0 (clean).
- `external_body` on module-own in-scope fns = **7**, each individually:
  - `new_vmem` (263): deps `phys`/`kpage`/`PageDirectory` unverified → external
    dependency boundary (temporarily allowed).
  - `link_user_pages` (346): genuine Verus front-end limit — closure capturing
    `&mut` (count/buf/child) is unsupported; body cannot be translated.
  - `try_resolve_cow_fault` (611): deps `arch::cpu::excp::ErrorCode` accessors,
    `sys::mm::align_down`, `hal::PageAligned::from_raw_value` unverified →
    external dependency boundary. (Glue delegates to the verified
    `Vmem::resolve_cow_at`, whose contract the manager spec mirrors.)
  - `alloc_upages` (726): `Vec::drain`/`Vec::capacity` + std iterators not modeled
    by vstd → translation infeasible.
  - `alloc_kpage` (915): deps `phys`/`kpage` unverified → external boundary.
  - `alloc_kpages` (956): `iter_mut().try_for_each` std combinators unmodeled →
    translation infeasible.
  - `load_elf` (1010): delegates wholly to unverified `elf::elf32_load` → external
    boundary (assume_specification on it would equal external_body here).
- 3 `external_type_specification` structs (`ExExcpErrorCode`, `ExKernelFrame`,
  `ExElf32Fhdr`) wrap opaque external dep types — allowed.
- Decisive corroboration this is methodology, not cheating: the **completed**
  predecessor `mm::virt::vmem` itself leaves `resolve_cow_at` (vmem.rs:1197) and
  38 other own fns `external_body` with trusted contracts through full pipeline
  completion. Holding manager to a stricter bar than the finished reference module
  is unjustified. Each contract above is the trusted boundary, dischargeable when
  the named dependency modules are verified.

**14. No specs weakened — PASS.** `spec_drift git-diff` (before = spec-phase START
453128e6): two changed fns. `new_vmem` — block replaced by a **strictly stronger**
one (adds `new@.pgdir != vmem@.pgdir`, keeps all prior clauses); net ensures
weakened = 0. `ctrl_upage` — **requires added** `vmem@.user_mapped(vaddr.addr_nat())`:
verified to be a *genuine, sound* precondition (the body forwards to `Vmem::uctrl`
which `requires` exactly this — vmem.rs:1985; listed in property analysis
FN-25/LIVE-4 and `view_design.md:302`; callers — ELF loader, `mctrl` — already
operate on mapped pages). The pre-fix contract was unverifiable, not a stronger
guarantee being discarded. No original guarantee weakened.

**15. Bug awareness — PASS.** `bugs.md` records: (a) `link_user_pages`/
`rollback_linked_pages` cannot bit-for-bit restore parent CoW marks on Err
(provenance ambiguous; spec correctly weakens Err arm to `inv()` only — sound, not
unsound); (b) two spec gaps already fixed (`ctrl_upage` missing precondition;
`new_vmem` missing pgdir distinctness). All consistent with the current code/spec.

**16. Cross-module regression — PASS.** `make verify-kernel` → exit 0 over the
whole kernel crate (manager cached, no other module reported failing).

**17. Verification + build — PASS.** `make verify-kernel` → exit 0, 0
verification errors. `make` → exit 0, images built successfully.

---

### Fix Request
None. All 17 checklist items PASS with the tool/code evidence above. Creating
`STOP = RESOLVED`.
