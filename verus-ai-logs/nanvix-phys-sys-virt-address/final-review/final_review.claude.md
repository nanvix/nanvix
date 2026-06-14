# Final Comprehensive Review (claude-opus-4.8): sys-virt-address

Independent, strict, read-only final verification of module `sys-virt-address`.
Target files:
- `src/libs/sys/src/sys/mm/address/virt.rs`
- `src/libs/sys/src/sys/mm/address/virt.spec.rs`
- `src/libs/sys/src/sys/mm/address/virt.proof.rs`

Branch: `verus-ai-prove-bottom-up`. No source/spec/proof file was modified
(only the source mtime was `touch`ed to force a non-cached re-verification).

In-scope functions (only these judged for spec coverage):
`VirtualAddress::into_raw_value`, `VirtualAddress::from_raw_value` (inherent),
`VirtualAddress::new`, and the type `VirtualAddress` (View + inv).

---

## Checklist

### Caller Analysis
- [x] `caller_analysis.md` exists and enumerates in-scope items with call sites
  and expectations (new ×8, inherent from_raw_value ×5, `Address::into_raw_value`
  ×3, type ×32). Round-trip identity flagged as the KEY invariant.

### View Design
- [x] View exists and is sound: `impl View for VirtualAddress { type V = int;
  closed spec fn view(&self) -> int { self.0 as int } }` (virt.rs:321–328).
  Matches the abstract resource (a `usize`-tagged integer). No `inv` is required
  — the newtype accepts every `usize` (totality), so there is no representation
  invariant to state; absence of `inv` is correct, not a gap.

### Specification
- [x] `VirtualAddress::new` has `ensures result@ == value as int` (virt.rs:48–51).
- [x] `VirtualAddress::from_raw_value` (inherent) has `ensures result@ ==
  raw_addr as int` (virt.rs:65–68).
- [ ] **`VirtualAddress::into_raw_value` (i.e. `Address::into_raw_value`, the
  ONLY `into_raw_value` defined, virt.rs:253) has NO `#[verus_spec]` / ensures.**
  The enclosing `impl Address for VirtualAddress` (virt.rs:167) is not even
  annotated with `#[verus_verify]`. This is an in-scope function with 3 external
  callers → **coverage gap → checklist item UNCHECKED**.

### Proving
- [x] 0 `admit()` in target files.
- [x] `make verify-sys` → 6 verified, 0 errors, exit 0 (fresh, non-cached run).

### Cheating Elimination
- [x] admit=0, assume=0, external_body=0, assume_specification=0, cfg-gated
  exec (cheating)=0 across the three target files. verify.sh cheating check:
  `✅ No cheating detected`.

### Bug Recording
- [x] No verification failures were discovered (6 verified / 0 errors); `bugs.md`
  legitimately does not exist (no true bugs to record). The missing
  `into_raw_value` spec is a coverage gap, not a bug.

**Net: one checklist item (Specification / into_raw_value caller coverage) is
UNCHECKED.**

---

## Spec Quality

Specs present are correct and idiomatic (attribute style, the repo norm):

```
#[verus_spec(result => ensures result@ == value as int)]      // new        (virt.rs:48)
#[verus_spec(result => ensures result@ == raw_addr as int)]   // from_raw   (virt.rs:65)
```

- Not tautological, not subsumed, not one-sided: each binds the abstract view of
  the result to the integer argument — exactly the "purity / no hidden state"
  and "constructor equivalence" invariants from `caller_analysis.md`.
- Not code-as-spec: `result@` is the abstract View (`int`), not the concrete
  field; the `closed` view body keeps the representation hidden while the
  `ensures` still exposes the needed `@ == arg` fact to callers.
- Totality is encoded structurally (return type `Self`, no `requires`, no
  `Result`/panic) — appropriate for an infallible constructor; no error-path
  spec is owed.

**Defect:** `into_raw_value` has no `ensures`. The caller-critical
**round-trip identity** `from_raw_value(x).into_raw_value() == x` /
`new(x).into_raw_value() == x` is therefore NOT provable by callers. Because the
View is `closed`, external callers cannot relate `into_raw_value`'s `usize`
result to `self@` without an explicit `ensures result == self@` (or
`result as int == self@`). The two constructor specs alone are insufficient to
discharge what `mm/mmio.rs:67` and `pm/sync.rs:37,65` rely on.

---

## Caller Coverage

Covered: **3/4** in-scope items.

| In-scope item | Callers | Spec present | Status |
|---|---|---|---|
| `VirtualAddress::new` | 8 | `ensures result@ == value as int` | Covered |
| `VirtualAddress::from_raw_value` (inherent) | 5 | `ensures result@ == raw_addr as int` | Covered |
| `Address::into_raw_value` | 3 (mmio.rs:67, sync.rs:37, sync.rs:65) | **none** | **MISSING** |
| type `VirtualAddress` | 32 refs | View present (no inv needed) | Covered |

Missing: **[`VirtualAddress::into_raw_value` / `Address::into_raw_value`]** — no
`#[verus_spec]` ensures (virt.rs:253; impl block virt.rs:167 lacks
`#[verus_verify]`). This breaks the round-trip identity that `caller_analysis.md`
designates the KEY invariant for the module.

Evidence:
```
$ grep -n "verus_spec\|into_raw_value" virt.rs
48:    #[verus_spec(result =>
65:    #[verus_spec(result =>
253:    fn into_raw_value(self) -> usize {     <-- no annotation above it
```

---

## Proof Completeness

- `admit`: **0** (target files). `grep -n admit virt.rs virt.spec.rs virt.proof.rs` → none.
- `external_body` not in tcb-allowed.md: **0** (there are 0 external_body total in
  the target files, so the tcb list is not engaged).
- `virt.spec.rs` and `virt.proof.rs` are both empty harness shells
  (`verus! { } // verus!`) — all specs live inline via attributes.

---

## TCB Compliance

**YES.** Zero `external_body` in the target files; nothing needs to be (or is)
listed in `verus-ai-logs/tcb-allowed.md`. No new TCB entries introduced.

---

## Guardrails Compliance

Counts across `virt.rs`, `virt.spec.rs`, `virt.proof.rs`:

| Dimension | Count |
|---|---|
| admit | 0 |
| assume | 0 |
| external_body | 0 |
| assume_specification | 0 |
| cfg-gated exec (cheating) | 0 |

Notes on `#[cfg(...)]` usage (all benign, none are cheating):
- virt.rs:9,11 — `#[cfg(verus_keep_ghost)] include!("virt.spec.rs"/"virt.proof.rs")`
  (standard ghost-harness include).
- virt.rs:39,296 — `#[cfg(target_pointer_width = "32")]` on a `static_assert` and a
  `From<VirtualAddress> for u32` impl (legitimate platform gating, present in the
  pre-verification baseline, not used to hide exec from Verus).

verify.sh confirms: `cheating: assume=0 external_body=0 admit=0 trusted=0
no_decreases=0 cfg_gate=0 … status: CLEAN`.

admit=0 and assume=0 → no BLOCKER on this dimension.

---

## AST Consistency

**PASS (semantically). Tool reports 3 nominal mismatches that are name-collision
false positives; the exec code is byte-identical to baseline.**

`ast_consistency.py … summary`:
```
⚠️  3 mismatched (16 functions match)
VirtualAddress::align_down   MISMATCH
VirtualAddress::align_up     MISMATCH
VirtualAddress::is_aligned   MISMATCH
Consistent: ❌ NO (matched=16 mismatched=3 missing=0 extra=0)
```

Root cause (investigated, not assumed): `align_up`, `align_down`, `is_aligned`
are each defined **twice** in this file — an inherent method (`-> Self` /
`-> bool`) and the trait method `Address::…` (`-> Result<…, Error>`). The
checker keys functions by bare name and pairs the inherent definition against the
trait definition, e.g.:
```
$ ast_consistency.py … diff --name VirtualAddress::align_down
-    pub fn align_down(&self, align: Alignment) -> Self {
-        VirtualAddress::new(mm::align_down(self.0, align))
+    fn align_down(&self, align: Alignment) -> Result<Self, Error> {
+        Ok(self.align_down(align))
```
This is the inherent-vs-trait pair, not a verification-introduced change.

Ground-truth diff of all exec code vs the pre-verification baseline
(`git diff ca7e88be8 HEAD -- virt.rs`) shows the ONLY changes are: added
`#[verus_verify]`, two `#[verus_spec]` ensures (new, from_raw_value), an impl
block split, and removal of one duplicate `use ::vstd::prelude::*;`. **No exec
function body was modified.** No `// VERUS REWRITE` comments exist
(`grep "VERUS REWRITE" virt.rs` → none), so there are no rewrites to validate for
semantic equivalence.

Conclusion: no genuine exec divergence; AST consistency is sound. (The checker's
known limitation with overloaded inherent/trait method names is the sole cause of
the reported count.)

---

## Verification

**PASS.** Fresh (cache-busted) run of `make verify-sys`:
```
=== Results ===
  6 verified
  0 errors
  Exit code : 0
=== Summary ===
  verification: 6 verified, 0 errors (exit 0)
  cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0
  coverage: 2/255 exec functions have contracts
  status: CLEAN
```
Log: `verus-ai-logs/verify-sys/verus-logs/verus_2026-06-15_06-00-43.log`.

`spec_drift.py git-diff … --before HEAD` reports "Ensures removed: 1
(`result@ == raw_addr as int` on `from_raw_value`)". This is a **false positive**
of the same dual-`from_raw_value` name collision (inherent has the ensures, trait
`Address::from_raw_value` does not). Ground truth: the pre-verification baseline
had **empty** spec files and zero `#[verus_spec]` annotations, so specs were only
**added**, never weakened. No original guarantee was weakened.

---

## Bug Summary

Total true bugs: **0**.

- `bugs.md` does not exist for this module (`ls` → No such file or directory) —
  correct, since no verification failure or unsoundness was found.
- No verification failure surfaced during proving (6 verified / 0 errors).
- The `into_raw_value` missing spec is explicitly **not** a bug per the
  bug-reporting skill — it is a specification/coverage gap.
- No surviving unresolved verification failure to classify.

---

## Issues (highest priority first)

1. **[BLOCKER — coverage gap] In-scope `VirtualAddress::into_raw_value`
   (`Address::into_raw_value`, virt.rs:253) has no `#[verus_spec]` ensures.**
   The enclosing `impl Address for VirtualAddress` (virt.rs:167) is not annotated
   with `#[verus_verify]`. Three external callers (mmio.rs:67, sync.rs:37,
   sync.rs:65) depend on it as the inverse of construction; without
   `ensures result == self@` (or `result as int == self@`), the module's KEY
   round-trip-identity invariant is unprovable by callers. This leaves
   Caller Coverage at 3/4 and the Specification checklist item unchecked.

2. **[INFO — tooling] AST `summary` shows 3 mismatches and `spec_drift` shows 1
   "ensures removed".** Both are false positives caused by overloaded
   inherent/trait method names (`align_up`/`align_down`/`is_aligned`/
   `from_raw_value` each defined twice). Verified harmless via
   `git diff ca7e88be8 HEAD` (no exec body changed; specs only added). No action
   on the code; noted so a reviewer does not mistake them for real regressions.

---

## Result: **FAIL**

Rationale (strict): a PASS requires every checklist item checked. The
Specification / Caller-coverage item for the in-scope function
`VirtualAddress::into_raw_value` is **UNCHECKED** — it has no `#[verus_spec]`
ensures, so caller coverage is 3/4 and the module's designated key invariant
(round-trip identity) cannot be discharged by its callers. All other dimensions
pass cleanly (6 verified / 0 errors, admit=0, assume=0, external_body=0, TCB
clean, no genuine AST divergence, no true bugs), but the single unchecked item
forces an overall **FAIL**.
