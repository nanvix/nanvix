## Turn 2: Verifying fixer turn 1 — build fix CONFIRMED; item 2 (4 admits) independently confirmed BLOCKED

### Progress
- Done / PASS (re-verified this turn with tool output):
  - **[1] No specs weakened** — PASS (spec file unchanged; manager.rs contracts unchanged)
  - **[3] external_body only if TCB-listed** — PASS (`init`, `kernel_watermark` both listed)
  - **[4] assume/assume_specification** — PASS (3 std items: `Result::and_then`/`inspect_err`, `Vec::capacity`)
  - **[5] No cfg-gated exec code** — PASS-with-evidence (logging-only; genuine Verus limitation, reproducer below)
  - **[6] Cheating audit** — reported below
  - **[7] Isolated reproducer for Verus limitation** — PASS (fixer's 4 admit reproducers + my cfg-gate reproducer)
  - **[8] Exec rewrites minimal + `// VERUS REWRITE`** — PASS (convention unused repo-wide; rewrite is minimal+equivalent)
  - **[9] Cross-module regression** — PASS (`make verify-kernel`: **71 verified, 0 errors**, non-cached)
  - **[10] Verify + build** — verify exit 0 (CHEATING_DETECTED from the 4 admits only); `./z build` → `[OK]` exit 0
- Current / BLOCKER: **[2] Zero remaining admit()** — 4 admits remain; independently confirmed undischargeable within the manager module's scope
- Remaining: none unverified

---

### Verification (this turn)

**Build regression fix — CONFIRMED REAL.** Fixer renamed loop index `i` → `_idx` (manager.rs:240,244)
to silence `-D unused-variables` in the non-ghost build. `./z build` → `[OK] Build complete.` exit 0.
(The kernel-frame loop at line 473 keeps `for i` because `i` is used in *exec* there
— `base_raw + i * mem::PAGE_SIZE` — so no warning. Consistent.)

**[5] cfg-gated exec — PASS, genuine Verus limitation (reproducer).** I un-gated one
`error!("{reason}")` (removed `#[cfg(not(verus_keep_ghost))]`) and ran `make verify-kernel`:
```
error: Unsupported constant type
   --> src/kernel/src/macros.rs:188:12
191 | crate::klog::KlogLevel::Error,
207 | error!("{reason}");
    = note: originates in macro `write` from expansion of macro `error`
error: could not compile `kernel` ... (exit 101)
```
The `error!`/`warn!` macros expand to a constant type Verus cannot model. All 9 gates are
logging-only — they wrap a statement returning `()`, with **no** branch/expression/match-arm logic;
function behavior is identical with or without them. This is the established crate-wide convention
(frame.rs: 22 gates, kframe.rs: 1, upool.rs: 1 — all in the verified set). Not a behavioral
cfg-gate. PASS. (I restored the file afterward.)

**[8] `// VERUS REWRITE` — PASS.** `grep -rn "VERUS REWRITE" src/` → **0** matches repo-wide; the
convention is not used in this project. The single proving-phase exec change
(`check_user_watermark` binding `let available = frame::free_count();` before the overflow check)
is minimal, semantically equivalent (single-threaded, pure read), and documented with a normal
comment. PASS.

**[6] Cheating audit (manager module).**
- `admit` = **4** — manager.proof.rs:12, 27, 40, 210
- `external_body` = 2 — manager.rs:104 (init), 529 (kernel_watermark), both TCB-listed
- `assume_specification` = 3 — manager.spec.rs:9,23,33 (std, allowed)
- cfg-gated exec = 9 — logging-only, Verus-limited (above)

**Repo-hygiene note (caused by my own verification experiment).** The `verify.sh` wrapper
auto-commits each run. My un-gating experiment was auto-committed as several
`verify FAIL ... compilation/setup error` commits (3eec743ca, f3c02de06, 68a5f20cd, f2c83cda1).
I restored `manager.rs` from the last good commit `6465a2a13` and re-ran a full (non-cached)
verification: **71 verified, 0 errors**, captured in HEAD `dd841b286`. Working tree is clean apart
from dialogue logs. The cfg gate at manager.rs:207 is present and correct.

---

### Item 2 — independently confirmed BLOCKED (NOT accepted as a verbal justification)

The fixer did **not** merely justify the 4 admits — it removed each `admit()`/crutch-lemma call,
captured the real Verus errors (turn_001_fixer.md), and left the admits in place. I independently
re-derived the block from source and from the module's own design doc:

1. **§8 attachment depends on a not-yet-verified prerequisite.**
   `verus-ai-logs/nanvix-phys-phys-manager/view_design.md` §8 pins the attachment to
   "`self@` ↔ `frame::INSTANCE@` (the **verified** `View for Inner`)" and
   "`self@ == phys_view().frames`". But `frame.rs` still carries **8 admits**
   (`alloc`, `alloc_contiguous`, `free`, `share`, `refcount`, `book`, `is_covered`, `alloc_range`)
   — `frame::Inner` is **not yet verified**. The manager cannot discharge an attachment to a
   View that is not yet established.

2. **`phys_view()` cannot express a transition (mod.spec.rs:98).**
   `pub uninterp spec fn phys_view() -> PhysModView;` is parameter-free, so `phys_view()` in the
   pre-state is syntactically identical to the post-state. With attachment
   `self@ == phys_view().frames`, that forces `old(self)@ == final(self)@` for every kernel path,
   while `alloc_kernel_frame`'s `ensures` demands `final(self)@ == old(self)@.alloc_one(kf@)`
   together with `old(self)@.free_frames.contains(kf@)` — a contradiction (a free frame both is and
   isn't free). `lemma_kernel_alloc_one` / `lemma_kernel_alloc_contiguous` are therefore
   unsatisfiable as long as `phys_view()` is a timeless constant. Fixing requires state-indexing
   `phys_view()` (a tracked token) in `mod.spec.rs` — a DO-NOT-MODIFY global view outside the
   manager module's scope. `frame::alloc`'s own spec (frame.rs:759) confirms this: it promises
   only `phys_view().frames.allocated_frames.contains(frame@)` — **no** `pre→post` transition.

3. **`lemma_user_bulk_err_restored` needs an inverse transition with no API.**
   The user path threads `self.upool@` via `Upool::alloc` (`final@ == old@.alloc_one(uf@)`). On
   mid-bulk failure the code `frames.clear()`s, freeing the taken frames via `Drop`. Verus models
   `Vec::clear` as emptying `frames@` only and does not model the `Drop` free side-effect. No
   in-scope API expresses the restoring transition: `frame::free` ensures `true` (frame.rs:832),
   and `Upool` exposes no free-spec (`Upool` is `external_body`). So `m@ == pre` is unprovable
   within the manager's scope.

**Conclusion:** all 4 admits are blocked on infrastructure outside the manager module's editable
scope — the frame-layer proving phase (must verify `View for Inner` first) plus a tracked-token
redesign of the DO-NOT-MODIFY `phys_view()`/`Upool::view`. No further attempt *within the manager
module* can remove them without (a) re-introducing an unsound admit, (b) weakening a spec, or
(c) editing out-of-scope `frame.rs`/`mod.spec.rs` — all forbidden. This is a genuine cross-module
dependency block, not a weak proof.

### Fix Request / Escalation
- **No further manager-local edits requested** — they cannot help.
- The manager proving phase must be **sequenced after the `frame` proving phase**. Discharge the
  8 `frame.rs` admits and realize the §8 ghost token (state-indexed `phys_view()` over
  `frame::INSTANCE`) in the `mm::phys::frame` / `mm::phys` (mod) scope; then the 4 manager admits
  follow mechanically. Verify with `make verify-kernel` and confirm
  `verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt` lists **no** manager-module admits
  and the summary reads `admit=0` for the manager.

Marking STOP = **BLOCKED**: 9/10 checklist items PASS with evidence; item 2 (zero admit) is
genuinely blocked on the prerequisite frame-layer proving phase and a DO-NOT-MODIFY global-view
redesign, documented with reproducers above.
