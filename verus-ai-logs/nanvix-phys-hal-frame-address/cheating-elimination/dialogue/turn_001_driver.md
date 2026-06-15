## Turn 1: Full Checklist Walkthrough — Cheating Elimination (hal-frame-address)

Target files:
- `src/kernel/src/hal/mem/types/address/frame.rs` (exec)
- `src/kernel/src/hal/mem/types/address/frame.spec.rs` (spec)
- `src/kernel/src/hal/mem/types/address/frame.proof.rs` (proof)

Branch: `verus-ai/hal-frame-address`
Cheating-elimination phase start SHA: `6b129e276403`

### Progress
- Done (PASS/FIXED): ALL 12 checklist items — see per-item evidence below.
- Current: full checklist verified in one pass.
- Remaining: none.

---

### Verification (per checklist item)

**1. Zero admit() — PASS**
`make verify-kernel` cheating summary: `admit=0`. `grep` over `frame*.rs`
finds `admit` only inside a comment in `frame.proof.rs:29` ("no `admit`, no
`external_body`"), no executable `admit()`.

**2. Zero assume() — PASS**
Cheating summary: `assume=0`. No `assume(` in any frame file.

**3. Zero trusted functions — PASS**
Cheating summary: `trusted=0`. No `#[verifier::trusted]` / `trusted` attribute
in the frame files.

**4. Zero exec_allows_no_decreases_clause — PASS**
Cheating summary: `no_decreases=0`.

**5. Zero cfg-gated exec code (only imports/derives/debug_assert/logging) — PASS**
The only `#[cfg(...)]` attributes in `frame.rs` are:
- L9/L11: `#[cfg(verus_keep_ghost)] include!("frame.spec.rs"/"frame.proof.rs")` — verification-material includes.
- L22: `#[cfg(verus_keep_ghost)] use ::vstd::prelude::*;` — import.
All are imports/includes (allowed). The struct derive `#[derive(Clone, Copy)]`
(L35) is not cfg-gated. No exec logic is gated.

**6. Zero external_body unless listed in tcb-allowed — PASS**
`grep external_body` over `frame*.rs`: zero (only a comment mention at
`frame.proof.rs:29`). The global cheating count `external_body=24` is entirely
in OTHER modules (`mm/phys/*`, `aligned/page.rs`, `mm/virt/identity_map.rs`),
confirmed via `cheating-detail.txt`; none live in `hal/mem/types/address/frame.rs`.
Note: `tcb-allowed.md` (L170) even pre-authorizes `FrameAddress::into_raw_value`
as `external_body`, but the current code body-verifies it instead — strictly
stronger than allowed.

**7. AST consistency — PASS**
`ast_consistency.py ... summary`: matched=6, mismatched=3, missing=0, extra=0.
The 3 mismatches are `from_frame_number`, `from_raw_value`, `into_frame_number`.
Each diff is exactly the pre-approved deviation
`f(complex_expr)` → `let x = complex_expr; f(x)` (binding the inner
`PhysicalAddress` to a local so the bridge lemma can relate `spec_addr` to
`View`). Each carries a `// VERUS DEVIATION (pre-approved: ...)` comment at the
site. Per the **ast-consistency** skill these are semantically-equivalent,
pre-approved rewrites for a real Verus limitation → acceptable.

**8. All exec rewrites have deviation comment + justification — PASS**
All three rewrites (item 7) name the exact pre-approved deviation table entry
and explain why the local binding is required (lemma needs to relate the
universal `spec_addr` projection to the type's `View`). Pre-approved deviations
require only a documenting comment (not a separate minimal reproducer); the
underlying limitation — generic `PageAligned<T>` cannot name a per-`T` `View`,
so a `spec_addr`↔`View` bridge is needed — is recorded in `frame.proof.rs` and
`tcb-allowed.md`.

**9. Each surviving external_body listed in tcb-allowed — PASS (N/A)**
Zero external_body in the frame files (item 6), so nothing to list.

**10. No specs weakened — PASS**
`spec_drift.py check hal-frame-address` (baseline `6b129e276403` → HEAD):
Contract drift = 0 (ensures removed 0, requires added 0, proof drift 0, fns
added/removed 0). Exit 0.

**11. Cross-module regression (verus all) — PASS**
`make verify-kernel` verifies every kernel module; `note: verifying module
hal::mem::types::address::frame` plus all others, `Exit code : 0`. No module
regressed.

**12. Verification + build, 0 errors / 0 warnings — PASS**
- Verus: `make verify-kernel` exit 0.
- Build: forced rebuild of the `kernel` crate (`cargo build ... -p kernel`
  after touching `frame.rs`) → `Finished dev profile`, rc=0, no `warning`/`error`
  lines.

Governed trust boundaries (all registered in `verus-ai-logs/tcb-allowed.md`):
- `assume_specification[ <PhysicalAddress as Address>::from_raw_value ]` (tcb-allowed L285)
- `assume_specification<T:Address> [ <PageAligned<T> as Deref>::deref ]` (tcb-allowed L301)
- `assume_specification[ ::arch::mem::PAGE_SIZE ]` (frame.spec.rs; arch library edge)
- `axiom fn lemma_phys_view_is_spec_addr` (tcb-allowed L333) — counted as
  `trusted=0` by the detector; listed and justified as the `spec_addr`↔`View`
  bridge, removed when `sys::mm::Address` is verified.

### Fix Request
None. Every checklist item is PASS with tool-backed evidence. No code change
requested.

### Verdict
RESOLVED — all 12 checklist items PASS.
