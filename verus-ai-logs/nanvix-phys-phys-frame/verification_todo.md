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

---

## DECISIVE cross-reference (third review pass): these admits are owned by the phys-manager phase and are UNSOUND to convert

The four admits are **already owned and documented** by the separate
`nanvix-phys-phys-manager` phase:
`verus-ai-logs/nanvix-phys-phys-manager/verification_todo.md`. That phase's
harness-generated analysis (with **committed isolated reproducers** under
`verus-ai-logs/nanvix-phys-phys-manager/cheating-elimination/reproducers/`)
established the following, which forbids any "elimination" in the phys-frame
phase:

1. **The postconditions are provably FALSE in editable scope, not merely
   unproven.** `View for PhysMemoryManager` (do-not-modify, `manager.spec.rs`)
   fixes `self@ == self.upool@`, and the kernel-alloc bodies call only the free
   functions `frame::alloc`/`alloc_contiguous`/`free` (none take `self.upool`),
   so Verus derives `final(self)@ == old(self)@`. The lemma goals then reduce to
   `v == v.alloc_one(a)` / `v == v.book_all(S)` — **false**. The admits bridge a
   goal with no model in the manager's editable scope.

2. **Converting 3 of the 4 to `external_body`/axiom is UNSOUND — proven by
   reproducers.** `alloc_one_unsound.rs` and `others_unsound.rs` show that
   `lemma_kernel_alloc_one`, `lemma_kernel_alloc_contiguous`, and
   `lemma_user_bulk_err_restored` as free-standing axioms let a caller prove
   `false` (`exploit() ensures false` → `1 verified, 0 errors`). Authoring such
   axioms would make the **entire** verification unsound — a strictly worse
   violation than the admit. Per **verus-constraints**/**spec-design**, axioms
   require human approval and must be sound; these are neither. I therefore must
   NOT convert them.

3. **The only sound resolution is out-of-scope for this phase.** It requires a
   *versioned/tracked global-partition token* threaded through the `frame`
   free-function layer (`frame::alloc`/`alloc_contiguous`/`free`) plus a
   singleton-attachment token produced by `init`/`Upool::new`. That is an **exec
   signature change** to the frame wrappers (forbidden by the source-integrity /
   ast-consistency rules and by "Do not change function signatures"), and those
   wrappers are `external_body` TCB-allowed precisely because the singleton's
   post-mutation `phys_view()` transition is deferred to this token. It is a
   coordinated cross-module design step, owned jointly by the frame
   proving/token-instrumentation work and the phys-manager phase — not by a
   phys-frame *cheating-elimination* pass that must preserve exec code and author
   no axioms.

4. **Hard-rule conflict.** The phys-frame task's hard rules state "Do not touch
   unlisted functions." All four lemmas (`lemma_manager_attached`,
   `lemma_kernel_alloc_one`, `lemma_kernel_alloc_contiguous`,
   `lemma_user_bulk_err_restored`) are in `manager.proof.rs` and are **unlisted**.
   The verified manager functions that depend on them
   (`alloc_user_frame`, `alloc_many_user_frames`, `alloc_kernel_frame`,
   `alloc_many_kernel_frames` — all verified in-body) mean the lemmas are
   load-bearing and cannot be deleted without breaking verified, out-of-scope
   manager code.

**Conclusion:** within the phys-frame cheating-elimination phase's authority,
these four admits can be neither (a) proven (unsatisfiable in editable scope),
(b) soundly converted (reproducers show axiom form is unsound), (c) deleted
(load-bearing for verified manager code), nor (d) touched at all (unlisted
functions). The sound fix is the §8 tracked-token instrumentation of the frame
free-function layer, which is an exec-signature change outside this phase's
mandate. This is a genuine architectural hand-off, not a skipped proof.
