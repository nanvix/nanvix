# Final Verification Review — `hal::mem::types::address::phys` (`PhysicalAddress`)

Independent, strict final review. All claims re-derived from source and tool runs;
shared evidence used only as a cross-check.

In-scope targets: `PhysicalAddress` (type: `View` + `inv`), `from_number`,
`into_frame_number`, `from_mmio_address`.

---

## (a) Orchestrator Checklist Verdicts

### Caller Analysis
- [x] **Targets identified & leaf-module status confirmed** — `caller_analysis.md`
  enumerates the 3 in-scope exec fns + the type; module is an intra-crate leaf
  (no external dependents). Verified against `phys.rs` impl blocks.
- [x] **Every caller expectation mapped to a contract** — see section (c); 10/10
  expectations covered.

### View Design
- [x] **View = single integer (raw address)** — `impl View for PhysicalAddress`
  in `phys.spec.rs:147` returns `self.0@` (`type V = int`, `closed`). Matches the
  caller-perspective abstraction (`view() == into_raw_value()`).
- [x] **Foreign `FrameNumber` projected by uninterp ghost fn** — `spec_frame_raw_value`
  (orphan-rule-correct), as `view_design.md` prescribes. Registered opaque via
  `ExFrameNumber` (`external_type_specification`).
- [x] **`inv()` is the load-bearing type invariant** — `spec_frame_number(self@) <=
  spec_max_frame_number()` (representable frame number); not alignment, not
  RAM-validity. Correct per design rationale.

### Specification
- [x] **All 3 in-scope exec fns carry `#[verus_spec]`** — confirmed in `phys.rs`
  (from_mmio_address:121, from_number:149, into_frame_number:191).
- [x] **Error/total arms meaningful** — from_mmio_address `result is Ok` +
  identity; into_frame_number totality via `inv()` requires; from_number infallible
  `-> Self`. No one-sided or tautological arms.
- [x] **No floating specs** — every spec fn/assume_spec connects to an exec contract.

### Proving
- [x] **0 `admit()`** in the in-scope module (grep + module verify: `admit=0`).
- [x] **Overflow / alignment / totality obligations discharged** — re-derived below
  (section g/i); all sound.
- [x] **`make verify-kernel` exit 0, 0 errors** — re-ran myself (section h).

### Cheating Elimination
- [x] **assume=0, admit=0** in module and globally.
- [x] **Only `external_body` = `ExFrameNumber`**, listed in `tcb-allowed.md`.
- [x] **cfg gates do not gate exec code** — 3 gates (lines 9, 11, 30) cover only
  `include!`/imports.

### Bug Recording
- [x] **No bugs.md and none required** — no code defect exists in the in-scope
  functions (overflow proven impossible; frame math correct).

---

## (b) Spec Quality Assessment

**`inv()`** — minimal, load-bearing, single clause. It is the exact fact that makes
`into_frame_number`'s internal `unwrap()` total. Correctly excludes alignment (MMIO
may be unaligned) and RAM-validity (`from_mmio_address` bypasses it). Established by
both verified constructors; consumed by `into_frame_number`'s `requires`. Per
spec-design "Scattered Invariant" rule, this universally-needed property correctly
lives in `inv()` rather than per-method `requires`.

**`View`** — abstracts to `int` raw address, delegating to inner `VirtualAddress`.
Clean, declarable from signature alone (passes the "independent from code" test).

**`from_number`** — 3 ensures: `result@ == frame*FRAME_SIZE`, `result@ % page_size
== 0`, `result.inv()`. Functional + alignment + structural. The alignment clause is
mathematically derivable from clause 1 but is a deliberate caller-facing convenience
(sole caller immediately feeds `PageAligned::from_address`, which checks alignment) —
acceptable, not a true subsumption violation since it spares the caller a nonlinear
mod lemma. `inv()` is **not** subsumed (needs the external `fr <= MAX` fact).

**`into_frame_number`** — `requires self.inv()`; `ensures spec_frame_raw_value(result)
== spec_frame_number(self@)` (`== self@ / page_size == addr >> FRAME_SHIFT`). Total
projection, exactly the caller contract. Good.

**`from_mmio_address`** — `requires spec_frame_number(addr@) <= max` encodes the
`unsafe` obligation as a proof obligation (frame-representability); `ensures Ok` +
identity (`r@ == addr@`) + `r.inv()`. Correctly models "bypass RAM check, identity
wrap, satisfy invariant so the result may flow into `into_frame_number`".

**assume_specification trust boundaries (6)** — all at the arch/sys library edge,
each minimal and load-bearing: `FRAME_SIZE` (positivity), `FRAME_SHIFT`
(`pow2==page_size`, `<BITS`), `VirtualAddress::new`/`into_raw_value` (newtype
identity), `FrameNumber::into_raw_value` (index + range), `FrameNumber::from_raw_value`
(`Some` iff `<= max`). None over-strong; the `FrameNumber` range facts are the
exact load-bearing inputs to the overflow and totality proofs. Note: the boundary
encodes `FrameNumber::MAX == usize::MAX/FRAME_SIZE - 1` (i.e. `MAX_ADDRESS ==
usize::MAX`); this is the governed, internally-consistent arch-edge assumption
(permitted, in `tcb-allowed.md`), used identically for both `into`/`from_raw_value`,
so the round-trip is sound.

**Verdict: Spec quality PASS.**

---

## (c) Caller Coverage — 10/10, no gaps

| Caller expectation (from `caller_analysis.md`) | Contract element | ✓ |
|---|---|---|
| `from_number`: page-aligned result | `result@ % spec_page_size() == 0` | ✓ |
| `from_number`: `== frame * FRAME_SIZE` | `result@ == spec_from_number(spec_frame_raw_value(frame))` | ✓ |
| `from_number`: round-trip inverse of `into_frame_number` | composition of `from_number.ensures` + `into_frame_number.ensures` (`(fr*p)/p == fr`) | ✓ |
| `into_frame_number`: totality (never panics) | `requires self.inv()` ⇒ index `<= max` ⇒ `unwrap` total | ✓ |
| `into_frame_number`: `== addr >> FRAME_SHIFT` (`addr/FRAME_SIZE`) | `spec_frame_raw_value(result) == spec_frame_number(self@)` | ✓ |
| `into_frame_number`: in-range allocator index | guaranteed by `inv()` (`result <= max`) | ✓ |
| `from_mmio_address`: identity wrap | `Ok(r) ==> r@ == addr@` | ✓ |
| `from_mmio_address`: bypass RAM-validity check | body has no validity call; `result is Ok` always | ✓ |
| `from_mmio_address`: returns `Ok` | `ensures result is Ok` | ✓ |
| Type invariant: representable frame number | `inv() == spec_frame_number(self@) <= spec_max_frame_number()` | ✓ |

**Covered 10/10. Gaps: none.**

---

## (d) Proof Completeness

- `admit()` in in-scope module: **0** (grep over phys.rs/spec.rs/proof.rs + module
  verify `admit=0`). → not a blocker.
- `external_body` in in-scope module: **1** — `ExFrameNumber`
  (`phys.spec.rs:38-40`, `external_type_specification` + `external_body`). Listed in
  `tcb-allowed.md` ("Allowed `external_type_specification` — phys proof target"). →
  not a blocker.
- `phys.proof.rs` is empty (`verus! { }`); all proof work is inline `proof!` blocks
  in `phys.rs` — legitimate.

---

## (e) TCB Compliance

- The phys module's only `external_body` is `ExFrameNumber` → **explicitly listed**
  in `tcb-allowed.md` (lines 236–250).
- Cheating-detail confirms phys's sole entry: `phys.spec.rs:40 ExFrameNumber
  (struct): external_type_spec`.
- No new trust boundary introduced or justified. Global `external_body=25` — all 25
  cross-checked against `tcb-allowed.md` (shared evidence inventory + my
  cheating-detail read). **PASS.**

---

## (f) Guardrail Counts (in-scope phys module)

| Guardrail | Count | Notes |
|---|---|---|
| `admit()` | **0** | no blocker |
| `assume(...)` | **0** | no blocker |
| `external_body` | **1** | `ExFrameNumber` — in tcb-allowed ✓ |
| `assume_specification` | **6** | all arch/sys library-edge, in tcb-allowed ✓ |
| cfg-gated **exec** code | **0** | 3 `cfg(verus_keep_ghost)` gates cover only `include!` (L9,L11) + duplicate `use ::vstd::prelude::*` import (L30); the unconditional `use vstd::prelude::*` is at L8 |

Global (module verify, re-run): `assume=0 external_body=25 admit=0 trusted=0
no_decreases=0 cfg_gate=8`. **No blocker.**

---

## (g) AST Consistency — PASS (independently re-derived)

Original exec from `git show 654e9211f~1:.../phys.rs`:

```rust
// from_number (original)
let addr: usize = frame.into_raw_value() * mem::FRAME_SIZE;
Self(VirtualAddress::new(addr))

// into_frame_number (original)
let raw_addr: usize = self.0.into_raw_value();
let frame_number: usize = raw_addr >> mem::FRAME_SHIFT;
FrameNumber::from_raw_value(frame_number).unwrap()
```

**`from_number` equivalence:** verified binds `frame_raw = frame.into_raw_value()`,
`page_size = mem::FRAME_SIZE`, then `addr = frame_raw * page_size`. Substituting the
locals: `addr = frame.into_raw_value() * mem::FRAME_SIZE` — *identical value, identical
evaluation order, identical side effects, identical `usize` multiply*. The two
inserted `proof!` blocks are ghost (erased). Matches pre-approved deviation
`f(complex_expr) → let x = complex_expr; f(x)`. **Equivalent.**

**`into_frame_number` equivalence:** verified binds `shift = mem::FRAME_SHIFT` then
`frame_number = raw_addr >> shift`. Substituting: `frame_number = raw_addr >>
mem::FRAME_SHIFT` — identical. `raw_addr` is bound first in both; one inserted ghost
`proof!`. **Equivalent.**

Both carry `// VERUS DEVIATION (pre-approved ...)` comments. No genuine MISMATCH.
14/16 elements MATCH; the 2 flagged are these semantically-equivalent rewrites.
**AST Consistency PASS.**

---

## (h) Verification — PASS

Re-ran `make verify-kernel MODULE=hal::mem::types::address::phys` myself:
- Exit code **0**.
- "✅ No cheating detected in module hal::mem::types::address::phys."
- `Global: assume=0 external_body=25 admit=0 trusted=0 cfg_gate=8`.
- Coverage 3/16 (the 3 in-scope; the 13 untouched out-of-scope fns are intentionally
  uncontracted — not flagged as gaps).
- Commit history: `732490ead [verus] verify PASS: ...phys (4 verified, 0 errors)`.

**Verification PASS.**

---

## (i) Bug Summary

**No bugs — correctly no `bugs.md`.**

Re-derived the two potential defect sites:

1. **Overflow in `from_number` (`fr * page_size`)** — proven impossible. With `p =
   page_size > 0`, `fr = spec_frame_raw_value(frame)`, `0 <= fr <= usize::MAX/p - 1`:
   `lemma_fundamental_div_mod` + `lemma_mod_division_less_than_divisor` give
   `p*(m/p) <= m`; the nonlinear step gives `fr*p <= (m/p - 1)*p = p*(m/p) - p <= m -
   p <= m`. The multiply never exceeds `usize::MAX`. The original code was therefore
   already correct (guarded by `FrameNumber`'s construction range) — **not** a missed
   bug.
2. **Panic in `into_frame_number` (`.unwrap()`)** — proven total. `raw_addr >> shift
   == self@ / p` (`lemma_usize_shr_is_div`, `pow2(shift)==p`); `self.inv()` bounds it
   by `spec_max_frame_number()`, so `from_raw_value` returns `Some` and `unwrap`
   cannot panic.

Frame math (`from_number` ∘ `into_frame_number` round-trip, alignment) is correct.
Classification per bug-reporting skill: no True Bug, no Context-Dependent issue, no
auto-fixable defect.

---

## (j) Issues (highest priority first)

1. *(Minor / non-blocking)* `from_number`'s `result@ % spec_page_size() == 0` is
   mathematically derivable from `result@ == frame*FRAME_SIZE`. Retained deliberately
   as a caller-facing convenience (sole caller calls `PageAligned::from_address`
   immediately). Not a subsumption violation worth removing — improves caller
   ergonomics by avoiding a nonlinear mod lemma at the call site.
2. *(Informational)* The arch-edge `assume_specification`s encode `FrameNumber::MAX ==
   usize::MAX/FRAME_SIZE - 1`. This is a governed library-edge trust assumption
   (in `tcb-allowed.md`), used consistently for both `into`/`from_raw_value`, so the
   round-trip stays internally sound. Will be discharged when `arch` is verified.

No blocking issues.

---

## (k) Result: **PASS**

Every checklist item passes: caller coverage 10/10, spec quality sound, 0 admit / 0
assume, the single `external_body` (`ExFrameNumber`) and all 6 `assume_specification`s
are pre-approved in `tcb-allowed.md`, no exec code is cfg-gated, the 2 AST deviations
are independently confirmed semantically equivalent, `make verify-kernel` is exit 0
with 0 errors, and no code defect exists (overflow and panic both proven impossible).
