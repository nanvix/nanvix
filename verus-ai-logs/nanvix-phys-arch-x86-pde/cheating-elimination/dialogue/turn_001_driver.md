## Turn 1: Full checklist walk-through (focus: AST consistency / exec rewrite documentation)

### Progress
- Done (PASS):
  - Zero `admit()` — `cheating: admit=0`
  - Zero `assume()` — `cheating: assume=0`
  - Zero trusted functions — `cheating: trusted=0`
  - Zero `exec_allows_no_decreases_clause` — `cheating: no_decreases=0`
  - Zero cfg-gated exec code — only `#[cfg(verus_keep_ghost)] include!("pde.spec.rs"/"pde.proof.rs")` (ghost spec/proof inclusion = allowed import)
  - Zero unlisted `external_body` — pde.rs/spec/proof contain **0** `external_body`. The 3 crate-wide hits are in other modules (`mod.rs:80 invlpg`, `table.rs:209 read`, `table.rs:246 write`) and are all listed in `verus-ai-logs/tcb-allowed.md`. Out of pde scope → PASS.
  - Surviving `external_body` listed in tcb file — N/A for pde (none); the 3 cross-module ones are listed.
  - No specs weakened — `spec_drift.py git-diff ... --before 39e930c91` (cheating-elimination START) exit 0, "No contract drift detected".
  - Cross-module regression — `make verify-arch` exit 0, all arch modules pass.
  - Verification — `make verify-arch` exit 0, `Finished dev profile`, 0 errors, 0 warnings; cheating check exit 0.
- Current: **AST consistency** + **exec-rewrite documentation** (FAIL — one undocumented rewrite).
- Remaining: none (every item verified; one FAIL to fix).

### Verification
Commands run and evidence:

1. `make verify-arch` → exit 0.
   `cheating: assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=0`.

2. `cat verus-ai-logs/verify-arch/verus-logs/cheating-detail.txt`:
   ```
   - x86/mem/paging/mod.rs:80 invlpg: external_body
   - x86/mem/paging/table.rs:209 read: external_body
   - x86/mem/paging/table.rs:246 write: external_body
   ```
   None in pde.* — confirmed by grep over pde.rs/pde.spec.rs/pde.proof.rs (only `#[cfg(verus_keep_ghost)]` on the two `include!`s).

3. Spec-drift (cheating-elimination START `39e930c91` → working tree):
   `spec_drift.py git-diff src/libs/arch/src/x86/mem/paging/pde.rs --before 39e930c91` → exit 0, 0 contract drift.

4. AST consistency vs original (`dev:...pde.rs`, extracted to /tmp/pde_orig.rs):
   `ast_consistency.py /tmp/pde_orig.rs <pde.rs> count` → **`⚠️ 1 mismatched (22 functions match)`**.
   Mismatch is `PageDirectoryEntry::frame_address`. Diff:
   ```
        pub fn frame_address(&self) -> usize {
   -        self.frame.into_raw_value() << crate::mem::FRAME_SHIFT
   +        let raw: usize = self.frame.into_raw_value();
   +
   +        raw << crate::mem::FRAME_SHIFT
        }
   ```
   Current pde.rs body (lines 420–424):
   ```rust
   pub fn frame_address(&self) -> usize {
       let raw: usize = self.frame.into_raw_value();
       proof! { lemma_frame_address(raw); }
       raw << crate::mem::FRAME_SHIFT
   }
   ```

This is the pre-approved "intermediate value for assertions" deviation
(`f(complex_expr)` → `let x = complex_expr; f(x)`): the exec result of
`into_raw_value()` must be bound to a local so the *same* value can be passed to
the proof lemma `lemma_frame_address(raw)` (a proof fn cannot take the exec call
directly) and reused in the shift. The rewrite is therefore acceptable in
principle — **but it carries no documenting comment**, violating the checklist
item "All exec rewrites have VERUS REWRITE comment and minimal reproducer".

### Fix Request (FAIL — items: "AST consistency" / "exec rewrites documented")

This is the only open item. Justification in this review is **not** a fix — the
code must carry the documentation.

Required change in `src/libs/arch/src/x86/mem/paging/pde.rs`, function
`frame_address` (around line 420): add a `VERUS REWRITE` comment at the rewrite
site documenting (a) which pre-approved deviation this is, (b) why the original
single-expression body cannot stay, and (c) a minimal reproducer of the Verus
limitation. For example:

```rust
pub fn frame_address(&self) -> usize {
    // VERUS REWRITE (pre-approved: "intermediate value for assertions").
    // Original: `self.frame.into_raw_value() << crate::mem::FRAME_SHIFT`.
    // The proof obligation (no-overflow + alignment of the shift) is discharged by
    // `lemma_frame_address`, which must receive the *same* runtime word the shift
    // uses. A `proof!` block cannot contain the exec call `self.frame.into_raw_value()`,
    // so the result is bound to `raw` once and shared by the lemma and the shift.
    // Minimal reproducer (fails — exec call inside proof):
    //   proof! { lemma_frame_address(self.frame.into_raw_value()); } // ERROR: cannot
    //   call exec function `into_raw_value` from proof mode
    let raw: usize = self.frame.into_raw_value();
    proof! { lemma_frame_address(raw); }
    raw << crate::mem::FRAME_SHIFT
}
```

Verify after the change:
1. `make verify-arch` → must remain exit 0, 0 errors, 0 warnings.
2. Re-run AST consistency; the rewrite stays a MISMATCH textually, so confirm it
   is the **only** one and that it is now a documented, semantically-equivalent
   rewrite:
   ```
   git show dev:src/libs/arch/src/x86/mem/paging/pde.rs > /tmp/pde_orig.rs
   python /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py \
       /tmp/pde_orig.rs src/libs/arch/src/x86/mem/paging/pde.rs summary
   ```
   Expected: only `PageDirectoryEntry::frame_address` differs, and it now has the
   `VERUS REWRITE` comment + reproducer above it.

(If instead you can collapse the body back to the original single expression while
still discharging the proof — e.g. via a `by` block or an `assert(...) by` that
avoids the local — that is preferred since it removes the mismatch entirely. Show
`make verify-arch` exit 0 and a clean `ast_consistency ... count` if you take that
route.)
