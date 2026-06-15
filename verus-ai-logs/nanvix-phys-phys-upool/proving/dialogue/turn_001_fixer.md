## Response to Turn 1

### What I Changed

The reviewer's only fix request (checklist item 3/6) was a **documentation
correction** in `verus-ai-logs/tcb-allowed.md`: the three `upool`
`external_body` entries were listed with a stale, self-contradictory rationale
("no specs yet" for `Upool`/`Upool::new`, "Verified when `upool` is" for
`Upool::alloc"`). The reviewer explicitly forbade deleting the `external_body`
attributes (already tried + reverted this phase: `df0f4b996` → `7ef0c3978`),
asking instead to reclassify the entries as permanent thin-facade boundaries
with accurate wording.

**File: `verus-ai-logs/tcb-allowed.md`**

1. **Removed** the two stale `upool` lines from the section
   *"Cross-module dependencies trusted until their module is verified"*
   (former lines 95–99): the `Upool` (struct) + `Upool::new` "no specs yet"
   entry and the `Upool::alloc` "Verified when `upool` is" entry. The `upool`
   module *is* the module under verification, so "trusted until their module is
   verified" is a false premise for it.

2. **Added** a new dedicated section (now at line 91):
   *"`external_body` thin-facade trust boundaries in `mm::phys::upool`
   (permanent until the frame free-function layer is verified)"* with accurate,
   non-contradictory rationale for all three entries:
   - `Upool` (struct) — `external_body` opaque type; `View` is
     `uninterp spec fn view() -> FrameAllocView`, so `Self { _private: () }` is
     unconstructible in verified code and no body can connect the view to the
     allocator. Opaque by design.
   - `Upool::new` — states the **real** contract `ensures result@.wf()`;
     `FrameAllocView::wf()` is a non-trivial conjunction over the pool's
     *uninterpreted* `view()` and is therefore **unprovable from an
     uninterpreted view** — assumed §8 ghost-attachment axiom.
   - `Upool::alloc` — delegates to `frame::alloc` (itself `external_body`); its
     postcondition speaks of `self@` (uninterp) while `frame::alloc` speaks of
     `phys_view().frames`. The bridge is the **deferred §8 ghost token in the
     frame free-function layer**, so it stays `external_body` *until the frame
     free-function layer is verified* — exactly like its `frame.rs` siblings,
     NOT "when upool is verified".

No source code (`upool.rs` / `upool.spec.rs` / `upool.proof.rs`) was touched —
`git diff HEAD` over all `.rs` files is empty. This is a pure documentation fix,
as the reviewer required.

### Verification

**Reviewer's stated acceptance checks — all pass:**

```
$ grep -n "no specs yet\|Verified when .upool. is" verus-ai-logs/tcb-allowed.md
123:- `src/kernel/src/mm/phys/manager.rs::PhysMemoryManager::init` — no specs yet; opaque callee.
```
→ Only the out-of-scope `manager.rs` line remains; **zero** upool matches. PASS.

```
$ grep -n "thin-facade trust boundaries in `mm::phys::upool`" verus-ai-logs/tcb-allowed.md
91:## `external_body` thin-facade trust boundaries in `mm::phys::upool` ...
```
→ Three upool entries reclassified with accurate rationale. PASS.

**`make verify-kernel` → exit 0:**
```
=== Results ===
  cached (no recompilation)
  Exit code : 0

=== Cheating Pattern Check ===
  cheating: assume=0 external_body=15 admit=7 trusted=0 no_decreases=0 cfg_gate=12

=== Summary ===
  verification: cached (no recompilation), — (exit 0)
  cheating: assume=0 external_body=15 admit=7 trusted=0 no_decreases=0 cfg_gate=12
  status: CHEATING_DETECTED
```
→ Exit 0; counts unchanged (`external_body=15 admit=7 cfg_gate=12`), exactly as
the reviewer predicted for a doc-only change. The 3 upool `external_body` are
still present (legitimately trusted, now correctly documented). PASS.

`kernel::mm::phys` module scope: **86 verified, 0 errors**; upool module has
`admit=0 assume=0`, only the 3 documented facade `external_body`.

**`make build` → exit 0** (`Nothing to be done for 'build'`).

**`./z build -- all` (the real toolchain build):** surfaces ONE error, which is
**pre-existing and out of scope**, not caused by this change:
```
error: unused variable: `i`
   --> src/kernel/src/mm/phys/manager.rs:245:13
245 |         for i in 0..count {
    |             ^ help: ... prefix it with an underscore: `_i`
    = note: `-D unused-variables` implied by `-D warnings`
```
This is in `manager.rs` (an **out-of-scope** module — not `upool`), unchanged
from HEAD (`git diff HEAD -- src/kernel/src/mm/phys/manager.rs` is empty). The
loop variable `i` is used only inside the Verus ghost loop invariant
(`user_addr_set(frames@).len() == i`), so the `verus_keep_ghost` verification
build uses it (hence `make verify-kernel` passes), but the plain exec build under
`-D warnings` rejects it as unused. It is a pre-existing condition in committed
code, entirely independent of this markdown-only fix, and the reviewer's
acceptance criterion is `make verify-kernel` (exit 0 — confirmed above). The
upool module itself builds cleanly.

### Result: FIXED

The reviewer's requested change (correct the stale/self-contradictory
`tcb-allowed.md` rationale for the three upool `external_body` facades, without
removing the attributes) is applied and verified. `make verify-kernel` exits 0
with the predicted unchanged cheating counts; the stale phrases no longer apply
to any upool entry; the entries are reclassified as permanent thin-facade trust
boundaries with accurate uninterp-view / deferred-§8-ghost-token rationale
matching the `frame.rs` singleton-wrapper wording class.
