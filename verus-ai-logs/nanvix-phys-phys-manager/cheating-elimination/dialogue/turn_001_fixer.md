## Response to Turn 1

### What I Changed

**No source line was changed in `manager.rs` / `manager.spec.rs` /
`manager.proof.rs`** — and I will show below, with executable Verus output, that
*any* in-scope change able to remove the 4 `admit()`s is either (i) rejected by
Verus (unprovable) or (ii) **provably unsound** (lets a caller derive `false`).
I did **not** stop at prose: I ran every approach the reviewer named and pasted
the exact results. New artifacts committed:

- `cheating-elimination/reproducers/alloc_one_realbody.rs`
- `cheating-elimination/reproducers/alloc_one_unsound.rs`
- `cheating-elimination/reproducers/others_unsound.rs`
- `verification_todo.md` — updated with the unsoundness finding + missing-contract list.

The four targeted obligations are:

| Lemma | Line | ensures |
|-------|------|---------|
| `lemma_manager_attached` | manager.proof.rs:12 | `m@ == phys_view().frames` |
| `lemma_kernel_alloc_one` | manager.proof.rs:27 | `pre.free_frames.contains(addr)`, `post == pre.alloc_one(addr)` |
| `lemma_kernel_alloc_contiguous` | manager.proof.rs:40 | `post == pre.book_all(kernel_addr_set(frames))`, … |
| `lemma_user_bulk_err_restored` | manager.proof.rs:210 | `m@ == pre` |

---

### Attempts (each tried, with exact Verus output)

#### Approach A — discharge in-scope (remove the `admit()`, prove from hypotheses)

Removed all four `admit()`s and ran `make verify-kernel MODULE=mm::phys`:

```
error: postcondition not satisfied --> manager.proof.rs:14:9   (m@ == phys_view().frames)
error: postcondition not satisfied --> manager.proof.rs:31:9   (pre.free_frames.contains(addr))
error: postcondition not satisfied --> manager.proof.rs:32:9   (post == pre.alloc_one(addr))
error: postcondition not satisfied --> manager.proof.rs:49:9   (pre.all_free(kernel_addr_set(frames)))
error: postcondition not satisfied --> manager.proof.rs:50:9   (post == pre.book_all(kernel_addr_set(frames)))
error: postcondition not satisfied --> manager.proof.rs:214:9  (m@ == pre)
verification results:: 38 verified, 4 errors
```

Isolated minimal reproducer for `lemma_kernel_alloc_one`
(`reproducers/alloc_one_realbody.rs`, run with standalone `verus`):

```
error: postcondition not satisfied --> alloc_one_realbody.rs:32:9   pre.free_frames.contains(addr)
error: postcondition not satisfied --> alloc_one_realbody.rs:33:9   post == pre.alloc_one(addr)
verification results:: 0 verified, 1 errors
```

**Why:** the lemma bodies have only `requires pre.wf()`; the transition facts are
absent from every in-scope contract. The caller cannot supply them either — see
the missing-contract analysis below.

#### Approach B — reviewer's fallback: convert to a tcb-allowed `external_body` boundary

Tested `#[verifier::external_body]` on the lemmas (the only Verus mechanism for a
"trusted boundary"). For 3 of 4 this **injects a provably-false axiom**.

`reproducers/alloc_one_unsound.rs` — `lemma_kernel_alloc_one` as `external_body`,
with an `exploit() ensures false` that calls it on an empty-free `wf` partition:

```
verification results:: 1 verified, 0 errors
```

(The verified function is `exploit() ensures false` — Verus accepts a proof of
`false`. The axiom claims `pre.free_frames.contains(addr)` for *arbitrary* `addr`,
so picking `addr ∉ free_frames` yields a contradiction.)

`reproducers/others_unsound.rs` — `lemma_user_bulk_err_restored` as
`external_body` (`m@ == pre` for arbitrary `wf pre`), `exploit_err_restored()`
calls it on two distinct `wf` partitions:

```
verification results:: 1 verified, 0 errors
```

(`m@ == p1` and `m@ == p2` ⟹ `p1 == p2` for arbitrary distinct partitions ⟹
`false`.) `lemma_kernel_alloc_contiguous` has the same universal-over-arbitrary-
`frames` shape and is unsound for the identical reason.

This is exactly the soundness landmine the **proving** reviewer flagged when it
deleted `lemma_kernel_alloc_err_empty` (`bugs.md` OBS-3). Converting these to
`external_body` is therefore strictly *worse* than the `admit()` (a silent
`false`-deriving axiom vs. a gated one), so I refused to do it.

Only `lemma_manager_attached` is a *sound* trust axiom (both `m@` and
`phys_view()` are `uninterp`, no counterexample constructible). But converting it
alone leaves 3 admits and does not reach `admit=0`; and even granting it, the
kernel-step lemmas stay unprovable (Approach C).

#### Approach C — reviewer option (a): thread the §8 ghost token

A `Tracked<token>` must be *produced* somewhere with a known value. The only
producers are `frame::instance()`, `Upool::new`, `init` — all `external_body` and
all **out of scope** (`frame.rs` functions are unlisted; the 6 in-scope manager
fns cannot gain a `Tracked` param without changing signatures that out-of-scope
callers in `mm::virt` depend on, and `PhysMemoryManager` cannot gain a ghost
field without an exec/struct change). There is **no in-scope source** for the
token. Confirmed structurally: `phys_view()` is a **parameter-free** `uninterp`
constant, so asserting `lemma_manager_attached` at both the pre- and post-points
of a `frame::alloc` call forces `old(self)@ == self@ == phys_view().frames` — i.e.
`pre == post` — which contradicts the required `post == pre.alloc_one(addr)`. The
constant accessor cannot express a `v → v'` step; a versioned/tracked token in the
frame layer is required.

### Missing contract (precisely which callee must supply what)

- `lemma_manager_attached`: needs a contract relating `Upool@` / `PhysMemoryManager@`
  to `phys_view().frames`. `frame::instance()` (frame.rs:675) exports
  `(*r)@ == phys_view().frames` for the **Inner** singleton, but nothing relates
  the manager's view source `Upool@` to it. Requires an attachment token produced
  by `init`/`Upool::new` (external_body, out of scope).
- `lemma_kernel_alloc_one`: callee `frame::alloc()` (frame.rs:769) ensures only
  `Ok(frame) => frame.inv() && phys_view().frames.allocated_frames.contains(frame@)`.
  Must additionally export the *stepped* transition
  `phys_view'().frames == phys_view().frames.alloc_one(frame@)` via a versioned token.
- `lemma_kernel_alloc_contiguous`: callee `frame::alloc_contiguous()` (frame.rs:799)
  ensures only base alignment + range bound; must export
  `phys_view'().frames == phys_view().frames.book_all(region_frame_addrs(base, count·PS))`.
- `lemma_user_bulk_err_restored`: restoration is via `Vec::clear() → UserFrame::drop
  → frame::free`. Verus does not model `Drop`; needs a token-aware `frame::free`
  step per dropped handle (out of scope) or an exec redesign (forbidden — changes
  behavior/AST).

### Verification

`make verify-kernel MODULE=mm::phys` (source byte-identical to phase start —
`git diff a8d643993 -- manager.{rs,spec.rs,proof.rs}` is empty):

```
verification results:: 42 verified, 0 errors
Global: assume=0 external_body=18 admit=24 trusted=0 cfg_gate=15
status: CHEATING_DETECTED
```

Manager admits still present (the 4 above). No spec weakened
(`git diff` empty ⟹ spec-drift = 0). No new `assume`/`external_body`.

`./z build` (exec build, Verus erased):

```
[OK]    Build complete.   (exit 0)
```

### Result: NEEDS_GUIDANCE

The 4 `admit()`s **cannot be eliminated within `manager.{rs,spec.rs,proof.rs}`**.
Proven, not asserted:

1. **Unprovable in-scope** — Approach A: real bodies → `postcondition not satisfied`
   (`38 verified, 4 errors`; isolated `alloc_one_realbody.rs` → `0 verified, 1 errors`).
2. **Unsound to convert** — Approach B: `external_body` axioms for 3 of 4 verify a
   proof of `false` (`alloc_one_unsound.rs`, `others_unsound.rs` → `1 verified, 0
   errors`). The reviewer's tcb-boundary fallback is therefore unsafe here, exactly
   like the deleted `lemma_kernel_alloc_err_empty`.
3. **No in-scope token** — Approach C: every token producer is an out-of-scope
   `external_body` frame-layer fn, and the parameter-free `phys_view()` cannot
   express the `v → v'` step.

**Required guidance / unblock:** verify and token-instrument the `frame`
free-function layer (`nanvix-phys-phys-frame`) — which itself still carries 8
`admit()`s (`frame.rs:136/213/298/379/442/497/535/583`) — to export (a) a singleton
attachment token (`Upool@ == phys_view().frames`) and (b) a versioned partition-step
token threaded through `frame::alloc`/`alloc_contiguous`/`free`. All four manager
admits then discharge mechanically. This is a cross-module sequencing dependency,
not a manager-scope proof gap; reproducers are committed under
`cheating-elimination/reproducers/`.
