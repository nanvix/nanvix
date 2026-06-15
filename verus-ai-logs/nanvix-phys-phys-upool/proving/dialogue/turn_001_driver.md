## Turn 1: Full checklist walkthrough — Proving phase, phys-upool

### Progress
- PASS: (1) spec-drift / no specs weakened, (2) zero admit, (4) zero assume,
  (5) no cfg-gated exec code, (7) Verus-limitation reproducer (N/A), (8) exec
  rewrites (none), (9) cross-module regression, (10) verification + build.
- Current FAIL: (3)/(6) external_body audit — the 3 upool `external_body` are
  *listed* in `tcb-allowed.md`, but their listed justification is stale and
  self-contradictory. Strict reviewer will not license an `external_body` on a
  rationale that is factually false.
- Remaining: none unchecked.

### Verification

Command run: `make verify-kernel` → exit 0, `kernel::all` cached PASS.
Result: **86 verified, 0 errors, 0 warnings** (the only `note:` lines are
informational auto-trigger advisories in `frame.rs`, not warnings).
Cheating scan (whole kernel): `assume=0 external_body=15 admit=7 cfg_gate=12`.

Module-scoped evidence from
`verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt`:
```
mm/phys/upool.rs:221 Upool (struct): external_body
mm/phys/upool.rs:246 new:            external_body
mm/phys/upool.rs:279 alloc:          external_body
```
No `admit`, no `assume`, no other `external_body` in the module
(confirmed by grep over `upool*.rs`).

Per-item findings:

1. **No specs weakened (spec-drift).** PASS.
   `git diff 77e8f3e1b(proving START) HEAD -- upool.rs upool.spec.rs
   upool.proof.rs` is **empty** — the spec surface was untouched for the entire
   proving phase (the `strip-external-body` commit `df0f4b996` and the re-add
   `7ef0c3978` net to zero). Contracts are non-trivial: `UserFrame::share`
   carries the full Ok/Err refcount/allocated disjunction;
   `UserFrame::refcount` pins `count == phys_view().frames.refcounts[self@]`;
   `Upool::alloc` carries the `alloc_one` transition + empty-pool `Err` arm.
   No weakening.

2. **Zero admit.** PASS. 0 in `upool*` (cheating-detail + grep).

3. **Zero external_body unless listed in tcb-allowed — HARD RULE.** **FAIL
   (justification accuracy).** All three (`Upool` struct L220/221, `Upool::new`
   L241/246, `Upool::alloc` L262/279) ARE listed, so the literal rule is met.
   BUT the listing rationale in `verus-ai-logs/tcb-allowed.md` (section
   "Cross-module dependencies trusted until their module is verified") is now
   **false and self-contradictory**:
   - Entry says `Upool` (struct) and `Upool::new` have **"no specs yet"** — false:
     `new` now carries `ensures result@.wf()`.
   - Entry says `Upool::alloc` is **"Verified when `upool` is."** — but `upool`
     is being verified *right now* and `alloc` remains `external_body`. The
     proving phase even *attempted* the strip (`df0f4b996` removed `new`/`alloc`
     `external_body`) and had to **re-add** it (`7ef0c3978`) because verification
     failed. So the promise in the tcb entry cannot be fulfilled by this module.

   The `external_body` themselves are *design-forced* (the `Upool` view is
   `uninterp spec fn view()` over the global `FrameAllocView`; `wf()` is a
   non-trivial conjunction that cannot be discharged from an uninterpreted view,
   and `frame::alloc`'s contract speaks of `phys_view().frames`, not `self@`, so
   the transition can only be bridged by the deferred §8 ghost token). That is
   legitimate and mirrors the verified-`Inner` + `external_body`-singleton-wrapper
   pattern already used in `frame.rs`. The problem is purely that the **tcb-allowed
   rationale lies about the current state**, and a strict reviewer cannot accept a
   trust boundary licensed by a false justification.

4. **Zero assume / assume_specification.** PASS. Global `assume=0`; grep shows
   none in `upool*`.

5. **No cfg-gated exec code (branches/expressions/match arms).** PASS. The only
   `cfg(verus_keep_ghost)` gates in `upool.rs` are: spec/proof `include!` (L9/L11),
   the `verus! { ... }` ghost block holding the `View` impls (L37 — *spec* code,
   the mandatory ghost idiom, not exec), and the `error!` log inside `Drop::drop`
   (L203, the sanctioned non-ghost logging pattern the scanner explicitly
   excludes). No exec branch / expression / match arm is cfg-gated.

6. **Cheating audit (exact counts/locations) — upool module:**
   - `admit`: **0**
   - `external_body`: **3** — `Upool` struct `upool.rs:221`, `Upool::new`
     `upool.rs:246`, `Upool::alloc` `upool.rs:279`
   - `assume`/`assume_specification`: **0**
   - cfg-gated **exec** code: **0** (1 ghost `verus!` block + 1 sanctioned log
     macro, neither is exec cheating)
   Reported. The actionable defect is the external_body justification (item 3).

7. **Claimed Verus limitation has an isolated reproducer.** PASS / N/A. The
   module claims **no** Verus parser/feature limitation: there is no
   `// VERUS REWRITE`, no `verus-unsupported.md` entry for upool. The three
   `external_body` are justified by the uninterp-view / ghost-token *design*, not
   by a Verus limitation, so no isolated reproducer is owed.

8. **Exec rewrites minimal & semantically equivalent (`// VERUS REWRITE`).**
   PASS. `grep "VERUS REWRITE" upool*.rs` → none. The exec bodies (`UserFrame::*`,
   `Drop::drop`, `Upool::*`) are the original, unrewritten implementations.

9. **Cross-module regression (`make verify-kernel`, all modules).** PASS.
   `kernel::all` cached PASS, exit 0; `kernel::mm::phys` reports 86 verified, 0
   errors. No module regressed.

10. **Verification + build, 0 errors / 0 warnings.** PASS. `make verify-kernel`
    exit 0; `grep -c "warning:"` on the verus log = 0; build `Finished` cleanly.
    (Informational auto-trigger `note:` lines in `frame.rs` are not warnings and
    are out of scope for upool.)

### Fix Request

One concrete change is required before this turn can be RESOLVED.

**Update `verus-ai-logs/tcb-allowed.md` so the three `upool` `external_body`
entries are accurate and non-contradictory.** Move them out of the
"Cross-module dependencies trusted until their module is verified" section
(whose premise — "trusted until their module is verified" — is now false, since
upool *is* the module under verification) and document them as **permanent
thin-facade trust boundaries**, using the same wording class already applied to
the `frame.rs` singleton wrappers (e.g. `frame::alloc` …"external_body until the
free-function layer is verified"). Specifically:

1. Delete/replace the stale claims:
   - Remove "no specs yet" for `Upool` (struct) and `Upool::new`. State the
     real contract: `Upool`'s `View` is `uninterp` over the global
     `FrameAllocView`; `Upool::new` carries `ensures result@.wf()`, which is a
     non-trivial `FrameAllocView::wf()` conjunction that is **unprovable from an
     uninterpreted view**, hence the body is an assumed §8 ghost-attachment axiom.
   - Remove "Verified when `upool` is" for `Upool::alloc`. State that `alloc`
     delegates to `frame::alloc` (itself `external_body`) and that its postcondition
     speaks of `self@` (uninterp) while `frame::alloc`'s contract speaks of
     `phys_view().frames`; the bridge is the **deferred §8 ghost token in the
     free-function layer**, so `alloc` stays `external_body` *until the frame
     free-function layer is verified* — exactly like its `frame.rs` siblings, NOT
     "when upool is verified".

2. Keep the entries listed (they remain legitimately trusted), just with correct
   rationale.

Do **not** instead delete the `external_body` attributes — that path was already
tried this phase (`git show df0f4b996`) and reverted (`git show 7ef0c3978`)
because verification fails without them; an explanation that they "can't be
removed" is not a substitute for fixing the doc.

**Verify the fix:**
- `grep -n "no specs yet\|Verified when .upool. is" verus-ai-logs/tcb-allowed.md`
  must return nothing for the upool entries.
- `git diff -- verus-ai-logs/tcb-allowed.md` shows the three upool entries
  reclassified with accurate rationale.
- `make verify-kernel` still exits 0 with `external_body=15 admit=7 cfg_gate=12`
  (unchanged — this is a documentation correction, no code change).

All other checklist items are PASS. STOP will be created once item 3's doc fix is
applied and re-verified.
