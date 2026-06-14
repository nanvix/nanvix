## Turn 1: Full checklist walkthrough (cheating-elimination, phys-kframe)

Reviewer verdict: **all checklist items PASS** with concrete evidence below. No
fix request issued. Scope = `kframe.rs`, `kframe.spec.rs`, `kframe.proof.rs`.

### Progress
- Done (PASS): all 11 checklist items (see Verification).
- Current: none — review complete.
- Remaining: none.

### Verification

Commands run by the reviewer (not trusting the fixer's claims):
- `make verify-kernel` → exit 0. Harness summary:
  `cheating: assume=0 external_body=28 admit=0 trusted=0 no_decreases=0 cfg_gate=9`,
  all 5 `mm::phys` modules verified, status line is `CHEATING_DETECTED` only
  because it counts the **global** TCB-allowed shim total (28), not a kframe gap.
- `./z build -- all` → `[OK] Build complete.`, 0 compiler errors, 0 compiler
  warnings. (The `Warning: Sysroot directory ... not found` line is a harness
  cosmetic note, not a rustc/Verus warning.)
- `git --no-pager diff dev HEAD -- src/kernel/src/mm/phys/kframe.rs` to establish
  the AST/spec-drift baseline.
- Targeted greps over the three kframe files for every cheating pattern.
- `verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt` to attribute each
  flagged construct to a specific function.

Item-by-item:

1. **Zero admit()** — PASS. `grep -n admit kframe.rs kframe.spec.rs
   kframe.proof.rs` → empty. Harness `admit=0`.

2. **Zero assume()** — PASS. `grep -nE 'assume\s*\(|assume!'` → empty. Harness
   `assume=0`. (`assume_specification`, a distinct trait-spec mechanism, is
   handled under items 6/9.)

3. **Zero trusted functions** — PASS. Harness `trusted=0`. The token "trusted"
   appears only inside a prose comment in `kframe.spec.rs:31`; no `#[verifier::trusted]`.

4. **Zero exec_allows_no_decreases_clause** — PASS. Harness `no_decreases=0`;
   grep over the three files finds none.

5. **Zero cfg-gated exec code** — PASS. The only `#[cfg(...)]` gates are at
   `kframe.rs:15,17,32,47`: `include!("kframe.spec.rs")`, `include!("kframe.proof.rs")`,
   `use ::vstd::prelude::*;`, and the `verus! { impl View for KernelFrame ... }`
   block — all ghost/spec/import scaffolding under `verus_keep_ghost`. No exec
   function body or exec behavior is cfg-gated.

6. **external_body only if TCB-listed** — PASS. `cheating-detail.txt` shows the
   sole kframe external_body is `mm/phys/kframe.rs:141 clear`. It is listed in
   `verus-ai-logs/tcb-allowed.md` ("Allowed external_body" →
   `KernelFrame::clear` / `deref` / `deref_mut`). `clear` materializes a
   `*mut u8` from `usize` and writes via the identity-map `memset` backend — a
   raw-memory op Verus cannot model. No unlisted external_body exists.

7. **AST consistency — zero mismatches** — PASS. `git diff dev HEAD -- kframe.rs`
   shows every exec body (`new`, `base`, `clear`, `deref`, `deref_mut`, `drop`)
   is **byte-identical** to the base; the only additions are ghost-gated imports,
   the `View` impl, and `#[verus_verify]`/`#[verus_spec(...)]` annotations. No
   semantic, time-, or space-complexity change. Matches the established sibling
   pattern (`frame.rs`, `manager.rs`, `upool.rs`).

8. **All exec rewrites have VERUS REWRITE comment + reproducer** — PASS
   (vacuous). `grep -n 'VERUS REWRITE'` → empty, and item 7 proves there are **no**
   exec rewrites, so the rule has nothing to apply to.

9. **Each surviving external_body / trust boundary is TCB-listed** — PASS.
   - `KernelFrame::clear` (external_body) — listed (item 6).
   - `<PageAligned<T> as Address>::from_raw_value` (`assume_specification`,
     `kframe.spec.rs:33`) — external `sys::mm::Address` trait method; explicitly
     listed under "Allowed assume_specification — sys::mm::Address trait method"
     in `tcb-allowed.md`. Not eliminable without verifying `hal::mem`.

10. **No specs weakened (spec-drift)** — PASS. The `dev` base carried **no**
    Verus contracts for these functions, so nothing pre-existing could be
    weakened. The three added contracts are strict net-new guarantees:
    `new` ⇒ `frame@ == base@` (address identity), `base` ⇒ `result@ == self@`
    (pure read), `drop` ⇒ `phys_view().inv()` with `opens_invariants none` /
    `no_unwind`. Exec semantics unchanged (item 7). No `ensures` was loosened to
    `true` except where the function is itself TCB-trusted.

11. **Cross-module regression** — PASS. `make verify-kernel` verified modules
    `mm::phys`, `mm::phys::frame`, `mm::phys::kframe`, `mm::phys::manager`,
    `mm::phys::upool`, exit 0. No module regressed.

12. **Verification + build, 0 errors / 0 warnings** — PASS. `make verify-kernel`
    exit 0 (no `warning` lines in the verus log); `./z build -- all` completed
    with 0 errors and 0 compiler warnings.

### Fix Request
None. Every checklist item is PASS with tool-output evidence. The two surviving
trust boundaries (`clear` external_body, `from_raw_value` assume_specification)
are both enumerated in `verus-ai-logs/tcb-allowed.md` and are not eliminable
without verifying out-of-scope dependency modules (`hal::mem`, `mm::virt`).

**Result: RESOLVED.**
