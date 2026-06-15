## Turn 3: Re-review after fixer turn 2

### Progress
- FIXED since turn 2:
  - **FR-A (compile)** — all `&mut self` postconditions now use `final(self)@`/`old(self)@`. **Verus runs to completion, exit 0, 0 verification errors.**
  - **FR-B (coverage)** — 12 → **27/27 in-scope entry points** carry contracts (guardrail `30/1069`, the extra 3 are contracted helpers).
  - **FR-C (spec strength)** — `map` pins the frame (`old@.spec_map(v, uframe.addr_nat(), perms)`, no existential); `map_kpage` uses `old@.spec_map_kpage(v, kpage.addr_nat(), rdwr_perms())`; all five mutators ensure `final(self).inv()`. `unmap` returns `f.addr_nat() == old@.user[v].frame` + `spec_unmap`; `kctrl` captures dry-run⇒commit. Excellent.
  - **FR-D (admit)** — `admit=0`, `assume=0`. Both redundant `admit()`s removed.
- PASS now: items 3 (inv preservation), 5 (no weakening — specs strengthened), 7, 8 (no `assume_specification`), 13-partial (admit/assume gone), 14 (clean slate), 15/16/17 (Verus exit 0, all modules cached-pass).
- Still FAIL (this turn's focus): item 1/2/6 — **`for_each_user_mapping` has `requires` but no `ensures`** (empty postcondition for the most-used iterator).
- Residual (acceptable-deferred, but miscount to correct): item 13 — **5** bare `external_body` helpers (fixer claimed 4; missed `try_find_user_frame`).
- Current: `for_each_user_mapping` ensures gap.

---

### Verification performed

`make verify-kernel`:
```
note: verifying module mm::virt::vmem
=== Results ===  Exit code : 0   (cached, no recompilation)
cheating: assume=0 external_body=54 admit=0 trusted=0
coverage: 30/1069 exec functions have contracts
status: CHEATING_DETECTED   (external_body placeholders only)
```
Verus itself: **0 verification errors, exit 0** — the FR-A blocker is gone. `CHEATING_DETECTED` is solely the spec-phase `external_body` placeholder boundary that turn 2 explicitly accepted ("external_body + a real contract is the sanctioned placeholder").

Verified the new spec vocabulary exists in `vmem.spec.rs`: `addr_nat()` projections for the address/page/frame types (L121-149), `perms_view()` (L154-158), `rdwr_perms()` (L172), `phys_mem_size()` literal (L186-189, with a documented note that the generated `config::kernel::MEMORY_SIZE` const is not reachable from spec context).

Read every in-scope contract (scanned all 29 `#[verus_spec]` blocks). Findings:

- **Strong, caller-faithful Ok+Err specs:** `map_kpage`, `map`, `mark_user_page_cow`, `unmark_user_page_cow`, `resolve_cow_at`, `resolve_cow_for_region`, `user_vaddr_to_paddr`, `copy_from_user_unaligned`, `copy_to_user_unaligned_unchecked`, `copy_to_user_unaligned`, `copy_user_to_user`, `memset`, `unmap`, `uctrl`, `kctrl`, `pgdir`, `is_user_addr/region`, `is_kernel_addr/region`, `is_physical_region`. Verified against `caller_analysis.md` rows — they match.
- **`Err(_) => true`** appears in `new`, `clone`, `is_user_page_mapped`, `find_user_frame`, `try_find_user_pte`. These are all **constructors or `&self` queries** — there is no `old(self)` side-effect to deny, and each carries the real content in its `Ok` arm. **Accepted** (not the kind of tautology the checklist targets, which is a mutator hiding a state change).
- **`load`** (`&self`, L269): `requires self.inv()`, no `ensures`. The View design *deliberately* puts load's only effect (CR3 := pgdir) out of scope (Rejected Alternative #2), and `&self` makes `self@ == old@` automatic, so there is genuinely no in-View postcondition. **Accepted.**
- **`for_each_user_mapping`** (`&self`, L921): `requires self.inv()`, **no `ensures` at all** — empty postcondition. ❌ This is the most-used iterator (`caller_analysis.md`: `link_user_pages`, `rollback_linked_pages` rely on **complete coverage of present user mappings**; `view_design.md` sketch: "invokes `f` exactly on `self@.user.dom()`"). With no ensures the contract gives callers nothing to reason with.

Bare `external_body` helpers (no contract) — challenged individually:
1. `allocate_kernel_page_table` (L399) — returns concrete `PageTable<PageTableStorage>`; representation the View abstracts (`internal_inv()==true`). Deferred.
2. `allocate_user_page_table` (L420) — same. Deferred.
3. `lookup_user_page_table` (L672) — returns `&mut PageTable` representation. Deferred.
4. `lookup_kernel_page_table` (L717) — same. Deferred.
5. `try_find_user_frame` (L824) — **fixer's report claimed only 4 bare helpers and did not list this one.** Returns `Option<FrameAddress>`; *does* have an abstract counterpart (`self@.user_mapped(v)` / `self@.user[v].frame`), so unlike the other four it is contractible now.

---

### Assessment

The specification is close to complete and high quality. Two items block `RESOLVED`:

**A. `for_each_user_mapping` — missing ensures (item 1/2/6): FAIL.**
**B. `try_find_user_frame` — bare `external_body` despite having an expressible abstract postcondition, and miscounted in the fixer report (item 13): FAIL.**

The other four bare helpers (`allocate_*`, `lookup_*`) return pure internal representation that the View abstracts away (`internal_inv()==true`); they have no abstract postcondition until the representation is modeled in the **proving** phase. **Accepted as deferred** — but they must stay `external_body` *with* that justification, not be forgotten.

---

### Fix Request

**FR-E (item 1/2/6). Add a meaningful `ensures` to `for_each_user_mapping` (L916-919).**
At minimum it must express the caller-relevant facts from `caller_analysis.md`/`view_design.md`:
- read-only over the View in a *non-subsumed* way, and
- the coverage property: `f` is invoked exactly on the present user pages `self@.user.dom()`, each key a page-aligned user address, short-circuiting on the first `Err`.
Use the Verus `FnMut` closure-spec machinery (`requires`/`ensures` on the `F` bound, e.g. constrain `f`'s precondition to `self@.user_mapped(v) && spec_is_user_addr(v) && is_page_aligned(v)`). If — and only if — you determine the full callback-coverage spec genuinely belongs to the proving phase, you must (a) still add a non-trivial `ensures` capturing what *is* expressible now, and (b) prove that claim with tool output, not prose. Justification alone is not a fix.
Verify: `grep -A6 'fn for_each_user_mapping' shows an `ensures`; `make verify-kernel` still exits 0.

**FR-F (item 13). Contract `try_find_user_frame` (L824) and correct the bare-helper accounting.**
`try_find_user_frame` returns `Result<Option<FrameAddress>, Error>` and has an abstract counterpart — give it a real `ensures` (e.g. `Ok(Some(fr)) ==> self@.user_mapped(vaddr.addr_nat()) && fr.addr_nat() == self@.user[vaddr.addr_nat()].frame`; `Ok(None) ==> !self@.user_mapped(vaddr.addr_nat())`). Then, for the remaining four genuinely-representation-only helpers (`allocate_kernel_page_table`, `allocate_user_page_table`, `lookup_user_page_table`, `lookup_kernel_page_table`), leave a one-line comment on each stating they are bare `external_body` deferred to the proving phase because the value they return is internal representation abstracted by `internal_inv()`.
Verify: re-run the classification; only those four helpers remain bare `external_body`, each with the deferral comment; `make verify-kernel` exits 0.

---

### STOP / ROLLBACK
Not creating `STOP` — items 1/2/6 (`for_each_user_mapping`) and 13 (`try_find_user_frame`) are not yet PASS. Not requesting `ROLLBACK`: the View design is sound and every remaining fix is in-phase (add two ensures clauses). Once FR-E and FR-F land with Verus still at exit 0, the specification phase is `RESOLVED`.
