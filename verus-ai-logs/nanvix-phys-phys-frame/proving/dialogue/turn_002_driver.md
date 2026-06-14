## Turn 2: Verify fixer turn 1, re-walk checklist

### Progress
- FIXED — **Verus now runs** (parse errors gone). The three bare `proof {` → `proof! {`
  edits landed; `make verify-kernel` reaches the SMT stage instead of failing to compile.
- PASS — external_body all TCB-listed (10 in frame.rs, all in `tcb-allowed.md`).
- PASS — No cfg-gated exec (logging-only gates; unchanged).
- PASS — Cheating audit reported (counts below).
- PASS — Verus-limitation reproducers (documented cross-module token deferrals, not Verus bugs).
- **Current FAILs (still open):**
  1. **FAIL — Zero admit()**: 3 admits remain — `alloc`(137), `alloc_contiguous`(214), `alloc_range`(613).
  2. **FAIL — Verification 0 errors**: 4 verification errors in `free`, `share`, `refcount`, `book`.
  3. **FAIL — Zero assume_specification**: 2 intra-crate `assume_specification` still in `frame.spec.rs:31,38`.
- BLOCKED (until above fixed): "No specs weakened" spec-drift, exec-rewrite equivalence re-check,
  cross-module regression (kernel still fails: 71 verified / 4 errors).

---

### Verification (this turn — all run by me, not trusting fixer claims)

`make verify-kernel`:
```
verification results:: 71 verified, 4 errors
status: VERIFY_FAILED  (exit 101)
cheating: assume=0 external_body=19 admit=19 trusted=0 no_decreases=0 cfg_gate=15
```

**Parse-fix confirmed:** `grep "proof {" frame.rs` (excluding `proof! {`) → empty. FIX 1 landed.

**The 4 verification errors** (line → owning function, confirmed via `fn` boundary map):
| Err line | Function (fn @) | Kind |
|---|---|---|
| frame.rs:270 | `Inner::free` (fn @298) | postcondition not satisfied (`Ok(())` arm) |
| frame.rs:359 | `Inner::share` (fn @380) | postcondition not satisfied (`Ok(())` arm) |
| frame.rs:452 | `Inner::refcount` (fn @444) | **precondition** not satisfied — `lemma_alloc_contains(self, addr)` requires `addr % spec_page_size() == 0`, but `refcount`'s spec only requires `self.inv()` (no `frame.inv()`), so alignment is not in scope |
| frame.rs:497 | `Inner::book` (fn @511) | postcondition not satisfied (`Ok(())` arm) |

**The 3 admits** (from `cheating-detail.txt`, still present):
- `mm/phys/frame.rs:136 alloc: admit` (`proof! { admit(); }` @137)
- `mm/phys/frame.rs:213 alloc_contiguous: admit` (@214)
- `mm/phys/frame.rs:609 alloc_range: admit` (@613)

The admits are why `alloc`/`alloc_contiguous`/`alloc_range` do **not** appear in the 4-error list —
`admit()` masks their unproven postconditions. They are in-scope proof targets with full strong
specs and are NOT proven.

**The 2 assume_specification** (`grep assume_specification frame.spec.rs`):
- `frame.spec.rs:31` — `<crate::hal::mem::PageAligned<T> as ::sys::mm::Address>::into_raw_value`
- `frame.spec.rs:38` — `<crate::hal::mem::PageAligned<T> as ::core::ops::Deref>::deref`

Both intra-crate (kernel `hal::mem`), not std/external. Still present. The fixer's own turn-1 notes
observed that the `Address` trait already declares `into_raw_value` with
`#[verus_spec(ensures result as int == self@)]` (sys/mm/address/mod.rs) — i.e. the `into_raw_value`
placeholder is **redundant** and removable. That investigation was not acted on.

**Net assessment of fixer turn 1:** only FIX 1 (unblock parse) was actually completed. FIX 2 and
FIX 3 were investigated and delegated to a background agent, but the committed tree still contains
all 3 admits, 4 fresh proof errors, and both `assume_specification`s. Investigation is not a fix.

---

### Fix Request (this turn)

Address in this order. Do NOT weaken any spec listed "do not modify" in `verus-ai.toml`
(`Inner::alloc`, `alloc_contiguous`, `alloc_range`, `book`, `free`, `is_covered`, `refcount`,
`share`). Do NOT add `admit`/`external_body`/`assume` to make these pass.

#### FIX A — discharge the 4 verification errors with real proofs
Complete the in-body proofs so each method verifies against its existing spec:
1. `Inner::free` (fn @298) — postcondition at frame.rs:270. Prove the
   allocated→free / refcount-removal `FrameAllocView` transition.
2. `Inner::share` (fn @380) — postcondition at frame.rs:359. Prove the refcount `+1` /
   view-preservation transition.
3. `Inner::book` (fn @511) — postcondition at frame.rs:497. Prove the free→allocated transition.
4. `Inner::refcount` (fn @444) — **precondition** failure at frame.rs:452. `refcount`'s spec only
   guarantees `self.inv()`, not `frame.inv()`, so `frame@` may be unaligned and the
   `lemma_alloc_contains` precondition (`addr % spec_page_size() == 0`) is not available. An
   unaligned address is never a tracked frame, so the `Ok` arm's
   `allocated_frames.contains(frame@)` is genuinely unreachable for unaligned input. Fix by making
   the body **reject** non-page-aligned addresses on the early-return path (a sound missing-input
   validation), or otherwise establish alignment in-scope — do NOT add `frame.inv()` to the
   `requires` (that would change/strengthen a "do not modify" spec and could break callers).

Verify: `make verify-kernel` → these 4 errors gone.

#### FIX B — remove the 3 admits (Zero admit)
Replace `proof! { admit(); }` in `alloc`(137), `alloc_contiguous`(214), `alloc_range`(613) with
real proofs that discharge their existing strong specs.
Verify: `grep -n "admit" src/kernel/src/mm/phys/frame.rs` → empty, AND
`verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt` lists **no** `mm/phys/frame.rs:* admit`.

#### FIX C — remove the 2 intra-crate assume_specification (Zero assume_specification)
`frame.spec.rs:31` (`PageAligned::into_raw_value`) is redundant with the inherited
`Address::into_raw_value` trait spec — remove it and confirm the bodies still verify.
For `frame.spec.rs:38` (`PageAligned::deref`): either remove it by relying on a real verified spec,
or add the real `#[verus_spec]` at the `PageAligned`/address layer. A prose justification that it is
"trusted until the address layer is verified" is NOT acceptable here — the checklist permits
`assume_specification` only for std/external crates, and `PageAligned` is workspace-internal.
Verify: `grep -n assume_specification src/kernel/src/mm/phys/frame.spec.rs` → empty.

#### After A–C
- Re-run `make verify-kernel`: must report **0 errors, 0 warnings**, kernel-wide, and the cheating
  check must show `admit=0` and `assume=0` with no frame-module `assume_specification`.
- Then I will re-check spec-drift (no weakening), exec-rewrite equivalence
  (`// VERUS BUG FIX` at frame.rs:445-447 / 551-553), and cross-module regression.

---

### Verdict: **NOT RESOLVED.** 3 admits + 4 verification errors + 2 intra-crate
`assume_specification` remain. No STOP file created. Fixer: do the proofs (FIX A, FIX B) and remove
the assumes (FIX C); report `make verify-kernel` output after each.
