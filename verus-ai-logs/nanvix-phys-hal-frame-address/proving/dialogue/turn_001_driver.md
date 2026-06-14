## Turn 1: Full checklist walkthrough — FAIL on `assume_specification`

### Progress
- Done (PASS):
  - No specs weakened (spec drift)
  - Zero remaining `admit()`
  - Zero `external_body` (none in target module)
  - No cfg-gated exec code
  - Cheating audit (counts + locations reported)
  - Verus-limitation isolated reproducer (N/A — none claimed)
  - Exec rewrites minimal / `// VERUS REWRITE` (none performed)
  - Cross-module regression (`make verify-kernel` all modules)
  - Verification + build: 0 errors, 0 warnings
- Current (FAIL): **Zero assume/assume_specification**
- Remaining: none — all other items verified PASS this turn.

### Scope
Target module = `hal::mem::types::address::frame` →
`frame.rs`, `frame.spec.rs`, `frame.proof.rs` only. The sibling `phys` module
(`phys.proof.rs` etc.) and `mm/phys`, `mm/virt` are NOT in scope; their cheating
counts are pre-existing and excluded below.

### Verification (commands run)

1. `make verify-kernel MODULE=hal::mem::types::address::frame` → **exit 0**, module
   verified. Module-scoped cheating: `cfg-gated exec code: 1` (only).
2. `make verify-kernel` (all) → **exit 0**, all modules verified/cached.
   Global cheating: `assume=0 external_body=11 admit=29 cfg_gate=15` — every entry in
   `verus-logs/cheating-detail.txt` is in `mm/phys`, `mm/virt`, or sibling
   `hal/.../phys.proof.rs`; **none in `frame.rs`/`frame.spec.rs`/`frame.proof.rs`**.
3. `./z build -- all-kernel` → **Build complete, 0 errors, 0 warnings.**
4. `grep -nE "admit|external_body|assume|VERUS REWRITE|no_decreases"` on the three
   frame files → only hit is `frame.spec.rs:20` (`assume_specification`) plus a comment
   reference in `frame.rs:40`.
5. Spec-drift: `git diff 316396206^..HEAD` (proving START parent → HEAD) on the three
   files:
   - `frame.rs` — **no change** (contracts identical).
   - `frame.spec.rs` — **no change**.
   - `frame.proof.rs` — only the two `admit()` bodies replaced by real proofs
     (`lemma_mod_multiples_basic`, `lemma_fundamental_div_mod`); **ensures clauses
     unchanged**. No weakening.

#### Per-item results
- **No specs weakened** — PASS. Contracts byte-identical across the proving phase; only
  proof bodies were discharged.
- **Zero `admit()`** — PASS. `frame.proof.rs` now has 0 `admit()`; both lemmas carry
  real proofs. No `admit` anywhere in the target module.
- **Zero `external_body`** — PASS. 0 in the target module. The 11 global ones are all
  in `mm/phys`/`mm/virt`/`kframe` and listed in `tcb-allowed.md`.
- **No cfg-gated exec code** — PASS. The 1 flagged `cfg_gate` is `frame.rs:36`
  `#[cfg(verus_keep_ghost)] verus! { … }`, whose body is **ghost-only**:
  `spec_page_size()` (spec fn), `impl View for FrameAddress` (spec fn `view`),
  `inv()` (spec fn). No exec branch/expression/match arm is gated; all exec methods
  (`new`, `from_raw_value`, `into_raw_value`, `from_frame_number`,
  `into_frame_number`, …) are outside any cfg. Compiled exec == verified exec.
- **Cheating audit (target module)** — admit=**0**, external_body=**0**,
  `assume(...)`=**0**, assume_specification=**1** (`frame.spec.rs:20`),
  cfg-gated exec=**0** (the 1 cfg_gate is ghost-only, `frame.rs:36`).
- **Verus-limitation reproducer** — PASS (N/A). No claimed Verus limitation / no
  `verus-unsupported.md` for this module.
- **Exec rewrites minimal** — PASS. `frame.rs` unchanged during proving; no
  `// VERUS REWRITE` comments; nothing to review.
- **Cross-module regression** — PASS. Full `make verify-kernel` exit 0.
- **Verification + build** — PASS. Module verify exit 0; full verify exit 0; kernel
  build 0 errors / 0 warnings.

- **Zero assume/assume_specification** — **FAIL.**
  `frame.spec.rs:20`:
  ```
  pub assume_specification[ <PhysicalAddress as ::sys::mm::Address>::from_raw_value ](value: usize)
      -> (result: Result<PhysicalAddress, ::sys::error::Error>)
      ensures match result { Ok(pa) => pa@ == value as int, Err(_) => true };
  ```
  `PhysicalAddress` is a **kernel-internal** type
  (`crate::hal::mem::types::address::phys`); its `Address::from_raw_value` body lives in
  `phys.rs:185` and carries **no** `#[verus_spec]` (confirmed). This is therefore an
  **intra-crate** assumed contract, not the std/external-bottom boundary the checklist
  permits. It is also **not recorded** in `verus-ai-logs/tcb-allowed.md` — that file's
  intra-crate `assume_specification` allow-list covers `mm/phys/frame.spec.rs`
  declarations only (`FrameAddress::from_frame_number/into_frame_number`,
  `PhysicalAddress::into_frame_number`, `PageAligned…`); `<PhysicalAddress as
  Address>::from_raw_value` is absent.

### Fix Request (item: Zero assume/assume_specification)

This `assume_specification` is genuinely required for bottom-up verification (the frame
`from_raw_value` ensures `fa@ == raw_addr` depends on it and `phys`'s impl is unspecced),
so it cannot simply be deleted. Resolve it one of two ways:

- **Preferred — supersede it.** Add a real `#[verus_spec]` to
  `<PhysicalAddress as ::sys::mm::Address>::from_raw_value` in
  `src/kernel/src/hal/mem/types/address/phys.rs:185`:
  ```
  #[verus_spec(result =>
      ensures match result { Ok(pa) => pa@ == value as int, Err(_) => true })]
  fn from_raw_value(value: usize) -> Result<Self, Error> { … }
  ```
  then **delete** the `assume_specification` block (`frame.spec.rs:11–28`, keeping
  `verus! { }` if still needed). If `phys` is out of your editable scope, use the
  fallback.

- **Fallback — document the bottom-up placeholder.** Add an entry to
  `verus-ai-logs/tcb-allowed.md` under
  *"`assume_specification` for not-yet-verified callees"* recording
  `src/kernel/src/hal/mem/types/address/frame.spec.rs::<PhysicalAddress as
  ::sys::mm::Address>::from_raw_value`, with the same "trusted only until the HAL
  address layer is verified; superseded by the real `#[verus_spec]` then" rationale used
  for the existing intra-crate entries. The justification must state that `phys.rs:185`
  currently has no `#[verus_spec]` (verified above).

**Do not** respond with a justification only — change `phys.rs` (and remove the block) or
edit `tcb-allowed.md`.

**Verify the fix:**
1. `grep -n "assume_specification" src/kernel/src/hal/mem/types/address/frame.spec.rs`
   — must be empty (Preferred) OR
   `grep -n "from_raw_value" verus-ai-logs/tcb-allowed.md` must show the new entry
   (Fallback).
2. `make verify-kernel MODULE=hal::mem::types::address::frame` → exit 0, module verified.
3. `make verify-kernel` → exit 0 (no cross-module regression).
4. `./z build -- all-kernel` → 0 errors, 0 warnings.
