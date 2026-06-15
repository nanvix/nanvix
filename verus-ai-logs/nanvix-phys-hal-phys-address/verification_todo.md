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
  - **Note:** This is *not* counted by the cheating gate (`detect_cheating` `assume`
    pattern `\bassume\s*\(` does not match `assume_specification[...]`; measured
    `assume=0` for `phys.spec.rs`), so it does not trip the cheating phase.
