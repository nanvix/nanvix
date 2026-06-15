# Final Verification Review — `hal-frame-address`

Scope reviewed: `src/kernel/src/hal/mem/types/address/frame.rs`, `frame.spec.rs`, `frame.proof.rs`; in-scope target only `FrameAddress` View/inv plus `into_raw_value`, `into_frame_number`, `from_raw_value`, `from_frame_number`.

Result is **FAIL** due to a real specification completeness gap on `FrameAddress::from_raw_value` (error/liveness semantics), not due to proof/TCB/AST guardrails.

## Spec Quality

### View and invariant

- `FrameAddress` has a caller-abstract scalar View: `type V = int`, `view(&self) == self.0@` (`frame.spec.rs:57-62`). This matches caller use of `frame@` as a physical address in allocator/MMU specs (`caller_analysis.md:68-70`, `caller_analysis.md:116-130`).
- `view` is `closed` (`frame.spec.rs:60`), so the `PageAligned<PhysicalAddress>` representation is hidden while callers still reason about the address integer.
- `inv()` is `pub open` and states page alignment plus frame-number representability (`frame.spec.rs:80-83`). This is the right abstraction-level invariant for `into_frame_number` totality and bitmap/refcount indexing (`caller_analysis.md:88-102`).
- No machine-type misuse in the target API contracts: the View is mathematical `int`; machine `usize` appears only at raw-value boundaries (`frame.rs:95-100`, `frame.rs:138-142`).

### Function contracts

- `into_raw_value`: good minimal accessor spec. `ensures result as int == self@` (`frame.rs:95-100`) directly covers raw address identity for pointer arithmetic/CR3/PTE users (`caller_analysis.md:116-130`).
- `into_frame_number`: good core functional spec. `requires self.inv()` and `ensures spec_frame_raw_value(result) == spec_frame_number(self@)` (`frame.rs:64-70`) cover total projection to the allocator/MMU frame index. The upper-bound part is supplied by `self.inv()` (`frame.spec.rs:80-83`).
- `from_frame_number`: good success/liveness spec. It guarantees `result is Ok` and on success `fa@ == spec_from_number(spec_frame_raw_value(frame_number)) && fa.inv()` (`frame.rs:116-122`). Although `caller_analysis.md:76-78` mentions an Err path for out-of-range frame numbers, an `arch::FrameNumber` value is already range-checked by construction (`number.rs:17-21`, `number.rs:47-53`) and modeled as in range by the trusted projection (`phys.spec.rs:114-120`), so unconditional success is stronger and appropriate.
- **`from_raw_value`: incomplete.** Its target contract only says `Ok(fa) ==> fa@ == raw_addr as int && fa.inv()` (`frame.rs:138-142`). It has no `match` and no Err arm. Therefore the spec permits spurious `Err` for any input, including a valid aligned address, and gives callers no abstract reason for failure. This violates the spec-design error-path/liveness rules and misses caller expectations from `caller_analysis.md:105-114`.
- Related trust-boundary weakness: the local `assume_specification` for `<PhysicalAddress as Address>::from_raw_value` uses `Err(_) => true` (`frame.spec.rs:110-118`). Even if platform physical-validity is opaque, this tautological failure arm is exactly the one-sided error-spec anti-pattern; it also prevents `FrameAddress::from_raw_value` from exposing a useful invalid/not-aligned failure predicate.

## Caller Coverage (Covered 18/20 + Missing)

Independent caller scan agrees with the main shape of `caller_analysis.md`, but found two additional real `FrameAddress::from_raw_value` call sites in `mm/phys/manager.rs:430` and `mm/phys/manager.rs:438`; these are not spec/comment false positives in the current tree.

| Target | Caller expectations checked | Covered | Missing |
|---|---:|---:|---|
| `FrameAddress` type/View/inv | 5 | 5 | None. Copy newtype exists (`frame.rs:34-36`); View is physical-address `int` (`frame.spec.rs:57-62`); inv covers alignment and representable frame number (`frame.spec.rs:80-83`); representation remains hidden by closed View. |
| `from_frame_number` | 5 | 5 | None. Success is guaranteed (`frame.rs:116-122`), which is stronger than the stale caller-analysis Err expectation. Address scaling and `fa.inv()` are explicit. Round-trip is derivable with `into_frame_number`'s spec and `spec_from_number`/`spec_frame_number` definitions (`phys.spec.rs:63-72`). |
| `into_frame_number` | 4 | 4 | None. `requires self.inv()` (`frame.rs:64-67`) captures totality; result projection is exact (`frame.rs:67-69`); representability follows from `FrameAddress::inv()` (`frame.spec.rs:80-83`). |
| `from_raw_value` | 4 | 2 | **Missing Err semantics**: no guarantee that `Err` means invalid/not page-aligned. **Missing liveness/success condition**: no guarantee that a valid aligned raw address succeeds. This matters for boot identity mapping (`boot_init.rs:203-208`) and contiguous kernel frame wrapping/cleanup (`manager.rs:426-438`). |
| `into_raw_value` | 2 | 2 | None. Raw identity is explicit (`frame.rs:95-100`) and directly supports the pointer arithmetic/casts listed in `caller_analysis.md:121-130`. |

Missing properties to add for `from_raw_value`:

1. A `match result { Ok(fa) => ..., Err(e) => ... }` postcondition instead of success-only implication.
2. A caller-usable abstract failure/liveness predicate, e.g. at minimum `Err ==> raw_addr as int % spec_page_size() != 0 || !valid_physical_raw(raw_addr)` and `valid_physical_raw(raw_addr) && raw_addr % PAGE_SIZE == 0 ==> Ok`, where `valid_physical_raw` is the chosen external-bottom physical-validity predicate.

## Proof Completeness

Independent guardrail count over the three frame files, excluding comments:

- `admit()`: **0**
- `external_body`: **0**

This satisfies the hard proof-completeness gate: no remaining `admit()` and no current-module `external_body` placeholders. The old bridge proof gap described in `bugs.md:9-31` is no longer an `admit`; it is now an explicit governed axiom (`frame.proof.rs:38-41`).

## TCB Compliance

Frame-module trust mechanisms found:

- `assume_specification[::arch::mem::PAGE_SIZE]` (`frame.spec.rs:45-48`). This boundary is referenced as the existing `::arch::mem::PAGE_SIZE` frame boundary in `tcb-allowed.md:181-190`, `tcb-allowed.md:217-226`, and `tcb-allowed.md:272-280`.
- `assume_specification[<PhysicalAddress as Address>::from_raw_value]` (`frame.spec.rs:110-119`), listed in `tcb-allowed.md:285-300`.
- `assume_specification[<PageAligned<T> as core::ops::Deref>::deref]` (`frame.spec.rs:129-134`), listed in `tcb-allowed.md:301-309`.
- `axiom fn lemma_phys_view_is_spec_addr` (`frame.proof.rs:38-41`), listed in `tcb-allowed.md:311-334`.

No frame-module `external_body` or `assume_specification`/`axiom fn` is missing from the pre-approved TCB. Note: the `PAGE_SIZE` entry is documented in prose rather than as a bullet; this is not a blocker, but the TCB file would be clearer if it had an explicit bullet for it.

## Guardrails Compliance (exact counts)

Independent counts over `frame.rs`, `frame.spec.rs`, `frame.proof.rs`:

| Construct | Count | Evidence |
|---|---:|---|
| `admit()` | 0 | independent grep/Python count |
| `assume()` | 0 | independent grep/Python count |
| real `external_body` | 0 | independent grep/Python count; comment-only mentions excluded |
| `assume_specification` | 3 | `frame.spec.rs:45`, `frame.spec.rs:110`, `frame.spec.rs:129` |
| cfg-gated exec | 0 | cfg gates are only includes/imports: `frame.rs:9-12`, `frame.rs:22-23` |
| `axiom fn` | 1 | `frame.proof.rs:38-41` |

No guardrail blocker from `admit`, `assume`, or cfg-gated exec code.

## AST Consistency (PASS with documented deviations)

Independent run of `ast_consistency.py src/kernel/src/hal/mem/types/address/frame.rs` reports **6/9 function matches and 3 MISMATCHes**: `from_frame_number`, `from_raw_value`, `into_frame_number`. These match the authoritative summary (`authoritative_data.md:21-24`).

Per-diff verdicts:

1. `FrameAddress::from_frame_number`: **semantically equivalent documented rewrite**. Original nested call `PageAligned::from_address(PhysicalAddress::from_number(frame_number))?` is split into `let physical_address = PhysicalAddress::from_number(frame_number); PageAligned::from_address(physical_address)?` (`frame.rs:122-131`). Same evaluation order and same `?` site.
2. `FrameAddress::from_raw_value`: **semantically equivalent documented rewrite**. Original `PhysicalAddress::from_raw_value(raw_addr)?` remains before `PageAligned::from_address(...) ?`, now bound to a local (`frame.rs:142-151`). Same failure order and return behavior.
3. `FrameAddress::into_frame_number`: **semantically equivalent documented rewrite**. Original auto-deref call is made explicit by copying the inner `PhysicalAddress` (`frame.rs:70-79`). `PhysicalAddress` is `Copy` (`phys.rs:42-44`), and `PageAligned<T>::deref` returns `&self.0` (`page.rs:183-188`), so this preserves the value and call target.

Each mismatch has a `// VERUS DEVIATION (pre-approved: f(complex_expr) -> let x = complex_expr; f(x))` comment (`frame.rs:71-75`, `frame.rs:123-127`, `frame.rs:143-147`). I found no separate isolated reproducer under this module's logs; because these are exactly the ast-consistency skill's pre-approved intermediate-binding deviation, I do **not** classify the absence of a reproducer as an AST blocker. It is a documentation gap only if the project now requires reproducers even for pre-approved deviations.

## Verification (verify-kernel + make verify/bitmap analysis)

- Accept `make verify-kernel` PASS from the authoritative run: exit 0, no frame-file cheating details, and frame guardrail counts clean (`authoritative_data.md:3-8`).
- `make build` exited 0/cached (`authoritative_data.md:29`).
- Full `make verify` failed in `verify-bitmap` due to vstd `std_specs/atomic.rs` generic mismatch errors before reaching kernel verification (`authoritative_data.md:31-34`). The changed files are frame files plus `mm/phys/upool.spec.rs` (`authoritative_data.md:36-40`), not bitmap/vstd. I therefore agree this is unrelated/pre-existing/environmental, not a frame regression.
- Independent `spec_drift.py check nanvix-phys-hal-frame-address` in the current cheating-elimination phase reports 0 drift. Independent `spec_drift.py git-diff ... --before verus-ai/sys-virt-address` reports one review item: `into_frame_number` gained `requires self.inv()` plus a new ensures. Because the function was previously unspecced and callers hold/prove `FrameAddress::inv()`, this is not a weakening blocker.

## Bug Summary

- `bugs.md` reports no code bugs and classifies the former `spec_addr(&pa) == pa@` proof gap as a false positive / external-bottom trust boundary (`bugs.md:5-31`). I agree: it is not a code bug, and it is now an explicit TCB-listed axiom (`frame.proof.rs:38-41`, `tcb-allowed.md:311-334`).
- No real code bug appears masked by `external_body` in the frame module; there is no frame-module `external_body`.
- The `upool.spec.rs` change strengthens `UserFrame::inv()` from page-alignment only to alignment plus representable frame number (`upool.spec.rs:62-65`). This is a lock-step strengthening with `FrameAddress::inv()`, not a weakening of guarantees. It is outside the named target files but acceptable as a consequential spec adjustment: `UserFrame@ == self.addr@` (`upool.spec.rs:35-40`) and the added conjunct is exactly the invariant required by frame allocator shims (`upool.spec.rs:43-65`). It should remain documented as a cross-module consequential change.

## Issues (priority-ordered)

1. **HIGH — Incomplete `FrameAddress::from_raw_value` error/liveness spec.**  
   Evidence: target contract is success-only (`frame.rs:138-142`); caller expectations include invalid/not-aligned Err semantics (`caller_analysis.md:105-114`); real current callers propagate or branch on conversion failure (`boot_init.rs:203-208`, `manager.rs:430-438`). Add a `match` postcondition and an abstract validity predicate strong enough to rule out spurious `Err` for valid aligned raw addresses.

2. **MEDIUM — Tautological external-bottom Err arm for `PhysicalAddress::from_raw_value`.**  
   Evidence: `Err(_) => true` in `frame.spec.rs:113-118`. This may be intentionally opaque, but it is one-sided and directly causes the target constructor to lack failure semantics. If physical validity is platform-specific, model it as an uninterpreted but named predicate rather than `true`.

3. **LOW — TCB/documentation polish for `PAGE_SIZE` and AST reproducer.**  
   `::arch::mem::PAGE_SIZE` is referenced as allowed in TCB prose but lacks a clear bullet. AST deviations have inline pre-approved comments but no isolated reproducer; acceptable under the current pre-approved deviation rule, but worth documenting if stricter evidence is desired.

## Result: FAIL

Guardrails, TCB registration, AST consistency, and `verify-kernel` are clean. The failure is specification completeness: `FrameAddress::from_raw_value` does not specify its Err path or success/liveness condition for valid aligned raw addresses, and its key dependency has a tautological `Err(_) => true` assumption. This leaves real caller expectations under-specified even though the current proof passes.
