## Turn 1: Full checklist walk-through (Proving phase, phys-frame)

### Progress
- Done (verified this turn):
  - PASS — Zero external_body unless TCB-listed
  - PASS — No cfg-gated exec code (logging-only gates)
  - PASS — Cheating audit reported
  - PASS — Verus-limitation reproducer requirement (frame deferrals are documented cross-module token design, not Verus limitations)
- Current FAILs (must be fixed, in priority order):
  1. **FAIL — Verification: build does not compile** (14 parse errors; Verus never ran) → blocks items below
  2. **FAIL — Zero remaining admit()** (3 admits in core proof targets)
  3. **FAIL — Zero assume/assume_specification** (2 intra-crate `assume_specification`)
  4. **BLOCKED — No specs weakened** (cannot run spec-drift until build compiles)
  5. **BLOCKED — Exec rewrites minimal/equivalent** (`// VERUS BUG FIX` rewrites — re-verify once build runs)
  6. **FAIL — Cross-module regression** (`make verify-kernel` fails: compilation/setup error)
- Remaining: re-check items 4/5/6 after build + admits are fixed.

---

### Verification (commands run + findings)

**`make verify-kernel`** → result:
```
=== Results ===
  0 verified
  compilation/setup error (verus did not run)
  Exit code : 101
error: could not compile `kernel` (bin "kernel") due to 14 previous errors
cheating: assume=0 external_body=19 admit=19 trusted=0 no_decreases=0 cfg_gate=15
```
Verus **did not run** — the kernel crate fails to *parse*. Every proving guarantee is therefore
unverified.

**Root cause of the 14 parse errors** — three in-body proof blocks use the bare keyword
`proof { ... }` instead of the attribute-style macro `proof! { ... }`. In this codebase exec
bodies are plain Rust with `#[verus_verify]`/`#[verus_spec]` attributes (not a `verus!{}` wrapper),
so `proof { ... }` is parsed as a struct literal and `frame@` is rejected. Confirmed: every other
in-body proof in this file and in `manager.rs` uses `proof! { ... }` (e.g. frame.rs:137,214,613;
manager.rs:201,220,228,...).

Offending sites (bare `proof {`):
- `src/kernel/src/mm/phys/frame.rs:448` (in `Inner::refcount`)
- `src/kernel/src/mm/phys/frame.rs:470` (in `Inner::refcount`)
- `src/kernel/src/mm/phys/frame.rs:555` (in `Inner::is_covered`)

Error spans 449–454, 471–473, 556–563 all trace back to these three blocks.

**Cheating audit (frame module only):**
- `admit`: **3** — all `proof! { admit(); }`:
  - `frame.rs:137` — `Inner::alloc`
  - `frame.rs:214` — `Inner::alloc_contiguous`
  - `frame.rs:613` — `Inner::alloc_range`
  These are **in-scope proof targets** carrying full strong specs (e.g. `Inner::alloc` ensures the
  exact `FrameAllocView` transition, frame.rs:115–135). `admit()` discharges nothing — these
  functions are NOT proven.
- `external_body`: **10** in frame.rs — all present in `verus-ai-logs/tcb-allowed.md`:
  `instance`(703), `init`(741), `alloc`(797), `alloc_contiguous`(827), `free_count`(848),
  `free`(866), `book`(908), `alloc_range`(929), `share`(949), `refcount`(970). → PASS.
- `assume_specification`: **2** in `frame.spec.rs:31` and `frame.spec.rs:38`
  (`crate::hal::mem::PageAligned<T>::into_raw_value` and `...::deref`). Both are **intra-crate**
  (kernel `hal::mem`), NOT std/external. → FAIL (see item below).
- cfg-gated exec: the `#[cfg(not(verus_keep_ghost))]` gates in frame.rs all guard
  `error!`/`debug_assert_eq!` **logging only** (e.g. 141–142, 147–148, 154–155, 164–165); control
  flow (`return Err(...)`) is never gated. The `#[cfg_attr(verus_keep_ghost, verus_spec(invariant
  false))]` at 224/625/653 are loop-invariant annotations, not exec branches. → PASS.

---

### Fix Request (address in this order)

#### FIX 1 (prerequisite — unbreak the build)
In `src/kernel/src/mm/phys/frame.rs`, change the three bare `proof {` keyword blocks to the
`proof! { ... }` macro form (matching frame.rs:137/214/613 and all of manager.rs):
- line 448: `proof {` → `proof! {`
- line 470: `proof {` → `proof! {`
- line 555: `proof {` → `proof! {`

Verify: `make verify-kernel` must now actually *run* Verus (no "compilation/setup error", no parse
errors). Do not proceed until parsing succeeds.

#### FIX 2 (the substantive proving gap — Zero admit)
Remove all three `proof! { admit(); }` and replace with real proofs so the functions verify
against their existing specs (do NOT weaken the specs):
- `frame.rs:137` `Inner::alloc`
- `frame.rs:214` `Inner::alloc_contiguous`
- `frame.rs:613` `Inner::alloc_range`

Verify: `make verify-kernel` reports these functions **verified** and the cheating check shows
`admit=0` for the frame module (`verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt` must
no longer list `mm/phys/frame.rs:* admit`). Justification is not acceptable — these are in-scope
proof targets; produce proofs or tool output showing them verified.

#### FIX 3 (Zero assume/assume_specification — intra-crate)
`frame.spec.rs:31` and `:38` are `assume_specification` for `crate::hal::mem::PageAligned<T>`
(`into_raw_value`, `deref`) — workspace-internal, not std/external, so they violate the checklist's
"only external-bottom trust boundaries for std/external crates" rule. The TCB doc itself notes the
sibling `TruncatedMemoryRegion::start/size` placeholders were already removed once
`hal::mem::types::region` gained real `#[verus_spec]` contracts.
Required: either (a) remove these two `assume_specification`s by relying on the real verified
`PageAligned` specs if they now exist, or (b) show concrete tool output proving no real spec is
available and that `PageAligned` is a genuine external-bottom boundary. A prose justification is
not a fix.
Verify: `grep -n assume_specification src/kernel/src/mm/phys/frame.spec.rs` returns nothing, OR
provide the evidence in (b).

#### After FIX 1–3, re-run for the BLOCKED items
- Spec-drift / "no specs weakened": re-confirm the existing top-level specs in `verus-ai.toml`
  "do not modify" list (`Inner::alloc`, `Inner::alloc_contiguous`, `Inner::alloc_range`,
  `Inner::book`, `Inner::free`, `Inner::is_covered`, `Inner::refcount`, `Inner::share`) are
  byte-for-byte unchanged and that the discharged proofs did not relax any `ensures`.
- `// VERUS BUG FIX` exec rewrites (frame.rs:445–447, 551–553 replace
  `into_frame_number()` with `into_raw_value()/FRAME_SIZE`): confirm semantic equivalence holds and
  that they verify under the new proofs.
- Cross-module regression: `make verify-kernel` (whole kernel) must report 0 errors, 0 warnings.

---

### Verdict: **NOT RESOLVED.** Build is broken and Verus never ran. Do not create STOP.
Fixer: start with FIX 1, then FIX 2, then FIX 3, and report `make verify-kernel` output after each.
