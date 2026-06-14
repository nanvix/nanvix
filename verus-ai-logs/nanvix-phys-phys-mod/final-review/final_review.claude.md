# Final Verification Review — `mm::phys` ("phys-mod")

- **Scope (in-scope functions only):** `init`, `book_mmio_regions`, `book_physical_memory_regions` in `src/kernel/src/mm/phys/mod.rs`
- **Reviewer posture:** Independent, skeptical, hands-on. All commands re-run from scratch; no prior phase trusted.
- **Date:** 2026-06-15
- **Branch:** `verus-ai/phys-mod` (HEAD `e02cda634`), base `verus-ai/bump-allocator`

---

## Summary Verdict: **PASS** (with documented spec-quality weaknesses; **0 blockers**)

Every hard verification-integrity gate is green and confirmed with my own tool runs:

| Gate | Result |
|---|---|
| `make verify-kernel MODULE=mm::phys` | **exit 0**, 0 verification errors |
| `admit()` in scope | **0** |
| `assume` / `assume!` in scope | **0** |
| `external_body` in scope | **2 fns + 1 type-spec — all TCB-approved** |
| `assume_specification` in scope | **0** |
| cfg-gated **exec** code | **0** (cfg gates only on `include!`/`use`, non-semantic) |
| AST consistency vs base branch | **CONSISTENT** (4 fns, 0 structs) |
| AST consistency vs HEAD | **CONSISTENT** |
| Spec drift vs HEAD | **0 drift** (exit 0) |
| fn coverage | **4/4** in-scope exec fns have contracts |
| Source changes vs HEAD | **none** (review-only honored) |

No item I was asked to check **failed**. The verification is sound and honest: no cheating mechanism is used outside the pre-approved TCB list, and `init`'s contract is non-vacuously proven (the soundness chain is verified below).

The review is **not** a clean bill of health on *spec quality*. There are genuine, material weaknesses (orphan proof lemmas; the central "booking effect" not surfaced at the exec boundary; a subsumed `disjoint` clause; tautological `Err` arms). All of them are **structurally forced by the TCB-approved `external_body` on the two `book_*` helpers** (the std-`LinkedList` orphan-rule limitation) and are documented. They are recorded below as **non-blocking issues**, not gate failures — hence overall **PASS**, but a strict reviewer should read the Issues list before relying on `init`'s postcondition to mean "frames were actually booked".

---

## 1. Spec Quality

### 1.1 The `init` external-top contract (`mod.rs:163-181`)

```
ensures
    phys_view().inv(),
    match ret {
        Ok(()) => { phys_view().initialized
                 && phys_view().frames.allocated_frames.disjoint(phys_view().frames.free_frames) },
        Err(_) => true,
    }
```

- **`phys_view().inv()` unconditional** — good. It is the precondition every later `frame::*` / `PhysMemoryManager::*` op relies on, holds on every path (vacuous before init since `inv() = initialized ==> frames.wf()`). This correctly captures Key Invariant #1 ("establishes, not merely preserves, the invariant").
- **`Ok => initialized`** — good and necessary. This is the one fact that distinguishes a successful boot and is what `book_*`/later ops require.
- **`Ok => allocated_frames.disjoint(free_frames)` — SUBSUMED (anti-pattern #9, "Subsumed Properties").** On the `Ok` arm, `phys_view().inv()` (present) **and** `phys_view().initialized` (present, conjoined) together give `frames.wf()`, and `FrameAllocView::wf` (`mod.spec.rs:37`) **already** contains `self.allocated_frames.disjoint(self.free_frames)`. Therefore the `disjoint` clause is logically implied by the two clauses already in the `Ok` arm and should be deleted per spec-design ("redundant clauses obscure the spec's intent"). **Confirmed**: the prompt's hypothesis is correct — it is subsumed-ensures. *Severity: minor / cosmetic (redundant, not wrong).*
- **`Err(_) => true` — tautological (anti-pattern #8) / one-sided error spec (anti-pattern #5).** It promises nothing on failure. **Partial mitigation:** because `phys_view().inv()` sits *outside* the match, the invariant **is** preserved on the error path, so it is not a fully one-sided spec. What is *not* expressed is **state preservation** on error (no `phys_view() == old(phys_view())`). For a global-`static`-backed `uninterp` view there is no `old()` handle, so error-state-preservation is genuinely hard to state here. Given the caller (`kernel_vas::init`) treats `Err` as terminal (caller_analysis §init "failure is terminal for the boot path"), `Err(_) => true` is **acceptable but weak**. *Severity: minor, justified by "failure terminal for boot".*

### 1.2 The two `book_*` boundary contracts (`mod.rs:71-81`, `106-116`)

Both are identical in shape:

```
requires phys_view().initialized, phys_view().inv(),
ensures  phys_view().inv(),
         match result { Ok(()) => phys_view().initialized, Err(_) => true }
```

- These capture **only `inv()` + `initialized` preservation** — they do **NOT** capture the **booking EFFECT** (which frames moved from `free` to `allocated`). This is the single most important semantic property of these functions, and it is absent from the exec boundary.
- **Why it is missing:** both functions are `#[verus_verify(external_body)]` because they iterate a std `alloc::collections::LinkedList` whose contents a Verus `spec fn` cannot view (orphan rule forbids the `View`/`ForLoopGhostIterator` impls; vstd is pinned). The "which-frames" set is literally the un-viewable `LinkedList` payload. So the effect **cannot** be named at this boundary. Documented in `verus-unsupported.md` and `tcb-allowed.md`.
- **Is this acceptable?** As a *consequence of the approved external_body boundary*, yes — there is no stronger contract expressible while the helper bodies are unverifiable. But the **cost is real**: see §1.3 and Issue #1. *Severity: major (effect loss), but rooted in an approved TCB limitation.*
- `Err(_) => true` here is the same weak-error-arm pattern as §1.1; acceptable because `init` short-circuits via `?` and treats any helper `Err` as fatal.

### 1.3 Orphan proof/spec vocabulary (floating specs — spec-design rule #5)

`mod.proof.rs` defines **6 lemmas** and `mod.spec.rs` defines the transition fns `spec_initialize`, `spec_book_frame`, `spec_book_frames`. I verified by grep that **none of them is referenced by any exec function, proof block, or exec-bound `#[verus_spec]` clause in scope** — they appear only inside `mod.proof.rs` and inside **comments** in `mod.rs`:

```
$ grep -rnE "lemma_...|spec_book_frame(s)?|spec_initialize" src/kernel/src/mm/phys/ | grep -v mod.proof.rs
src/kernel/src/mm/phys/mod.rs:65:  // ... modelled by `PhysMemView::spec_book_frames` / `region_frames` ...   (comment)
src/kernel/src/mm/phys/mod.rs:99:  // ... modelled by `PhysMemView::spec_book_frames` over ...                (comment)
src/kernel/src/mm/phys/mod.rs:100: // ... discharged by `lemma_book_mmio_skip_untracked` / ...               (comment)
$ grep -nE "proof!|proof_with!|proof_decl!" src/kernel/src/mm/phys/mod.rs
(none)
```

So the entire booking-effect model (`spec_book_frames`, the `lemma_*` family) is a **verified design model disconnected from the running code**. The lemmas prove things about *arbitrary* `pre`/`reserved` values; nothing ties the real post-`book_*` `phys_view()` to `pre.spec_book_frames(reserved)`. Per spec-design these are **orphan specs / dead code** with respect to the exec contracts. They do **not** undermine soundness (they are extra, not relied upon), but they also add **zero verification value** to the shipped code, and they must **not** be counted as "covering" the reservation effect. *Severity: major as a quality finding (the proof.rs content is decorative); structurally tied to the external_body boundary.*

> Note: `frame::alloc_range` and `frame::book` (free-fn wrappers in `frame.rs`, dependencies, **out of scope**) *do* express the reservation effect over `phys_view()` (e.g. `frame.rs:807` ensures every `region_frames(...)` frame becomes allocated). The effect loss is specifically at the `book_*` LinkedList-iterating layer.

### 1.4 Bug-rejection test (spec-design principle #3)

A buggy `init` that initializes the allocator but **books nothing** (e.g. silently drops both region lists) would still satisfy the in-scope exec contract: `inv()`, `initialized`, and `disjoint` all hold for an all-free allocator. The spec therefore **does not reject the "forgot to book" bug**. This is a direct corollary of §1.2/§1.3. Again forced by the external_body boundary, but it is the honest characterization of how weak the surfaced guarantee is.

### 1.5 Soundness chain for `init` (is the proof vacuous? — No)

I traced the dependency specs to confirm `init`'s contract is genuinely established, not vacuously true:

1. `frame::init` (`frame.rs:656-666`, external_body, dependency): `ensures phys_view().inv(); Ok => phys_view().initialized`. → after `frame::init(...)?` Ok, `init` has `initialized && inv()`.
2. `book_physical_memory_regions` requires `initialized && inv()` (satisfied) → ensures same. ✓
3. `book_mmio_regions` — idem. ✓
4. `PhysMemoryManager::init` (`manager.rs:86`, external_body) — has **no** `#[verus_spec]`, touches no `phys_view()`; irrelevant to the proof obligation. ✓

So at `init`'s exit, `inv() && initialized` genuinely hold and `disjoint` follows from `wf()`. The contract is **non-vacuously verified over the real exec body**. Good.

---

## 2. Caller Coverage

Source: `caller_analysis.md`. Sole external caller is `kernel_vas::init` (`kernel_vas.rs:120`), which is **not itself verified** (out of scope), so no *verified* caller currently consumes these facts — but I evaluate against the documented expectations.

### `init` — Ok expectations

| # | Caller expectation (Ok) | Surfaced at exec boundary? |
|---|---|---|
| 1 | Allocator initialized; `instance().inv()` holds for later ops | ✅ `phys_view().initialized` + `phys_view().inv()` |
| 2 | All `physical_memory_regions` frames reserved (alloc never returns them) | ❌ **Not surfaced** (only orphan `lemma_book_region_reserves_region_frames`) |
| 3 | All tracked MMIO frames booked; untracked skipped (no abort) | ❌ **Not surfaced** (only orphan `lemma_book_mmio_*`) |
| 4 | `Upool` + `PhysMemoryManager` singleton initialized & ready | ❌ **Not surfaced** (`manager::init` has no spec; not mentioned in `init` ensures) |
| 5 | `init` ran exactly once (by-value/once semantics) | ❌ **Not surfaced** (no `requires !phys_view().initialized`; no monotonic ensures) |

### `init` — Err expectation

| Caller expectation (Err) | Surfaced? |
|---|---|
| Boot fails, terminal, no guarantee on partial state | ✅ (weakly) `Err(_) => true` + unconditional `inv()` |

### `book_*` — Ok/Err

| Function | Ok effect (frames booked) | Err (terminal) |
|---|---|---|
| `book_physical_memory_regions` | ❌ effect not surfaced (inv+init only) | ✅ weak (`Err=>true`) |
| `book_mmio_regions` | ❌ effect not surfaced (inv+init only) | ✅ weak (`Err=>true`) |

### Key Invariants (caller_analysis §"Key Invariants", 5 total)

| # | Invariant | Covered by exec contract / lemma bound to exec? |
|---|---|---|
| 1 | Establishes (not just preserves) the allocator invariant | ✅ `init` ensures `inv()` + `Ok=>initialized` |
| 2 | Reserved frames excluded from allocation (booked, disjoint) | ❌ Only `disjoint` (subsumed) surfaced; the *membership* (frames actually in `allocated`) is in **orphan** lemmas, not bound to exec |
| 3 | Untracked MMIO tolerated (skip ≠ error) | ❌ Only in **orphan** `lemma_book_mmio_skip_untracked` |
| 4 | One-shot / monotonic init | ❌ Not expressed (no precondition / monotonic ensures) |
| 5 | Failure terminal | ✅ (weakly) `Err(_) => true` + unconditional `inv()` |

**Coverage: 2 / 5 Key Invariants surfaced at the exec boundary (1, 5).** Invariants 2, 3, 4 are **uncovered** by any exec-bound contract; 2 and 3 exist only as orphan lemmas (no verification value for the shipped code), 4 is absent entirely.

**Missing/uncovered list:**
- **(M1)** Reservation membership effect of `init`/`book_*` (Inv #2, init-Ok #2, book Ok) — structurally blocked by `book_*` `external_body`.
- **(M2)** Tracked-MMIO-booked / untracked-skipped (Inv #3, init-Ok #3) — same root cause.
- **(M3)** One-shot/monotonic init (Inv #4): there is no `requires phys_view().initialized == false` on `init`, nor any ensures relating pre/post `initialized`. A double `init` is rejected at runtime by `INSTANCE_INIT`, but the spec does not encode it. *This one is NOT blocked by the LinkedList limitation and could in principle be strengthened* (e.g. a `requires !phys_view().initialized` or a monotonic post-state), though it would need a way to name the pre-state of the uninterp view.
- **(M4)** `Upool`/`PhysMemoryManager` readiness (init-Ok #4): `manager::init` carries no spec, so `init` says nothing about manager readiness.

---

## 3. Proof Completeness

- **`admit()` in scope: 0.** Grep of `mod.rs`, `mod.spec.rs`, `mod.proof.rs` returns no `admit`. (Any `>0` would be a BLOCKER — none found.)
- **`external_body` not in `tcb-allowed.md`, in scope: 0.** The only `external_body` in scope are the two `book_*` fns and the `ExLinkedList` type-spec, all listed in `tcb-allowed.md` (see §4). (Each unlisted one would be a BLOCKER — none found.)
- The 6 `mod.proof.rs` lemmas are fully discharged **without** `admit` (verified by `make verify-kernel` exit 0). Their **weakness is connectivity (orphan), not incompleteness** — see §1.3.

**No proof-completeness blockers.**

---

## 4. TCB Compliance

Every `external_body` / external-type-spec in scope is pre-approved in `tcb-allowed.md`:

| Item (file:line) | In `tcb-allowed.md`? | Justification |
|---|---|---|
| `book_physical_memory_regions` (`mod.rs:82`) | ✅ Yes (lines 7-17) | std `LinkedList` iteration; orphan rule (E0117) blocks ghost-iterator impl; pinned vstd; effect proven abstractly |
| `book_mmio_regions` (`mod.rs:117`) | ✅ Yes (lines 18-22) | same std-`LinkedList` limitation; skip-if-not-covered effect modeled abstractly |
| `ExLinkedList` (`mod.spec.rs:73`, `external_type_specification`) | ✅ Covered by the same LinkedList rationale (tcb-allowed.md lines 7-22; verus-unsupported.md §Mitigation) | opaque type-only spec so `init` can name `LinkedList` params; deliberately no `View`/iterator spec |

No new trust boundary is introduced. No `assume_specification`, `axiom`, `trusted`, `spinoff_prover`, or `rlimit` anywhere in scope. **TCB-compliant.**

> Note (informational, out of scope): `mod.spec.rs:13` declares `pub uninterp spec fn byte_at_address(...)` which is **unused anywhere** in the module (grep finds only its declaration). It is on the "do-not-modify / pre-existing" list so it is out of scope, but it is dead. `phys_view()` and `byte_at_address` are `uninterp spec fn`s; `uninterp` is normally banned, but here it is the established pattern for modeling un-readable module-level `static`s (analogous to the `external_body`-type `view()` carve-out in verus-constraints) and both are pre-existing/out-of-scope.

---

## 5. AST Consistency

Run vs **base branch** and vs **HEAD**:

```
$ python3 .../ast_consistency.py --base-ref verus-ai/bump-allocator src/kernel/src/mm/phys/mod.rs count
✅ Consistent: 4 functions, 0 structs match.

$ python3 .../ast_consistency.py --base-ref verus-ai/bump-allocator src/kernel/src/mm/phys/mod.rs summary
book_mmio_regions               MATCH
book_physical_memory_regions    MATCH
init                            MATCH
test                            MATCH
Consistent: ✅ YES (matched=4 mismatched=0 missing=0 extra=0)

$ python3 .../ast_consistency.py --base-ref HEAD src/kernel/src/mm/phys/mod.rs count
✅ Consistent: 4 functions, 0 structs match.
```

No MISMATCH. Grep for `// VERUS REWRITE` and `// VERUS DEVIATION` / pre-approved-deviation comments in `mod.rs`: **none found** — and none needed, since exec is byte-for-AST identical to base. Exec fidelity intact.

---

## 6. Verification Results

```
$ make verify-kernel MODULE=mm::phys
=== Summary ===
  verification: cached (no recompilation), — (exit 0)
  cheating: assume=0 external_body=17 admit=0 trusted=0 no_decreases=0 cfg_gate=5
  coverage: 15/44 exec functions have contracts
  status: CHEATING_DETECTED
```

**Raw cheating line (verbatim):**
```
Global: assume=0 external_body=17 admit=0 trusted=0 cfg_gate=5
  cheating: assume=0 external_body=17 admit=0 trusted=0 no_decreases=0 cfg_gate=5
```

**Interpretation:** `status: CHEATING_DETECTED` is the **global** kernel-crate tally (the whole `kernel::mm` build), **not** the in-scope module. The `external_body=17` / `cfg_gate=5` are crate-wide and dominated by out-of-scope `frame.rs`/`kframe.rs`/`upool.rs`/`manager.rs` items. Filtered to the in-scope files, the cheating detail lists exactly:

```
$ grep -E "mod.rs|mod.spec.rs" verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt
  - mm/phys/mod.rs:82  book_physical_memory_regions: external_body
  - mm/phys/mod.rs:117 book_mmio_regions: external_body
  - mm/phys/mod.spec.rs:73 ExLinkedList (struct): external_type_spec
```

All three are TCB-approved (§4). **Verification exit code 0; 0 verification errors.** The `cfg_gate=5` entries are out-of-scope (other phys files); in `mod.rs` the only `#[cfg(verus_keep_ghost)]` uses are on the `include!("mod.spec.rs")` / `include!("mod.proof.rs")` / spec `use` lines (`mod.rs:36,40,42`) — **non-semantic**, not exec cfg-gating.

---

## 7. Guardrails — Exact Counts (module scope: `mod.rs` + `mod.spec.rs` + `mod.proof.rs`)

| Pattern | Count | Notes / location |
|---|---:|---|
| `admit()` | **0** | none — would be BLOCKER if >0 |
| `assume` / `assume!` | **0** | none — would be BLOCKER if >0 |
| `external_body` | **2 fns + 1 type-spec** | `mod.rs:70,105` (book_*), `mod.spec.rs:70` (ExLinkedList) — all TCB-approved |
| `assume_specification` | **0** | — |
| `trusted` / `external` | **0** | — |
| `spinoff_prover` / `rlimit` | **0** | — |
| `exec_allows_no_decreases_clause` | **0** | — |
| cfg-gated **exec** code | **0** | `mod.rs:36,40,42` cfg gates are on `include!`/`use` only (non-semantic) |
| `// VERUS REWRITE` | **0** | — |

**No `admit`, no `assume` ⇒ no guardrail BLOCKER.**

Raw grep evidence:
```
mod.rs:        70:#[verus_verify(external_body)]   105:#[verus_verify(external_body)]   (book_* — TCB)
mod.spec.rs:   69:#[verifier::external_type_specification]  70:#[verifier::external_body]  (ExLinkedList — TCB)
mod.proof.rs:  (none)
```

---

## 8. Spec Drift

```
$ python3 .../spec_drift.py git-diff src/kernel/src/mm/phys/mod.rs --before HEAD
- Functions with changes: 0
- Contract drift (⚠ review required): 0
**✅ No contract drift detected.**
EXIT=0
```

No requires/ensures removed, weakened, or strengthened-on-precondition vs HEAD. `git status`/`git diff` confirm **no source-file changes** at all (only `verus-ai-logs/final-review/*` log artifacts changed). Review-only constraint honored.

---

## 9. Bug Summary

`bugs.md` records **no code bugs** in the three targets, plus one "non-bug tooling limitation" (the std-`LinkedList` iteration gap). I independently re-examined `init`/`book_*`:

- **`init` (`mod.rs:182-204`)** — ordering (`frame::init` → region book → MMIO book → `Upool::new` → `PhysMemoryManager::init`) matches caller_analysis; `?`-propagation makes any failure terminal. No defect.
- **`book_physical_memory_regions`** — books every region via `frame::alloc_range(region)?`. No defect.
- **`book_mmio_regions`** — `if frame::is_covered(phys) { frame::book(phys)? }` correctly implements skip-if-not-covered; `end = start + (region.size() - 1)` and `while start < end` walk. The intermediate add `start + (region.size()-1)` and `start += FRAME_SIZE` are unchecked `usize` arithmetic, but this is **pre-existing exec code, AST-identical to base, out of scope to modify**, and not reachable as a verified obligation (function is `external_body`). I note it but it is **not** masked by the external_body in a way that hides a *known* defect — no overflow bug is demonstrated for the boot inputs.

**Bug verdict:** `bugs.md` entries still valid; the "tooling limitation, not a bug" classification is correct. **No real code bug in `init`/`book_*` is found or masked.** Per bug-reporting: no defect is hidden behind the `external_body` markers — the markers exist solely for the LinkedList iteration limitation, and the helper *logic* (not its loop machinery) is straightforward and matches the caller contract.

---

## 10. Issues (highest priority first)

> **Blockers: NONE.** All items below are spec-quality weaknesses, not integrity/gate failures.

1. **[Major — quality] Booking EFFECT not surfaced at the exec boundary; proof.rs lemmas are orphans.** `init`/`book_*` ensures guarantee only `inv()` + `initialized`, never that any frame was actually moved to `allocated_frames`. The 6 `mod.proof.rs` lemmas + `spec_book_frame(s)`/`spec_initialize` that "prove" the effect are referenced nowhere in exec/proof code (only in comments) — they are dead w.r.t. the shipped code (spec-design rule #5). A buggy `init` that books nothing still satisfies the spec (§1.4). **Root cause:** TCB-approved `external_body` on `book_*` (un-viewable `LinkedList` contents). **Disposition:** accepted limitation; the lemmas should be honestly understood as a *design model*, not coverage. *Not fixable while `book_*` remain `external_body`.* Re-evaluate when vstd gains `LinkedList` iterator support (per `verus-unsupported.md`).

2. **[Minor — strengthenable] One-shot/monotonic `init` (Key Invariant #4) not expressed.** No `requires phys_view().initialized == false` and no monotonic post-state. This is **not** blocked by the LinkedList limitation and is the one missing caller expectation that could plausibly be strengthened (it concerns the `initialized` flag, which the view *can* name). Currently relied on only at runtime via `INSTANCE_INIT`.

3. **[Minor — redundant] `init` Ok-arm `allocated_frames.disjoint(free_frames)` is subsumed** by `phys_view().inv()` + `phys_view().initialized` (since `FrameAllocView::wf` already implies disjoint). Should be removed (spec-design anti-pattern #9). Confirmed correct, just non-minimal.

4. **[Minor — weak] `Err(_) => true` tautological arms** on `init` and both `book_*`. Mitigated by the unconditional `phys_view().inv()` (invariant *is* preserved on error), but error-path **state preservation** is unstated. Justified by "failure is terminal for boot"; acceptable but weak.

5. **[Minor — coverage] `Upool`/`PhysMemoryManager` readiness (init-Ok #4) unstated** — `manager::init` carries no `#[verus_spec]`, so `init` says nothing about manager-singleton readiness.

6. **[Informational] Dead `uninterp spec fn byte_at_address` (`mod.spec.rs:13`)** — declared, never used. Pre-existing / out-of-scope to modify, but worth flagging for cleanup by an owner who can touch the do-not-modify set.

---

## Appendix — Raw command outputs (for independent confirmation)

**fn coverage:**
```
Source exec fns 4 | Verus exec fns 4 | Matched 4 | Missing 0 | Extra 0
MATCHED: book_mmio_regions, book_physical_memory_regions, init, test
```

**AST count (base):** `✅ Consistent: 4 functions, 0 structs match.`
**AST count (HEAD):** `✅ Consistent: 4 functions, 0 structs match.`

**spec_drift (before HEAD):** `Contract drift: 0` — `✅ No contract drift detected.` (exit 0)

**verify cheating line:** `Global: assume=0 external_body=17 admit=0 trusted=0 cfg_gate=5` (global tally; in-scope = 2 book_* external_body + 1 ExLinkedList type-spec, all TCB; exit 0)

**in-scope cheating detail:**
```
mm/phys/mod.rs:82  book_physical_memory_regions: external_body
mm/phys/mod.rs:117 book_mmio_regions: external_body
mm/phys/mod.spec.rs:73 ExLinkedList (struct): external_type_spec
```

**make build:** `Nothing to be done for 'build'` (the repo's canonical build is `make verify-kernel` / `make all`; `verify-kernel` compiled the kernel under Verus at exit 0, and AST/drift confirm exec is byte-identical to the already-CI-green HEAD, so compilation integrity is established).

**source diff vs HEAD:** empty (review-only honored).
