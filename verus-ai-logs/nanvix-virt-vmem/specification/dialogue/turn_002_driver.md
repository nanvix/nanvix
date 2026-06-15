## Turn 2: Re-review after fixer turn 1

### Progress
- FIXED since turn 1: FR-1 partial — the `pgdir: 0` E0308 is gone (now `Ghost::assume_new()`); Verus gets *further* but still does not run. FR-5 recorded in `bugs.md` (BUG-001). Spec coverage raised 2 → 12.
- Still FAIL (blocking): item 15 (build — 5 compile errors), item 16 (cross-module), item 17 (verify). Verus did **not** run (`0 verified`, exit 101).
- Still FAIL (content): item 1 (coverage 12/25), item 2 (caller coverage), item 4/6 (tautological/weak `Err`), item 5 (subsumed/weaker-than-design), item 12 (loop invariants), item 13 (cheating: 43 external_body + 2 admit).
- PASS/vacuous: item 7, 8 (no `assume_specification`), item 14 (clean slate).
- Not yet assessable until compile: item 3 (inv preservation), 10, 11.
- Current: blocking compile failure (FR-A) is top priority.

---

### Verification performed

`make verify-kernel` → **VERIFY_FAILED**, exit 101, Verus did not run. Five `rustc`/Verus errors, all the same class:
```
error: to dereference a mutable reference parameter in a postcondition,
       disambiguate by wrapping it in either `old` or `final`
  --> vmem.rs:299  (map_kpage)
  --> vmem.rs:432  (map)
  --> vmem.rs:946  (mark_user_page_cow)
  --> vmem.rs:987  (unmark_user_page_cow)
  --> vmem.rs:1078 (resolve_cow_at)
```
Guardrails: `coverage: 12/1069`; `cheating: external_body=43 admit=2 trusted=0`.
`cheating-detail.txt` still lists `new:124 admit`/`clone:212 admit` **and** both are now `external_body` too.

Confirmed the repo convention in already-verified code (`src/libs/bitmap/src/lib.rs` L250-262): post-state is written `final(self)@`, pre-state `old(self)@`. Bare `self@` in a `&mut self` `ensures` is rejected by this Verus version.

Read all 12 contracts. The 12 specced entry points are: `new`, `clone`, `map_kpage`, `map`, `is_user_page_mapped`, `is_user_addr`, `is_user_region`, `is_physical_region`, `try_find_user_pte`, `mark_user_page_cow`, `unmark_user_page_cow`, `resolve_cow_at`.

---

### Per-item findings

**Item 15/16/17 — Build / cross-module / verify: FAIL.** 5 compile errors (above); `0 verified`. Because the whole `kernel` crate fails `cargo check`, all previously-verified modules are also blocked.

**Item 1 — Coverage: FAIL (12/25).** 13 in-scope entry points still have **no** contract:
`load`, `pgdir`, `for_each_user_mapping`, `resolve_cow_for_region`, `user_vaddr_to_paddr`, `copy_from_user_unaligned`, `copy_to_user_unaligned_unchecked`, `copy_to_user_unaligned`, `copy_user_to_user`, `memset`, `unmap`, `uctrl`, `kctrl`.

**Item 2 — Caller coverage: FAIL.** The 13 missing functions carry caller obligations from `caller_analysis.md` (e.g. `unmap` returned frame `== old@.user[v].frame` + `self@ == old@.spec_unmap(v)`; `kctrl`/`copy_to_user_unaligned_unchecked` dry-run⇒commit; `resolve_cow_for_region` post `region_cow_resolved`). Unmet.

**Item 3 — View consistency / inv preservation: FAIL.** None of the five mutators (`map_kpage`, `map`, `mark_user_page_cow`, `unmark_user_page_cow`, `resolve_cow_at`) re-establish `final(self).inv()` in their `Ok` ensures. `inv()` is required as a precondition but never re-promised, so a caller cannot chain two mutating calls. `new`/`clone` correctly ensure `v.inv()`; mutators must ensure `final(self).inv()`.

**Item 4 / 6 — Tautological / weak Err ensures: FAIL.**
- `is_user_page_mapped` (L521) and `try_find_user_pte` (L843) use `Err(_) => true`. For these read-only (`&self`) queries the Ok arm is fine, but the functions never state `self@ == old@`; minor.
- More importantly, several Ok arms are weaker than the View design (see item 5).

**Item 5 — Subsumed / weaker-than-design ensures: FAIL (two concrete weakenings).**
1. **`map` (L433):** `exists|f: nat| self@ == old(self)@.spec_map(v, f, perms)`. The existential over the frame is **weaker** than `view_design.md` (`self@ == old@.spec_map(v, frame, perms)` with the *actual* mapped frame). A caller cannot learn *which* frame backs `v`; `user_vaddr_to_paddr`/`unmap` round-trips that depend on the frame identity are unprovable. Pin the frame (project `uframe`'s physical address to `nat`) instead of existential.
2. **`map_kpage` (L298-302):** Ok arm only asserts `kernel_mapped(v)` + `user`/`pgdir` unchanged. `view_design.md` specifies `self@ == old@.spec_map_kpage(v, frame, perms)`. The current spec does not pin the kernel frame or perms and does not constrain *other* kernel keys, so it is strictly weaker than the designed transition.

**Item 12 — Loop invariants: FAIL (deferred-by-external_body).** Every in-scope function is `external_body`, so Verus does not check loop bodies and emits no "missing invariant" error — but that is *because* the bodies are unverified, not because invariants exist. The four loops in `new`/`clone` and all others still lack `invariant` clauses; this resurfaces the moment `external_body` is removed in the proving phase.

**Item 13 — No cheating on module's own functions: FAIL (worse than turn 1).** `external_body` rose 29 → **43**; every in-scope function is now `external_body`, i.e. all 12 contracts are *trusted, not proven*. Individually flagged module-owned offenders:
- `new` (L124) — `external_body` **and** redundant `admit()` (L190). Remove the `admit()` (dead under external_body).
- `clone` (L213) — `external_body` **and** redundant `admit()` (L255). Remove the `admit()`.
- `map_kpage` (L306), `map` (L439), `is_user_page_mapped` (L524), `is_user_addr` (L539), `is_user_region` (L562), `is_physical_region` (L628), `try_find_user_pte` (L846), `mark_user_page_cow` (L950), `unmark_user_page_cow` (L991), `resolve_cow_at` (L1088) — all `external_body`.
- Still-uncontracted private helpers from turn 1 remain bare `external_body` with **no** `ensures`: `is_kernel_addr` (L577), `is_kernel_region` (L596), `allocate_kernel_page_table` (L390), `allocate_user_page_table` (L411), `lookup_user_page_table` (L596 region), `lookup_kernel_page_table`, `find_user_frame`, `replace_user_page_cow_frame`.

I acknowledge the spec phase cannot *prove* bodies, so `external_body`+contract is an acceptable *temporary* placeholder for the proving phase — **but** (a) the two `admit()`s are pure cheating with no purpose under `external_body` and must be deleted now, and (b) every `external_body` helper that has no `ensures` is an unconstrained trust hole: give it a real `ensures` (e.g. `is_kernel_addr` → `ensures ret == spec_is_kernel_addr(virt_addr.addr_nat())`, `is_kernel_region` → `ensures ret == spec_is_kernel_region(start.addr_nat(), size as nat)`) so the boundary is meaningful.

**Item 7/8 — assume_specification: PASS (none present).**
**Item 14 — spec drift: PASS (clean slate, nothing weakened upstream).**

---

### Fix Request (ordered; re-run `make verify-kernel` after FR-A)

**FR-A (BLOCKING — compile). Replace bare post-state `self@` with `final(self)@` in every `&mut self` postcondition; keep pre-state as `old(self)@`.** Exact sites:
- `map_kpage` L299-303: `self@.kernel_mapped(...)` → `final(self)@.kernel_mapped(...)`; `self@.user`/`self@.pgdir` → `final(self)@.user`/`final(self)@.pgdir`; `Err` arm `self@ == old(self)@` → `final(self)@ == old(self)@`.
- `map` L432-436: `self@.user_mapped(...)` → `final(self)@...`; `self@ == old(self)@.spec_map(...)` → `final(self)@ == old(self)@.spec_map(...)`; `Err` arm likewise.
- `mark_user_page_cow` L946-947, `unmark_user_page_cow` L987-988, `resolve_cow_at` L1078/1081/1085: every LHS `self@` that denotes the post-state → `final(self)@`; `Err` arms `self@ == old(self)@` → `final(self)@ == old(self)@`.
Note Verus reports only the *first* bare `self@` per function, so fix **all** occurrences, not just the 5 flagged lines. Verify: `make verify-kernel` must get past `cargo check` and Verus must actually run (non-zero "verified" or real verification errors, not `compilation/setup error`).

**FR-B (coverage, items 1/2/3/6). Add `#[verus_spec]` contracts to the 13 missing entry points**, transcribing `view_design.md` §sketches + `caller_analysis.md` rows. Each mutator's `Ok` arm must (i) use `final(self)@`/`old(self)@`, (ii) re-establish `final(self).inv()`, (iii) use the designed `spec_*` transition, and (iv) give a meaningful `Err` arm (`final(self)@ == old(self)@` where the caller relies on no-partial-state — `unmap`, `uctrl`, `kctrl`, `resolve_cow_for_region`, `copy_to_user_unaligned`). Read-only (`&self`) functions: state `self@` unchanged where callers rely on it.

**FR-C (spec strength, items 3/5). Strengthen the two weak mutator specs and add inv-preservation:**
- `map` L433: drop the existential; pin the frame to the mapped `uframe`'s physical address (project to `nat`) so `final(self)@ == old(self)@.spec_map(v, frame, perms)`.
- `map_kpage` L298-302: replace with `final(self)@ == old(self)@.spec_map_kpage(v, frame, perms)` (pin kernel frame + perms), per design.
- Add `final(self).inv()` to the `Ok` arm of all five mutators.

**FR-D (cheating, item 13). Remove the two redundant `admit()`s and contract the uncontracted `external_body` helpers:**
- Delete `proof! { admit(); }` at L190 (`new`) and L255 (`clone`) — they do nothing under `external_body` and trip the cheating gate.
- Give `is_kernel_addr`, `is_kernel_region`, `allocate_kernel_page_table`, `allocate_user_page_table`, `lookup_user_page_table`, `lookup_kernel_page_table`, `find_user_frame`, `replace_user_page_cow_frame` a real `ensures` (not a bare `external_body`). The pure ones (`is_kernel_addr`, `is_kernel_region`) should be `ensures ret == spec_is_kernel_addr(...)` / `spec_is_kernel_region(...)`.
- Verify: `cheating-detail.txt` must show `admit=0` and no contract-less `external_body` for module-owned functions; remaining `external_body` entries are the documented proving-phase obligations, each carrying a real contract.

### STOP / ROLLBACK
Not creating `STOP` — items 1,2,3,5,6,12,13,15,16,17 unresolved. Not requesting `ROLLBACK`: the View design is sound; all remaining work is in-phase (fix `final(self)@`, finish 13 contracts, strengthen 2 specs, drop 2 admits). Compile fix (FR-A) is the gating step.
