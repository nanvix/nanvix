# Verus Unsupported Constructs

Genuine Verus front-end limitations encountered during verification. These are
**not** proof gaps or code bugs: the verifier cannot parse/lower the construct,
so no proof strategy can address them. Per the `verus-constraints` skill they are
recorded here (exec code is left untouched; no `external_body`/`assume_specification`
workaround is added on the affected module's own functions).

_No outstanding entries._

## Resolved

### `<VirtualAddress as Address>::into_raw_value` — newtype identity (RESOLVED)

- **Module**: `sys::mm::address::virt` (`src/libs/sys/src/sys/mm/address/virt.rs`)
- **Spec** (from `view_design.md`): `ensures result as int == self@`.

Previously held as a consumer-side `assume_specification` trust boundary because
the *trait* method `<VirtualAddress as Address>::into_raw_value` cannot be
body-verified without verifying the whole `impl Address for VirtualAddress`
(which pulls the unsupported `usize as *const u8` casts in `as_ptr`/`as_mut_ptr`
into scope).

This is now **body-verified in `sys`** by adding an inherent
`VirtualAddress::into_raw_value(self) -> usize` (in the dedicated
`#[verus_verify] impl VirtualAddress` block) carrying
`ensures result as int == self@`. The inherent method **shadows** the trait method
for every concrete `VirtualAddress` caller — exactly the pattern already used for
`VirtualAddress::new` and the inherent `VirtualAddress::from_raw_value`. The
consumer-side `assume_specification[ <VirtualAddress as Address>::into_raw_value ]`
in `kernel`'s `phys.spec.rs` was therefore removed; `make verify` (sys + kernel)
passes with 0 errors and no cheating introduced.
