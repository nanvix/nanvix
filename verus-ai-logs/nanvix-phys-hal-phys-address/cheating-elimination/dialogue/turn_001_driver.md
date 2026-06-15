## Turn 1: AST consistency (+ exec-rewrite documentation)

### Progress
- Done (PASS, verified this turn):
  - **Zero admit()** — `grep` over `phys.rs/phys.spec.rs/phys.proof.rs` finds no
    `admit`. The detector's `cheating-detail.txt` lists admit only in
    `frame.proof.rs`, `mm/phys/manager.proof.rs`, `mm/virt/*` — none in `address/phys`.
  - **Zero assume()** — detector `assume=0`; no `assume(` in the three files.
  - **Zero trusted functions** — detector `trusted=0`; none in scope.
  - **Zero exec_allows_no_decreases_clause** — detector `no_decreases=0`.
  - **Zero cfg-gated exec** — the only `#[cfg(verus_keep_ghost)]` in `phys.rs`
    (lines 9, 11) gate `include!("phys.spec.rs")` / `include!("phys.proof.rs")`,
    i.e. ghost spec/proof inclusion (allowed). No exec divergence.
  - **Zero external_body in scope** — detector `cheating-detail.txt` has no
    `address/phys` entry; the only `external_body` text in the files is a comment
    (`phys.spec.rs:52`). The crate-wide `external_body=19` are all out-of-scope
    (`mm/phys/*`, `arch/*`, `bump_allocator`, …).
  - **No surviving in-scope external_body** → TCB-allowed check N/A (nothing to list).
  - **No specs weakened** — `python3 scripts/spec_drift.py check hal-phys-address`
    → exit 0, `Contract drift: 0` (baseline = cheating-elim start `0044c358`).
  - **Cross-module regression** — `make verify-kernel` exit 0; crate-wide cheating
    counts unchanged from base (`admit=14 external_body=19 cfg_gate=19`), all residue
    out-of-scope.
- Current: **AST consistency / exec-rewrite documentation — FAIL (this turn).**
- Remaining (not yet verified): final **Verification + build_cmd 0 errors / 0
  warnings** (build not yet run this review).

### Verification
Ran `make verify-kernel` → exit 0, module `hal::mem::types::address::phys` verified.

Ran the **ast-consistency** skill against the auto-detected pre-verification baseline
(`/tmp/ast_orig…`, extracted from the module's original source; confirmed equal to
`git show dev:…/phys.rs`):

```
python3 scripts/ast_consistency.py src/kernel/src/hal/mem/types/address/phys.rs summary
→ Consistent: ❌ NO (matched=14 mismatched=2 missing=0 extra=1)
```

Three divergences from the original exec interface, all introduced during the
verification pipeline (not present on `dev`):

1. **`PhysicalAddress::from_number` — MISMATCH (undocumented rewrite).**
   - Original (`dev`): `let addr: usize = frame.into_raw_value() * mem::FRAME_SIZE;`
   - Verified: split into `let addr_raw = frame.into_raw_value();` then
     `let addr = addr_raw * mem::FRAME_SIZE;`
   - **No `VERUS REWRITE` comment, no minimal reproducer.** `grep "VERUS REWRITE\|VERUS DEVIATION"` over the files → none.

2. **`PhysicalAddress::into_frame_number` — MISMATCH (insufficiently documented rewrite).**
   - Original (`dev`): `let frame_number: usize = raw_addr >> mem::FRAME_SHIFT;`
   - Verified: `let shift: usize = mem::FRAME_SHIFT;` then `… raw_addr >> shift;`
   - Carries an explanatory comment (“Bind `FRAME_SHIFT` once so the proof relates
     the shift…”) but **not** the required `VERUS REWRITE` tag and **no** minimal
     reproducer demonstrating the Verus limitation.

3. **`PhysicalAddress::clone_address` — EXTRA_IN_VERUS.**
   - Not present on `dev`; the `Address` trait method `clone_address`
     (`src/libs/sys/src/sys/mm/address/mod.rs:88`) and this impl were both added in
     pipeline commit `40a4c4b60`. It is an exec **interface addition** with no
     `VERUS REWRITE`/justification recorded.

The fix report's AST claim (“`git diff verus-ai-prove` empty → byte-identical, zero
mismatches”) is misleading: it diffed against the *same* branch tip. The skill's
auto-detected pre-verification baseline shows the real divergences above.

The two rewrites look semantically equivalent (pre-approved “intermediate value”
deviation), but the checklist requires **zero mismatches OR** documented
verified-Verus-limitation rewrites with a `VERUS REWRITE` comment **and** a minimal
reproducer. Neither requirement is met. Justification text in a report is not a fix.

### Fix Request
Address all three, then re-run the tool to prove zero mismatches.

1. **`from_number` (phys.rs ~142-149):** *Default = restore the original exec.*
   Replace the two-line split with the original single line:
   ```rust
   let addr: usize = frame.into_raw_value() * mem::FRAME_SIZE;
   ```
   (keep the `proof! { lemma_from_number_no_overflow(frame); }` block) and re-run
   `make verify-kernel`. If — and only if — verification then fails, restore the
   minimal `addr_raw` binding and add directly above it:
   ```rust
   // VERUS REWRITE: <one-line reason Verus needs the intermediate binding>
   // Reproducer: verus-ai-logs/nanvix-phys-hal-phys-address/cheating-elimination/repro/from_number.rs
   ```
   and commit that minimal reproducer file.

2. **`into_frame_number` (phys.rs ~159-170):** Same procedure — first try restoring
   `let frame_number: usize = raw_addr >> mem::FRAME_SHIFT;` and re-verify. If Verus
   genuinely cannot relate the shift to `spec_page_size() == pow2(shift)` without the
   `shift` binding, keep the binding but replace the current comment with a proper
   `VERUS REWRITE:` tag plus a committed minimal reproducer
   (`…/cheating-elimination/repro/into_frame_number.rs`) that fails on the original
   form and passes on the rewrite.

3. **`clone_address` (phys.rs ~272-274):** This is an added trait method spanning the
   out-of-scope `sys` trait and `page.rs`/`pgtab.rs`, so it cannot simply be deleted.
   Document it: add a `VERUS REWRITE:` (interface-addition) comment on the impl
   explaining the verified Verus limitation that required replacing `derive(Clone)` /
   `Clone::clone` with an explicit `clone_address` trait method, and record it in
   `verification_todo.md`. If you instead claim it is pre-existing/out-of-scope, show
   tool output proving it (e.g. it exists on the true module baseline) — a verbal claim
   is not accepted.

**Verify after fixing:**
```bash
python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py \
  src/kernel/src/hal/mem/types/address/phys.rs count   # must report 0 mismatched, 0 extra
make verify-kernel                                       # must stay exit 0
grep -rn "VERUS REWRITE" src/kernel/src/hal/mem/types/address/phys.rs
```
Do not proceed to the final build/warnings item until `ast_consistency … count`
reports zero mismatches and zero extras (or every surviving divergence carries a
`VERUS REWRITE` comment + committed reproducer).
