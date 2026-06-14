# Verification TODO — phys-manager (cheating-elimination)

Honest hand-off of the proof gaps that could **not** be discharged within the
`kernel::mm::phys::manager` editable scope (`manager.rs`, `manager.spec.rs`,
`manager.proof.rs`). Recording here does **not** make the phase pass: each
remaining `admit()` still trips the cheating gate, so the phase result is
**BLOCKER**.

All four are the **same root cause**: the §8 global ghost-token attachment is
realized in the `frame` free-function layer, which is **out of scope** and
itself **still unverified** (`frame.rs` carries 8 `admit()`s — `alloc`,
`alloc_contiguous`, `free`, `share`, `refcount`, `book`, `is_covered`,
`alloc_range`). Until that layer is verified and exposes a tracked
partition-token, the manager cannot link its abstract view `self@`
(`= self.upool@`, a `closed` view over the `external_body` opaque `Upool`) to
the global `phys_view().frames`, nor express the pre→post transition of a
free-function allocator.

Evidence: removing all four `admit()`s and re-running
`make verify-kernel MODULE=mm::phys` yields exactly four
`postcondition not satisfied` errors at the lemma seams
(`38 verified, 4 errors`), with no collateral. Restoring them returns the tree
to `42 verified, 0 errors`.

---

## 1. `lemma_manager_attached` — `manager.proof.rs:12`

- **Postcondition that fails:** `m@ == phys_view().frames` (`manager.proof.rs:14`).
- **Verus error:** `error: postcondition not satisfied --> manager.proof.rs:14:9`.
- **Why blocked:** `m@` reduces to `m.upool@`. `Upool` is `external_body`
  (`upool.rs:221`, TCB-listed) with no spec-readable state, so `m.upool@` is
  opaque; `phys_view()` is a parameter-free `uninterp` constant
  (`mod.spec.rs:98`, a **DO-NOT-MODIFY** global view). No contract in the
  manager's editable scope links the two — the link is the singleton ghost
  token over `frame::INSTANCE` / the `PhysMemoryManager` singleton, owned by the
  out-of-scope `frame` layer.
- **Unblock prerequisite:** verify `frame.rs` and have `frame::instance()` /
  `Upool` expose a tracked token whose value is `phys_view().frames`; then this
  lemma becomes `token.value() == self.upool@` by the token invariant.

## 2. `lemma_kernel_alloc_one` — `manager.proof.rs:27`

- **Postconditions that fail:** `pre.free_frames.contains(addr)` and
  `post == pre.alloc_one(addr)` (`manager.proof.rs:31-32`).
- **Verus error:** `error: postcondition not satisfied --> manager.proof.rs:31:9`
  and `:32:9`.
- **Why blocked:** the caller `alloc_kernel_frame` obtains its frame from
  `frame::alloc()` — a **free function** (`frame.rs:769`) that takes no `self`.
  In the Verus exec model `self` is untouched across the call, so the only
  provable fact is `final(self)@ == old(self)@`, while the spec demands the
  transition `final(self)@ == old(self)@.alloc_one(addr)`. `frame::alloc`'s
  contract can only reference the parameter-free constant `phys_view().frames`
  (it cannot express a `v -> v'` step). Contrast the **user** path, which is
  fully proven precisely because `Upool::alloc` takes `&mut self`
  (`upool.rs:279`, `final@ == old@.alloc_one(uf@)`) and threads the transition.
- **Unblock prerequisite:** a versioned/stepped tracked token threaded through
  `frame::alloc` (signature change to the out-of-scope `frame` layer) so the
  free→reserved step is expressible.

## 3. `lemma_kernel_alloc_contiguous` — `manager.proof.rs:40`

- **Postconditions that fail:** `frames.len() == count`,
  `kernel_frames_contiguous(frames, count)`,
  `post == pre.book_all(kernel_addr_set(frames))`,
  `pre.all_free(kernel_addr_set(frames))` (`manager.proof.rs:49-52`).
- **Verus error:** `error: postcondition not satisfied --> manager.proof.rs:49:9`
  and `:50:9`.
- **Why blocked:** identical to #2 but for the contiguous bulk path.
  `frame::alloc_contiguous` (`frame.rs:799`) is a free function returning only a
  base address plus a range bound; the per-frame free→reserved transition over
  the global partition is inexpressible without the stepped token.
- **Unblock prerequisite:** same as #2.

## 4. `lemma_user_bulk_err_restored` — `manager.proof.rs:210`

- **Postcondition that fails:** `m@ == pre` (`manager.proof.rs:214`).
- **Verus error:** `error: postcondition not satisfied --> manager.proof.rs:214:9`.
- **Why blocked:** on a mid-bulk failure `alloc_many_user_frames` calls
  `frames.clear()`, which **drops** the already-allocated `UserFrame`s; their
  `Drop` calls `frame::free` to return the frames, restoring the partition.
  Verus does **not** model `Drop` side effects, and `frame::free` is a free
  function that does not touch `self.upool@`, so after the proven loop
  transitions `self.upool@ == g_old.book_all(...)` and the model cannot reduce it
  back to `pre`. This is a genuine Drop-semantics + free-function gap.
- **Unblock prerequisite:** model the `UserFrame::drop` → `frame::free` release
  on the tracked partition token (again rooted in the out-of-scope `frame`
  layer), or an exec redesign that releases frames through a `&mut`-threaded
  method instead of `Drop` (out of scope — would change exec behavior).

---

## Sequencing recommendation

Verify the `frame` free-function layer (`nanvix-phys-phys-frame`) **first** so
it exposes a tracked global partition token. All four manager admits then
discharge mechanically:

- #1 from the token's value invariant,
- #2/#3 from the token's `alloc`/`alloc_contiguous` step lemmas,
- #4 from the token's `free` step applied per dropped handle.

No spec was weakened and no `external_body`/`assume` was introduced to mask
these gaps.

---

## Update (cheating-elimination turn 1 fixer): conversion-to-boundary is UNSOUND

The reviewer's fallback ("convert the admit into a tcb-allowed.md-listed
`external_body` boundary") was **tested and rejected by Verus** for 3 of the 4
lemmas: as free-standing `#[verifier::external_body]` axioms they are
**provably false** (a caller derives `false`). Isolated reproducers are committed
under `cheating-elimination/reproducers/`:

- `alloc_one_realbody.rs` — `lemma_kernel_alloc_one` with a real body →
  `0 verified, 1 errors` (postconditions `pre.free_frames.contains(addr)` and
  `post == pre.alloc_one(addr)` not satisfied): **unprovable in-scope**.
- `alloc_one_unsound.rs` — same lemma as `external_body` axiom → `1 verified,
  0 errors` where the verified proof is `exploit() ensures false`: **unsound**.
- `others_unsound.rs` — `lemma_user_bulk_err_restored` as `external_body` axiom
  → `1 verified, 0 errors` proving `false`: **unsound**.
  `lemma_kernel_alloc_contiguous` is the same universal-over-arbitrary-`frames`
  shape as `alloc_one`, hence equally unsound as an axiom.

Only `lemma_manager_attached` (`m@ == phys_view().frames`) is a *sound* trust
axiom (both sides `uninterp`, no counterexample constructible), but even granting
it, the kernel-step lemmas remain unprovable because the **parameter-free**
`phys_view()` cannot express a `v → v'` transition (asserting attachment at the
pre- and post-points forces `pre == post`, contradicting `post == pre.alloc_one`).

**Net:** the 4 admits can be eliminated only by an out-of-scope change to the
`frame` free-function layer (a versioned/tracked global-partition token threaded
through `frame::alloc`/`alloc_contiguous`/`free`, plus a singleton attachment
token produced by `init`/`Upool::new`). They cannot be discharged or *soundly*
converted within `manager.{rs,spec.rs,proof.rs}`.
