# Bugs — `sys::mm::address::virt` (`VirtualAddress`)

None.

No code bugs were found in the in-scope functions. `VirtualAddress::new`, the
inherent `VirtualAddress::from_raw_value`, and `into_raw_value` are pure,
infallible newtype identity operations and match their caller-relied contracts
exactly (round-trip identity `new(a).into_raw_value() == a`).

The one item that could not be verified — `<VirtualAddress as Address>::into_raw_value`
— is a **Verus front-end limitation**, not a code bug (trait-impl whole-impl
verification pulls in the unsupported `usize as *const u8` casts of the sibling
`as_ptr`/`as_mut_ptr` methods). It is recorded in
`verus-ai-logs/verus-unsupported.md`; its contract is preserved by the existing
consumer-side `assume_specification` trust boundary.
