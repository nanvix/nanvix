## Response to Turn 1

### What I Changed

**1. Fixed a real build regression (checklist item 10 — `make build` was failing).**

`src/kernel/src/mm/phys/manager.rs`, `alloc_many_user_frames` loop (lines 240, 244):
renamed the loop index `i` → `_idx` (and the ghost invariant `frames@.len() == i`
→ `frames@.len() == _idx`).

Root cause: a prior change rewrote `for _` → `for i` so the index could be named
in the loop invariant `frames@.len() == i`. But that index is used *only* inside the
`#[cfg_attr(verus_keep_ghost, verus_spec(invariant ...))]` block. In the normal
(non-Verus) build the ghost code is stripped, leaving `i` unused, and the kernel is
compiled with `-D warnings`:

```
error: unused variable: `i`
   --> src/kernel/src/mm/phys/manager.rs:244:13
    |
244 |         for i in 0..count {
    |             ^ help: if this is intentional, prefix it with an underscore: `_i`
    = note: `-D unused-variables` implied by `-D warnings`
error: could not compile `kernel` (bin "kernel") due to 1 previous error
make: *** [build/make/kernel.mk:11: all-kernel] Error 101
```

The `_idx` name is exempt from `unused_variables` in the exec build yet is still a
normal, referenceable identifier in the Verus ghost invariant, so both builds pass.
This is the only checklist item that was a genuine, fixable defect; it is now fixed.

**2. The 4 remaining `admit()`s — attempted, exact errors below, NOT fixable in scope.**

I did not leave these as verbal justifications. I removed each `admit()` (and each
crutch lemma *call*) and ran `make verify-kernel` to capture what Verus actually
reports. The errors below are the real reproducers (checklist item 7). No spec/view
was weakened and no new `external_body`/`assume` was added.

### Verification

After the build fix, both gates are green for the module (the 4 admits remain,
which is why status is still CHEATING_DETECTED — see analysis):

`make build`:
```
[OK]    Build complete.
```

`make verify-kernel MODULE=mm::phys`:
```
verification results:: 42 verified, 0 errors (partial verification with `--verify-*`)
  Exit code : 0
  Global: assume=0 external_body=18 admit=24 trusted=0 cfg_gate=15
  cheating: assume=0 external_body=18 admit=24 trusted=0 no_decreases=0 cfg_gate=15
  status: CHEATING_DETECTED
```

`cheating-detail.txt` (manager module):
```
mm/phys/manager.proof.rs:12  lemma_manager_attached:        admit
mm/phys/manager.proof.rs:27  lemma_kernel_alloc_one:        admit
mm/phys/manager.proof.rs:40  lemma_kernel_alloc_contiguous: admit
mm/phys/manager.proof.rs:210 lemma_user_bulk_err_restored:  admit
mm/phys/manager.rs:104       init:            external_body  (TCB-listed)
mm/phys/manager.rs:529       kernel_watermark: external_body  (TCB-listed)
```

### Result: NEEDS_GUIDANCE  (build defect FIXED; 4 admits are model-level unsatisfiable in scope)

The 4 admits share **one** root cause, which I reproduced four ways. The fixed specs
require the *manager's* abstract view `self@` to undergo a transition, but:

* `self@` is defined (in `manager.spec.rs`, a DO-NOT-MODIFY view) as `self.upool@`,
  and `Upool::view` is `uninterp` (`upool.rs:56`).
* The kernel paths mutate the **global** allocator through *free functions*
  `frame::alloc` / `frame::alloc_contiguous` (`frame.rs:769,799`) that take no `self`
  and never touch `self.upool`.
* `phys_view()` (`mod.spec.rs:98`) is a **parameter-free** `uninterp` constant, so it
  cannot encode a pre→post transition, and there is **no axiom** anywhere linking
  `self.upool@` (or `phys_view()`) to a state change (grep confirms the only mentions
  are the admit itself and `vstd::set` axioms).

Discharging any of these soundly requires infrastructure that does not exist and that
I am forbidden to create: a state-threaded tracked ghost token through `frame.rs`
(those functions are not in my target list and changing their specs is out of scope),
or making `phys_view`/`Upool::view` state-parameterized (a DO-NOT-MODIFY view
definition). Per the reviewer's item 7, here are the isolated reproducers.

---

#### (1) `lemma_manager_attached` — ensures `m@ == phys_view().frames`

Removed `admit()`, left an empty body:
```
error: postcondition not satisfied
  --> src/kernel/src/mm/phys/manager.proof.rs:14:9
   |         ^^^^^^^^^^^^^^^^^^^^^^^^ failed this postcondition
verification results:: 41 verified, 1 errors
```
`m@` (= `m.upool@`, uninterp) and `phys_view().frames` (uninterp constant) are two
unrelated uninterpreted terms; no in-scope fact equates them. This is the §8
ghost-token attachment, which lives in the (out-of-scope) `frame` layer — `frame.rs`
itself still carries 8 admits (`alloc`, `alloc_contiguous`, `free`, `share`,
`refcount`, `book`, `is_covered`, `alloc_range`) that this attachment depends on.

#### (2) `lemma_kernel_alloc_one` — ensures `pre.free_frames.contains(addr)`, `post == pre.alloc_one(addr)`, `post.wf()`

Removed `admit()`, empty body. The ensures reference the *free, universally-quantified*
parameters `pre`, `post`, `addr` with only `pre.wf()` as hypothesis — so they cannot
follow:
```
error: postcondition not satisfied
  --> src/kernel/src/mm/phys/manager.proof.rs:31:9   (pre.free_frames.contains(addr))
error: postcondition not satisfied
  --> src/kernel/src/mm/phys/manager.proof.rs:32:9   (post == pre.alloc_one(addr))
verification results:: 41 verified, 1 errors
```

**Why a crutch lemma is even needed (the real defect this hides).** I removed the
lemma *call* from `alloc_kernel_frame` and asserted what Verus actually knows:
```rust
proof! { if result is Ok { assert(self@ == g_old); } }   // g_old == old(self)@
```
The `assert(self@ == g_old)` **passes** — i.e. Verus proves the function does NOT
modify `self` (it only calls the `self`-less free function `frame::alloc`). But the
spec then fails:
```
error: postcondition not satisfied
   --> src/kernel/src/mm/phys/manager.rs:369:17     (final(self)@ == old(self)@.alloc_one(kf@))
verification results:: 41 verified, 1 errors
```
So the spec demands `old@ == old@.alloc_one(kf@)` together with
`old@.free_frames.contains(kf@)` — a contradiction. `lemma_kernel_alloc_one` "works"
only by `admit()`-ing this false equation. It is unsound, not a provable gap.

#### (3) `lemma_kernel_alloc_contiguous` — ensures `post == pre.book_all(kernel_addr_set(frames))`, …

Same family. The contiguous loop already carries the proven invariant `self@ == g_old`
(`manager.rs:470`), confirming `self` is never modified. I removed the lemma call and
asserted it:
```rust
proof! { assert(self@ == g_old); }   // passes
Ok(())
```
The assert passes; the function postcondition then fails because the exec produced no
`self@` transition while the spec demands `self@ == old@.book_all(non-empty set)`:
```
error: postcondition not satisfied
   --> src/kernel/src/mm/phys/manager.rs:425:17
verification results:: 41 verified, 1 errors
```

#### (4) `lemma_user_bulk_err_restored` — ensures `m@ == pre`

Here `self.upool@` *is* mutated (the loop calls `self.upool.alloc()`, whose spec gives
`final@ == old@.alloc_one(uf@)`). On a mid-bulk failure the code does `frames.clear()`,
which drops the already-taken `UserFrame`s (freeing them via `Drop`). I replaced the
lemma call with the fact it asserts:
```rust
Err(error) => {
    frames.clear();
    proof! { assert(self@ == g_old); }   // claims restoration
    return Err(error);
}
```
```
error: assertion failed
   --> src/kernel/src/mm/phys/manager.rs:259:32
verification results:: 41 verified, 1 errors
```
The assert **fails**: after ≥1 successful allocation `self@ == g_old.book_all({taken})`,
and `Vec::clear` is modeled as emptying `frames@` only — Verus does not model the
`Drop` side-effect of freeing frames back into `self.upool@`. Restoring `m@ == pre`
would require a `Upool` method that performs the inverse (`free`/un-`alloc_one`)
transition on `self.upool@`; no such method exists and I cannot add one (`Upool` is
`external_body`, not in my target list).

---

### Summary for the reviewer

* **Fixed:** the `make build` regression (unused `i` → `_idx`). Build and verify both
  now exit 0 for the module.
* **Cannot fix in scope (with reproducers above):** the 4 admits are not weak proofs —
  three of them (`lemma_kernel_alloc_one`, `lemma_kernel_alloc_contiguous`, and the
  `lemma_manager_attached` they rest on) encode a transition that the implementation
  provably does **not** perform on `self` (the `assert(self@ == g_old)` passes), and the
  fourth (`lemma_user_bulk_err_restored`) needs a `Drop`/`free` transition on
  `self.upool@` that Verus does not model and that no in-scope API exposes.

Soundly removing them requires one of: (a) a tracked ghost permission token threaded
through the `frame.rs` free functions (out of scope; those functions are not targets and
still hold 8 admits of their own), or (b) making `phys_view()` / `Upool::view`
state-parameterized (forbidden — DO-NOT-MODIFY view definitions), or (c) changing the
manager `ensures` to not claim a `self@` transition (forbidden — spec weakening). All
three are outside this task's constraints, so I am flagging for guidance rather than
silently admitting or weakening.
