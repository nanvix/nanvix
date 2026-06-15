## Turn 1: Full Checklist Walkthrough — arch-frame-number

Scope: `src/libs/arch/src/x86/mem/paging/frame/number.rs` (+ `number.spec.rs`,
`number.proof.rs`). Verus command: `make verify-arch`.

### Progress
- Done (PASS, with evidence below): all 12 checklist items.
- Current: final verification + build.
- Remaining: none.

### Verification

**`make verify-arch`** → exit 0. Cheating-pattern summary (crate-wide):
`assume=0 external_body=3 admit=1 trusted=0 no_decreases=0 cfg_gate=4`.
Last non-cached verification result: `48 verified, 0 errors`.

Crucially, the crate-wide residue is **not** in the arch-frame-number scope.
`cheating-detail.txt` localizes every violation to other modules (their own
pipeline phases), all of which are TCB-listed where required:
```
- x86/mem/paging/mod.rs:80 invlpg: external_body          (tcb-allowed.md ✔)
- x86/mem/paging/table.proof.rs:8 lemma_entry_roundtrip: admit   (paging-table module)
- x86/mem/paging/table.rs:209 read: external_body          (tcb-allowed.md ✔)
- x86/mem/paging/table.rs:246 write: external_body         (tcb-allowed.md ✔)
```

Per-file scan of the three target files
(`grep -nE 'admit|assume|external_body|trusted|no_decreases|#\[cfg|external|unsafe'`):
- `number.rs`: only lines 9 & 11 → `#[cfg(verus_keep_ghost)]` guarding
  `include!("number.spec.rs")` / `include!("number.proof.rs")` — ghost imports.
- `number.spec.rs`: single hit is the word "assumed" inside a comment (line 25),
  not code.
- `number.proof.rs`: none (file is `verus! { }`).

Item-by-item:

1. **Zero admit()** — PASS. None in number.rs/spec/proof. The crate `admit=1` is
   `table.proof.rs:8` (paging-table module, out of scope).
2. **Zero assume()** — PASS. `assume=0`; the only textual hit is a comment word.
3. **Zero trusted functions** — PASS. `trusted=0`.
4. **Zero exec_allows_no_decreases_clause** — PASS. `no_decreases=0`.
5. **Zero cfg-gated exec code** — PASS. The only `#[cfg]` in scope guards the two
   ghost `include!` of spec/proof — explicitly allowed (imports of ghost code).
   No exec body is cfg-gated.
6. **Zero external_body unless TCB-listed** — PASS. No `external_body` in
   number.rs. (The 3 crate-wide ones are all listed in `verus-ai-logs/tcb-allowed.md`.)
7. **AST consistency** — PASS. `ast_consistency.py number.rs count` →
   `✅ Consistent: 4 functions, 1 structs match.`
8. **All exec rewrites have VERUS REWRITE comment + reproducer** — PASS (n/a).
   AST check shows exec unchanged; there are no exec rewrites to annotate.
9. **Each surviving external_body TCB-listed** — PASS. Zero in scope.
10. **No specs weakened** — PASS. `spec_drift.py check arch-frame-number`
    (baseline `c410af898d4b` → HEAD): `Contract drift: 0`, ensures removed 0,
    requires added 0, functions removed 0. `✅ No contract drift detected.`
11. **Cross-module regression (`make verify-arch`)** — PASS. Exit 0,
    `48 verified, 0 errors`; all verified modules pass.
12. **Verification + build** — PASS. `make verify-arch` exit 0, 0 errors; the
    cargo build step (`Finished dev profile`) completes with no warnings/errors
    in the log.

### Fix Request
None. Every checklist item is PASS with concrete tool evidence. The
arch-frame-number cheating-elimination phase is clean — number.rs/spec/proof
contain no admit/assume/trusted/external_body/no-decreases/cfg-gated-exec, AST
is unchanged, and no spec drift occurred. Remaining crate-wide cheating residue
belongs to sibling modules (paging-table, paging-mod) handled in their own
phases and is TCB-authorized where it is `external_body`.

**Verdict: RESOLVED.**
