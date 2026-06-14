# Verification TODOs — phys-frame cheating elimination

## Status of the phys-frame deliverable (frame.rs / frame.proof.rs / frame.spec.rs)

**Clean.** Zero `admit()`, zero `assume()`, zero `assume_specification`, zero
`trusted`, zero `exec_allows_no_decreases_clause`. The only `external_body`
functions in `frame.rs` (`instance`, `init`, `alloc`, `alloc_contiguous`,
`free_count`, `free`, `book`, `alloc_range`, `share`, `refcount`) are every one
on the human-approved `verus-ai-logs/tcb-allowed.md` list, all on **exec** fns
(none on proof fns). All in-scope `Inner::*` allocator methods verify in-body
(`make verify-kernel MODULE=mm::phys` → **58 verified, 0 errors**).

## Genuinely-stuck proofs (the 4 flagged `admit()`s)

The 4 flagged `admit()`s are **not** in the phys-frame target file. They live in
`src/kernel/src/mm/phys/manager.proof.rs` (the **phys-manager** module), are
**pre-existing** (committed before this phase started — `git diff 50e4de7c8 HEAD`
shows phys-frame work touched only log files), and are **out of the phys-frame
target scope** (the task's "verification-order target functions" are all in
`frame.rs`; manager lemmas are unlisted; hard rule "Do not touch unlisted
functions").

They are nonetheless flagged because `make verify-kernel MODULE=mm::phys`
compiles the whole `mm::phys` module and the cheating detector counts admits
module-wide.

### Root cause — the §8 global ghost-token deferral

`phys_view()` (`mod.spec.rs:98`) is a **parameter-free `uninterp spec fn`** — a
single global with no pre/post temporal index. `frame::alloc()` /
`frame::alloc_contiguous()` mutate the global `frame::INSTANCE` singleton
**without borrowing the manager `self`**, so Verus cannot observe `self@`
changing across those calls. The manager's view (`self@ == self.upool@`) is tied
to the global frame partition only through `lemma_manager_attached`
(`self@ == phys_view().frames`). Because there is no temporal index on
`phys_view()`, the pre-call manager state and the post-call global state cannot
both be named, so the per-call allocation transition cannot be expressed
soundly. `view_design.md` §8 specifies that this transition is realized in the
proving phase by a **ghost token over the `frame::INSTANCE`/`PhysMemoryManager`
singletons**; that token has not been realized.

### Why they cannot be discharged in this phase (empirical + structural)

Removing the 4 `admit()`s yields (`make verify-kernel MODULE=mm::phys`):

```
error: postcondition not satisfied
  --> manager.proof.rs:14:9   m@ == phys_view().frames            (lemma_manager_attached)
  --> manager.proof.rs:30:9   pre.free_frames.contains(addr)      (lemma_kernel_alloc_one)
  --> manager.proof.rs:31:9   post == pre.alloc_one(addr)         (lemma_kernel_alloc_one)
  --> manager.proof.rs:47:9   frames.len()==count … (contiguous)  (lemma_kernel_alloc_contiguous)
  --> manager.proof.rs:48:9   post == pre.book_all(kernel_addr_set(frames))
  --> manager.proof.rs:211:9  m@ == pre                           (lemma_user_bulk_err_restored)
verification results:: 54 verified, 4 errors
```

Each postcondition is **mathematically not derivable** from the lemma's
preconditions:

| Lemma (line) | Postcondition | Why unprovable from `requires` |
|--------------|---------------|--------------------------------|
| `lemma_manager_attached` (16) | `m@ == phys_view().frames` | `phys_view()` is `uninterp` and has no defined relation to `m.upool@`. Pure external-bottom attachment axiom. |
| `lemma_kernel_alloc_one` (35) | `pre.free_frames.contains(addr)`, `post == pre.alloc_one(addr)` | `addr`/`post` are free parameters; for arbitrary `addr` with only `pre.wf()`, membership/transition is false. The fact comes from the `frame::alloc` global mutation, invisible to `self`. |
| `lemma_kernel_alloc_contiguous` (55) | contiguity + `post == pre.book_all(...)` + `all_free(...)` | same: encodes the global contiguous-alloc transition not visible at the call site. |
| `lemma_user_bulk_err_restored` (216) | `m@ == pre` | `Vec::clear()` drops the taken `UserFrame`s; their `Drop` returns frames to the pool, but `Drop` side-effects are **not modeled in exec**, so the restoration is unprovable. |

### Why no legitimate in-scope fix exists

- **Cannot author the axiom.** `lemma_manager_attached` /
  `lemma_user_bulk_err_restored` are external-bottom trust assumptions. Per the
  **spec-design** and **verus-constraints** skills, `assume_specification` /
  `axiom` may only be written from the **human-approved** list. `tcb-allowed.md`
  governs `external_body` only and does **not** list these admits, so authoring
  axioms here would be unapproved cheating.
- **Cannot realize the §8 token in-scope.** Making `frame::alloc()`'s mutation
  visible to the manager requires threading a `Tracked` token **parameter**
  through `frame::alloc` / `Upool::alloc` (exec **signature change**) or adding
  tracked fields to the singleton structs — both forbidden by the
  source-integrity / anti-pattern rules ("Never change function signatures";
  "Do not add ghost/tracked fields to exec structs"). It is also the
  phys-manager proving phase's responsibility, not phys-frame's.
- **Cannot strengthen the lemmas' `requires`.** The call sites
  (`manager.rs:229/258/299/391/510`) invoke these lemmas precisely because they
  cannot derive the transition; moving the obligation into `requires` only
  relocates the same undischargeable goal to the caller.

### Required resolution (hand-off)

One of:
1. **Human approval** to record these four as external-bottom assumptions
   (move `lemma_manager_attached`, `lemma_user_bulk_err_restored`, and the
   `lemma_kernel_alloc_*` transition facts into the approved
   `assume_specification`/axiom list, e.g. an entry in `tcb-allowed.md` or the
   property-analysis Needed-Assumptions checklist), **or**
2. **Realization of the §8 global ghost token** during the phys-manager proving
   phase (a `tracked` permission over `frame::INSTANCE`/`PhysMemoryManager`),
   after which all four lemmas verify in-body and the admits are removed.

Both are outside the phys-frame phase's authority and scope.
