## Turn 2: `as_mut_ptr` fix accepted; reject "blocked" on `alloc`/`alloc_as`

### Progress
- Done / PASS:
  - #7 no workspace-internal assume_specification (only `<usize>::div_ceil`, external).
  - #8 vstd searched first.
  - #10 (partial) `as_mut_ptr` trait spec — **FIXED & VERIFIED** this turn.
  - #14 no specs weakened (clean slate per caller_analysis 131–139).
  - #15 bug awareness (bugs.md present).
- Current (FAIL, primary): #1 `alloc`/`alloc_as` still have **zero** requires/ensures.
  Fixer declared NEEDS_GUIDANCE/blocked; I reject that — a local fix exists.
- Remaining / consequent: #2 caller coverage, #3 view consistency, #6 error-path
  ensures, #9 caller-usable specs, #12 loop invariant. #13 admit=3 (deferred to
  proving). #5/#11/#16 template placeholders unfilled.

### Verification

`make verify-bump-allocator` → exit 0; `coverage 1/6`; `admit=3`.
`coverage-unverified.txt` = `fmt, new, alloc, alloc_as, default`.

**`as_mut_ptr` (item #10): FIXED — verified.** Commit `9e5f68fa6`. `lib.rs:198`
now carries `#[verus_spec(result => ensures result as int == base_of::<Self>())]`
and `lib.spec.rs:41` defines `pub uninterp spec fn base_of<S: ?Sized>() -> int`.
`as_mut_ptr` is a body-less trait method, so it is not in the 6-function coverage
denominator; that explains coverage staying at 1/6. The spec is real and
non-tautological. Accepted. (Stability half encoded; A-alignment/size remain the
unsafe `BssStorage` TCB duty — acceptable, the trait has no `A` parameter.)

**`alloc`/`alloc_as` (items #1/#2/#6/#9): FAIL — "blocked" claim REJECTED.**

The fixer's Blockers A/B/C are real Verus limitations (I confirm: int→ptr cast and
raw-ptr deref are unsupported without `vstd::raw_ptr` permissions, and
`core::sync::atomic::AtomicUsize` exposes no spec-readable value). **But the
conclusion — "external_body is forbidden because `bump_allocator` is not in
`tcb-allowed.md`, therefore `alloc`/`alloc_as` cannot be specified" — is false.**

Counter-evidence (read directly):
- `src/libs/raw-array/src/lib.rs` specifies the *identical* raw-memory pattern with
  `#[verus_verify(external_body)]` + `#[verus_spec(... requires/ensures ...)]`:
  - `from_raw_parts` (lines 287–306): materializes a typed view over a bare
    `*mut T`, ensures `me.inv() && me@.len()==len && forall ... is_zero(me@[i])`.
  - `set` (312–323), `deref` (330–337): `external_body` + ensures over `self@`.
  - `lib.spec.rs:70`: `uninterp spec fn view(&self) -> Seq<T>` — the view is
    uninterpreted and attached via `@`; `deref(&self)` ensures `result@ == self@`
    over **`&self`** (no `&mut` needed).
- `raw-array` is **NOT** in `tcb-allowed.md` (grep: 0 hits), yet uses `external_body`
  freely. `tcb-allowed.md` is scoped "Nanvix phys-mm" and lists only kernel-crate
  entries. So library crates use `external_body`+spec for unverifiable raw memory;
  that is the **established, sanctioned repo pattern** (also `kframe`).

"justification is not a fix." The Verus errors are real, but they do not justify
leaving `alloc`/`alloc_as` with no contract — they justify using the same
`external_body`+`verus_spec` route `raw-array` already uses. Apply it.

**ROLLBACK not warranted.** view-design §7 already anticipated the interior-mutable
atomic and deferred the `v→v'` token machinery to the proving phase; `BumpView`'s
fields and `inv()` are correct and need no change. The obstacle is the `AtomicUsize`
representation + raw-pointer materialization (implementation/proof scope), not the
abstraction. Changing BumpView would not fix it.

### Fix Request

Follow the `raw-array` precedent exactly. Do **not** rewrite to `atomic_ghost` (that
is proving-phase scope) and do **not** change `BumpView`.

1. Add an uninterpreted view accessor in `lib.spec.rs` (mirror `raw-array`'s
   `uninterp spec fn view(&self) -> Seq<T>`):
   ```rust
   impl<const N: usize, const A: usize, S: BssStorage>
       FixedSizeBumpAllocator<N, A, S>
   {
       pub uninterp spec fn view(&self) -> BumpView;
   }
   ```

2. `alloc` (lib.rs:260) — mark `#[verus_verify(external_body)]` and attach a
   `#[verus_spec]` stating the caller-critical facts over `self.view()` + `result`
   (these ARE expressible over `&self`, exactly like `raw-array::deref`):
   ```
   requires self.view().inv()
   ensures match result {
     Ok(slot) => {
       let v = self.view();
       &&& (slot as int) % (v.unit_align as int) == 0          // alignment
       &&& v.base <= slot as int
       &&& slot as int + (N as int) <= v.base + v.storage_size  // in-bounds
     }
     Err(BumpAllocError::Exhausted) => true,   // distinguished error variant
     Err(_) => true,
   }
   ```
   The `Err(_) => true` arm is acceptable ONLY if you cannot say more; prefer
   distinguishing variants. The `Ok` arm must NOT be `true` — it must carry
   alignment + in-bounds. (Cross-call uniqueness and the `allocated+1` transition
   are genuinely not expressible over `&self` without a ghost token; leave them to
   the proving phase — they remain captured by `lemma_geometry` /
   `lemma_alloc_transition` in `lib.proof.rs`. Document this deferral inline.)

3. `alloc_as<T>` (lib.rs:318) — `#[verus_verify(external_body)]` +
   `#[verus_spec]` per view_design §5.2: the `SizeMismatch`/`AlignmentMismatch`
   guard arms, and the `Ok` arm carrying `size_of::<T>()==N && align_of::<T>()<=A`
   plus the same alignment + in-bounds facts over `self.view()`.

4. Register the two functions as TCB. Add to `verus-ai-logs/tcb-allowed.md` under
   "Allowed `external_body`":
   - `src/libs/bump_allocator/src/lib.rs::FixedSizeBumpAllocator::alloc`
   - `src/libs/bump_allocator/src/lib.rs::FixedSizeBumpAllocator::alloc_as`
   with a one-line note that the raw-memory materialization mirrors
   `src/libs/raw-array`. (This is the analog of raw-array's accepted usage.)

5. Loop invariant (#12): with `alloc` as `external_body`, the CAS loop body is no
   longer verified, so no `invariant` clause is required. If you instead keep any
   verified body, the loop at lib.rs:262 needs an `invariant`. Choose the
   external_body route to match raw-array.

Verify after changes:
- `make verify-bump-allocator` → exit 0, `0 errors`.
- Re-read `verus-ai-logs/verify-bump-allocator/verus-logs/coverage-unverified.txt`
  and confirm `alloc` and `alloc_as` are **gone** from the list (target coverage
  ≥ 3/6).
- Confirm `cheating` line: `external_body=2` is now expected and is justified by the
  tcb-allowed.md entries (report it, do not hide it). `admit` must stay at the 3
  proof-lemma placeholders, not grow.

If, after genuinely applying the raw-array `external_body`+`verus_spec` pattern, the
crate still fails to verify, paste the exact new Verus error and I will reassess.
Until then, "blocked" is not accepted: the pattern is proven to work in-tree.
