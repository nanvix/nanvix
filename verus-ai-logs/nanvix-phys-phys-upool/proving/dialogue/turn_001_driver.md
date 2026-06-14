## Turn 1: Full checklist walk-through — phys-upool proving

### Progress
- Done (PASS): all 10 checklist items (see per-item verdicts below)
- Current: full first-pass review complete
- Remaining: none

### Verification (commands run by reviewer)
- `make verify-kernel` → Exit code 0. Modules verified: `mm::phys`, `mm::phys::frame`,
  `mm::phys::kframe`, `mm::phys::manager`, `mm::phys::upool`. Cheating summary (whole crate):
  `assume=0 external_body=18 admit=24 cfg_gate=15`.
- `./z build -- all` → Exit 0, no compiler warnings (only an unrelated
  "Sysroot directory ... not found; skipping symlink update" info line).
- `grep` on `src/kernel/src/mm/phys/upool*.rs` for `admit`/`assume`/`external_body`/`cfg`.
- `cheating-detail.txt`: upool-scoped entries → `upool.rs:221 Upool (struct)`,
  `upool.rs:246 new`, `upool.rs:279 alloc` (all `external_body`).
- Git diff `2a696dd2b` (specification) → `HEAD`, and proving commit `33397d3e1` diff.
- Proving prover log `proving/prover-claude-20260615-032040.txt` (lines 330/344 contain the
  raw Verus error).
- `git log -S` on `verus-ai-logs/tcb-allowed.md` to confirm allowlist provenance.

---

### Item-by-item verdicts

**1. No specs weakened (spec drift)** — **PASS**
Diff specification(`2a696dd2b`)→`HEAD` for the three upool files: every `requires`/`ensures`
was *added or unchanged*; none removed or weakened. The proving commit `33397d3e1` only
toggled the `#[verus_verify(external_body)]` attribute on `Upool::new`/`alloc` (proof concern,
not a spec). `UserFrame::inv` and all six `UserFrame` method contracts are intact and at full
strength. No spec drift.

**2. Zero remaining admit()** — **PASS**
`grep` for `admit` in `upool.rs`/`upool.spec.rs`/`upool.proof.rs` → 0 hits. The 24 crate-wide
admits are all in out-of-scope modules (`frame.rs`, `manager.proof.rs`, `identity_map.*`, address
layer).

**3. Zero external_body unless listed in tcb-allowed** — **PASS**
Three `external_body` in upool, each handled individually:
- `upool.rs:220 Upool (struct)` — listed in `tcb-allowed.md` (line 87). Opaque facade whose real
  state is the global frame allocator; `View` is `uninterp`.
- `upool.rs:241 Upool::new` — listed (line 87). Substantiated by **actual Verus tool output**:
  proving log lines 330/344 show `error: disallowed: constructor for an opaque datatype` when the
  attribute was stripped (proving commit `33397d3e1`). Stripping then re-adding is recorded in git.
- `upool.rs:262 Upool::alloc` — listed (lines 89–91). The `alloc_one` transition is over the
  `uninterp` `Upool::view`, which has no axiom linking it to the global `phys_view()` that
  `frame::alloc` actually mutates; not derivable in a checked body.
Allowlist provenance verified: the upool entries were added in earlier `[verus]` phase commits
(`54a1d5c94`, `c70a2c329`), **not** during this proving phase — so they were not self-added to
evade the rule. HARD RULE satisfied.

**4. Zero assume/assume_specification** — **PASS**
`grep assume` in upool files → 0. The crate cheating summary also reports `assume=0`.

**5. No cfg-gated exec code** — **PASS**
Only one cfg gate in upool exec: `upool.rs:203` `#[cfg(not(verus_keep_ghost))] error!(...)` inside
`Drop::drop`. It gates a pure logging side-effect only; the enclosing `if let Err(e) = frame::free(..)`
branch and the `frame::free` call are verified under Verus. This is the uniform codebase convention
for `error!`/`info!` logging (frame.rs ~20 sites, manager.rs ~9, kframe.rs 1 — all already verified),
an external-bottom logging boundary, not a verification-relevant branch/expression/match-arm gate.

**6. Cheating audit (counts + locations)** — **PASS (reported)**
upool module exact counts:
- `admit` = 0
- `assume` = 0
- `external_body` = 3 → `upool.rs:220` (struct), `upool.rs:241` (`new`), `upool.rs:262` (`alloc`)
- cfg-gated exec = 1 → `upool.rs:203` (`error!` logging in `Drop::drop`)
All three `external_body` are in `tcb-allowed.md`; the cfg gate is logging-only.

**7. Claimed Verus limitation has isolated reproducer / tool evidence** — **PASS**
The only claimed hard Verus limitation (opaque-datatype constructor) is backed by the literal Verus
error in the proving log (`disallowed: constructor for an opaque datatype`, lines 330/344). The
message itself names the exact disallowed construct (constructing an `external_body` datatype in a
checked body), satisfying the "show tool output proving it's correct" alternative. The `Upool::alloc`
case is a structural derivability gap (uninterp view ⊥ `phys_view()`), not a tool crash, and is
documented in `bugs.md`.

**8. Exec rewrites minimal & semantically equivalent** — **PASS**
No `// VERUS REWRITE` comments and no exec-body rewrites in the proving phase. The proving diff
(`33397d3e1`) touches only attributes; all `UserFrame`/`Upool` method bodies are byte-identical to
the specification phase. Nothing to review for semantic equivalence.

**9. Cross-module regression** — **PASS**
`make verify-kernel` (whole crate, not module-scoped) → Exit 0 with all `mm::phys::*` modules
emitting `verifying module` and `Finished`. No regression in previously-verified modules.

**10. Verification + build, 0 errors / 0 warnings** — **PASS**
`make verify-kernel` → exit 0, no warnings. `./z build -- all` → exit 0, no compiler warnings.

---

### Fix Request
None. Every checklist item is PASS with concrete tool/code evidence. The three upool
`external_body` items are genuinely irreducible in this phase (opaque-constructor Verus error +
uninterp-view⊥`phys_view()`), all pre-listed in `tcb-allowed.md`, and the lone cfg gate is a
logging-only external boundary consistent with all sibling verified modules. No specs weakened,
no admit/assume, no exec rewrites.

Result: **RESOLVED.**
