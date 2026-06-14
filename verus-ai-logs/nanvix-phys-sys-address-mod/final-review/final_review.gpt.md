# Final Review — sys-address-mod
- model = gpt-5.3-codex

## Spec Quality (per-function)

### 1) `from_raw_value` (trait declaration)
- Contract in `src/libs/sys/src/sys/mm/address/mod.rs:54-60`:
  - `Ok(a) => a@ == raw_addr as int`
  - `Err(e) => e.code == ErrorCode::BadAddress`
- Assessment: non-tautological; success and error arms both specified using `match` (good spec-design pattern).
- Limitation: no success/failure predicate (no liveness condition such as when `Ok` must happen). This is a real strength gap for range reasoning.
- Soundness of dropped bidirectional range arm: **sound to drop**. `view_design.md:200-207` documents that `PhysicalAddress` can reject sparse addresses even when `raw <= max_addr`; thus `Err <=> raw > max_addr` would be false.

### 2) `into_raw_value` (trait declaration)
- Contract in `src/libs/sys/src/sys/mm/address/mod.rs:63-66`:
  - `result as int == self@`
- Assessment: strong and direct; exactly what callers need for lossless extraction.

### 3) `is_aligned` (trait declaration)
- Contract in `src/libs/sys/src/sys/mm/address/mod.rs:135-139`:
  - `result matches Ok(aligned) && aligned == spec_addr_is_aligned(self@, align)`
- Helper predicate definition in `src/libs/sys/src/sys/mm/address/mod.spec.rs:8-10`:
  - `spec_addr_is_aligned(v, align) := v % spec_align_value(align) == 0`
- Assessment: declarative and strong; pins exact boolean semantics and forces `Ok` arm.

## Caller Coverage

Covered **5/8** expectations from `verus-ai-logs/nanvix-phys-sys-address-mod/caller_analysis.md`.

Covered:
1. `Ok(a) => a@ == raw` for `from_raw_value` (caller_analysis.md:40-43,91-93) — covered by mod.rs:57.
2. `Err => BadAddress` for `from_raw_value` (caller_analysis.md:41-43,92-93) — covered by mod.rs:58.
3. `into_raw_value` exactness (caller_analysis.md:43-44,98-100) — covered by mod.rs:65.
4. `is_aligned` boolean meaning (caller_analysis.md:45-46,105-106,123) — covered by mod.rs:137-139 + mod.spec.rs:8-10.
5. “Err reserved for invalid alignment” (caller_analysis.md:108-109) — trait spec is stronger (`is_aligned` always returns `Ok(...)`).

Missing (genuinely not expressed in these trait contracts):
1. Bidirectional/range-style success condition for `from_raw_value` (caller_analysis.md:72-73,121-123).
2. Explicit `Eq/Ord` coherence with `@` (caller_analysis.md:124).
3. Explicit wrapper-forwarding preservation facts for `PageAligned`/`PageTableAligned` (caller_analysis.md:125-126) at this trait interface.

## Proof Completeness

- `mod.proof.rs` is effectively empty (`src/libs/sys/src/sys/mm/address/mod.proof.rs:1` is `verus! { }`).
- Cheating primitives in `src/libs/sys/src/sys/mm/address` (exact command output):

```text
$ cd /home/ruize/nanvix-phy-specs-bottom-up && dir=src/libs/sys/src/sys/mm/address && echo "[admit]" && rg -n "\badmit\s*!?\(" "$dir" || true && echo "count=$(rg -n "\badmit\s*!?\(" "$dir" | wc -l)" && echo "[assume]" && rg -n "\bassume\s*!?\(" "$dir" || true && echo "count=$(rg -n "\bassume\s*!?\(" "$dir" | wc -l)" && echo "[external_body]" && rg -n "external_body" "$dir" || true && echo "count=$(rg -n "external_body" "$dir" | wc -l)" && echo "[assume_specification]" && rg -n "assume_specification" "$dir" || true && echo "count=$(rg -n "assume_specification" "$dir" | wc -l)" && echo "[trusted]" && rg -n "\btrusted\b" "$dir" || true && echo "count=$(rg -n "\btrusted\b" "$dir" | wc -l)" && echo "[exec_allows_no_decreases_clause]" && rg -n "exec_allows_no_decreases_clause" "$dir" || true && echo "count=$(rg -n "exec_allows_no_decreases_clause" "$dir" | wc -l)" && echo "[cfg attrs]" && rg -n "#\[cfg[^\]]*\]" "$dir" && echo "count=$(rg -n "#\[cfg[^\]]*\]" "$dir" | wc -l)"
[admit]
count=0
[assume]
count=0
[external_body]
count=0
[assume_specification]
count=0
[trusted]
count=0
[exec_allows_no_decreases_clause]
count=0
[cfg attrs]
src/libs/sys/src/sys/mm/address/virt.rs:9:#[cfg(verus_keep_ghost)]
src/libs/sys/src/sys/mm/address/virt.rs:11:#[cfg(verus_keep_ghost)]
src/libs/sys/src/sys/mm/address/virt.rs:39:#[cfg(target_pointer_width = "32")]
src/libs/sys/src/sys/mm/address/virt.rs:296:#[cfg(target_pointer_width = "32")]
src/libs/sys/src/sys/mm/address/mod.rs:9:#[cfg(verus_keep_ghost)]
src/libs/sys/src/sys/mm/address/mod.rs:11:#[cfg(verus_keep_ghost)]
count=6
```

- `external_body-not-in-TCB`: **0** (no `external_body` occurrences in scope, so none to reconcile against `verus-ai-logs/tcb-allowed.md`).

## TCB Compliance

**YES**
- No `external_body` in `src/libs/sys/src/sys/mm/address`.
- Therefore no new trust boundary introduced and no non-approved TCB usage.

## Guardrails Compliance

- admit: **0**
- assume: **0**
- external_body: **0**
- assume_specification: **0**
- cfg-gated-exec: **0** (all 6 `#[cfg(...)]` hits are benign exceptions explicitly allowed: `#[cfg(verus_keep_ghost)] include!` lines and `#[cfg(target_pointer_width)]` in out-of-scope `virt.rs`).

(From `make verify-sys` summary too: `cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0`.)

## AST Consistency

**PASS**

Exact command output:

```text
$ cd /home/ruize/nanvix-phy-specs-bottom-up && python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py src/libs/sys/src/sys/mm/address/mod.rs
✅ All exec functions consistent.
# AST Consistency Report: ast_orig_h7bm2m0o
...
- **Consistent: YES**
```

`// VERUS REWRITE` findings:

```text
$ rg -n "//\s*VERUS REWRITE" src/libs/sys/src/sys/mm/address/mod.rs
No matches found.
```

## Verification

**PASS** (error count: **0**, exit code **0**)

Exact command output excerpt:

```text
$ cd /home/ruize/nanvix-phy-specs-bottom-up && make verify-sys
...
=== Results ===
  cached (no recompilation)
  —
  Exit code : 0
...
=== Summary ===
  verification: cached (no recompilation), — (exit 0)
  cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0
  coverage: 2/255 exec functions have contracts
  status: CLEAN
```

Spec drift check:

```text
$ cd /home/ruize/nanvix-phy-specs-bottom-up && python3 /home/ruize/verus-ai-exp/verus-ai/scripts/spec_drift.py git-diff src/libs/sys/src/sys/mm/address/mod.rs --before HEAD
# Spec Drift Report
...
- **Contract drift (⚠ review required): 0**
...
**✅ No contract drift detected.**
```

## Bug Summary

- Prompt-specified path `verus-ai-logs/nanvix-phys-sys-address-mod/bugs.md` is absent in this repo snapshot.
- Available module bug log read at `verus-ai-logs/nanvix-phys-sys-address-mod/specification/bugs.md` reports no bugs:
  - line 3: `No fundamentally incorrect code found.`
  - line 15: `Status: clean.`
- No unrecorded real code defect found in `mod.rs` trait declarations for in-scope methods.

## Issues (priority order)

1. **P1 (spec strength gap):** `from_raw_value` lacks an abstract success predicate/liveness condition (when success is guaranteed). This limits callers proving success conditions generically.
2. **P2 (interface coverage gap):** trait-level contracts do not encode Eq/Ord coherence with `@` or wrapper preservation facts; those expectations remain outside this module’s contracts.
3. **P3 (artifact path mismatch):** requested bugs path is missing; actual bug log is under `specification/bugs.md`.

## Result

**PASS**

All explicit blockers are clear: `admit=0`, `assume=0`, no `external_body`, AST consistent, verify-sys exit 0, and checklist checks executed.
