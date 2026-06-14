# Verification TODOs: hal-frame-address

## Remaining trusted assumption (TCB-approved, not a proof gap in this module)

- **`<PhysicalAddress as ::sys::mm::Address>::from_raw_value`**
  (`src/kernel/src/hal/mem/types/address/frame.spec.rs:20`, `assume_specification`)
  - **Blocking code pattern:** `FrameAddress::from_raw_value` calls
    `PhysicalAddress::from_raw_value`, whose body in the sibling `phys` module
    (`src/kernel/src/hal/mem/types/address/phys.rs:185`) currently carries **no**
    `#[verus_spec]` contract. Bottom-up verification of the frame module therefore
    needs a contract for that callee to discharge
    `ensures Ok(fa) => fa@ == raw_addr as int`.
  - **Status:** External-bottom placeholder, explicitly listed in
    `verus-ai-logs/tcb-allowed.md` (intra-crate placeholder for bottom-up proving
    of `hal::mem::types::address::frame`). It is **not** counted by the cheating
    gate (`assume_specification` is a module-level declaration, not an `assume(...)`
    call or `admit()`), and the frame module scans CLEAN.
  - **Elimination path:** Removed when the `phys` (`hal::mem::types::address::phys`)
    module is verified — its `Address::from_raw_value` will then gain its own
    `#[verus_spec]`, superseding this placeholder. Out of scope here because the
    target functions are the `FrameAddress` methods only; touching `phys.rs` would
    modify an unlisted function in another module.

## Proof gaps

None. All in-scope `FrameAddress` functions
(`into_raw_value`, `into_frame_number`, `from_raw_value`, `from_frame_number`)
verify with real proofs (`6 verified, 0 errors`). No `admit()` / `assume()` remain.
