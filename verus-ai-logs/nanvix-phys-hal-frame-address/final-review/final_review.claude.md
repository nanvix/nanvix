# Final Verification Review — `hal-frame-address`

**Target module:** `src/kernel/src/hal/mem/types/address/frame.rs`
(+ `frame.spec.rs`, `frame.proof.rs`)
**In-scope functions:** `FrameAddress` (type: `View` + `inv`),
`into_raw_value`, `into_frame_number`, `from_raw_value`, `from_frame_number`.
**Branch:** `verus-ai/hal-frame-address` · **Reviewer:** independent strict pass.
**Method:** Read all 3 source files, caller/view/bug artifacts, TCB allowlist,
verus-unsupported note, and the six mandated skills. Independently re-ran the
read-only checkers (`ast_consistency.py`, `spec_drift.py`, guardrail greps).
`make verify*`/`make build` results taken from the orchestrator's authoritative
data (not re-run, per instruction).

---

## Spec Quality

The View/inv and the four contracts are caller-driven, declarative, and use the
shared address-tower vocabulary. Per spec-design they are sound.

**View / inv** (`frame.spec.rs:57–84`)
- `type V = int; closed view = self.0@` — single caller-observable quantity (the
  frame's base physical address as an unbounded integer). `closed` hides the
  two-level newtype delegation; passes the substitution test (representation may
  change without affecting callers). ✔
- `inv()` (`pub open`): `self@ % spec_page_size() == 0 && spec_frame_number(self@)
  <= spec_max_frame_number()`. Both conjuncts are load-bearing: alignment is
  relied on at every MMU/allocator site; representability is what makes
  `into_frame_number` total (its inner `FrameNumber::from_raw_value(..).unwrap()`
  cannot panic). Stated purely over `self@`. ✔

**Contracts**
- `into_raw_value` → `ensures result as int == self@` (`frame.rs:95–101`).
  Raw-value identity, directly caller-usable. ✔ *(Note: was an `external_body`
  trust boundary upstream; now body-verified against the inner
  `PageAligned::into_raw_value` dependency spec — a verification **improvement**.)*
- `into_frame_number` → `requires self.inv(); ensures spec_frame_raw_value(result)
  == spec_frame_number(self@)` (`frame.rs:64–69`). The returned index equals
  `self@ / PAGE_SIZE`; the `requires self.inv()` is the universally-needed
  precondition (correctly an invariant, not scattered). ✔
- `from_frame_number` → `ensures result is Ok; Ok(fa) ==> fa@ ==
  spec_from_number(spec_frame_raw_value(frame_number)) && fa.inv()`
  (`frame.rs:116–121`). Proves construction **never fails** (a strengthening over
  the caller's `?`-handling), and `fa@ == frame_number * PAGE_SIZE`. ✔
- `from_raw_value` → `ensures Ok(fa) ==> fa@ == raw_addr as int && fa.inv()`
  (`frame.rs:138–141`). Newtype identity + alignment on success. ✔

Round-trip closes algebraically: `spec_from_number(n) = n*PAGE_SIZE`,
`spec_frame_number(a) = a/PAGE_SIZE` (`phys.spec.rs:65–72`, concrete defs), so
`from_frame_number(n).into_frame_number()`'s index `= (n*PAGE_SIZE)/PAGE_SIZE = n`.

**Observations (non-blocking):**
1. **One-sided error spec on `from_raw_value`** — the ensures has only the
   `Ok(..) ==>` arm (Err implicitly `true`), and the underlying
   `assume_specification[<PhysicalAddress>::from_raw_value]` carries
   `Err(_) => true` (`frame.spec.rs:117`). Spec-design flags one-sided/tautological
   error specs, but this is the legitimate **dynamic-condition** exception: the
   physical-validity predicate is platform-specific (un-dischargeable statically),
   the sole caller (`boot_init.rs:207`) only branches `Ok`/`Err`, and it mirrors
   the verified sibling `phys.rs` contract. Acceptable.
2. **`uninterp` spec fns.** `spec_page_size()` (`frame.spec.rs:42`) is uninterp;
   so are `spec_frame_raw_value` / `spec_addr` (`phys.spec.rs:50`,
   `page.spec.rs:31`). Spec-design bans `uninterp` as a verification escape, but
   these are the sanctioned exceptions: `spec_page_size` is an external-bottom
   **hardware constant** pinned to `::arch::mem::PAGE_SIZE` via
   `assume_specification` (governed, not free), and the other two are the Views of
   **foreign external types** (`arch::FrameNumber`, bare `T: Address`) with no
   reachable datatype registration. None is paired with an axiom that injects
   arbitrary properties. The arithmetic helpers that matter for the contracts
   (`spec_frame_number`, `spec_from_number`, `spec_max_frame_number`) are all
   **concrete**. `spec_page_size` is defined **once** (frame is the canonical
   definer; `page.rs`/`phys.spec.rs`/`region.spec.rs`/`upool.spec.rs` import it),
   so the whole tower shares one definition — no inconsistency.

No tautological success postconditions, no subsumed ensures, no machine-type
leakage into the View. **Spec quality: PASS.**

---

## Caller Coverage — Covered 5 / 5

Per `caller_analysis.md`, the caller-perspective invariants are: page alignment,
raw-value identity, frame-number round-trip, View = physical address (`int`), and
value-free error path. Mapping each to a contract:

| Caller expectation | Contract evidence | Status |
|---|---|---|
| Opaque page-aligned frame address; `inv()` (alignment) available | `inv()` conjunct 1 (`frame.spec.rs:81`) | ✔ |
| View = abstract physical address `int` (specs reason on `frame@`) | `view = self.0@`, `type V = int` (`frame.spec.rs:57–62`) | ✔ |
| `into_raw_value` == literal physical address | `result as int == self@` | ✔ |
| `from_raw_value` Ok ⇒ `fa@ == raw_addr` + aligned; Err propagated | `Ok(fa) ==> fa@ == raw_addr as int && fa.inv()`; Err value-free (dynamic) | ✔ |
| `from_frame_number` Ok ⇒ `fa@ == n*PAGE_SIZE` + aligned; never spuriously fails | `result is Ok` + `fa@ == spec_from_number(spec_frame_raw_value(n)) && fa.inv()` | ✔ |
| `into_frame_number` total; index == `self@/PAGE_SIZE`; bounded | `spec_frame_raw_value(result) == spec_frame_number(self@)`; bound via `requires inv` | ✔ |
| Round-trip `from_frame_number(n).into_frame_number() == n` | Algebraic from the two contracts (inverse helpers) | ✔ |

**Missing properties:** none material. The only un-stated semantics is
`from_raw_value`'s Err condition, which is intentionally value-free (dynamic
platform predicate; the single caller only branches on `Ok`/`Err`). No in-scope
caller depends on a full address upper bound, correctly excluded from `inv()`
(view_design Rejected Alternative #6). **Caller coverage: PASS.**

---

## Proof Completeness

- `admit()` in frame module: **0** (independently grepped `frame.{rs,spec.rs,proof.rs}`).
- `external_body` in frame module: **0** (only appears inside prose comments at
  `frame.proof.rs:29`).
- Cross-module dependency specs used (`PhysicalAddress::from_number`,
  `PageAligned::from_address`, `into_raw_value`) are trusted contracts under
  `--verify-module frame` — standard and sound; they become fully proven when
  their home modules verify.

No `admit()` ⇒ **no blocker. Proof completeness: PASS.**

---

## TCB Compliance

All trust-boundary items in the frame module are accounted for in
`tcb-allowed.md`:

| Item | Location | Allowlist | Verdict |
|---|---|---|---|
| `axiom fn lemma_phys_view_is_spec_addr` | `frame.proof.rs:38` | §"Allowed `axiom fn` … frame view/spec_addr bridge" (L311–334) | ✔ listed |
| `assume_specification[<PhysicalAddress>::from_raw_value]` | `frame.spec.rs:110` | §"…frame library edge" (L272–) | ✔ listed |
| `assume_specification[<PageAligned<T> as Deref>::deref]` | `frame.spec.rs:129` | §"…frame library edge" (L272–) | ✔ listed |
| `assume_specification[::arch::mem::PAGE_SIZE]` | `frame.spec.rs:45` | acknowledged as the pre-existing established boundary (L189/226/257/279) | ⚠ acknowledged, no dedicated bullet |
| `external_body` | — | none present | ✔ n/a |

**Notes (non-blocking):**
- `::arch::mem::PAGE_SIZE` is a **pre-existing** `assume_specification` (predates
  this effort — see `caller_analysis.md` "Spec glue"). `tcb-allowed.md` cites it
  four times as the established precedent the newer boundaries mirror, but gives
  it no standalone bullet. Recommend adding an explicit one-line entry for
  completeness; not a soundness issue (it is a genuine `arch` hardware-constant
  edge, identical in kind to the listed boundaries).
- `tcb-allowed.md:170` still lists `FrameAddress::into_raw_value` as an allowed
  `external_body`, but the code now **body-verifies** it (uses *less* trust than
  permitted). Stale/over-permissive entry — harmless; could be pruned.

**TCB compliance: PASS** (with two documentation nits, no blocker).

---

## Guardrails Compliance (exact counts — frame module)

Independently grepped over `frame.rs`, `frame.spec.rs`, `frame.proof.rs`
(excluding matches inside comments where noted):

| Guardrail | Count | Blocker? |
|---|---|---|
| `admit()` | **0** | — |
| `assume(` (not `assume_specification`) | **0** | — |
| `external_body` | **0** (3 textual hits are all in prose comments) | — |
| `assume_specification` | **3** (PAGE_SIZE, `<PhysicalAddress>::from_raw_value`, `<PageAligned<T> as Deref>::deref`) | all TCB-covered |
| `axiom fn` | **1** (`lemma_phys_view_is_spec_addr`) | TCB-covered |
| `uninterp spec fn` | **1** (`spec_page_size`) | governed external-bottom |
| cfg-gated exec | **0** (`cfg(verus_keep_ghost)` only on `include!`/`use` — verification material, not exec branches) | — |

`admit == 0` and `assume == 0`. **Guardrails: PASS.**

---

## AST Consistency — PASS

`ast_consistency.py summary`: matched **6**, mismatched **3**, missing 0, extra 0.
The 3 MISMATCHes are the three in-scope rewritten bodies; each is a documented,
**pre-approved** `f(complex_expr)` → `let x = complex_expr; f(x)` deviation
(ast-consistency skill deviation table) carrying a `// VERUS DEVIATION` comment.

| Function | Diff | Semantic verdict |
|---|---|---|
| `from_frame_number` (`frame.rs:122`) | `Ok(Self(from_address(from_number(n))?))` → bind `physical_address` then call | Single eval, same `?`. **Equivalent.** ✔ |
| `from_raw_value` (`frame.rs:142`) | `Ok(Self(from_address(from_raw_value(raw)?)?))` → bind, preserving inner-`?`-before-outer-`?` order | `?` order preserved. **Equivalent.** ✔ |
| `into_frame_number` (`frame.rs:70`) | `self.0.into_frame_number()` → `let pa: PhysicalAddress = *self.0; pa.into_frame_number()` | Explicit `Copy` deref of the auto-`Deref` target; same value, same method, `into_` takes `self` by value. **Equivalent.** ✔ |

Each carries the required comment. Per the ast-consistency skill, the
`f(complex_expr)→let` rewrite is in the **pre-approved** table, which requires a
comment but **no** separate isolated reproducer. The genuine underlying Verus
limitation that necessitates the bridge lemma / trust boundaries (`error: Verus
does not support this cast: usize to *const u8`) is documented with its minimal
trigger in `verus-unsupported.md`. **No genuine semantic divergence.**
**AST consistency: PASS.**

---

## Verification

- **`make verify-kernel` → PASS (exit 0, cached).** Crate-wide cheating snapshot:
  `assume=0 external_body=24 admit=0 trusted=0 no_decreases=0`; none of the 24
  crate `external_body`s fall in `hal/mem/types/address/frame.*`. This is the
  authoritative signal that the frame contracts (and the lock-step `upool`
  change) verify.
- **`make build` → exit 0.** Exec code compiles (Verus constructs erased).
- **`make verify` (full cross-module) → FAILS at `verify-bitmap` (exit 101).**
  Assessment: **genuinely unrelated / pre-existing**, not a regression from this
  change. Evidence:
  - The failure is 9 `vstd` compile errors in the registry dependency
    `std_specs/atomic.rs` (`ExAtomic`/`AtomicBool` generics macro mismatch:
    "expected generics to match: expected, found bool") — a toolchain/vstd-version
    issue in the `bitmap` crate's dependency graph.
  - `make verify` runs `verify-<crates>` (bitmap first) **before** `verify-kernel`
    and stops at the first failure, so it never reaches the frame module.
  - This change's code footprint is `frame.{rs,spec.rs,proof.rs}` +
    `mm/phys/upool.spec.rs` only — it touches neither the `bitmap` crate nor
    `vstd`. `verify-kernel` (which *does* cover the frame module) passes
    standalone.
  - Conclusion: pre-existing environmental failure, **not** a blocker for this
    module.

**Verification: PASS for the in-scope module.**

---

## Bug Summary

`bugs.md` records **no code bugs**. The only entry is the former single `admit()`
— the bridge fact `spec_addr(&pa) == pa@` for a `PhysicalAddress`. Reconciliation:

- **Status: valid classification (False Positive / external-bottom trust boundary).**
  `spec_addr<T: Address>` is `uninterp` (must apply to a bare `T: Address` with no
  `View<V=int>` bound); nothing in scope constrains it for the concrete
  `PhysicalAddress`, and the only relating fact (`Deref::deref`) is tautological
  once unfolded. The equality is semantically true (both sides are the physical
  address) but derivable only when `impl Address for PhysicalAddress` is verified
  — currently blocked by its `usize as *const/*mut u8` sibling casts
  (`verus-unsupported.md`).
- **Resolution is honest:** discharged via the governed `axiom fn`
  `lemma_phys_view_is_spec_addr` (no `admit`, no `assume`, no `external_body`),
  TCB-registered. **No real code bug is masked** — the axiom states a true
  newtype-identity fact and weakens no target contract; it is removed when the
  `Address` trait is verified.

**`upool.spec.rs` lock-step change:** `UserFrame::inv()` gains the conjunct
`spec_frame_number(self@) <= spec_max_frame_number()` (`upool.spec.rs` diff).
This is a **STRENGTHENING** (an added conjunct restricts the predicate), **not a
weakening** — confirmed by `spec_drift.py` (0 contract drift vs HEAD; the base-vs
delta is an *addition*). It is required so `UserFrame::inv()` stays in lock-step
with `FrameAddress::inv()` (related by `self@ == addr@`), keeping the
representability precondition that `frame::share`/`frame::refcount` demand
satisfiable from a well-formed handle. Soundness: every `UserFrame` establisher
must now also discharge representability, and `make verify-kernel` passing proves
they do — so the strengthening is honestly maintained, not a cheating vector
(burden is paid at construction). It is `pub open` (visible) and outside the named
target list but a necessary consequential `.spec.rs` change.

---

## Issues (highest priority first)

1. *(Low / documentation)* `::arch::mem::PAGE_SIZE`'s `assume_specification`
   (`frame.spec.rs:45`) is only **acknowledged** in `tcb-allowed.md` (cited as the
   established precedent at L189/226/257/279), without a dedicated bullet. It is a
   pre-existing, in-kind hardware-edge boundary — add an explicit entry for
   audit completeness. **Not a blocker.**
2. *(Low / documentation)* `tcb-allowed.md:170` lists
   `FrameAddress::into_raw_value` as an allowed `external_body`, but it is now
   body-verified. Stale, over-permissive (uses less trust than allowed); prune
   when convenient. **Not a blocker.**
3. *(Informational)* `from_raw_value`'s error arm is value-free / one-sided. This
   is the sanctioned dynamic-condition pattern (mirrors the verified `phys.rs`
   sibling and the underlying `Err(_) => true` trust contract), not an oversight.
   **Not a blocker.**
4. *(Informational)* `make verify` fails at `verify-bitmap` on a pre-existing
   `vstd`/toolchain incompatibility, unrelated to this change. **Not a blocker.**

No genuine blockers (no `admit`, no `assume`, no un-allowlisted `external_body`,
no spec weakening, no semantic AST divergence, no masked code bug).

---

## Result: **PASS**

**Justification.** The four in-scope contracts plus the `FrameAddress` View/inv
are correct, complete against every caller expectation in `caller_analysis.md`
(5/5), declarative, and caller-usable. Guardrails are clean: `admit=0`,
`assume=0`, `external_body=0`; the 3 `assume_specification`s and 1 `axiom fn` are
each TCB-registered, and the single `uninterp spec fn` is a governed
external-bottom hardware constant. The 3 AST mismatches are pre-approved,
commented, semantically-equivalent intermediate-value rewrites. `spec_drift`
shows zero contract drift; the `upool.spec.rs` lock-step edit is a sound
strengthening, not a weakening. `make verify-kernel` passes (exit 0) and `make
build` succeeds; the `make verify` failure is an isolated, pre-existing
`vstd`/`bitmap` toolchain issue with no causal link to this change. `bugs.md`
records no code bug, and the former bridge `admit()` is honestly discharged via a
TCB-governed axiom that masks no real defect. All identified issues are
low-severity documentation/informational nits.
