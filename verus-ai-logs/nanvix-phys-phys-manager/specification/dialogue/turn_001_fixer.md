## Response to Turn 1

### What I Changed

Strengthened the **user** allocation contracts (and only those) so the kernel-watermark
policy split is caller-observable. The kernel variants were left untouched, so the two
families of contracts are now distinct.

**File:** `src/kernel/src/mm/phys/manager.rs`

1. **`alloc_user_frame`** (`#[verus_spec]` `ensures`, `Ok(frame)` arm) — added two conjuncts:
   ```
   Ok(frame) => {
       &&& phys_view().frames.allocated_frames.contains(frame@)
       &&& frame@ % spec_page_size() == 0
       &&& phys_view().frames.free_frames.finite()      // NEW
       &&& spec_watermark_ok(phys_view().frames, 0)     // NEW
   },
   ```

2. **`alloc_many_user_frames`** (`#[verus_spec]` `ensures`, `Ok(())` arm) — added the same
   two conjuncts:
   ```
   Ok(()) => {
       &&& final(frames)@.len() == count
       &&& forall|i: int| 0 <= i < count as int ==>
               #[trigger] phys_view().frames.allocated_frames.contains(final(frames)@[i]@)
       &&& phys_view().frames.free_frames.finite()      // NEW
       &&& spec_watermark_ok(phys_view().frames, 0)     // NEW
   },
   ```

3. **`alloc_kernel_frame`** and **`alloc_many_kernel_frames`** — deliberately **unchanged**.
   The kernel variants are watermark-exempt, so their `Ok` arms intentionally omit
   `spec_watermark_ok`. This is what makes the user and kernel contracts distinct (per the
   policy split that justifies `check_user_watermark`'s existence).

`spec_watermark_ok` and `phys_view` were already in scope (used by `check_user_watermark`),
so no new imports were needed. `spec_watermark_ok(v, 0)` expands to
`v.free_frames.len() >= spec_kernel_watermark() + 0`, i.e. the post-state guarantee that
servicing the user request left at least `KERNEL_WATERMARK` frames free — exactly the
load-bearing fact the watermark gate provides. `free_frames.finite()` is added because
`PhysMemView::inv` (`mod.spec.rs:105`, `initialized ==> frames.wf()`) does not imply
finiteness, and `Seq/Set::len()` is only meaningful on a finite set.

The single-frame `ensures` blocks are no longer byte-identical:
- `alloc_user_frame` `Ok` arm now has 4 conjuncts (incl. `free_frames.finite()` +
  `spec_watermark_ok(..., 0)`).
- `alloc_kernel_frame` `Ok` arm still has 2 conjuncts (`allocated_frames.contains` +
  page-alignment).

### Verification

**`make verify-kernel MODULE=mm::phys`** → Exit 0:
```
verification results:: 11 verified, 0 errors (partial verification with `--verify-*`)
  verification: 11 verified, 0 errors (exit 0)
  status: CHEATING_DETECTED
```
(`CHEATING_DETECTED` reflects only the pre-existing, tcb-allowed `external_body` shims and
the spec-phase `admit()` lemmas — no new errors, no new cheating items.)

**`make build`** → Exit 0, 0 compiler errors (dual compilation intact).

**Contract-divergence check** (`sed -n` of the two single-frame `ensures`):
- user `Ok` arm: `allocated_frames.contains` + page-align + `free_frames.finite()` +
  `spec_watermark_ok(..., 0)`
- kernel `Ok` arm: `allocated_frames.contains` + page-align only

The blocks are no longer identical, as required.

### Result: FIXED
