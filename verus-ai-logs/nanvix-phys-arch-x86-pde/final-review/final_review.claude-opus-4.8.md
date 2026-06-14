# Final Verification Review — `arch-x86-pde`

- **Reviewer:** independent strict final verification (claude-opus-4.8)
- **Date:** 2026-06-15
- **Repo:** `/home/ruize/nanvix-phy-specs-bottom-up`
- **Branch:** `verus-ai-prove-bottom-up`
- **Module files:**
  - `src/libs/arch/src/x86/mem/paging/pde.rs`
  - `src/libs/arch/src/x86/mem/paging/pde.spec.rs`
  - `src/libs/arch/src/x86/mem/paging/pde.proof.rs`
- **In-scope functions (ONLY):** `PageDirectoryEntry::new`, `PageDirectoryEntryFlags::new`,
  `PageDirectoryEntry::is_present`, `PageDirectoryEntryFlags::is_present`,
  `PageDirectoryEntry::frame_address`

---

## Checklist

- [x] **1. Spec quality** — all 5 in-scope external-top contracts are correct, complete, declarative, caller-usable.
- [x] **2. Caller coverage** — 5/5 in-scope functions covered; all 6 caller invariants discharged. No failure paths exist (all total).
- [x] **3. Proof completeness** — `admit()` = 0, `external_body` in pde files = 0.
- [x] **4. TCB compliance** — 3 crate `external_body` (invlpg, table::read, table::write); all on tcb-allowed.md; none in pde.
- [x] **5. AST consistency** — `✅ Consistent: 23 functions, 2 structs match.` No `// VERUS REWRITE` comments to check.
- [x] **6. Verification** — `make verify-arch`: 47 verified, 0 errors (exit 0). Cross-module `make verify`: exit 0.
- [x] **7. Guardrails** — admit=0, assume=0, assume_specification=0, external_body(pde)=0, uninterp=0, cfg-gated EXEC=0.
- [x] **8. Bug reconciliation** — bugs.md = "None"; reconciled against final code; no surviving/undocumented failures.

---

## Spec Quality

Read `pde.spec.rs` and every `#[verus_spec]` `ensures` in `pde.rs`. Assessment per in-scope function:

| Function | Contract (ensures) | Verdict |
|---|---|---|
| `PageDirectoryEntryFlags::new` | `result@ == spec_pde_flags_new(present,…,page_size)` | **Good.** Records all 8 flag args faithfully via `spec_*_set` projections. Total, no `requires`. Declarative; written from signature + module purpose (caller invariant 1). |
| `PageDirectoryEntry::new` | `result@ == spec_pde_new(flags@, frame@)` and `result.inv()` | **Good.** Pairs *exact* flags with *exact* frame, and establishes the frame-bound type invariant (needed so `frame_address` is overflow-free/total). Total constructor (caller invariant 2). |
| `PageDirectoryEntry::is_present` | `result == self@.flags.present` | **Good.** Delegates presence to flags view (caller invariant 3). Pure/total. |
| `PageDirectoryEntryFlags::is_present` | `result == self@.present` | **Good.** Internal-only consumer; underpins the entry-level presence spec. Pure/total. |
| `PageDirectoryEntry::frame_address` | `result as int == self@.frame * FRAME_SIZE` and `result % FRAME_SIZE == 0` | **Good.** Derives physical base from the frame index (never stored) and guarantees page alignment (caller invariants 2 & 4). Total, no `requires`; bound comes from `FrameNumber` type invariant. |

Spec-design principles satisfied:
- **View is `closed`** (`PageDirectoryEntryFlags`, `PageDirectoryEntry`) → bit-packing hidden ⇒ encoding independence (caller invariant 6).
- **Abstract types**: `PdeFlagsView` (8 `bool`), `PdeView { flags, frame: int }`. No `Vec`/raw-word leakage.
- **`inv()`**: flags `inv` is vacuously `true` (no cross-field constraint — correct); entry `inv` = `0 <= frame <= FrameNumber::spec_max()`, the single real constraint, inherited verbatim from `FrameNumber`.
- **No anti-patterns**: no one-sided error specs (functions are total — no Result/Option in scope), no tautologies, no operational/code-as-spec, no subsumed clauses, no `uninterp`.
- **Caller-equivalence check (`frame_address`)**: caller_analysis expects `frame_address() == frame.into_raw_value() << FRAME_SHIFT`. `FrameNumber::into_raw_value` ensures `result as int == self@` (number.rs:80–83) and the PDE view sets `frame == self.frame@`. Since `FRAME_SIZE == 2^FRAME_SHIFT`, `self@.frame * FRAME_SIZE == into_raw_value() << FRAME_SHIFT`. Equivalence is sound. ✅

---

## Caller Coverage

Source: `caller_analysis.md`. The 5 in-scope functions and their caller expectations:

| # | Function | Caller expectation | Mapped contract | Covered |
|---|---|---|---|---|
| 1 | `PageDirectoryEntryFlags::new` | records all 8 flags; total pure ctor; `is_present()==(present==Present)` | `result@ == spec_pde_flags_new(...)` ⊕ `is_present` ensures | ✅ |
| 2 | `PageDirectoryEntry::new` | pairs exact flags+frame; `is_present()==flags.is_present()`; `frame_address()==frame<<FRAME_SHIFT`; total | `result@ == spec_pde_new(flags@,frame@)`, `result.inv()` | ✅ |
| 3 | `PageDirectoryEntry::is_present` | returns the present bit as constructed; pure/total guard | `result == self@.flags.present` | ✅ |
| 4 | `PageDirectoryEntryFlags::is_present` | true iff `present == Present` (internal) | `result == self@.present` | ✅ |
| 5 | `PageDirectoryEntry::frame_address` | physical base of frame; page-aligned; inverse of `new`'s frame | `result == self@.frame*FRAME_SIZE`, `result%FRAME_SIZE==0` | ✅ |

Caller "Key Invariants" 1–6 reconciliation:
1. flags ctor fidelity → covered (1 ⊕ 4). 2. entry ctor fidelity → covered (2 ⊕ 3 ⊕ 5).
3. presence delegation → covered (3 references `self@.flags.present`). 4. frame alignment → covered (5, `% FRAME_SIZE == 0`).
5. purity/totality → all 5 have no `requires`, no Result/Option, no mutation. 6. encoding independence → `closed` views.

**Covered: 5/5. Missing: none.** Failure-path coverage: **N/A** — every in-scope function is total (no `Err`/`None` return), confirmed by signatures; success-only specs are correct here.

> Note: `find_callers_output.md` LSP reported 0 external callers (false negative — cross-crate `kernel` callers not indexed). Cross-module `make verify` (below) exercises the real kernel consumers via `identity_map.spec.rs` `assume_specification` and passes.

---

## Proof Completeness

Command:
```
grep -c "admit"  pde.rs/spec/proof      → 0 / 0 / 0
grep -rn external_body  (pde files)     → (none)
```
- **`admit()` count = 0** across all three pde files. (No BLOCKER.)
- **`external_body` in pde files = 0.** (No BLOCKER.)
- `pde.proof.rs` discharges `lemma_frame_address` fully (vstd `lemma_usize_shl_is_mul`, `lemma2_to64`, div/mod + nonlinear lemmas) — no placeholders.

---

## TCB Compliance

```
$ grep -rn external_body src/libs/arch/src/
src/libs/arch/src/x86/mem/paging/mod.rs:79     #[verus_verify(external_body)]  (invlpg)
src/libs/arch/src/x86/mem/paging/table.rs:202  #[verus_verify(external_body)]  (read)
src/libs/arch/src/x86/mem/paging/table.rs:241  #[verus_verify(external_body)]  (write)
```
Fresh-run cheating detail (`verus-ai-logs/verify-arch/verus-logs/cheating-detail.txt`):
```
- x86/mem/paging/mod.rs:80   invlpg: external_body
- x86/mem/paging/table.rs:209 read:  external_body
- x86/mem/paging/table.rs:246 write: external_body
```
Cross-checked against `verus-ai-logs/tcb-allowed.md`:
- `mod.rs::invlpg` — listed ("speccing `arch::x86::mem::paging` (`mod.rs`)"). ✅
- `table.rs::read` — listed ("speccing `arch::x86::mem::paging::table`"). ✅
- `table.rs::write` — listed (same section). ✅

**All 3 are on the allow-list; none are in pde files. No unlisted `external_body`. PASS.**

---

## Guardrails Compliance (exact counts, pde files only)

| Dimension | pde.rs | pde.spec.rs | pde.proof.rs | BLOCKER? |
|---|---:|---:|---:|---|
| `admit` | 0 | 0 | 0 | admit>0 ⇒ no |
| `assume(` | 0 | 0 | 0 | assume>0 ⇒ no |
| `external_body` | 0 | 0 | 0 | no |
| `assume_specification` | 0 | 0 | 0 | no |
| `uninterp` | 0 | 0 | 0 | no |
| cfg-gated **EXEC** code (`cfg(not(verus_keep_ghost))`) | 0 | 0 | 0 | no |

**Ghost-include classification:** `pde.rs:9` and `pde.rs:11` are `#[cfg(verus_keep_ghost)]` guarding
`include!("pde.spec.rs")` / `include!("pde.proof.rs")`. These are **GHOST includes**, not cfg-gated exec
code — correctly classified as non-cheating. No EXEC bodies/match-arms/expressions are cfg-gated.

Whole-crate guardrails (fresh `make verify-arch`): `assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=0`.
(`verification_todo.md` confirms the 2 previously cfg-gated exec items in `pde.rs` were eliminated during cheating-elimination; current cfg_gate=0.)

---

## AST Consistency — PASS

```
$ python3 .../ast_consistency.py src/libs/arch/src/x86/mem/paging/pde.rs count
✅ Consistent: 23 functions, 2 structs match.
```
No `// VERUS REWRITE` / `// VERUS DEVIATION` / `// VERUS BUG FIX` comments present in any pde file
(grep exit 1, zero matches) ⇒ no exec rewrites to audit for semantic equivalence. **No MISMATCH.**

Supporting tools:
- `fn_coverage.py`: Source exec fns 15 / Verus exec fns 15 / **Matched 15, Missing 0, Extra 0**.
- `spec_drift.py git-diff … --before HEAD`: **✅ No contract drift detected** (0 functions changed, 0 ensures removed, 0 requires added).

---

## Verification — PASS

Fresh, non-cached run (touched all three pde files to force recompilation):
```
$ make verify-arch
    Checking arch v0.16.17 ...
verification results:: 47 verified, 0 errors
=== Results === 47 verified / 0 errors / Exit code : 0
cheating: assume=0 external_body=3 admit=0 trusted=0 no_decreases=0 cfg_gate=0
```
Cross-module:
```
$ make verify   (all crates + kernel)
arch:   47 verified, 0 errors (exit 0)  cheating: external_body=3
kernel: 76 verified, 0 errors (exit 0)  cheating: admit=36 external_body=12 cfg_gate=15  [OUT OF SCOPE]
bitmap/sys/nanvix-slab/bump-allocator/raw-array: exit 0
```
The arch crate (home of pde) verifies cleanly. The kernel crate's residual cheating
(admit/external_body/cfg_gate) lives in **other, out-of-scope modules**; the kernel still
verifies exit 0, confirming the pde external-top contracts are consumed soundly by the real
callers (`identity_map.spec.rs` `assume_specification`s).

---

## Bug Summary

`bugs.md` states **"None."** Reconciliation against final code:
- No `admit`/`assume`/`external_body` in pde files, no code changes required — consistent with bugs.md.
- `frame_address` overflow-freedom is genuinely discharged by `lemma_frame_address` using the
  `FrameNumber::spec_max()` bound — matches the bugs.md narrative.
- No surviving unresolved verification failures (verify-arch = 0 errors). `verification_todo.md` = "None. Zero proof gaps remain."
- **No undocumented bugs found** during this review. No True-Bug / Context-Dependent classification required.

---

## Issues (priority order)

**None.** No blockers, no majors, no minors. (Out-of-scope observation only: the kernel crate
carries admit=36/external_body=12/cfg_gate=15 in other modules — not part of this review and
does not affect the pde module's verification.)

---

## Result: **PASS**

All 8 checklist items pass with concrete evidence:
- admit=0, assume=0 (BLOCKER gates clear)
- 0 unlisted `external_body` (3 crate items all on tcb-allowed.md; 0 in pde files)
- AST consistency ✅ (23 fns / 2 structs), no exec rewrites, 0 spec drift, 0 missing fns
- `make verify-arch` = 47 verified / 0 errors (fresh, non-cached); `make verify` exit 0
- Caller coverage 5/5, all 6 caller invariants discharged, no missing expectations
- bugs.md reconciled ("None"), no undocumented failures

**Verdict: PASS — the `arch-x86-pde` module is soundly and completely verified for all in-scope functions.**
