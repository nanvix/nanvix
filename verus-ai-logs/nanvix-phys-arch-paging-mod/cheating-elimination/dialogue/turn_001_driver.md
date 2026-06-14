## Turn 1: Full checklist walkthrough — arch-paging-mod

### Progress
- Done (all PASS): Zero admit; Zero assume; Zero trusted; Zero no_decreases;
  Zero cfg-gated exec; external_body allowlisted; AST consistency; exec-rewrite
  comments (vacuous); surviving external_body listed; no spec weakening;
  cross-module regression; verification + build.
- Current: none.
- Remaining: none.

### Scope
In-scope files (only): `mod.rs`, `mod.spec.rs`, `mod.proof.rs` under
`src/libs/arch/src/x86/mem/paging/`. In-scope target function: `invlpg` only
(per `verus-ai.toml` and `tcb-allowed.md`). Out-of-scope, untouched (hard rule):
`table.rs::read`, `table.rs::write` (external_body), `table.proof.rs:8
lemma_entry_roundtrip` (admit) — these belong to the `arch-paging-table` module's
own elimination phase.

### Verification

**`make verify-arch`** — Exit code 0 (verification passes).
Crate-wide cheating counter: `assume=0 external_body=3 admit=1 trusted=0
no_decreases=0 cfg_gate=4`. `status: CHEATING_DETECTED` is driven *entirely* by
out-of-scope `table` items.

`cheating-detail.txt` (crate-wide breakdown):
```
- x86/mem/paging/mod.rs:80 invlpg: external_body        <-- in scope
- x86/mem/paging/table.proof.rs:8 lemma_entry_roundtrip: admit   (out of scope)
- x86/mem/paging/table.rs:209 read: external_body               (out of scope)
- x86/mem/paging/table.rs:246 write: external_body              (out of scope)
```

**In-scope grep** (`mod.rs|mod.spec.rs|mod.proof.rs`) for
`admit|assume|external_body|exec_allows_no_decreases|trusted`:
- `mod.rs:79 #[verus_verify(external_body)]` (invlpg) — the only real construct.
- `mod.rs:77` `assume_specification` appears only inside a `//` comment (not code).
- `mod.spec.rs` and `mod.proof.rs` are empty (`verus! { }`).

Per-item determination:

1. **Zero admit()** — PASS. 0 in-scope. (admit at `table.proof.rs:8` is a
   different module, untouched per hard rule.)
2. **Zero assume()** — PASS. 0 in-scope; line 77 hit is a comment.
3. **Zero trusted functions** — PASS. `trusted=0` crate-wide.
4. **Zero exec_allows_no_decreases_clause** — PASS. `no_decreases=0`.
5. **Zero cfg-gated exec code** — PASS. The two `#[cfg(verus_keep_ghost)]` at
   `mod.rs:8,10` gate `include!("mod.spec.rs")` / `include!("mod.proof.rs")` —
   the project-standard ghost spec/proof inclusion pattern, not exec gating. No
   exec function is cfg-gated.
6. **Zero external_body unless listed in `tcb-allowed.md`** — PASS. Single
   external_body `mod.rs:80 invlpg` is listed verbatim in
   `verus-ai-logs/tcb-allowed.md` ("`external_body` introduced while speccing
   `arch::x86::mem::paging` (`mod.rs`)" → `…/mod.rs::invlpg`). Rationale:
   single `core::arch::asm!` `invlpg` block — inline-asm is unsupported by Verus;
   external-bottom hardware TLB trust boundary, empty faithful contract.
7. **AST consistency: zero mismatches** — PASS. `git diff dev -- mod.rs` shows
   the exec signature and body of `invlpg` are byte-identical to base `dev`. Only
   additions: `use vstd::prelude::*;`, two cfg-gated ghost `include!` lines, a
   trust-boundary doc comment, and the `#[verus_verify(external_body)]` attribute.
   No exec signature, body, semantics, time, or space complexity changed.
   `git diff` of `mod.spec.rs`/`mod.proof.rs` vs base: both empty (`verus! { }`).
8. **All exec rewrites have VERUS REWRITE comment + reproducer** — PASS (vacuous).
   No exec body was rewritten; `invlpg`'s body is unchanged from `dev`.
9. **Each surviving external_body listed** — PASS. `invlpg` listed (see item 6);
   backing doc `nanvix-phys-arch-paging-mod/verus-unsupported.md` present.
10. **No specs weakened (spec drift)** — PASS. Base `dev` had no Verus contract on
    `invlpg`; the new contract is empty (no `requires`, trivial `ensures`),
    matching the inherited upstream `assume_specification[
    ::arch::mem::paging::invlpg ]` (no `requires`/`ensures`). No prior guarantee
    existed to weaken; the empty contract is faithful to the hardware-only effect.
11. **Cross-module regression (`make verify-arch`)** — PASS. Verus exit code 0;
    all verified modules pass. `CHEATING_DETECTED` is a cheating-counter status,
    not a verification failure.
12. **Verification + build: 0 errors, 0 warnings** — PASS. Verus verification
    exit 0; cargo build `Finished dev profile` with no warnings/errors in the log.

### Fix Request
None. Every in-scope checklist item is PASS with concrete tool evidence. The only
in-scope cheating construct (`invlpg` `external_body`) is on the TCB allowlist,
which the checklist explicitly permits. Out-of-scope `table` items are correctly
excluded by the hard rule and tracked in their own module's phase.

**Result: RESOLVED.**
