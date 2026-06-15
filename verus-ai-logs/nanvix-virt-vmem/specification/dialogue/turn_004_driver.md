## Turn 4: Final re-review after fixer turn 3 — RESOLVED

### Progress
- Done (PASS/FIXED): **all 17 checklist items.**
- FIXED this turn: FR-E (`for_each_user_mapping` ensures), FR-F (`try_find_user_frame` contract + four-helper deferral comments).
- Remaining: none.

### Verification performed

**`make verify-kernel`** (re-run by me): `Exit code : 0`, **0 verification errors**. Guardrail: `assume=0 admit=0 trusted=0 no_decreases=0`, `external_body=54`, coverage `31/35` in-scope (`31/1069` crate-wide). `status: CHEATING_DETECTED` is solely the accepted spec-phase `external_body` placeholder boundary.

**`./z build -- all`** (dual non-Verus build, re-run by me): `[OK] Build complete.` — cfg-gated specs are inert in the production build.

**FR-E — `for_each_user_mapping` (L941-958): FIXED.** Now carries a real closure spec:
- `requires`: `forall|v,pte| self@.user_mapped(v.addr_nat()) ==> call_requires(f,(v,pte))` — the callback accepts every present user page.
- `ensures Ok`: `forall|v| self@.user_mapped(v.addr_nat()) ==> exists|pte| call_ensures(f,(v,pte),Ok(()))` — coverage over all of `self@.user.dom()`, matching `view_design.md`'s "invokes `f` exactly on `self@.user.dom()`" and the `caller_analysis.md` complete-coverage expectation. Typechecks and verifies under Verus.

**FR-F — `try_find_user_frame` (L835-847): FIXED.** Now contracted: `Ok(Some(fr)) ==> self@.user_mapped(v) && fr.addr_nat() == self@.user[v].frame`; `Ok(None) ==> !self@.user_mapped(v)`. Coverage rose 30→31. The four remaining bare helpers (`allocate_kernel_page_table` L399, `allocate_user_page_table` L426, `lookup_user_page_table` L681, `lookup_kernel_page_table` L729) each now carry the required deferral comment ("Bare `external_body`, deferred to the proving phase: … the View abstracts away (`internal_inv()`) …"). Verified all four individually.

### Full checklist disposition

1. **Coverage — PASS.** All 27 in-scope entry points + 3 helpers contracted (31/35). The 4 uncontracted are representation-only helpers, justified-deferred with comments.
2. **Caller coverage — PASS.** Spot-checked `map`/`map_kpage`/`unmap`/`kctrl`/`resolve_cow_at`/`for_each_user_mapping`/copy paths against `caller_analysis.md`; specs match (frame identity pinned, dry-run⇒commit, CoW round-trip, coverage).
3. **View consistency / inv — PASS.** Specs reference `VmemView` fields + `spec_*` transitions; all five mutators ensure `final(self).inv()`.
4. **No tautological ensures — PASS.** Remaining `Err(_) => true` only on constructors (`new`,`clone`) and `&self` queries (`is_user_page_mapped`,`find_user_frame`,`try_find_user_frame`,`try_find_user_pte`) where there is no post-state side-effect to deny and the `Ok` arm carries the real content — examined individually, accepted.
5. **No subsumed ensures — PASS.** `map` frame-pinned (not existential); `map_kpage` uses full `spec_map_kpage`; no clause derivable-and-redundant from inv() found.
6. **Error paths meaningful — PASS.** Every mutator's `Err` arm asserts `final(self)@ == old(self)@`.
7. **No assume_specification for internal code — PASS.** `assume=0`; none present.
8. **vstd searched — PASS (vacuous).** No `assume_specification`.
9. **Specs caller-usable — PASS.** Written against `VmemView`/`spec_*` in `old`/`final` form usable in caller proofs.
10. **Trait obligations — PASS.** `Drop` carries `opens_invariants none` + `no_unwind`; ownership-release semantics documented and `external_body`-deferred for the proving phase, consistent with `caller_analysis.md`.
11. **Spec completeness (advisory) — PASS.** Intentional nondeterminism (`resolve_cow_at` existential frame with validity side-conditions) matches caller expectations.
12. **Loop invariants — PASS (deferred-correct).** Every in-scope exec fn is `external_body`, so Verus does not check loop bodies and emits no missing-invariant error; invariants are a proving-phase obligation once bodies are translated. No verification gap exists now.
13. **No cheating on own functions — PASS (with accepted boundary).** `admit=0`, `assume=0`, `trusted=0`. The `external_body` placeholders each carry a real contract (or a documented deferral for the 4 representation-only helpers) — the trust boundary reviewer turn 2 explicitly accepted for the spec phase.
14. **No specs weakened — PASS.** Clean slate (spec file was empty upstream); successive turns strengthened, never weakened (e.g. `map` existential → pinned frame).
15. **Bug awareness — PASS.** BUG-001 (`pgdir`/ghost placeholder) recorded in `bugs.md`; flagged for the proving phase.
16. **Cross-module regression — PASS.** `make verify-kernel` (whole `kernel` crate) exits 0; all modules cached-pass.
17. **Verification — PASS.** `make verify-kernel` exit 0, 0 errors; `./z build -- all` `[OK]`.

### Conclusion
Every checklist item is PASS or FIXED with concrete tool evidence. Creating `STOP` = `RESOLVED`. No rollback needed — the View design held throughout; all work was in-phase.
