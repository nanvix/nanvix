# Final Verification Review — `hal::mem::types::address::phys` (`PhysicalAddress`)

**Reviewer:** independent strict final verification (claude)
**Date:** 2026-06-15
**Branch:** `verus-ai-prove`
**Baseline (review START / original tip):** `b1a278fc7`
**In-scope targets:** `PhysicalAddress` (type / `View` / `inv`), `PhysicalAddress::from_number`,
`PhysicalAddress::into_frame_number`, `PhysicalAddress::from_mmio_address`.

> Methodology: every claim below was checked with tools (verus runs, AST/spec-drift
> scripts, grep, git). Prior-phase claims were **not** taken at face value. Two
> experiments were run (assume_specification removed) and the source was fully
> restored afterwards (verified byte-identical to `b1a278fc7`).

---

## TL;DR Verdict: **FAIL** (single, well-isolated, upstream-rooted blocker)

The three in-scope specs are correct, complete, caller-adequate, AST-consistent, drift-free,
and the **full** `make verify-kernel` passes deterministically (97 verified, 0 errors). The phys
module contributes **zero** `admit`/`assume()`/`external_body`/`cfg-gated-exec`.

It fails strict review on **one** dimension: `phys.spec.rs:61` carries an
`assume_specification` for `<::sys::mm::VirtualAddress as ::sys::mm::Address>::into_raw_value`
that is **genuinely required** (proven by experiment) yet targets a **workspace-internal
crate (`sys`)** and is **not recorded in `tcb-allowed.md`**. Per the `spec-design` checklist
("Never [assume_specification] for workspace-internal code"; "only external-bottom trust
boundaries for std/external crates allowed") and the task's "no new/undeclared trust boundaries"
rule, this is an unmet checklist criterion → FAIL. The proper fix is **upstream** (one
attribute in the `sys` crate); the phys module cannot eliminate it in-scope.

---

## Dimension 1 — Spec Quality (external-top API specs)

Source under review (`phys.rs`, exec) + `phys.spec.rs` (contracts) + `phys.proof.rs` (lemmas).

### `from_mmio_address(addr: VirtualAddress) -> Result<Self, Error>` (`unsafe`) — phys.rs:112–122
```
requires spec_frame_number(addr@) <= spec_max_frame_number(),
ensures  result is Ok,
         (result->Ok_0)@ == addr@,
         (result->Ok_0).inv(),
```
- **Correct & rejects bugs.** `result@ == addr@` pins the identity wrap; a body returning a
  different address or `Err` would be rejected. `result is Ok` is an accurate *total-success*
  statement (body is `Ok(Self(addr))`), not a one-sided error spec — the error arm is provably
  unreachable, which is stronger than "say nothing about Err".
- **`requires` justified, not weakening.** The frame-representable precondition is what lets the
  function deliver `result.inv()` (needed so callers may later call `into_frame_number` /
  `PageAligned::from_address`). It matches caller reality (MMIO addrs such as LAPIC `0xFEE0_0000`
  → frame `0xFEE00` ≤ max). It is the **original** spec (zero drift, see Dim 5).
- **Minor (non-blocking) observation:** `result.inv()` is *derivable* by a caller from
  `result@ == addr@` + the `requires` (spec-design anti-pattern #9 "subsumed"). I judge it
  *acceptable* — it is the key fact callers consume, and surfacing it directly satisfies
  spec-design principle #8 ("written for the caller"; an ensures the caller uses without
  re-deriving). Keep, but noted.

### `from_number(frame: FrameNumber) -> Self` — phys.rs:138–156
```
ensures result@ == spec_from_number(spec_frame_raw_value(frame)),   // == frame@ * spec_page_size()
```
- **Correct, declarative, caller-complete.** Exactly the caller contract
  (`result@ == frame.into_raw_value() * FRAME_SIZE`). Totality (no `Result`) matches signature.
  `FRAME_SIZE`-alignment and the `from_number ∘ into_frame_number` round-trip are **derivable**
  from this single product (a caller divides by `spec_page_size()`), so no extra clauses needed —
  good (no over-specification).
- Overflow-freedom of the internal multiply is discharged by `lemma_from_number_no_overflow`
  (proof.rs:17), which consumes `FrameNumber::into_raw_value`'s verified bound
  `0 <= self@ <= spec_max()` (arch, `number.rs:79–84`). Sound.

### `into_frame_number(self) -> FrameNumber` — phys.rs:160–175
```
requires self.inv(),
ensures  spec_frame_raw_value(result) == spec_frame_number(self@),  // == self@ / spec_page_size()
```
- **Correct & rejects bugs.** Pins the result to `self@ / FRAME_SIZE`; determinism /
  per-frame injectivity / inverse-of-`from_number` all follow from this functional equality
  (bitmap-index and PTE callers rely on exactly this). `requires self.inv()` is the precise,
  minimal precondition that underwrites the internal `FrameNumber::from_raw_value(..).unwrap()`
  (totality), proven via `lemma_frame_index` (proof.rs:50) + `lemma_usize_shr_is_div`.
- The shift-vs-divide detail is correctly abstracted to `/ spec_page_size()` (spec is simpler
  than code). Good.

### Type / `View` / `inv` — phys.rs:311–319, phys.spec.rs:43–45
- `View::V = int`, `view() = self.0@` (`closed`) — caller-abstract integer address, matches
  `view_design`/`caller_analysis`. `inv() := spec_frame_number(self@) <= spec_max_frame_number()`
  is `open` so allocator/page-table callers can rely on `into_frame_number` totality. Correct.

**Spec-quality verdict: PASS.** No tautological clauses; no one-sided error specs; the lone
subsumed-ish clause (`from_mmio` `inv()`) is a defensible caller-convenience.

---

## Dimension 2 — Caller Coverage (vs `caller_analysis.md`)

| Function | Caller expectation | Covered by | OK |
|---|---|---|---|
| type | opaque `Copy`/`Ord` integer (`view():int`); usable through `Address`; valid frame number | `View V=int`, `inv()` | ✓ |
| `from_number` | total (no `Result`) | signature `-> Self` | ✓ |
| `from_number` | `result@ == frame.into_raw_value()*FRAME_SIZE` | `ensures result@ == spec_from_number(spec_frame_raw_value(frame))` | ✓ |
| `from_number` | `FRAME_SIZE`-aligned base (for `PageAligned::from_address`) | derivable from product | ✓ |
| `from_number` | round-trip with `into_frame_number` | derivable | ✓ |
| `into_frame_number` | total / never panics for valid addr | `requires self.inv()` + verified `unwrap` | ✓ |
| `into_frame_number` | `result == self@ >> FRAME_SHIFT == self@/FRAME_SIZE` | `ensures spec_frame_raw_value(result)==spec_frame_number(self@)` | ✓ |
| `into_frame_number` | deterministic / per-frame injective; inverse of `from_number` | derivable from functional equality | ✓ |
| `from_mmio_address` | `Ok`, `result@ == addr@` (identity, bypasses RAM validator) | `ensures result is Ok, result@==addr@` (no RAM `requires`) | ✓ |
| `from_mmio_address` | succeeds outside RAM; `unsafe` contract; `Err` benign | no RAM check; `unsafe` kept; `Err` provably unreachable | ✓ |

**Coverage: 3/3 in-scope functions + type fully covered. No missing caller expectations.**

---

## Dimension 3 — Proof Completeness

`grep -nE 'admit\(|external_body' phys.rs phys.spec.rs phys.proof.rs` →
- `admit()`: **0**
- `external_body`: **0**

No `admit()` and no `external_body` anywhere in the three phys files. ✅ (No blocker.)

---

## Dimension 4 — TCB Compliance

The phys module declares **no** `external_body`, so there is nothing requiring a
`tcb-allowed.md` entry on that axis. ✅
(Caveat: the lone `assume_specification` is a *different* trust-boundary axis — see Dim 7 &
Special Investigation; it is **not** listed in `tcb-allowed.md`.)

---

## Dimension 5 — AST Consistency & `// VERUS REWRITE` review

```
$ python3 .../ast_consistency.py --base-ref verus-ai-prove .../phys.rs count
✅ Consistent: 17 functions, 1 structs match.   (matched=17 mismatched=0 missing=0 extra=0)
```
All exec functions and the struct MATCH — no MISMATCH. The two in-scope `// VERUS REWRITE`
sites were checked by hand against the **pre-verification** baseline (`git show dev:…`):

- `from_number` (phys.rs:142–156): original `let addr = frame.into_raw_value()*FRAME_SIZE;`
  → split into `let addr_raw = frame.into_raw_value(); proof!{…}; let addr = addr_raw*FRAME_SIZE;`.
  **Semantically identical** (same operands, same `*`, same order); it is the pre-approved
  deviation `f(complex_expr)` → `let x = complex_expr; f(x)`, mandated because `FrameNumber`'s
  type invariant is private to `arch` so the bound must enter context via `into_raw_value`'s
  postcondition. ✓
- `into_frame_number` (phys.rs:166–175): exec **byte-identical** to `dev`; only a `proof!{…}`
  block was inserted. ✓
- `from_mmio_address`: exec identical (`Ok(Self(addr))`). ✓

(The out-of-scope `clone_address` "VERUS REWRITE (interface addition)" is a mandatory `Address`
trait method from the `sys` crate; not in scope, exec is a trivial view-preserving copy — AST MATCH.)

**AST verdict: PASS.**

Additional integrity check — spec drift vs original `b1a278fc7`:
```
$ python3 .../spec_drift.py git-diff .../phys.rs --before b1a278fc7
Functions with changes: 0 ; Contract drift: 0 ; ✅ No contract drift detected.
```

---

## Dimension 6 — Verification (`make verify-kernel`, exit 0)

**Important caveat discovered:** the *cached* result is misleading. At review start the repo was
in a **stale-cache PASS** state (`9f9e0ff8c … cached (no recompilation), exit 0`). Forcing a clean
recompile is required to get ground truth.

Clean **full** `make verify-kernel` runs with the **original** source (assume_specification
present):

| Run | Mode | Result |
|---|---|---|
| 09:48:00 (`run1`, `touch phys.rs`) | full, fresh | **97 verified, 0 errors, exit 0** ✅ |
| 09:~50 (`run2`, `touch` all 3 phys files) | full, fresh | **97 verified, 0 errors, exit 0** ✅ |
| final (`touch phys.rs`) | full, fresh | **97 verified, 0 errors, exit 0** ✅ |

→ The full-crate verification **deterministically PASSES** (reproduced 3×, exit 0).

**phys-module contribution to the crate-wide cheating totals (`admit=12 external_body=19
cfg_gate=19`): ZERO.** The crate-wide `cheating-detail.txt` lists only `mm/phys/*`,
`mm/virt/*`, etc. (a *different* module — the frame allocator); grepping it for the address
module returns nothing. The module-scoped guardrail check confirms:
```
✅ No cheating detected in module hal::mem::types::address::phys.
```

**Two anomalies observed (secondary findings, not blockers for the full build):**
1. **`make verify-kernel MODULE=hal::mem::types::address::phys` reliably FAILS** (exit 101)
   with the `into_raw_value` "ignored … external" error, while the **full** build PASSES. The
   module-scoped invocation does not load the `sys` dependency's verus metadata the same way, so
   the cross-crate trait-impl resolution breaks. Module-scoped verify is **unreliable** here; the
   full build is authoritative.
2. One **full** clean build immediately after `touch` failed once (09:43:29) then passed on the
   next build (09:44:08) with identical source — a transient build-cache state while the `sys`
   dependency was rebuilt. Subsequent clean builds passed deterministically. Worth noting as
   build-cache fragility, but the steady-state result is PASS.

(Side note: `make verify-kernel` **auto-commits** every run. My two experiments were therefore
auto-committed by the pipeline; I restored the source and the working tree now matches
`b1a278fc7` exactly — see "Working-tree integrity" below.)

---

## Dimension 7 — Guardrails (all cheating dimensions, phys module)

`grep -nE 'admit\(|assume\(|assume_specification|external_body|external_fn_specification|external_type_specification|exec_allows_no_decreases|#\[cfg\(' phys.rs phys.spec.rs phys.proof.rs`:

| Dimension | Count | Locations |
|---|---|---|
| `admit()` | **0** | — |
| `assume(...)` (statement) | **0** | — |
| `external_body` | **0** | — |
| `assume_specification` | **1** | `phys.spec.rs:61` (`<sys::mm::VirtualAddress as Address>::into_raw_value`) |
| `external_*_specification` | 0 | — |
| `exec_allows_no_decreases` | 0 | — |
| cfg-gated **exec** | **0** | `phys.rs:9,11` are `#[cfg(verus_keep_ghost)] include!("…spec/proof.rs")` scaffolding — *not* gated exec code |

Hard blockers (`admit>0` / `assume()>0` / `external_body` ∉ tcb): **none**. ✅
Remaining flag: the single `assume_specification` — see Special Investigation below.

---

## Dimension 8 — Bug Reconciliation (vs `bugs.md`)

`bugs.md` records **"None."** Reconciled against final code: accurate.
- The two non-trivial obligations (`from_number` multiply overflow-freedom; `into_frame_number`
  shift==divide + frame-number fit) were genuine **proof gaps**, both discharged
  (`lemma_from_number_no_overflow`, `lemma_frame_index`). Not code defects.
- `from_mmio_address`'s `requires` is a precondition (context-correct), not a bug.
- No undiscovered bugs found. The specs are bug-rejecting (Dim 1). **Bug reconciliation: PASS.**

---

## SPECIAL INVESTIGATION — `assume_specification` at `phys.spec.rs:61`

**Question:** is `pub assume_specification[ <::sys::mm::VirtualAddress as ::sys::mm::Address>::into_raw_value ] … ensures result as int == addr@;`
still necessary, or a removable/redundant workspace-internal trust boundary?

### Experiment (authoritative = full build)
- **Original (present)** → full `make verify-kernel`: **97 verified, 0 errors, exit 0**.
- **Removed/commented** → full `make verify-kernel`: **exit 101**, hard compile error:
  ```
  error: cannot use function `sys::sys::mm::address::virt::impl&%1::into_raw_value` which is
  ignored because it is either declared outside the verus! macro or it is marked as `external`.
     --> src/kernel/src/hal/mem/types/address/phys.rs:167:31
         let raw_addr: usize = self.0.into_raw_value();
     = help: pub assume_specification [<sys::mm::VirtualAddress as sys::mm::Address>::into_raw_value] (_0: sys::mm::VirtualAddress) -> usize;
  ```
  (Source was restored after the experiment.)

### Root cause (verified in `sys` source)
- `src/libs/sys/src/sys/mm/address/mod.rs:63–67`: trait `Address::into_raw_value` **does** carry a
  verified `#[verus_spec] ensures result as int == self@`.
- **BUT** `src/libs/sys/src/sys/mm/address/virt.rs:167` `impl Address for VirtualAddress` is
  declared **outside** any `verus!` block (which starts at line 319) and is **not**
  `#[verus_verify]`-annotated. Therefore `<VirtualAddress as Address>::into_raw_value`
  (virt.rs:253) is **external/ignored** to Verus, and the trait's spec does **not** propagate to
  the kernel crate for this impl method.
- Contrast: `VirtualAddress::new` works *without* a placeholder because its `impl VirtualAddress`
  (virt.rs:47) **is** `#[verus_verify]` (virt.rs:46) — that is why the sibling `new`
  `assume_specification` was safely removable but this one was not.

### Conclusion
**(a) Genuinely required** — due to cross-crate trait-method spec resolution: the `sys`
`impl Address for VirtualAddress` block is external to Verus, so the kernel must supply the
contract. It is **load-bearing** (not dead/floating): removing it breaks the build.

**HOWEVER, it still fails the checklist:**
- `sys` is a **workspace crate** (`src/libs/sys`), verified by the same pipeline (`sys::all`
  PASS). The `spec-design` skill is explicit: *"`assume_specification` is only for third-party
  dependencies and std library functions … Never for workspace-internal code (any crate in the
  workspace)."* → checklist item **"No `assume_specification` for workspace-internal code"
  is UNMET**.
- Even under the more lenient `tcb-allowed.md` framing (which classifies the external `arch`
  crate's placeholders as temporary, *and lists them*), this declaration is **absent from
  `tcb-allowed.md`** → an **undeclared trust boundary** ("no new trust boundaries").
- The intended "superseded once the dependency is verified" mechanism is **broken here**: the
  `sys` address module *was* verified, but because its `Address` impl is external, the supersession
  never took effect. This is the regression that turned a stale-cache PASS into a clean-build need
  for the placeholder.

**Required remediation (upstream, out of phys scope):** annotate `impl Address for VirtualAddress`
in `src/libs/sys/.../virt.rs:167` with `#[verus_verify]` (mirroring `impl VirtualAddress` at
line 46). The verified trait spec then propagates and `phys.spec.rs:61` becomes removable (like
`new`'s was). Until then, at minimum the declaration must be **recorded in `tcb-allowed.md`** with
this rationale. Per `review-methodology` ("No justification accepted … needs redesign, not
defense"), the documented "it's required" status does not satisfy the zero-workspace-internal-
`assume_specification` criterion.

---

## Issues (highest priority first)

1. **[BLOCKER — checklist] Workspace-internal `assume_specification`** (`phys.spec.rs:61`,
   `<sys::mm::VirtualAddress as Address>::into_raw_value`). Required by the current build, but
   `sys` is a workspace crate → violates "No `assume_specification` for workspace-internal code",
   and it is **undeclared in `tcb-allowed.md`**. Root cause is upstream (`sys`'s `impl Address for
   VirtualAddress` is external to Verus). Fix: `#[verus_verify]` the sys impl (then delete this
   placeholder), or at minimum record it in `tcb-allowed.md`.
2. **[Secondary] Module-scoped verify is unreliable** —
   `make verify-kernel MODULE=hal::mem::types::address::phys` deterministically FAILS (exit 101)
   on the cross-crate `into_raw_value` resolution while the full build PASSES. Reviewers/CI must
   use the **full** verify for this module; do not trust the module-scoped guardrail's exit code.
3. **[Secondary] Stale-cache & build-cache fragility** — the at-rest "PASS (cached)" can mask a
   need for a clean rebuild; one post-`touch` full build failed transiently before passing. Treat
   cached PASS with suspicion; require a forced clean verify.
4. **[Minor / non-blocking] `from_mmio_address` `result.inv()`** is caller-derivable from
   `result@ == addr@` + the `requires` (mildly subsumed). Acceptable as a caller-convenience;
   noted only for completeness.

---

## Working-tree integrity (left as found)

- `phys.rs`, `phys.spec.rs`, `phys.proof.rs` working-tree content is **byte-identical to
  `b1a278fc7`** (`git diff b1a278fc7 -- …` empty for all three). AST consistent; zero spec drift.
- The `make verify-kernel` pipeline auto-committed each of my runs (including the two
  experiments); the source was restored and the final HEAD (`699f80163`) is a PASS with the
  original specs. No source edits remain. Only this review markdown (and pipeline-generated
  `final-review/*` logs) differ.

---

## FINAL VERDICT: **FAIL** (strict)

**Justification.** The phys module's own work is excellent: the three in-scope specs are correct,
declarative, bug-rejecting and caller-complete; AST is consistent; there is zero spec drift; the
proofs are real (no `admit`); the module contributes **zero** `admit`/`assume()`/`external_body`/
cfg-gated-exec; and the **full** `make verify-kernel` passes deterministically (97 verified, 0
errors, exit 0, reproduced 3×). On the hard blockers defined for this review (`admit>0`,
`assume()>0`, `external_body` ∉ `tcb-allowed.md`) it is clean.

It nonetheless **FAILS strict review** on one explicitly-named checklist criterion: the
**`assume_specification` at `phys.spec.rs:61` is for workspace-internal `sys` code and is
undeclared in `tcb-allowed.md`** ("No `assume_specification` for workspace-internal code" /
"only external-bottom trust boundaries for std/external crates allowed" / "no new trust
boundaries"). It is provably **required** today (build fails without it) only because the `sys`
crate left `impl Address for VirtualAddress` external to Verus — a one-line upstream gap. This is
a real integrity gap that must be closed (preferably by `#[verus_verify]`-ing the sys impl and
deleting the placeholder; otherwise by recording it in `tcb-allowed.md`).

Because a named checklist criterion is unmet, the strict verdict is **FAIL** — with the caveat
that the remediation is small, upstream-rooted, and does not touch the phys module's
(otherwise sound) specs or proofs.
