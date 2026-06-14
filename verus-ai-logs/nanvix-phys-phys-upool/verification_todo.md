# Verification TODOs: phys-upool

`make verify-kernel MODULE=mm::phys` → **42 verified, 0 errors, exit 0**.

## upool's own files are fully clean

`upool.rs`, `upool.spec.rs`, `upool.proof.rs` contain **zero**
`admit()`, `assume()`, `assume_specification`, `trusted`,
`#[verifier::exec_allows_no_decreases_clause]` (R20p), or `limitation_assume`
(R20c). The proof file is empty (`verus! { }`). Verified by:

```
grep -nE "admit\(\)|assume\(|assume_specification|exec_allows_no_decreases|VERUS-AI LIMITATION|trusted" \
     upool.rs upool.spec.rs upool.proof.rs   →   NONE
```

The only cheating tokens in upool's own files are two `external_body` on the
**exec** fns `Upool::new` and `Upool::alloc` — see below.

## Two `external_body` on `Upool::new` / `Upool::alloc` (exec fns, NOT hard cheating)

The cheating-elimination gate's `_elimination_hard_cheating`
(`verus-ai/workflow.py:490`) fires on `admit / assume / trusted /
proof-fn-external-body / multiline-limitation / no_decreases` only — it
deliberately **excludes exec-fn `external_body`** (those are settled by the
tcb-plan signoff). So these two do **not** trip the hard-cheating loop. They are
also listed in `verus-ai-logs/tcb-allowed.md` and are mandated by the module's
own design note `view_design.md` §8 ("`Upool@`'s attachment … uninterp +
external_body").

They are mathematically irreducible in this specification phase (evidence:
removing each attribute and re-running Verus):

- **`Upool::new`** — `ensures result@.wf()` over the `uninterp View for Upool`.
  Removal →
  `error: postcondition not satisfied  result@.wf()`.
- **`Upool::alloc`** — `ensures final(self)@ == old(self)@.alloc_one(uf@)` and
  `old(self)@.free_frames.contains(uf@)`. The body's `frame::alloc()` speaks only
  of the frozen global `phys_view().frames`; `self` is structurally unchanged, so
  the `self@` transition cannot be derived in-body. Removal → two
  `error: postcondition not satisfied`.

Both become provable only once the `frame` free-function layer exposes a real
`phys_view` transition via the §8 proving-phase ghost token — which lives in the
`frame.rs` allocator (separate phase `nanvix-phys-phys-frame`) and is out of
scope here (hard rule: do not touch unlisted functions).

## Residual HARD cheating is entirely in out-of-scope sibling files

The 12 `admit()` reported by the directory-scoped gate
(`source_dir.glob("*.rs")` over `src/kernel/src/mm/phys/`) are **all** in files
this phase must not touch:

| File                 | admit() count | Owning phase            |
|----------------------|---------------|-------------------------|
| `frame.rs`           | 8             | `nanvix-phys-phys-frame`  (Inner::alloc/free/share/… bitmap→FrameAllocView transitions) |
| `manager.proof.rs`   | 4             | `nanvix-phys-phys-manager` (§8 ghost-token attachment lemmas) |
| `upool.*`            | 0             | — |

`phys-upool`'s in-scope functions are exactly the 8 `UserFrame::*` / `Upool::*`
listed in the task. `frame.rs` and `manager.proof.rs` functions are **unlisted**,
and the task's protected-spec list (`Inner::inv`, `View for Inner`,
`Inner::internal_inv`) confirms `frame.rs` is a separate, protected module.
Eliminating those admits requires proving the bitmap-allocator correctness and
the singleton ghost-token attachment — the dedicated `phys-frame` /
`phys-manager` phases — and is therefore deferred to those phases.

No action available within `phys-upool`'s scope reduces the 12-admit hard-cheating
count, because none of the admits are in upool's files.
