# Verification TODOs: hal-phys-address

Module: `kernel::hal::mem::types::address::phys`

No `admit()` / `assume()` / `external_body` proof gaps remain in this module. The module
verifies CLEAN (`make verify-kernel MODULE=hal::mem::types::address::phys`, exit 0,
"No cheating detected").

## Remaining cross-module placeholder (not a gate-counted cheating item)

- **Function:** `<::sys::mm::VirtualAddress as ::sys::mm::Address>::into_raw_value`
  - **Where:** `src/kernel/src/hal/mem/types/address/phys.spec.rs:61`
    (`assume_specification[...] ensures result as int == addr@`).
  - **Blocking pattern:** The concrete `impl Address for VirtualAddress`
    (`src/libs/sys/src/sys/mm/address/virt.rs:167`) is not annotated `#[verus_verify]`, so
    its `into_raw_value` (line 253) does not inherit the real, already-written trait
    contract `Address::into_raw_value` (`src/libs/sys/src/sys/mm/address/mod.rs:63-67`,
    `ensures result as int == self@`). Until that impl is verified in its own module, the
    `PhysicalAddress` proofs need this trusted spec to translate `self.0.into_raw_value()`
    in `into_frame_number`.
  - **Why not fixed here:** Resolving it requires editing the `sys` crate, which is a
    **separate verification target** (`sys::sys::mm::address::virt`) and outside this
    module's scope ("do not touch unlisted functions"). It is superseded automatically when
    that module verifies `impl Address for VirtualAddress` in-body — the same supersession
    path already taken for the `VirtualAddress::new` / `FRAME_SHIFT` /
    `FrameNumber::{from,into}_raw_value` placeholders (see `phys.spec.rs:56-77`).
## AST consistency — documented exec divergences from the `dev` baseline

The **ast-consistency** tool compares the verified module against the `dev` pre-verification
baseline. Two divergences remain; both are required by Verus / by an out-of-scope trait and
are documented in-source with `VERUS REWRITE` comments.

- **Function:** `PhysicalAddress::from_number` (`phys.rs`) — MISMATCH (required rewrite)
  - **Divergence:** `dev` had `let addr = frame.into_raw_value() * mem::FRAME_SIZE;` (one line);
    the verified form splits it into `let addr_raw = frame.into_raw_value();` /
    `proof! { lemma_from_number_no_overflow(frame); }` / `let addr = addr_raw * mem::FRAME_SIZE;`.
  - **Why required (proven, not asserted):** the no-overflow bound `frame@ <= spec_max()` is
    exposed only through `into_raw_value()`'s postcondition (the `FrameNumber` type invariant is
    private to the `arch` crate — `use_type_invariant(frame)` fails cross-crate with *"missing
    type invariant function"*). A single expression leaves no point to invoke the lemma between
    the call and the multiply, so Verus rejects the multiply with *"possible arithmetic
    underflow/overflow"*.
  - **Evidence:** committed minimal reproducer
    `cheating-elimination/repro/from_number.rs` — `verus from_number.rs` →
    *"error: possible arithmetic underflow/overflow ... frame.into_raw_value() * SIZE"* for the
    single-line `bad` form; the split `good` form verifies. `VERUS REWRITE` comment in `phys.rs`.
  - Semantics, time, and space complexity are identical (same value, same two ops).

- **Function:** `PhysicalAddress::clone_address` (`phys.rs`) — EXTRA_IN_VERUS (interface addition)
  - **Divergence:** not present on `dev`; present on the task base `verus-ai-prove`.
  - **Why it cannot be removed here:** `clone_address` is a **required** method of the
    `sys::mm::Address` trait, which gained it during the verus pipeline (verified contract
    `result@ == self@`). The trait lives in the out-of-scope `sys` crate
    (`src/libs/sys/src/sys/mm/address/mod.rs:88`); since `PhysicalAddress: Address`, the impl
    method is mandatory. Deleting it would break the trait impl and require editing the `sys`
    crate (separate verification target).
  - **Evidence:** `git show dev:.../mod.rs | grep -c clone_address` → 0;
    `git show verus-ai-prove:.../mod.rs | grep -c clone_address` → 1 (trait gained it after `dev`).
    `VERUS REWRITE` comment in `phys.rs`. Superseded/normalized when the `sys` address trait
    module is itself reconciled against its baseline.

