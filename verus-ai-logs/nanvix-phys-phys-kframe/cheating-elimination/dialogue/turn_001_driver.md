## Turn 1: Trusted functions — `assume_specification[ KernelFrame::map_frame ]` is untracked

### Progress
- Done (PASS, with evidence):
  - **Zero admit()** — `grep 'admit('` over `kframe.rs/.spec.rs/.proof.rs` → 0 hits. The 7 global
    admits reported by the tool (`cheating-detail.txt`) are all in `mm/phys/manager.proof.rs` and
    `mm/virt/identity_map.rs` — out of scope for kframe.
  - **Zero assume()** — `grep 'assume('` → 0 hits in kframe.
  - **Zero exec_allows_no_decreases_clause** — tool summary `no_decreases=0`; grep → 0.
  - **Zero literal `external_body`** in kframe — grep finds only the word inside comments; the tool
    confirms the module is `external_body`-free (the 15 global ones are in other modules).
  - **Zero cfg-gated exec code (beyond allowed)** — only cfg uses in kframe are
    `#[cfg(verus_keep_ghost)] include!(...)` (imports, lines 15/17) and
    `#[cfg(not(verus_keep_ghost))] error!(...)` (logging, line 197). Both are on the allowed list.
- Current: **Zero trusted functions** → FAIL (details below).
- Remaining (not yet verified this turn):
  - AST consistency (run **ast-consistency** skill).
  - Exec-rewrite comments + minimal reproducers (the `new`→`map_frame` split needs justification).
  - Per-trusted-item tcb-allowed cross-check (tightly coupled to the FAIL below).
  - Spec-drift (`spec-drift-check` skill) — original guarantees not weakened.
  - Cross-module regression — full `make verify-kernel` (no `MODULE=`).
  - Final verification + `make` build: 0 errors / 0 warnings.

### Verification
Commands run:
- `make verify-kernel MODULE=mm::phys::kframe` → Exit code 0. Cheating-pattern check:
  `✅ No cheating detected in module mm::phys::kframe` / `trusted=0 assume=0`.
  Coverage: `3/7 exec functions have contracts. Unverified: map_frame, clear, deref, deref_mut`.
- `git log -p` on `kframe.spec.rs` (commit `cf2b02367`) shows this phase **replaced the previous
  `external_body` on `KernelFrame::new` with**:
  ```rust
  pub assume_specification[ KernelFrame::map_frame ](base: FrameAddress) -> Result<(), Error>;
  ```
  (`kframe.spec.rs:34`). `new` is now machine-verified; the side effect was pushed into the new
  exec helper `map_frame` (`kframe.rs:97-109`), which calls `crate::mm::virt::identity_map_page`.
- Confirmed the cross-module precondition is real: `identity_map_page`
  (`mm/virt/identity_map.rs:699-717`) carries `requires identity_map_view().inv()` — a global
  invariant of the not-yet-verified `mm::virt` module, not dischargeable from `mm::phys`.

**The problem.** An `assume_specification` is a *trusted, unverified contract* — Verus assumes it
rather than proving it. The repo's own convention (see `verus-ai-logs/tcb-allowed.md`, which has a
dedicated `## assume_specification ...` section) is that **every** trusted contract — `external_body`
*and* `assume_specification` — must be enumerated in `tcb-allowed.md`, or it must be eliminated.

`KernelFrame::map_frame` is **not listed anywhere** in `tcb-allowed.md` (grep:
`grep -n map_frame verus-ai-logs/tcb-allowed.md` → 0 hits). Worse, `tcb-allowed.md` is now **stale**:
lines 92-93 still list `KernelFrame::new` as a "Cross-module dependency marked `external_body`
(eliminated when their module is verified)", but `new` no longer has `external_body` — it is fully
verified. So this phase *moved* the trust boundary from `new` to `map_frame` without updating the
ledger. The in-code comment (`kframe.spec.rs:25-33`) is a justification, not a tracking entry — and
justification is not the sanctioned escape hatch; the `tcb-allowed.md` listing is.

Verdict: **FAIL** on "Zero trusted functions" / "every surviving trusted contract must be listed in
`{tcb_allowed_file}`".

### Fix Request
Do **one** of the following (B is the sanctioned path given the genuine `mm::virt` boundary):

**(A) Eliminate the trusted contract.** If `map_frame`'s effect can be carried without an assumed
spec (e.g. thread `identity_map_view().inv()` as a `requires` on `new`/`map_frame` and discharge it
at the verified call site), remove the `assume_specification` entirely. If you claim this is
impossible, prove it — don't assert it: show the exact Verus error from attempting to call
`identity_map_page` from a verified `mm::phys` fn without the assumed boundary, and save the minimal
reproducer.

**(B) Track it in the TCB ledger (and de-stale it).** Edit `verus-ai-logs/tcb-allowed.md`:
1. **Add** an entry for `src/kernel/src/mm/phys/kframe.rs::KernelFrame::map_frame` under
   "Cross-module dependencies ... (eliminated when their module is verified)". The entry must state:
   it is an `assume_specification` (not `external_body`); the contract is **empty** (no `requires`,
   no abstract `ensures`); the rationale is that its sole effect is calling
   `mm::virt::identity_map_page`, whose `requires identity_map_view().inv()` is an `mm::virt` global
   invariant not dischargeable from `mm::phys`; and that it is removed when `mm::virt` is verified.
2. **Remove/replace the stale `KernelFrame::new` entry** (current lines 92-93): `new` is now
   verified and carries no `external_body`, so that line is factually wrong.

**Also (item 8, tied to this change):** the `new`→`map_frame` exec split is a verification-driven
exec rewrite. Add a `// VERUS REWRITE:` comment at the `map_frame` extraction site (`kframe.rs:97`)
pointing at the recorded reproducer / `tcb-allowed.md` entry, so the rewrite is traceable.

**Verify after fixing:**
- `grep -n map_frame verus-ai-logs/tcb-allowed.md` → must show the new entry.
- `grep -n 'KernelFrame::new' verus-ai-logs/tcb-allowed.md` → stale `external_body` entry gone.
- `make verify-kernel MODULE=mm::phys::kframe` → Exit 0, still `CLEAN`.
- Paste the tool's `=== Cheating Pattern Check ===` block as evidence.

Do not mark this item resolved until `tcb-allowed.md` reflects the actual trust surface (or the
`assume_specification` is gone). After it is resolved I will proceed to AST-consistency, spec-drift,
the full cross-module `make verify-kernel`, and the final build (0 errors / 0 warnings).
