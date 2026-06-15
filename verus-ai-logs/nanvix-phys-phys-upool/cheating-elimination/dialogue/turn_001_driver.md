## Turn 1: Full checklist walkthrough — phys-upool cheating elimination

### Progress
- Done (all PASS, evidence below):
  1. Zero admit()
  2. Zero assume()
  3. Zero trusted functions
  4. Zero exec_allows_no_decreases_clause
  5. Zero disallowed cfg-gated exec code
  6. Zero unlisted external_body
  7. AST consistency: zero mismatches
  8. All exec rewrites have VERUS REWRITE comment + reproducer (vacuous — no exec rewrites)
  9. Each surviving external_body listed in tcb-allowed.md
  10. No specs weakened (spec-drift)
  11. Cross-module regression (`make verify-kernel`)
  12. Verification + build: 0 errors, 0 warnings
- Current: none — all items verified.
- Remaining: none.

### Scope
Module under review: `src/kernel/src/mm/phys/upool.rs`, `upool.spec.rs`, `upool.proof.rs`.
The global `make verify-kernel` cheating counters aggregate the WHOLE kernel; out-of-scope
admit/external_body in `frame.rs`, `manager.proof.rs`, `mod.rs`, `identity_map.rs` are tracked
separately in `tcb-allowed.md` and are NOT part of this module's review.

### Verification (commands run + evidence)

**`make verify-kernel`** → Exit code 0. Verus log
(`verus-ai-logs/verify-kernel/verus-logs/verus_2026-06-15_13-16-00.log`):
`grep -ci warning` = 0, `grep -ci error` = 0. Only informational *trigger-confidence notes*
in `frame.rs:1186-1187` (out of scope; notes, not warnings). Build: "Finished dev profile … Exit code: 0".

**Per-item evidence:**

1. **admit** — `grep -n "admit" upool.rs upool.spec.rs upool.proof.rs` → none.
   `cheating-detail.txt` lists no admit for any `upool.rs` line. PASS.
2. **assume** — same grep → none. Global counter shows `assume=0`. PASS.
3. **trusted** — none in module; global `trusted=0`. PASS.
4. **no_decreases** — none in module; global `no_decreases=0`. PASS.
5. **cfg-gated exec** — only `cfg`s present:
   - L9/L11 `#[cfg(verus_keep_ghost)]` guard `include!("upool.spec.rs"/"upool.proof.rs")` — spec/proof imports (allowed).
   - L37 `#[cfg(verus_keep_ghost)]` guards the `verus! { … View impls … }` block — spec code (allowed).
   - L203 `#[cfg(not(verus_keep_ghost))]` guards the `error!("failed to free user frame…")`
     logging macro inside `Drop::drop` — **logging** (explicitly allowed). No exec logic is
     hidden/altered by any cfg gate. PASS.
6. **external_body (3, all listed)** — `cheating-detail.txt`:
   - `upool.rs:221 Upool (struct)` → tcb-allowed.md L101 ✅
   - `upool.rs:246 Upool::new` → tcb-allowed.md L106 ✅
   - `upool.rs:279 Upool::alloc` → tcb-allowed.md L112 ✅
   No unlisted external_body. PASS.
7. **AST consistency** — `git diff dev HEAD -- upool.rs` shows the diff is **purely additive**:
   `git diff dev HEAD -- upool.rs | grep -E '^-' | grep -v '^---'` → **zero deletion lines**
   (grep exit 1). Every exec function body (`UserFrame::new/address/leak/share/refcount/drop`,
   `Upool::new/alloc`) is byte-identical to the pre-verification `dev` source; only
   `#[verus_verify]` / `#[verus_spec]` attributes, the spec/proof `include!`s, the View impls,
   and the L203 logging cfg-gate were added. Therefore exec AST ≡ original for all functions —
   zero mismatches. PASS.
   (Note: the prior fix_report's `git diff verus-ai-prove-bottom-up` is a self-diff — HEAD is
   that branch — so it was trivially empty; the meaningful baseline is `dev`, checked here.)
8. **VERUS REWRITE comments** — there are **no** exec rewrites (item 7 proves zero exec body
   changes), so this requirement is vacuously satisfied. PASS.
9. **Each external_body justified** — all 3 enumerated in tcb-allowed.md §"external_body
   thin-facade trust boundaries in mm::phys::upool" (L91-118) with full rationale (opaque
   facade over the global frame allocator; `Upool::new` assumed §8 ghost-attachment axiom;
   `Upool::alloc` deferred §8 ghost token until the frame free-function layer is verified). PASS.
10. **No specs weakened (spec-drift)** — the spec/proof files are **newly added** (absent on
    `dev`: `git ls-tree dev -- upool.spec.rs upool.proof.rs` empty) and the diff added contracts
    without removing/loosening any prior contract (there were none). No pre-existing guarantee
    exists to weaken; contracts on the verified handle methods (`share`/`refcount` pin to
    `phys_view().frames`, `drop` is `no_unwind`/`opens_invariants none`) were strengthened from
    nothing. No drift. PASS.
11. **Cross-module regression** — `make verify-kernel` (full kernel crate) → exit 0, all verified
    modules pass; no regression introduced by upool. PASS.
12. **Verification + build, 0 errors / 0 warnings** — exit 0; log warnings=0, errors=0. PASS.

### Fix Request
None. All 12 checklist items PASS with concrete tool evidence. No code change required.

### Result: RESOLVED
