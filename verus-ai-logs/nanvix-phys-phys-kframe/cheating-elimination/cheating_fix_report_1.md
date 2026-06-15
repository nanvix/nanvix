# Cheating-Fix Pass: response to `cheating_report_1.md`

## TL;DR

- **The `mm::phys::kframe` module (this phase's scope) has ZERO cheating** and is fully
  verified — re-confirmed this pass: `make verify-kernel MODULE=mm::phys::kframe` →
  `3 verified, 0 errors`, `✅ No cheating detected`, `status: CLEAN`.
- **Every item in `cheating_report_1.md` lives in a *different, unlisted* module** (phys-frame,
  phys-manager, phys-mod, phys-upool) or the separate `mm::virt` subsystem — none are in
  `kframe.rs`/`kframe.spec.rs`/`kframe.proof.rs`.
- The 7 global `admit()`s are **genuine architectural blockers** that cannot be soundly
  eliminated without (a) modifying unlisted functions the task forbids touching, **and**
  (b) introducing ghost-token state threaded through exec signatures/structs, which the
  **ast-consistency / source-integrity** skill forbids. Fresh reproduction evidence below.

## Where the reported cheating actually is

| Report line | File / module | Phase that owns it | In kframe scope? |
|---|---|---|---|
| external_body ×7 | `frame.rs` (`instance`, `init`, `alloc`, …) | nanvix-phys-phys-**frame** | ❌ |
| external_body ×2 | `manager.rs` (`init`, `kernel_watermark`) | nanvix-phys-phys-**manager** | ❌ |
| external_body ×2 | `mod.rs` (`init`, `book_mmio_regions`) | nanvix-phys-phys-**mod** | ❌ |
| external_body ×3 | `upool.rs` (`Upool`, `alloc`, …) | nanvix-phys-phys-**upool** | ❌ |
| external_body_type_spec ×1 | `mod.spec.rs:66` | nanvix-phys-phys-**mod** | ❌ |
| admit ×4 | `manager.proof.rs:16,35,55,159` | nanvix-phys-phys-**manager** | ❌ |
| admit ×3 (global total 7) | `mm/virt/identity_map.rs:534,632,719` | nanvix-phys-**virt-identity-map** | ❌ |

`grep -i kframe verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt` → **empty**.
The cheating gate reports *crate-global* counts (`make verify-kernel` compiles the whole
kernel); none of the counted items are attributable to the kframe module.

## The 7 `admit()`s — why they are genuine blockers (with evidence)

### Root cause (shared by the 4 `manager.proof.rs` admits)
`phys_view()` is declared `pub uninterp spec fn phys_view() -> PhysModView`
(`mod.spec.rs:98`) — a **stateless constant**: every call returns the same fixed ghost value.
Yet it is used to model the **mutable, shared global frame partition** that
`frame::alloc()`/`free()` mutate. The manager's abstract view is
`PhysMemoryManager::view(&self) == self.upool@` (`manager.spec.rs:91`), and `upool@` is itself
`uninterp` (`upool.rs:59`). The 4 lemmas bridge these uninterpreted, stateless views to the
per-call global mutation. That bridge is only coherent if a `tracked` ghost token over the
`frame::INSTANCE` / `PhysMemoryManager` / `Upool` singletons is threaded through the exec
signatures — an **exec-signature/struct change** that the source-integrity (`ast-consistency`)
skill forbids, and these are **unlisted functions** the task forbids touching.

### Fresh reproduction (this pass) — `lemma_kernel_alloc_one`
I removed the lemma call from `alloc_kernel_frame` (manager.rs:398) and re-verified:

```
error: postcondition not satisfied
   --> src/kernel/src/mm/phys/manager.rs:376:17
    |
376 |                   Ok(kf) => {
    |                   ^^^^^^ failed this postcondition
...
383 |       pub fn alloc_kernel_frame(&mut self) -> Result<KernelFrame, Error> {
    | ...
402 | |     }
    | |_____- at the end of the function body
verification results:: 17 verified, 1 errors
```

This **proves** the lemma asserts `final(self)@ == old(self)@.alloc_one(kf@)` while the
implementation **never mutates `self`** (the frame is taken from the *global* `frame::alloc()`,
not from the user pool that backs `self@`). I.e. the lemma stands in for
`old(self)@ == old(self)@.alloc_one(addr)` — **false for the implementation**. (File restored
afterwards; manager re-verifies `18 verified, 0 errors`.)

The same structural defect holds for `lemma_kernel_alloc_contiguous` (bulk kernel path) and
`lemma_user_bulk_err_restored` (`m@ == pre` after `clear()`, which directly contradicts the
already-*proven* loop invariant `self@ == g_old.book_all(user_addr_set(frames@))`). And
`lemma_manager_attached` (`m@ == phys_view().frames`) equates two `uninterp` constants with no
axiom — and is mutually inconsistent with the `alloc_one` transition specs (a constant cannot
equal both `c` and `c.alloc_one(..)`).

### Why no sound elimination is reachable from here
- **admit/assume** — forbidden by the cheating gate (the very thing being removed).
- **`#[verifier::external_body]` proof fn / `trusted`** — explicitly disallowed by the request
  ("external_body on proof fns must be removed").
- **Relocate to an `external_body` *exec* wrapper** — would mean trusting a contract that is
  *provably false / mutually contradictory* (shown above), letting the module prove `false`
  (unsound) — worse than an isolated admit. Rejected on soundness grounds (spec-design skill).
- **Correct the manager's external-top specs** (e.g. kernel alloc → `final(self)@ == old(self)@`)
  — these are **unlisted** functions (task: "Do not touch unlisted functions"), *and* no
  locally-correct spec exists: `== old` makes the watermark free-count accounting claim a frame
  was *not* consumed (also false); only a stateful `phys_view()` is coherent.
- **Ghost-token attachment** (the real fix) — requires `tracked` state in exec
  signatures/structs → forbidden by `ast-consistency`.

The `mm/virt/identity_map.rs` admits (×3) belong to the separate `mm::virt` subsystem
(`identity_map_view()` precondition machinery), out of scope for any `mm::phys` phase.

## The 15 `external_body` — ledgered global-singleton/external-constant boundaries
All 15 are on **exec** functions (or one type spec), every one materializing a `static mut`
singleton (`frame::INSTANCE`, `PhysMemoryManager`/`Upool` singletons), a BSS-backed store, or an
external build-time constant — raw-memory/atomics operations Verus cannot model without a
`PointsTo` permission for the `static mut`. They are recorded in
`verus-ai-logs/tcb-allowed.md` and eliminated by each module's own phase when its singleton is
lifted onto verified `PointsTo`/token state. The request's removal mandate is scoped to
`external_body` **on proof fns** — there are **none**; all 15 are the accepted ledgered exec
boundary.

## Verification (this pass)

```
make verify-kernel MODULE=mm::phys::kframe   → 3 verified, 0 errors, status: CLEAN
make verify-kernel MODULE=mm::phys::manager  → 18 verified, 0 errors (restored)
make verify-kernel (full)                    → Exit 0; status: CHEATING_DETECTED
                                               (global admit=7 external_body=15, all out-of-scope)
```

## Result: BLOCKER (for the crate-global gate); phys-kframe scope is CLEAN

The phys-kframe module carries zero cheating and is fully verified. The remaining 7 admits +
15 external_body are crate-global items in unlisted sibling/`mm::virt` modules whose sound
elimination requires a **ghost-token re-architecture of `phys_view()`** (a stateful model of the
global frame partition) — a human-reviewed spec-design decision that is forbidden by the
source-integrity/ast-consistency rules (exec-signature changes) and out of scope for this phase.
No sound, skill-compliant local fix exists; an unsound paper-over (trusting a provably-false
contract) was deliberately **not** applied.
