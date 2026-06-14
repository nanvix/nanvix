## Turn 1: Full Proving Checklist Walkthrough

### Progress
- Done (PASS): all 10 checklist items verified — see per-item evidence below.
- Current: full sweep complete on turn 1; no FAIL items.
- Remaining: none.

### Scope
- Source : `src/kernel/src/mm/phys/manager.rs`
- Spec   : `src/kernel/src/mm/phys/manager.spec.rs`
- Proof  : `src/kernel/src/mm/phys/manager.proof.rs`
- In-scope target functions (per `verus-ai.toml` / verification-plan): `init`,
  `alloc_user_frame`, `check_user_watermark`, `alloc_many_user_frames`,
  `alloc_many_kernel_frames`, `alloc_kernel_frame`.
- Phase boundary commits: specification END = `9cfbc88fc`, proving START = `ec3624c76`, HEAD = `f3ad5238f`.

### Verification

Commands run:
- `make verify-kernel MODULE=mm::phys` → exit 0, "11 verified, 0 errors".
- `make verify-kernel` (all modules) → exit 0; modules `mm::phys`, `mm::phys::frame`,
  `mm::phys::kframe`, `mm::phys::manager`, `mm::phys::upool` all verified.
  Global cheating: `assume=0 external_body=22 admit=0 trusted=0 no_decreases=0 cfg_gate=9`.
- `./z build -- all` → `[OK] Build complete.`; kernel compiled clean.
- `git diff 9cfbc88fc HEAD -- manager.rs manager.spec.rs` → empty.
- `git diff 9cfbc88fc HEAD -- manager.proof.rs` → only the two `admit()` bodies discharged.
- `grep warning` over build + verus logs → none (excluding unrelated CAP_NET/Sysroot notes).

Per-item results:

1. **No specs weakened (spec drift)** — PASS.
   `git diff 9cfbc88fc HEAD` for `manager.rs` and `manager.spec.rs` is **empty**: every
   `requires`/`ensures` clause and every spec fn (`spec_watermark_ok`,
   `is_contiguous_run`, `kernel_frames_contiguous`, `spec_kernel_watermark`) is byte-for-byte
   identical to the specification-phase output. Proving touched only `manager.proof.rs`
   (proof bodies). No weakening possible — nothing changed.

2. **Zero admit()** — PASS. `admit=0` in cheating scan. Diff shows both
   `lemma_watermark_monotone` and `lemma_contiguous_run_distinct` had their `admit();`
   removed and replaced by real proofs; module verifies with admit=0.

3. **Zero external_body unless TCB-allowed** — PASS. manager.rs has exactly 6 `external_body`
   functions (lines 107 `init`, 198 `alloc_many_user_frames`, 267 `alloc_user_frame`,
   306 `check_user_watermark`, 352 `alloc_kernel_frame`, 409 `alloc_many_kernel_frames`).
   All 6 are explicitly enumerated in `verus-ai-logs/tcb-allowed.md` under
   "Allowed `external_body` — `PhysMemoryManager`". `get_mut` is not `external_body`
   (no verus attr) and is in the skip/exclude list.

4. **Zero assume/assume_specification** — PASS. `assume=0`. The only "assume" token in
   manager.rs is `PHYS_MEMORY_MANAGER.assume_init_mut()` (line 147) — a `MaybeUninit`
   std method, not a Verus `assume`/`assume_specification`; correctly not counted.

5. **No cfg-gated exec code (branches/expressions/match arms)** — PASS. The only `cfg`
   usages in manager.rs are: lines 9 & 11 `#[cfg(verus_keep_ghost)] include!(...)` (ghost
   include of spec/proof), and lines 97 & 291
   `#[cfg_attr(verus_keep_ghost, allow(verus_impl_method_marker))]` (a lint-allow attribute
   required because that lint exists only under Verus). None gate an exec branch,
   expression, or match arm. No cfg-gated exec code introduced by proving (manager.rs diff empty).

6. **Cheating audit (exact counts + locations)** — reported.
   - admit: **0** (whole kernel).
   - assume / assume_specification: **0**.
   - external_body in manager.rs: **6** — lines 107, 198, 267, 306, 352, 409; all TCB-allowed.
   - external_body whole-kernel: 22 (remaining 16 in frame.rs/mod.rs/upool.rs — prior phases, all TCB-allowed).
   - cfg_gate (detector, whole kernel): **9**. manager.rs contributes **2** (the lint-allow
     `cfg_attr` at lines 97 & 291, flagged because the detector captures the following
     `requires` token); these are attribute-level lint allows, **not** exec branches.
   - trusted: 0, no_decreases: 0.

7. **Verus limitation has isolated reproducer** — PASS (N/A for this phase). The 6 manager
   `external_body` are documented **trust boundaries** (mutate `static mut PHYS_MEMORY_MANAGER`,
   call un-specced upstream `frame::*`/`Upool` primitives, use side-effecting
   combinators/`error!`/`warn!`), not construct-level "Verus can't prove X" claims requiring a
   reproducer. The one genuine front-end-limitation claim in the broader module
   (LinkedList iteration) lives in out-of-scope `mod.rs` with analysis in `verus-unsupported.md`.

8. **Exec rewrites minimal & semantically equivalent (`// VERUS REWRITE`)** — PASS (vacuous).
   `grep "VERUS REWRITE"` over manager files → none. All 6 targets are `external_body` trust
   shims left byte-for-byte unchanged through proving (manager.rs diff empty); no exec rewrite
   was performed, so there is nothing to equivalence-check.

9. **Cross-module regression (`make verify-kernel`, all modules)** — PASS. Full-kernel run
   exits 0; all five `mm::phys*` modules verify; admit=0, assume=0, trusted=0, no_decreases=0.
   No regression in previously-verified modules.

10. **Verification + build, 0 errors / 0 warnings** — PASS. `make verify-kernel` exit 0,
    0 errors, 0 verus warnings. `./z build -- all` → `[OK] Build complete.`; kernel crate
    compiled with no warnings (log scan clean).

### Fix Request
None. All 10 checklist items PASS with tool-backed evidence. No code changes required.

### Verdict
RESOLVED.
