## Turn 1: Full checklist walkthrough (cheating-elimination, hal-page-aligned)

### Progress
- Done (PASS): admit, assume(), trusted, no_decreases, cfg-gated exec, external_body,
  AST consistency, exec-rewrite comments, surviving-external_body TCB, spec drift,
  cross-module regression, verification + build (0 errors / 0 warnings).
- Current: TCB documentation completeness for `assume_specification`.
- Remaining: none unverified — 1 FAIL to fix (see Fix Request).

### Scope
Files reviewed:
- `src/kernel/src/hal/mem/types/address/aligned/page.rs`
- `src/kernel/src/hal/mem/types/address/aligned/page.spec.rs`
- `src/kernel/src/hal/mem/types/address/aligned/page.proof.rs`

### Verification (commands run + evidence)

**`make verify-kernel`** → exit 0. `note: verifying module hal::mem::types::address::aligned::page`.
No `warning` lines in the verus log. Whole-kernel cheating counters
(`admit=16 external_body=19 cfg_gate=19`) are **all in other modules**
(`mm/phys/*`, `mm/virt/*`, `hal/.../frame.proof.rs`, `hal/.../phys.proof.rs`).
`verus-logs/cheating-detail.txt` (35 entries) contains **zero** `aligned/page` entries.

**`./z build -- all`** → exit 0, `[OK] Build complete.` No rustc warnings (only a benign
"sysroot directory not found; skipping symlink update" tooling note).

Per-item:

| # | Item | Result | Evidence |
|---|------|--------|----------|
| 1 | Zero `admit()` | PASS | `grep admit` over the 3 files → none. Not in cheating-detail.txt. |
| 2 | Zero `assume()` | PASS | `grep 'assume('` → none. Tool: `assume=0`. |
| 3 | Zero trusted functions | PASS (with finding, see #6/Fix) | Tool `trusted=0`. Two `assume_specification` (external boundaries) discussed below. |
| 4 | Zero `exec_allows_no_decreases_clause` | PASS | `grep` → none. Tool: `no_decreases=0`. |
| 5 | Zero cfg-gated exec code | PASS | Only 3 `#[cfg(verus_keep_ghost)]`: lines 9/11 = `include!` of spec/proof (ghost), line 219 = `verus!{}` ghost `inv` spec. No exec under cfg. |
| 6 | Zero `external_body` unless TCB-listed | PASS (module) | `grep external_body` → none in module; absent from cheating-detail.txt. |
| 7 | AST consistency: zero mismatches | PASS | `ast_consistency.py … summary`: `matched=17 mismatched=0 missing=0 extra=1`, verdict **Consistent: ✅ YES**; exec-only diffs = 0. The 1 `EXTRA_IN_VERUS` is `PageAligned::clone_address`, now a **required** method of the `Address` trait (`sys/mm/address/mod.rs` has `#[verus_spec] fn clone_address(&self) -> Self`). Impl body `PageAligned(self.0.clone_address())` is a faithful, minimal trait impl required to compile — not an exec-logic rewrite. |
| 8 | Exec rewrites have VERUS REWRITE comment + reproducer | PASS (N/A) | exec-only AST diff = 0 → no exec rewrites exist. |
| 9 | Each surviving `external_body` in TCB | PASS (N/A) | No `external_body` in module. |
| 10 | No specs weakened | PASS | `spec_drift.py check` (baseline `24143f2635bb` → HEAD): Functions with changes 0, contract drift 0, ensures removed 0, requires added 0. Exit 0. |
| 11 | Cross-module regression (`verify-kernel` all) | PASS | `make verify-kernel` exit 0; all modules cached/verified, 0 errors. Other-module admits/external_body are pre-existing TCB items, not regressions from this module. |
| 12 | Verification + build: 0 errors, 0 warnings | PASS | verify exit 0, no warnings; `./z build -- all` exit 0, no rustc warnings. |

### Finding (FAIL): undocumented `assume_specification` in the TCB list

`page.spec.rs` declares **two** trusted external specs via `assume_specification`:

1. `<PageAligned<T> as ::core::ops::Deref>::deref` (line 32) — **documented** in
   `verus-ai-logs/tcb-allowed.md:173`
   (`crate::hal::mem::PageAligned::<T> as Deref::deref`). ✅
2. `::arch::mem::PAGE_ALIGNMENT` (line 7) — **NOT documented** anywhere in
   `tcb-allowed.md` (`grep PAGE_ALIGNMENT tcb-allowed.md` → no match, exit 1). ❌

This project tracks `assume_specification` entries in `tcb-allowed.md` (it has a dedicated
`## assume_specification ...` section listing sibling arch constants such as
`::arch::mem::FRAME_SIZE`). The `PAGE_ALIGNMENT` trusted contract is a genuine TCB element
(an external arch-crate constant whose value Verus cannot resolve), so it is legitimate and
necessary — but it must be **enumerated and justified** in the TCB allowed list like
`FRAME_SIZE` is. An undocumented trusted spec is an incomplete TCB record and fails the
cheating-elimination bar.

### Fix Request

Add an entry for the `::arch::mem::PAGE_ALIGNMENT` `assume_specification` to
`/home/ruize/nanvix-phy-specs/verus-ai-logs/tcb-allowed.md`, in the
`## assume_specification ...` → "External crate (`arch`)" subsection, alongside
`::arch::mem::FRAME_SIZE`. The entry must state:

- The declaration site: `src/kernel/src/hal/mem/types/address/aligned/page.spec.rs:7`.
- Why it is external/unverifiable: `PAGE_ALIGNMENT` is a constant in the external `arch`
  crate; Verus cannot resolve its value.
- The trusted contract it assumes:
  `ensures ::sys::mm::spec_align_value(result) == spec_page_size()` — i.e. it pins the
  arch alignment's numeric value to `spec_page_size()`, the link `from_address` relies on
  to relate `is_aligned(PAGE_ALIGNMENT)` to `spec_aligned(addr@)`.

(Optional but recommended: also refresh the stale note in the `assume_specification`
section header that claims these declarations "live in `src/kernel/src/mm/phys/frame.spec.rs`"
— the `Deref::deref` entry now lives in `page.spec.rs`, per the comment at `page.spec.rs:26-37`.)

Do **not** weaken or remove the `assume_specification` (it is sound and required) — the fix
is documentation only.

**Verify after fix:**
- `grep -n PAGE_ALIGNMENT verus-ai-logs/tcb-allowed.md` → must return the new entry.
- `make verify-kernel` → exit 0, page module still verifies.
- Re-confirm `ast_consistency.py … summary` (still Consistent: YES) and
  `spec_drift.py check nanvix-phys-hal-page-aligned` (exit 0) are unaffected.

### Remaining
None unverified. 11/12 items PASS; 1 documentation FAIL pending the fix above. STOP not
created (item unresolved).
