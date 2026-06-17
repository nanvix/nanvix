# Polish Report: phys-manager

## Proof Extraction
- Blocks extracted: 1
  - `manager.rs` `alloc_many_user_frames` (old lines 228–233, the post-`check_user_watermark`
    block: `lemma_manager_attached` + `assert(self@ == g_old)` +
    `assert(g_old.user_alloc_ok(count))` + `lemma_user_bulk_base`)
    → `lemma_user_bulk_start` in `manager.proof.rs`.
    Call site is now a single 3-line `proof!` invoking `lemma_user_bulk_start(self, g_old, frames@, count as nat)`.
- Blocks kept inline: 9 (each ≤ 5 lines or a single lemma call / assert; confirmed by
  `check_proof_blocks.py … --all` → "9 total, 0 over 5 lines, all OK"):
  - `alloc_many_user_frames`: count==0 path (2 lemma calls), loop-body `lemma_user_bulk_step` (1),
    error-path `lemma_user_bulk_err_restored` (1).
  - `alloc_user_frame`: `lemma_manager_attached` (1).
  - `alloc_kernel_frame`: `if result is Ok { lemma_kernel_alloc_one(...) }` (5 lines, tightly
    coupled to `result->Ok_0`).
  - `alloc_many_kernel_frames`: `lemma_contig_no_overflow` (outer + inner cleanup loop) and
    `lemma_kernel_alloc_contiguous` (each ≤ 3 lines).

  Note: the 3 items flagged by the tool as ">5 clauses" are loop *invariants*, not extractable
  `proof {}` blocks. The user-bulk loop already factors its state through the `user_bulk_inv`
  spec predicate. The two kernel loop invariants reference `old(self)@` / `old(frames)@`, which
  cannot appear in a standalone spec fn, so extracting them would split each invariant and
  reduce readability; left as-is.

## Minimization
- Redundant assertions removed: 8
  - `alloc_many_user_frames`: removed seed block `assert(g_old == old(self)@)` + `assert(g_old.wf())`.
  - `alloc_many_kernel_frames`: removed seed block `assert(g_old == old(self)@)` + `assert(g_old.wf())`.
  - `lemma_user_bulk_step`: removed `assert(s2.len() == s.len() + 1)`,
    `assert(frames.push(uf).len() == frames.len() + 1)`,
    `assert(g_old.free_set().contains(uf@) && !s.contains(uf@))`,
    `assert(mview.free_set() =~= g_old.free_set().difference(s))`.
  - (Additionally, the two inline asserts `self@ == g_old` and `g_old.user_alloc_ok(count)`
    in the extracted block were subsumed by `lemma_user_bulk_start`.)
- Redundant lemmas/hints removed: 0
  - No dead spec functions (every spec fn / lemma is reachable from a `requires`/`ensures`/
    `invariant` or another lemma).
  - No duplicate lemmas (no two lemmas share identical `requires` + `ensures`).

## Verification
- `make verify-kernel MODULE=mm::phys`: 67 verified, 0 errors (exit 0).
- `make verify-kernel` (full): exit 0; cheating inventory unchanged from baseline
  (`assume=0 external_body=23 admit=4`) — no new admits/external_body introduced.
