# Final Verification Review — `hal-page-aligned` (Claude, independent/strict)

Date: 2026-06-15
Reviewer: Claude (independent final verification)
Branch: `verus-ai-prove`
Target module: `src/kernel/src/hal/mem/types/address/aligned/page.rs`
In-scope functions: `PageAligned::from_address`, `PageAligned::into_raw_value`, and the `PageAligned` type (View + `inv`).

---

## Spec Quality

**In-scope external-top contracts are correct, complete, non-tautological, and non-operational.**

### `PageAligned::from_address` — `page.rs:42-48`
```
ensures match ret {
    Ok(r)  => spec_aligned(addr@) && r@ == addr@ && r.inv(),
    Err(e) => !spec_aligned(addr@) && e.code == ErrorCode::BadAddress,
}
```
- `spec_aligned(v) := v % spec_page_size() == 0` (`page.spec.rs:15-17`).
- **Validate-not-normalize** is correctly captured: `Ok(r) => r@ == addr@` (value preserved, no silent re-align) and `r.inv()` (`inv` = `self@ % spec_page_size() == 0`, `page.rs:226-229`).
- **Bidirectional error**: `Err <=> !spec_aligned(addr@)` — the failure condition is the abstract negation of success, not a transcription of the `is_aligned(PAGE_ALIGNMENT)?` check. The extra `e.code == ErrorCode::BadAddress` is acceptable (matches the documented `Err(BadAddress)` API contract; callers may ignore it but it is not over-constraining).
- **Adversarial test:** a no-op (`Ok(Self(addr))` always) fails the `Err`/`!spec_aligned` arm → rejected; an align-down normalizer fails `r@ == addr@` → rejected; an always-`Err` fails the liveness implicit in the `Ok` arm being reachable for aligned inputs (the spec forces `Err => !spec_aligned`, so an aligned input cannot return `Err`) → rejected. Spec is sufficiently strong.
- Soundness of the proof chain confirmed end-to-end:
  - `Address::is_aligned` trait spec (`sys/.../address/mod.rs:135-140`): `Ok(aligned) && aligned == spec_addr_is_aligned(self@, align)`, with `spec_addr_is_aligned(v, a) := v % spec_align_value(a) == 0` (`address/mod.spec.rs:8-10`).
  - `PAGE_ALIGNMENT` model (`page.spec.rs:7-10`): `spec_align_value(PAGE_ALIGNMENT) == spec_page_size()`.
  - ⇒ `aligned == (addr@ % spec_page_size() == 0) == spec_aligned(addr@)`, closing both arms.

### `PageAligned::into_raw_value` — inherited from `Address` trait decl (`sys/.../address/mod.rs:63-67`)
```
ensures result as int == self@
```
- The `impl<T: Address> Address for PageAligned<T>` block is now `#[verus_verify]`'d (`page.rs:63-64`); `into_raw_value` (`page.rs:65-67`, body `self.0.into_raw_value()`) is **verified in-body** against the inherited contract, not trusted. `self@ == self.0@` (View, `page.rs:240-243`) + inner `T::into_raw_value` ensures `result as int == self.0@` discharges it.
- A total, value-preserving projection — exactly the upstream `FrameAddress::into_raw_value` expectation.

### View / `inv`
- `view(&self) -> int { self.0@ }` is `closed` (`page.rs:240-243`); `inv` is `open` (`page.rs:226-229`) so downstream layers can rely on page alignment. Matches `view_design.md` and `FrameAddress`'s mirror model. Scalar `int` View is justified (single caller-observable quantity).

**Verdict: Spec quality PASS.**

---

## Caller Coverage (Covered 2/2 in-scope; all caller expectations met)

From `caller_analysis.md`:

| Caller expectation | Spec clause covering it | Status |
|---|---|---|
| `from_address` `Ok` ⇒ wrapped addr page-aligned (`result.inv()`) | `Ok(r) => r.inv()` (`page.rs:45`) | ✅ |
| `from_address` `Ok` ⇒ value preserved (`result@ == addr@`, no rounding) | `Ok(r) => r@ == addr@` (`page.rs:45`) | ✅ |
| `from_address` `Err` ⇒ input not aligned, no value produced | `Err(e) => !spec_aligned(addr@)` (`page.rs:46`) | ✅ |
| `FrameAddress::from_raw_value` relies on `Ok => fa.inv()` (delegates to `from_address`) | `Ok(r) => r.inv()` | ✅ |
| `into_raw_value` returns `usize == self@` (total, side-effect-free) | trait `ensures result as int == self@` | ✅ |
| `FrameAddress::into_raw_value` body `self.0.into_raw_value()` relies on `result as int == self@` | inherited contract, verified in-body | ✅ |
| Type `PageAligned` used as type-level proof of alignment (`inv`) | `inv` open spec | ✅ |

**Missing list: NONE.** Out-of-scope callers (Deref-based, `align_up/down`, ordering) are correctly excluded per task scope; `Deref::deref` retains a trusted contract (`page.spec.rs:32-37`) for callers that need it.

---

## Proof Completeness

- `admit()` count in the 3 target files: **0** (locations: none). `page.proof.rs` is empty (`verus! { }`).
- `external_body` NOT in `tcb-allowed.md`: **0** (locations: none). The module has **zero** `external_body` of any kind (`page.rs`/`page.spec.rs`/`page.proof.rs`).
- Crate-wide `verify-kernel` reports `admit=16 external_body=19`, but `cheating-detail.txt` contains **no `aligned/page` entries** — every admit/external_body is in other (out-of-scope) modules (e.g. `mm/virt/identity_map`, `mm/phys/*`).

**Verdict: Proof completeness PASS.**

---

## TCB Compliance: YES

The module has no `external_body`. The 2 `assume_specification` external-bottom boundaries are both on the fixed allowlist:
- `::arch::mem::PAGE_ALIGNMENT` (`page.spec.rs:7-10`) — listed `tcb-allowed.md:168-178` (external `arch` const; `ensures spec_align_value(result) == spec_page_size()`).
- `<PageAligned<T> as Deref>::deref` (`page.spec.rs:32-37`) — listed `tcb-allowed.md:186` (`core` trait, no Verus contract to inherit; `ensures (*result)@ == a@`).

No new/unlisted trust boundary introduced.

---

## Guardrails Compliance

Counts across `page.rs` + `page.spec.rs` + `page.proof.rs`:

- `admit`: **0**
- `assume`: **0**
- `external_body`: **0**
- `assume_specification`: **2** — `page.spec.rs:7` (`PAGE_ALIGNMENT`), `page.spec.rs:32` (`Deref::deref`). Both in `tcb-allowed.md` (allowed external-bottom). ✅
- `cfg-gated exec`: **0**. `#[cfg(verus_keep_ghost)]` appears at `page.rs:9`, `page.rs:11` (gate `include!` of `page.spec.rs`/`page.proof.rs`) and `page.rs:219` (gates the `inv()` `verus!` ghost block). All three gate ghost/spec material only; none change EXEC behavior. The `View` block (`page.rs:234-246`) is an ungated plain `verus!` block (compiles in both modes). No `cfg(not(verus_keep_ghost))` exec shims exist.

**No blocker-class guardrail violations.**

---

## AST Consistency: PASS (0 MISMATCH)

Tool: `scripts/ast_consistency.py --base-ref de24f6057` (original = parent of `4e5637663 [verus-ai] caller-analysis START`).

```
Consistent: ✅ YES (matched=17 mismatched=0 missing=0 extra=1)
```
- In-scope `PageAligned::from_address` → **MATCH**, `PageAligned::into_raw_value` → **MATCH**, struct `PageAligned` → **MATCH**.
- 0 MISMATCH (the blocker condition). No `// VERUS REWRITE` / `// VERUS DEVIATION` comments exist in the module (grep: no matches) — nothing to semantically re-check.
- 1 `EXTRA_IN_VERUS`: `PageAligned::clone_address` (`page.rs:69-71`). This is **out-of-scope** and is a mechanical impl of the **new** `Address::clone_address` trait method added during the broader address-layer verification (trait decl `sys/.../address/mod.rs:84-88`, `ensures result@ == self@`). It is additive (does not alter any in-scope exec logic) and carries its own trait-level spec. `fn_coverage.py` corroborates: 17/17 source fns matched, 1 extra `clone_address` (6 callers, JUSTIFY). Not a blocker.

`spec_drift.py git-diff --before HEAD`: **✅ No contract drift detected** (0 functions changed; working tree == HEAD).

---

## Verification: PASS (0 errors)

Exact commands and results:

1. `cd /home/ruize/nanvix-phy-specs && make verify-kernel` → **Exit code 0**.
   - Summary: `verification: cached (no recompilation), — (exit 0)`; `cheating: assume=0 external_body=19 admit=16 ... cfg_gate=19` (all in out-of-scope modules; none in `aligned/page`).
   - Module note printed: `verifying module hal::mem::types::address::aligned::page`.
   - HEAD commit `1d4c8b8c1` (working tree unmodified for `page.rs`) records: `page (11 verified, 0 errors)`. The immediately prior commit `8208109b6` was `verify FAIL (10 verified, 1 errors)` — i.e. the 11th function (`into_raw_value`, in the now-`#[verus_verify]`'d trait impl) went FAIL→PASS.
2. `./z build -- check-kernel` → **Exit 0**, `"build-finished","success":true`, `[OK] Build complete.` — confirms normal-mode (non-Verus) exec still compiles (kernel crate recompiled, `fresh:false`).
3. `make build` → no-op target (`Nothing to be done`); the real build target is `all-kernel`/`check-kernel` (used above).

No build-lock conflict encountered.

---

## Bug Summary

**Total recorded in `bugs.md`: 1** (VERUS-TOOL-1).

### VERUS-TOOL-1 — Verus panic verifying generic trait impl with `#[verus_spec]` trait method
- **Classification (bug-reporting skill): False Positive / Verus tool limitation** (not a Nanvix code bug; `bugs.md` itself states "No Nanvix source logic is wrong").
- **Reconciliation verdict: STALE / RESOLVED.** `bugs.md` (Jun 14) claims `impl<T: Address> Address for PageAligned<T>` could not be `#[verus_verify]`'d, leaving `into_raw_value` trusted via the trait-decl spec. The **current** code contradicts this: `page.rs:63` is `#[verus_verify]` and `into_raw_value` (`page.rs:65-67`) is verified in-body. Git history confirms the resolution during the proving phase: `8208109b6` FAIL (10 verified) → `1d4c8b8c1` PASS (11 verified). The previously-required `assume_specification` for `PageAligned::<T> as Address::into_raw_value` (still listed `tcb-allowed.md:185`) has been **removed** from `page.spec.rs` (only `PAGE_ALIGNMENT` + `Deref::deref` remain), consistent with the fix; the comment at `page.spec.rs:19-23` documents that `into_raw_value` "is verified in-body … and no longer needs a trusted specification here."
- **Recording gap (non-blocking):** the resolution was reflected in code, `view_design.md` (lines 225-258 still describe `into_raw_value` as "tool-blocked / trusted") and `bugs.md` (Status: open) are now **out of date**. This is a documentation-hygiene issue, not a verification blocker. `tcb-allowed.md:185` likewise lists a boundary that no longer exists in the module (stale allowlist entry; harmless since unused).

**True bugs (any severity): NONE.** No surviving verification failure exists in the in-scope module (0 errors, 0 admit). No unrecorded bugs were discovered during this review.

---

## Issues (highest priority first)

1. **[Informational, non-blocking] Stale documentation.** `bugs.md` (VERUS-TOOL-1 Status: open) and `view_design.md:225-258` describe `into_raw_value` as tool-blocked/trusted, but it is now verified in-body. `tcb-allowed.md:185` still lists the removed `PageAligned … into_raw_value` assume_specification. Recommend marking VERUS-TOOL-1 resolved and pruning the stale allowlist entry. Does not affect soundness.
2. **[Informational, non-blocking] `clone_address` EXTRA_IN_VERUS.** A new exec method added to satisfy the broadened `Address` trait (out-of-scope here). Additive, spec'd at the trait level, 6 callers. Flagged JUSTIFY by `fn_coverage`; acceptable as part of the address-layer trait redesign, not an in-scope logic change.
3. **[Style, non-blocking] Attribute-style annotations.** The module uses `#[verus_verify]`/`#[verus_spec]` (attribute style) rather than `verus! { }` blocks for function contracts, contrary to the `verus-constraints` preference for `verus! { }`. This is the established, consistent convention across the whole address/`Address`-trait layer in this repo (and is auto-convertible), so it is noted, not penalized.

No blocker-class issues.

---

## Result: PASS

Zero blockers: admit=0, assume=0, external_body outside `tcb-allowed.md`=0, AST MISMATCH=0, `verify-kernel` exit 0 with 0 errors (11 verified for the module), and all in-scope caller expectations covered. The only findings are non-blocking documentation-staleness and one out-of-scope additive method.
