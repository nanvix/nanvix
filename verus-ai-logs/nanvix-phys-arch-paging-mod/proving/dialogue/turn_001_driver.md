## Turn 1: Full checklist review — arch-paging-mod (proving)

### Progress
- Done (PASS): all checklist items — see verdicts below.
- Current: final review and verdict.
- Remaining: none.

### Scope note
Review unit is `mod.rs` (module `arch::x86::mem::paging`). The crate-wide cheating
counter (`external_body=3`, `assume=1`, `cfg_gate=2`) aggregates the whole `arch`
crate. The `table.rs::read`/`table.rs::write` `external_body` and
`table.proof.rs::lemma_entry_roundtrip` `assume` belong to the **table** review unit
(separate, already committed, already in `tcb-allowed.md`). Per-function audit below
is scoped to the `mod.rs`/`mod.spec.rs`/`mod.proof.rs` triplet under review.

### Verification (commands run + evidence)

`make verify-arch` → exit 0. Module `<all>`, verification `cached (no recompilation)`,
status reports the crate-wide cheating tallies only (no verification errors).

`./z build -- all` → exit 0, `[OK] Build complete.` Re-grep for `warning:`/`error`
(excluding the benign "Sysroot directory not found" symlink notice) → **0 hits**.

`python3 scripts/spec_drift.py check nanvix-phys-arch-paging-mod` → **exit 0**,
baseline `731a8634e038` → HEAD: Functions with changes 0, contract drift 0,
ensures removed 0, requires added 0. "✅ No contract drift detected."

Cheating-detail (`verus-ai-logs/verify-arch/verus-logs/cheating-detail.txt`):
- `x86/mem/paging/mod.rs:80 invlpg: external_body`  ← in-scope
- `table.proof.rs:16 lemma_entry_roundtrip: assume` ← table unit (out of scope)
- `table.rs:209 read: external_body`               ← table unit (out of scope)
- `table.rs:246 write: external_body`              ← table unit (out of scope)

`grep -nE "admit|assume|external_body|assume_specification"` over the three mod files:
only `mod.rs:77` (the word "assume_specification" inside a doc comment) and
`mod.rs:79` (`#[verus_verify(external_body)]` on `invlpg`). No `admit`, no real
`assume`. `mod.spec.rs` and `mod.proof.rs` are both `verus! { }` (empty).

`grep -nE cfg mod.rs` → lines 8 and 10 only: `#[cfg(verus_keep_ghost)]` guarding
`include!("mod.spec.rs")` / `include!("mod.proof.rs")`. These gate spec/proof
inclusion (the standard Verus pattern), not exec branches/expressions/match arms.

### Verdicts

1. **No specs weakened** — PASS. `spec_drift.py check` exit 0, zero drift vs baseline `731a8634e038`.
2. **Zero remaining admit()** — PASS. 0 `admit` in mod triplet.
3. **Zero external_body unless TCB-allowed** — PASS. Exactly one: `mod.rs::invlpg`,
   explicitly listed in `verus-ai-logs/tcb-allowed.md` ("external_body introduced
   while speccing `arch::x86::mem::paging` (`mod.rs`)"). Per-function: `invlpg` =
   inline-asm `invlpg` instruction (TLB flush), external-bottom hardware boundary.
4. **Zero assume/assume_specification** — PASS. 0 real `assume` in mod triplet
   (the only textual hit is a doc-comment reference).
5. **No cfg-gated exec code** — PASS. The 2 cfg gates wrap `include!` of spec/proof
   files; no exec branch/expr/match arm is cfg-gated.
6. **Cheating audit (exact counts/locations, mod scope)** — PASS.
   - `admit`: 0
   - `external_body`: 1 → `mod.rs:80 invlpg` (TCB-allowed)
   - `assume`/`assume_specification`: 0
   - cfg-gated exec code: 0 (2 cfg gates are spec/proof `include!`, not exec)
7. **Claimed Verus limitation has isolated reproducer** — PASS.
   `verus-unsupported.md §1` gives a minimal, isolated reproducer (`fn flush` with a
   single `core::arch::asm!`) for the specific construct (inline-asm expressions),
   plus the exact error ("The verifier does not yet support ... inline-asm
   expressions"). Reproducer is isolated to the construct, not the full function.
8. **Exec rewrites minimal / `// VERUS REWRITE`** — PASS. No `VERUS REWRITE` comments
   and no exec rewrites in the mod triplet; `invlpg` body is the original upstream asm.
9. **Cross-module regression** — PASS. `make verify-arch` runs module `<all>` for the
   arch crate, exit 0 — every arch module verifies, not just the current one.
10. **Verification 0 errors/0 warnings** — PASS. `make verify-arch` exit 0;
    `./z build -- all` exit 0 with 0 compiler warnings/errors.

### Fix Request
None. All 10 checklist items PASS with tool-backed evidence. No code changes required.
