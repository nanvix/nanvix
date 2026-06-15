## Turn 2: Verify fixer's #10 fix + complete the remaining checklist (#1, #6, #7, #9)

### Progress
- Done (PASS/FIXED): #1, #2, #3, #4, #5, #6, #7, #8, #9, #10 — ALL items verified.
- Current: final confirmation of the blocking #10 fix and the items it had blocked (#1/#7/#9).
- Remaining: none.

### Verification (everything re-run by me, not trusted from the fixer's writeup)

**#10 Verification + build — FIXED (blocking failure resolved).**
- Confirmed the directive is gone: `grep "size_of usize"` over `frame.proof.rs` = no match.
- Confirmed the invariant strengthening: `frame.proof.rs:74` now carries
  `&&& self.bitmap@.num_bits <= FrameNumber::spec_max() + 1`, and the conversion bounds in
  `alloc` (`frame.rs:176-177`) and `alloc_contiguous` (`frame.rs:343-344`) are discharged from
  that invariant (`idx <= FrameNumber::spec_max()`), not from word size. Exactly the fix I required in turn 1.
- **Forced NON-CACHED** `make verify-kernel` (touched `frame.proof.rs` first):
  `verification results:: 112 verified, 0 errors` → `Exit code : 0`. **No `error[E0080]`.**
- Warnings: 0. The log contains only `note: automatically chose triggers ...` (informational
  trigger-selection notes, not warnings). `grep -cE "^warning|warning:"` = 0; `grep "error\["` = none.
- `./z build` (real non-ghost codegen on default `TARGET=x86`): `[OK] Build complete`, EXIT=0,
  no E0080, no compiler warnings. (The single "Warning: Sysroot directory ... not found" is an
  environment symlink note, unrelated to the code.)

**#1 No specs weakened — PASS.**
- Baseline = `1c13950bb [verus-ai] proving START: phys-frame`.
- `git diff 1c13950bb HEAD -- frame.spec.rs` → **empty** (spec file unchanged).
- Diffed every wrapper contract (`alloc`, `alloc_contiguous`, `free_count`, `free`, `book`,
  `alloc_range`, `share`, `refcount`, `is_covered`) requires/ensures lines (free_frames,
  allocated_frames, reserved, refcounts, covers, base@, usize::MAX, count>0, spec_page_size)
  baseline-vs-HEAD → **IDENTICAL CONTRACTS**. No guarantee weakened.
- `internal_inv` gained a conjunct (`num_bits <= spec_max()+1`): this is a **strengthening**
  of a `closed spec fn` (adds a guarantee), preserved by every method (`num_bits` never resized)
  and established by the allow-listed TCB boundary `init`/`instance` (`ensures (*r).inv()`).
  This is the exact change I demanded in turn 1, not drift.

**#2 Zero admit — PASS.** `grep -c admit()` over `frame.rs/.spec.rs/.proof.rs` = 0/0/0.

**#3 external_body only if allow-listed — PASS.** 8 in `frame.rs`: `instance`, `init`, `alloc`,
`alloc_contiguous`, `free_count`, `free`, `book`, `alloc_range` — each individually present in
`verus-ai-logs/tcb-allowed.md`. The diff added 6 of these during proving, but every one is the
documented "Cross-module dependency … eliminated when its module is verified" wrapper whose
post-mutation `phys_view().frames` postcondition `instance()` does not pin; allow-listed with
rationale. `share`/`refcount`/`is_covered` are NOT external_body. None in spec/proof files.

**#4 assume/assume_specification — PASS (with standing note).** `assume=0` (no bare `assume(`).
`frame.spec.rs` has 2 intra-crate `assume_specification` (`PageAligned::<T> as
Address::into_raw_value` / `as Deref::deref`), both allow-listed (`tcb-allowed.md` 159-160) and
**unchanged since proving start**. Standing note from turn 1: these are intra-crate, not the
literal "std/external only" class, but accepted per the established bottom-up convention. No regression.

**#5 No cfg-gated exec code — PASS.** Every `#[cfg(not(verus_keep_ghost))]` in `frame.rs` gates
only `error!(...)` logging or `debug_assert_eq!(...)`. The proving diff added only `proof!`/`assert`
blocks (ghost), no new cfg-gated exec branch/expr/match-arm. `#[cfg(verus_keep_ghost)]` gates only
the spec/proof `include!`.

**#6 Cheating audit (frame scope) — counts:** admit=0, external_body=8 (all allow-listed),
assume=0, assume_specification=2 (allow-listed), cfg-gated-exec=0 (logging/`debug_assert` only).

**#7 Claimed Verus limitation has an isolated reproducer — PASS.** The only limitation previously
claimed for this module (the `global size_of usize == 8` directive) was **removed**, not justified.
No outstanding limitation claim remains for phys-frame, so no reproducer is owed.

**#8 Exec rewrites minimal/equivalent — PASS.** The proving diff adds only ghost proof asserts;
the documented exec rewrite (`into_frame_number()` → `addr.into_raw_value()/FRAME_SIZE`, both
`== addr@/spec_page_size()`) is behaviour-preserving and was already in place.

**#9 Cross-module regression — PASS.** `make verify-kernel` <all> = 112 verified, 0 errors (covers
manager/mod/upool/kframe/frame). Full `make verify` (all crates) → bitmap, sys, nanvix-slab,
bump-allocator, arch all `Exit code : 0`; kernel `112 verified, 0 errors`, `Exit code : 0`. EXIT=0.

### Out-of-scope note (not a phys-frame failure)
`make verify-kernel TARGET=x86_64` aborts with `error[E0433]: cannot find Idt in idt`
(`hal/arch/x86_64/cpu/idt.rs:971`, references `idt::Idt`; only `Idte` exists). Confirmed
pre-existing: that file's last change is base commit `eef1b247e [kernel] F: Add x86_64 HAL`,
untouched by the fixer (whose diff is confined to `frame.rs`/`frame.proof.rs`/`bugs.md`). It is an
arch-IDT compile error that aborts before Verus runs and is independent of `mm::phys::frame`; the
frame fix is now target-agnostic (`num_bits <= spec_max()+1` for whatever `spec_max()` the target
defines). Out of scope for phys-frame proving; flagged for the arch owner.

### Result
All 10 checklist items PASS/FIXED with tool evidence. The blocking #10 build failure is resolved
on the default target (`make verify-kernel` non-cached → 112 verified, 0 errors, exit 0, no E0080,
0 warnings; `./z build` exit 0; `make verify` all crates exit 0). No spec weakened; no admit; all
external_body/assume_specification allow-listed; no cfg-gated exec; no outstanding limitation claim.

STOP = RESOLVED.
