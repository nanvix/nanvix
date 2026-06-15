## Turn 1: Full checklist walk-through — arch-x86-pte cheating elimination

### Progress
- Done (PASS, with evidence):
  1. Zero `admit()` — PASS
  2. Zero `assume()` — PASS
  3. Zero trusted functions — PASS
  4. Zero `exec_allows_no_decreases_clause` — PASS
  5. Zero cfg-gated exec code — PASS
  6. Zero unlisted `external_body` — PASS
  7. AST consistency: zero mismatches — PASS
  8. All exec rewrites carry VERUS REWRITE comment + reproducer — PASS (N/A: zero exec rewrites)
  9. Each surviving `external_body` TCB-listed — PASS (none in pte)
  10. No specs weakened (spec-drift) — PASS
  11. Cross-module regression (whole arch crate) — PASS
  12. Verification + build, 0 errors / 0 warnings — PASS
- Current: none — all items resolved.
- Remaining: none.

### Verification

In-scope module: `src/libs/arch/src/x86/mem/paging/pte.rs` (+ `pte.spec.rs`, `pte.proof.rs`).
Target functions: `PageTableEntry::new`, `PageTableEntryFlags::new`,
`PageTableEntry::is_present`, `PageTableEntryFlags::is_present`.

**`make verify-arch` (whole arch crate):**
```
verification: 48 verified, 0 errors (exit 0)
cheating: assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=2
```
`verus-logs/cheating-detail.txt` — every flagged construct is OUTSIDE the pte module:
```
x86/mem/paging/mod.rs:80         invlpg                 : external_body  (TCB-listed)
x86/mem/paging/table.proof.rs:16 lemma_entry_roundtrip  : assume         (TCB-listed broadcast axiom)
x86/mem/paging/table.rs:209      read                   : external_body  (TCB-listed)
x86/mem/paging/table.rs:246      write                  : external_body  (TCB-listed)
```
All four appear in `verus-ai-logs/tcb-allowed.md`. None is in pte.rs/pte.spec.rs/pte.proof.rs.

**Per-item evidence:**

1–4, 6, 9. `grep -nE 'admit|assume|external_body|trusted|no_decreases'` over the three pte files
returns nothing executable; cheating-detail attributes all residuals to sibling modules.
`admit=0 trusted=0 no_decreases=0`, and pte has zero `external_body`/`assume`. PASS.

5. cfg-gated exec code — **investigated in depth (this was the only non-trivial item).**
   `cfg_gate=2` is counted at `pte.rs:85` and `pte.rs:307`:
   `#[cfg_attr(verus_keep_ghost, allow(unused, verus_impl_method_marker))]` on the two `new`
   constructors. I tested whether they were gratuitous (the sibling `pde.rs::new` has no such attr):
   - Removed both lines → `make verify-arch` fails: `error[E0425]: cannot find function 'new' in
     this scope` (exit 101). They are **required** by the `#[verus_spec]` proc-macro expansion for
     these otherwise-unused impl constructors (it emits the `verus_impl_method_marker` lint).
   - They gate only a lint-`allow` attribute, never executable logic. The AST checker (item 7)
     reports both `new` bodies as `MATCH` regardless. There is **zero** divergent runtime logic
     under verus vs non-verus — same class as a derive/attribute, not "cfg-gated exec code".
   PASS. (File restored after the experiment; the accidental verify auto-commit of the broken state
   was reset out of history — HEAD is back on a clean PASS commit.)

7. AST consistency — `scripts/ast_consistency.py --base-ref dev pte.rs summary`:
   ```
   Consistent: YES (matched=23 mismatched=0 missing=0 extra=0)
   ```
   All 21 exec fns + 2 structs MATCH the original `dev` source. PASS.

8. Exec rewrites — `git diff dev HEAD -- pte.rs` shows the only exec-body delta is a ghost
   `proof! { use_type_invariant(frame); }` added to `PageTableEntry::new` (stripped from exec; the
   fn still MATCHes at AST level). No semantically-equivalent exec rewrite was introduced, so no
   `VERUS REWRITE` comment/reproducer is required. PASS (N/A).

10. Spec-drift — `git diff dev HEAD -- pte.rs`: changes are purely additive ghost annotations
    (`#[verus_verify]`, `#[verus_spec(...)]`, the `#[cfg_attr]` markers, the spec/proof `include!`s)
    plus the ghost `proof!` line. `grep '^-' ... | grep -iE 'ensures|requires|spec|fn'` → NONE
    removed. No original guarantee weakened. PASS.

11. Cross-module regression — `make verify-arch` runs the entire arch crate: 48 verified, exit 0.
    All verified modules pass. PASS.

12. Verification + build —
    - `make verify-arch`: 48 verified, 0 errors, exit 0.
    - `cargo build -p arch ... --target build/targets/x86-kernel.json`:
      `Finished dev profile ... in 0.56s` — 0 errors, 0 warnings.
    PASS.

### Fix Request

None. Every checklist item PASSES with tool-backed evidence. The pte module was already
cheating-free and AST-consistent; the only point requiring scrutiny (`cfg_gate=2`) was proven to
be a compiler-required verus lint-allow attribute that gates no executable code. No code changes
are needed. Writing STOP = RESOLVED.
