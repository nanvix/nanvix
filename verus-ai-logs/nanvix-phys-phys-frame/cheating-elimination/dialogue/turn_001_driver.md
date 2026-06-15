## Turn 1: Full checklist sweep — phys-frame cheating elimination

Scope: `src/kernel/src/mm/phys/frame.rs` (+ ghost `frame.spec.rs`, `frame.proof.rs`).
Base branch for AST/spec-drift comparison: `verus-ai/phys-kframe`.

### Progress
- Done (PASS): 1 admit, 2 assume, 3 trusted, 4 no_decreases, 5 cfg-gated exec,
  6 external_body allow-list, 7 AST consistency, 9 external_body re-confirm,
  10 spec drift / no weakening, 11 cross-module regression, 12 verification+build.
- Current: **8 — exec rewrites must carry `VERUS REWRITE` comment + minimal reproducer → FAIL.**
- Remaining: none unchecked (only item 8 is open).

### Verification (commands run + evidence)

Verification / build (exit codes verified by me, not taken on claim):
- `make verify-kernel MODULE=mm::phys` → **31 verified, 0 errors, Exit code 0**.
- `make verify-kernel` (all modules) → **Exit code 0**; `assume=0 external_body=23
  admit=0 trusted=0 no_decreases=0 cfg_gate=9` (global). The `CHEATING_DETECTED`
  banner is only the allow-listed-`external_body` flag, not a proof gap.
- `./z build -- all-kernel` → `Compiling kernel … Finished`, **0 warnings, 0 errors**.

Per-item evidence:

1. **admit() — PASS.** `admit=0` globally; `grep admit frame.{rs,spec.rs,proof.rs}` → none.
2. **assume() — PASS.** `assume=0`. The only `assume*` token in `frame.rs` is
   `INSTANCE.assume_init_mut()` (line 659) — a `core::mem::MaybeUninit` std method
   inside the allow-listed `external_body` `instance()`, not a Verus `assume()`.
3. **trusted — PASS.** `trusted=0`.
4. **exec_allows_no_decreases_clause — PASS.** `no_decreases=0`.
5. **cfg-gated exec code — PASS.** `frame.rs` has exactly two `#[cfg(...)]`:
   lines 49 & 52, both `#[cfg(verus_keep_ghost)]` gating `include!` of the ghost
   `frame.spec.rs` / `frame.proof.rs`. Ghost includes, not exec code.
6. **external_body allow-list — PASS.** 11 `external_body` fns in `frame.rs`:
   `Inner::alloc`(137), `Inner::alloc_contiguous`(210), `Inner::free`(290),
   `Inner::share`(368), `Inner::refcount`(428), `Inner::book`(481),
   `Inner::is_covered`(517), `Inner::alloc_range`(565), `instance`(652),
   `init`(689), `free`(888). Each individually checked against
   `verus-ai-logs/tcb-allowed.md`: the 8 `Inner::*` methods, `instance`, the
   `Drop`-path `free`, and the skip/exclude `init` are all listed. No unlisted
   `external_body`.
7. **AST consistency — PASS.** `git diff verus-ai/phys-kframe -- frame.rs
   frame.spec.rs frame.proof.rs`: `frame.spec.rs` unchanged; `frame.proof.rs`
   only adds the ghost `lemma_free_count`; every `frame.rs` hunk is at line 633+
   (the singleton/shim region). The `Inner` struct and all `Inner::*` method
   bodies/contracts (lines 1–632) are byte-identical to base. The shim exec
   deviations are zero-cost `let r/res` bindings plus erased `proof!` blocks
   (`auth.v.frames = (*r)@;`); `instance()` and the inner op are each still called
   exactly once → semantics, time, and space preserved.
8. **exec rewrite markers — FAIL.** See Fix Request below.
9. **surviving external_body listed — PASS.** Same per-function check as item 6.
10. **no specs weakened — PASS.** `frame.spec.rs` unchanged; the do-not-modify
    `Inner::*` contracts are byte-identical to base. The free-function shim
    contracts were **strengthened**, not weakened: they moved from a fixed
    `phys_view()` post-state (with `Err(_) => true`) to a `Tracked<&mut PhysAuth>`
    transition contract naming `old(auth)@`/`final(auth)@`, with the exact
    `spec_alloc_one`/`spec_alloc_set`/`spec_share` transition and **tightened**
    `Err` arms (`final(auth)@ == old(auth)@`, plus failure facts). No guarantee
    removed.
11. **cross-module regression — PASS.** Full `make verify-kernel` (all `mm::phys`
    modules: `frame`, `kframe`, `manager`, `upool`, `mod`) → Exit 0.
12. **verification + build — PASS.** `make verify-kernel MODULE=mm::phys` →
    31 verified / 0 errors / exit 0; `./z build -- all-kernel` → 0 warnings/errors.

### Fix Request (item 8 — `free_count` exec rewrite, `frame.rs:843–856`)

`free_count` rewrites the original base-branch exec body
`inner.bitmap.number_of_bits() - inner.bitmap.usage()` into split `let nbits` /
`let used` bindings. This **is** a genuine, necessary Verus-limitation rewrite — I
proved it: temporarily inlining the expression and running
`make verify-kernel MODULE=mm::phys` produced **`30 verified, 1 errors`**:

```
error: precondition not satisfied
   --> src/kernel/src/mm/phys/frame.rs: lemma_free_count(inner);
   ::: frame.proof.rs:95:9  inner.bitmap@.num_bits >= 0,  failed precondition
```

So the rewrite must stay. But checklist item 8 requires **every exec rewrite to
carry a `VERUS REWRITE` comment and a minimal reproducer.** This one currently
has a non-standard `// VERUS DEVIATION (pre-approved: ...)` comment and **no**
minimal reproducer. Justification text is not a substitute — make the code/marker
comply:

1. In `frame.rs` (the `free_count` body, lines ~845–849) replace the
   `// VERUS DEVIATION (pre-approved: ...)` comment with the project-standard
   `// VERUS REWRITE:` marker. The comment must (a) state the original idiom
   (`number_of_bits() - usage()`), (b) state that the split is semantically and
   cost-equivalent (`nbits - used` yields the identical `usize`), and (c) cite the
   Verus limitation (the `usize`→`num_bits >= 0` fact is only materialized at the
   `let nbits` binding point and cannot be recovered inside `lemma_free_count`
   because `Bitmap::inv()` hides the backing-slice bound).
2. Add a minimal reproducer demonstrating the failure, e.g.
   `verus-ai-logs/nanvix-phys-phys-frame/reproducers/04_free_count_inline_fails.rs`,
   reduced to the inlined `number_of_bits() - usage()` form that triggers the
   `num_bits >= 0` precondition error, and reference its path from the
   `VERUS REWRITE` comment.
3. Verify: `make verify-kernel MODULE=mm::phys` → must report **31 verified,
   0 errors, exit 0**, and `grep -n "VERUS REWRITE" src/kernel/src/mm/phys/frame.rs`
   must show the marker on `free_count`.

Do not weaken the contract, touch any `Inner::*` method, or change
`frame.spec.rs`. Only the `free_count` comment + a new reproducer file are in scope.
