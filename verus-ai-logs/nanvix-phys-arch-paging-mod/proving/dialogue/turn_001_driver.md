## Turn 1: Full checklist walkthrough — `arch::x86::mem::paging` (`mod.rs`)

Scope under review: `src/libs/arch/src/x86/mem/paging/mod.rs` + `mod.spec.rs` + `mod.proof.rs`.
Child modules (`flags`, `frame`, `pde`, `pte`, `table`) are out of scope — they have
their own review cycles. Crate-wide cheating counts include those modules and are
reported but not charged against this module.

### Progress
- Done (PASS): all 10 checklist items.
- Current: final sign-off.
- Remaining: none.

### Verification

**Command run (fresh, non-cached):** `make verify-arch`
```
=== Verus Verification (cargo mode) ===
verification results:: 48 verified, 0 errors
  Exit code : 0
  status: CHEATING_DETECTED   (crate-wide; charged to table module — see below)
```
(Forced recompile by `touch mod.rs`; no warnings emitted.)

**Crate-wide cheating detail** (`verus-logs/cheating-detail.txt`):
```
- x86/mem/paging/mod.rs:80  invlpg              external_body   <- THIS module
- x86/mem/paging/table.proof.rs:8 lemma_entry_roundtrip admit   <- table module
- x86/mem/paging/table.rs:209 read              external_body   <- table module
- x86/mem/paging/table.rs:246 write             external_body   <- table module
```

Per-item findings (this module only):

1. **No specs weakened — PASS.** `mod.spec.rs` and `mod.proof.rs` are both empty
   (`verus! { }`); there is no spec/proof function to weaken. The sole contract is
   `invlpg`'s empty contract (no `requires`/`ensures`), which is *equal in strength*
   to the inherited upstream `assume_specification[ ::arch::mem::paging::invlpg ]`
   (`identity_map.spec.rs:151`, no `requires`/`ensures`). Trust boundary moved from
   `assume_specification` → `external_body`; not weakened.

2. **Zero remaining `admit()` — PASS.** `grep admit` over mod.rs/spec/proof: none.
   The 1 crate-wide `admit` is `table.proof.rs:8` (different module, out of scope).

3. **Zero `external_body` unless in tcb-allowed — PASS.** Only `invlpg` (mod.rs:80).
   It IS listed in `verus-ai-logs/tcb-allowed.md` under "external_body introduced
   while speccing arch::x86::mem::paging (mod.rs)" with full justification
   (inline-asm hardware TLB trust boundary). HARD RULE satisfied.

4. **Zero `assume`/`assume_specification` — PASS.** `grep`: the only textual match is
   a *comment* on mod.rs:77 documenting the superseded upstream `assume_specification`.
   No actual `assume(...)` or `assume_specification[...]` declaration exists.

5. **No cfg-gated exec code — PASS.** mod.rs has exactly two `#[cfg(verus_keep_ghost)]`
   attributes (lines 8, 10), both gating `include!("mod.spec.rs")` / `include!("mod.proof.rs")`.
   These are ghost-include directives, not exec branches/expressions/match arms; the
   `count_cfg_gates` detector explicitly excludes `include!` targets, so this module
   contributes 0 to `cfg_gate`. (Crate-wide `cfg_gate=4` originates elsewhere.)

6. **Cheating audit — PASS (reported).** This module: `admit=0`, `external_body=1`
   (`invlpg` @ mod.rs:80, allow-listed), `assume=0`, cfg-gated exec=0.
   Crate-wide (incl. out-of-scope table module): `assume=0 external_body=3 admit=1
   cfg_gate=4`.

7. **Isolated reproducer for claimed Verus limitation — PASS.**
   `verus-unsupported.md` §1 records the inline-asm limitation with a *minimal* standalone
   reproducer (`pub unsafe fn flush(vaddr: usize) { core::arch::asm!(...) }`) and the exact
   error: `The verifier does not yet support the following Rust feature: inline-asm
   expressions`. This isolates the specific construct (`core::arch::asm!`), not just the
   full failing function.

8. **Exec rewrites minimal/semantically equivalent — PASS.** `grep "VERUS REWRITE"` in
   mod.rs: none. The `invlpg` body is unchanged from upstream; the only edit is the added
   `#[verus_verify(external_body)]` attribute + documentation. No behavioral rewrite.

9. **Cross-module regression — PASS.** `make verify-arch` (all modules): 48 verified,
   0 errors, exit 0. No previously-verified module regressed.

10. **Verification + build — PASS.** Fresh `make verify-arch`: 48 verified, 0 errors,
    0 warnings, exit 0.

### Fix Request
None. All 10 checklist items PASS with concrete tool evidence. The single `external_body`
(`invlpg`) is properly allow-listed in `tcb-allowed.md` and documented in
`verus-unsupported.md` with an isolated reproducer; the module introduces no `admit`,
`assume`/`assume_specification`, or cfg-gated exec code, weakens no specs, and the full
arch crate verifies cleanly (48/0/0).

Result: RESOLVED.
