# Final Verification Review — `mm::phys::upool`

## Checklist
- [ ] 1. Spec quality for 8 in-scope functions (gaps found; see blockers)
- [ ] 2. Caller coverage for all expectations (partial: 8/15)
- [x] 3. Proof completeness in upool files (`admit()==0`; only approved `external_body`)
- [x] 4. TCB compliance for upool `external_body`
- [x] 5. AST consistency check + `// VERUS REWRITE` inspection
- [x] 6. Verification command executed (`make verify-kernel MODULE=mm::phys`)
- [x] 7. Guardrail counts produced (exact upool-scoped counts + locations)
- [x] 8. Bug reconciliation vs `bugs.md`

## Spec Quality
In-scope functions reviewed:
- `UserFrame::new`, `UserFrame::address`, `UserFrame::leak`, `UserFrame::share`, `UserFrame::refcount`, `UserFrame::drop`, `Upool::new`, `Upool::alloc`

Findings:
- `Upool::new` and `Upool::alloc` contracts are strong and caller-usable (`wf`, `alloc_one`, error-path preservation).
- `UserFrame::share` / `refcount` have meaningful snapshot facts, but no explicit old→new global transition.
- `UserFrame::drop` has no functional postcondition (only `opens_invariants none`, `no_unwind`).
- Several ensures are weaker than caller intent (`new/address/leak/refcount/share-Err` do not specify no-state-change effects).
- Some ensures are subsumed/redundant (e.g., `result.inv()` derivable from equality + precondition in `new/address/leak`; `uf.inv()` similarly in `share` if `self.inv()`).
- View consistency is otherwise coherent (`UserFrame@ == addr@`, `Upool@ : FrameAllocView` uninterpreted).

## Caller Coverage
**Covered 8 / 15** caller expectations from `caller_analysis.md`.

Covered:
1. `UserFrame::new` returns handle with `result@ == addr@`.
2. `UserFrame::address` returns owned address (`result@ == self@`).
3. `UserFrame::leak` returns owned address.
4. `UserFrame::share` success aliases same frame (`uf@ == self@`).
5. `UserFrame::refcount` success returns current frame refcount value.
6. `Upool::new` establishes `result@.wf()`.
7. `Upool::alloc` success matches `alloc_one` from free set.
8. `Upool::alloc` error preserves state and requires empty free set.

Missing / not fully captured:
1. `UserFrame::new` no-allocation/no-refcount-change effect.
2. `UserFrame::address` purity/no-side-effect guarantee.
3. `UserFrame::leak` explicit suppress-drop/no-decrement guarantee.
4. `UserFrame::share` success refcount `+1` transition.
5. `UserFrame::share` error path guarantees unchanged global frame state.
6. `UserFrame::refcount` non-mutating guarantee.
7. `UserFrame::drop` release transition (`release` / last-ref free).

Intentional deferral note (`view_design.md` §8, `bugs.md`):
- The missing `share +1` and `drop release` transition ensures are documented as intentionally deferred because `phys_view()` is modeled as a 0-arg uninterpreted constant.
- Judgment: understandable modeling limitation, but still a **real contract gap** for strict caller-level guarantees in this final review.

## Proof Completeness
Scoped to:
- `src/kernel/src/mm/phys/upool.rs`
- `src/kernel/src/mm/phys/upool.spec.rs`
- `src/kernel/src/mm/phys/upool.proof.rs`

Counts:
- `admit(...)`: **0** ✅
- `external_body`: **2** (`Upool::new`, `Upool::alloc`) ✅

Blocker rule check:
- Any `admit()>0`: not triggered.
- Any non-allowed `external_body`: not triggered.

## TCB Compliance
Upool external bodies found:
- `src/kernel/src/mm/phys/upool.rs::Upool::new`
- `src/kernel/src/mm/phys/upool.rs::Upool::alloc`

Both are explicitly listed in `verus-ai-logs/tcb-allowed.md` (upool thin-facade section). ✅

## Guardrails Compliance (upool-scoped exact counts)
Files counted: `upool.rs`, `upool.spec.rs`, `upool.proof.rs`.

- `admit`: **0**
- `assume(...)`: **0**
- `external_body`: **2**
  - `upool.rs:250` attribute for `Upool::new` (fn at line 255)
  - `upool.rs:271` attribute for `Upool::alloc` (fn at line 288)
- `assume_specification`: **0**
- `cfg` occurrences total: **4**
  - `upool.rs:9`, `11`, `37` (`verus_keep_ghost` includes/imports; non-exec)
  - `upool.rs:207` (`cfg(not(verus_keep_ghost))` around logging in `drop`; exec-site cfg, allowed logging-only gate)

Guardrail blocker rule (`admit>0` or `assume>0`): **not triggered**.

## AST Consistency
Command(s):
- `python3 .../ast_consistency.py --base-ref HEAD src/kernel/src/mm/phys/upool.rs summary`
- `python3 .../ast_consistency.py --base-ref HEAD src/kernel/src/mm/phys/upool.rs count`

Result:
- **Consistent: YES** (8 functions, 2 structs matched; 0 mismatches).
- `// VERUS REWRITE` occurrences in upool files: **0**.

## Verification
Command:
- `cd /home/ruize/nanvix-phy-specs-bottom-up && make verify-kernel MODULE=mm::phys`

Observed:
- Verus run exit code: **0**
- Verus errors: **0**
- Status line: `verification: cached (no recompilation), — (exit 0)`
- Cheating detector summary (module/global):
  - Module `mm::phys`: `external_body=14`, `admit=4`, `cfg_gate=11`
  - Global: `assume=0 external_body=14 admit=7 trusted=0 cfg_gate=12`
- Upool-specific entries in cheating detail:
  - `upool.rs::Upool::new` external_body
  - `upool.rs::Upool::alloc` external_body
  - No upool-scoped `admit`/`assume` reported.

Scope separation:
- Non-upool admits are in files like `manager.proof.rs` and are outside the 8-function upool scope.

## Bug Summary
`bugs.md` states no code bugs, with one intentional deferred modeling item:
- `share/drop` refcount transition ensures not encoded due current `phys_view()` modeling.

Reconciliation with current code:
- Still true; deferral persists.
- Classification: **Context-dependent verification limitation / intentional deferral**, not a newly found runtime code bug.

## Issues (priority order)
1. **BLOCKER (P0)** — Incomplete caller contract coverage (8/15).
2. **BLOCKER (P0)** — `UserFrame::drop` lacks functional ensures for release semantics.
3. **BLOCKER (P0)** — `UserFrame::share` lacks explicit `+1` transition and err-path unchanged-state guarantee (documented deferral, still unresolved for strict final sign-off).
4. **P1** — Missing explicit purity/no-global-change guarantees for `new/address/leak/refcount` relative to caller expectations.

## Final Result: **FAIL**
Justification:
- Although upool-scoped cheating counts are clean (`admit=0`, `assume=0`) and TCB/AST checks pass, strict final criteria are not met because caller-required behavioral guarantees are only partially specified (8/15), with unresolved high-impact contract gaps centered on `share`/`drop` refcount semantics.
