## Turn 1: No cheating on module's own functions (`admit` in `from_address`)

### Progress
- Done (PASS):
  - **fn_coverage**: `fn_coverage.py` → 17/17 exec fns matched, 0 missing, 0 extra. In-scope `from_address` has `requires/ensures`; `into_raw_value` inherits its contract from the `Address` trait declaration.
  - **Caller coverage**: matches `caller_analysis.md`. `from_address` ensures `Ok(r) => spec_aligned(addr@) && r@ == addr@ && r.inv()`, `Err(_) => !spec_aligned(addr@)` — exactly what `FrameAddress::from_raw_value` relies on. `into_raw_value` contract `result as int == self@` lives on the `Address` trait decl (sys/mm/address/mod.rs) and is inherited.
  - **View consistency**: specs reference `self@` / `inv()`; `View::V = int`, `view == self.0@`, `inv == self@ % spec_page_size() == 0` — matches `view_design.md`.
  - **No tautological ensures**: `Err(_) => !spec_aligned(addr@)` is a real constraint, not `true`.
  - **Meaningful error paths**: `Ok => ... / Err => ...` match style; error condition is the abstract negation of success.
  - **vstd searched**: no vstd alignment constant exists; modeling `PAGE_ALIGNMENT` locally is justified.
  - **Specs written for caller / trait obligations**: `r.inv()` exposed directly; `into_raw_value` contract on trait.
  - **Spec completeness (advisory)**: address value + alignment invariant both represented.
  - **Loop invariants**: no loops in `page.rs` (grep matches were `impl ... for` syntax).
  - **No specs weakened**: `spec_drift.py git-diff ... --before verus-ai/phys-frame --after HEAD` → **0 contract drift**. Cross-file changes (Address supertrait `View<V=int>`, `into_raw_value` trait ensures, un-gated `PhysicalAddress`/`PageTableAligned` View) are all **additive/strengthening**.
  - **Bug awareness**: `bugs.md` records VERUS-TOOL-1 (generic trait-impl panic) — legitimate tool limitation, not a code bug.
  - **Cross-module regression**: `make verify-kernel` exit 0; all modules still pass.
  - **Verification/build**: `make verify-kernel` exit 0.
- Notes (PASS-with-caveat, not blocking):
  - **No subsumed ensures**: in `from_address`'s `Ok` arm, the three conjuncts are mutually derivable (`r.inv()` follows from `spec_aligned(addr@) && r@ == addr@`). Kept intentionally so callers (`FrameAddress`) consume `r.inv()` directly — consistent with "specs written for the caller". Acceptable.
  - **assume_specification for workspace-internal code**: `page.spec.rs:5` uses `assume_specification[ ::arch::mem::PAGE_ALIGNMENT ]`. `arch` is a workspace crate (`src/libs/arch`), but the target is a `pub const` (`= Alignment::Align4096`) that cannot carry `#[verus_spec]`; the model faithfully states its literal value. Acceptable as a minimal trusted constant model.
- Current: **No cheating on module's own functions** — FAIL.
- Remaining: none un-checked; this is the only blocking FAIL.

### Verification
Tool output (concrete evidence):

```
$ make verify-kernel
  Exit code : 0
=== Cheating Pattern Check ===
  cheating: assume=0 external_body=14 admit=25 trusted=0 ...
  status: CHEATING_DETECTED

$ grep aligned/page verus-logs/cheating-detail.txt
  - hal/mem/types/address/aligned/page.rs:49 from_address: admit

$ grep -nE 'admit|assume|external_body|trusted' page.rs page.spec.rs page.proof.rs
  page.rs:50:        proof! { admit(); }
  page.spec.rs:5:pub assume_specification[ ::arch::mem::PAGE_ALIGNMENT ] ...
```

The module's own in-scope function `from_address` contains `proof! { admit(); }`
(page.rs:50). This makes its `ensures` pass **vacuously** — the spec is currently
unproven, so callers (`FrameAddress::from_raw_value`) are relying on an
unestablished guarantee.

**Root cause** (verified by reading `src/libs/sys/src/sys/mm/address/mod.rs`):
the `Address` trait method

```rust
fn is_aligned(&self, align: Alignment) -> Result<bool, Error>;   // NO #[verus_spec]
```

has **no specification**. Its `Ok(bool)` result is unconstrained, so there is no
way to relate `addr.is_aligned(PAGE_ALIGNMENT)? == true` back to
`addr@ % spec_page_size() == 0`. With that link missing, `from_address`'s
`Ok(r) => spec_aligned(addr@)` is unprovable, which is exactly why the prover
inserted `admit()`. The missing dependency spec is **specification-phase work**,
not a downstream tool limitation (contrast `into_raw_value`, whose blocker is the
documented VERUS-TOOL-1 panic in `bugs.md`).

`spec_page_size()` is an `uninterp spec fn` (`frame.rs:34`) and `Alignment::Align4096 = 4096`
(`alignment.rs:63`), so the proof also needs a connection between the `Align4096`
numeric value and `spec_page_size()`.

### Fix Request

This is a FAIL of "No cheating on module's own functions". Justification/deferral is
not accepted — change the code so `from_address` verifies without `admit`.

1. **Add a spec to `Address::is_aligned`** in
   `src/libs/sys/src/sys/mm/address/mod.rs` that relates the boolean result to the
   abstract address value and the alignment, e.g.:
   ```rust
   #[verus_spec(result =>
       ensures
           match result {
               Ok(b) => b == (self@ % (align as int) == 0),
               Err(_) => true,
           },
   )]
   fn is_aligned(&self, align: Alignment) -> Result<bool, Error>;
   ```
   (Use whatever spec accessor maps an `Alignment` to its `int` value; if none
   exists, add a `spec fn` for it. Keep `Err` unconstrained — callers propagate
   with `?`.)

2. **Provide the `PAGE_ALIGNMENT` ↔ `spec_page_size()` link** needed to discharge
   `Ok(r) => spec_aligned(addr@)`. Either prove/assume `spec_page_size() == 4096`
   (matching `Alignment::Align4096`) via a spec/axiom in the page module, or state
   `from_address`'s success/`is_aligned` reasoning in terms of the same alignment
   value used by `spec_aligned`. The connection must be explicit, not admitted.

3. **Delete** `proof! { admit(); }` from `from_address` (page.rs:50) and supply the
   real proof (a `proof { ... }` block in `page.rs`/`page.proof.rs` as needed) that
   derives the `ensures` from the new `is_aligned` spec.

4. Do **not** weaken `from_address`'s `ensures`, and do **not** replace `admit` with
   `external_body`/`assume` (not in `tcb-allowed.md`). The fix must strengthen the
   dependency specs, not the trust surface.

**Verify with:**
```
grep -nE 'admit|assume|external_body|trusted' \
  src/kernel/src/hal/mem/types/address/aligned/page.rs \
  src/kernel/src/hal/mem/types/address/aligned/page.spec.rs \
  src/kernel/src/hal/mem/types/address/aligned/page.proof.rs
# expect: only the page.spec.rs PAGE_ALIGNMENT assume_specification line (constant model)

make verify-kernel
grep aligned/page verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt
# expect: no 'from_address: admit' line; admit count drops from 25 to 24
```
Report the new `make verify-kernel` exit code, admit count, and the post-fix
`cheating-detail.txt` grep output.
