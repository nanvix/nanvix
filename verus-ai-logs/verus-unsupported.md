# Verus Unsupported Constructs

Genuine Verus front-end limitations encountered during verification. These are
**not** proof gaps or code bugs: the verifier cannot parse/lower the construct,
so no proof strategy can address them. Per the `verus-constraints` skill they are
recorded here (exec code is left untouched; no `external_body`/`assume_specification`
workaround is added on the affected module's own functions).

## `<VirtualAddress as Address>::into_raw_value` — trait-impl whole-impl verification

- **Module**: `sys::mm::address::virt` (`src/libs/sys/src/sys/mm/address/virt.rs`)
- **In-scope target**: yes (`into_raw_value`, 102 callers).
- **Desired spec** (from `view_design.md`): `ensures result as int == self@`.

### Why it cannot be body-verified in `sys`

`into_raw_value` is a method of the `sys::mm::Address` trait, implemented by
`impl Address for VirtualAddress`. Verus requires that *to verify any single
method of a trait impl, the entire `impl` block must be verified*:

```
error: In order to verify any items of this trait impl, the entire impl must be
verified. Try wrapping the entire impl in the `verus!` macro.
```

Marking the whole `impl Address for VirtualAddress` verified (`#[verus_verify]`)
pulls the sibling methods `as_ptr` / `as_mut_ptr` into verification scope, which
contain raw-pointer casts the Verus front-end does not support:

```
error: Verus does not support this cast: `usize` to `*const u8`
error: Verus does not support this cast: `usize` to `*mut u8`
```

`as_ptr` / `as_mut_ptr` are out-of-scope functions; their exec bodies
(`self.0 as *const u8`) cannot be rewritten, and `external_body` is disallowed
here (not in `tcb-allowed.md`; the current module's own functions may not be
`external_body`).

The alternative — adding a `View<V = int>` bound to the `Address` trait so the
contract can be stated on the trait *declaration* (in `mm/address/mod.rs`) — is
documented to break `region.rs`, which uses `PageAligned<T>: Address` for a bare
`T: Address` (see
`src/kernel/src/hal/mem/types/address/aligned/page.spec.rs`, lines 20–21, 33–45).
`uninterp` projections are banned by the project constraints, so the generic
`spec_addr<T: Address>` route is also unavailable.

### Trust boundary (unchanged, pre-existing)

The newtype-identity fact is therefore preserved as an `assume_specification`
**consumer-side** trust boundary, exactly as the codebase already draws it for
`<PageAligned<T> as Address>::into_raw_value` and `FrameAddress::into_raw_value`:

- `src/kernel/src/hal/mem/types/address/phys.spec.rs`:
  `assume_specification[ <VirtualAddress as Address>::into_raw_value ]`
  `ensures result as int == addr@`.

This is "discharged when the `Address` trait itself is verified" — i.e. when
Verus gains support for verifying a trait impl while excluding sibling methods
that use unsupported pointer casts (or those casts become supported).

### What *was* verified

`VirtualAddress::new` and the inherent `VirtualAddress::from_raw_value` are
self-less associated functions placed in a dedicated `#[verus_verify] impl
VirtualAddress` block (the established `phys.rs` pattern) and are body-verified
natively in `sys`:

- `new`            → `ensures result@ == value as int && result.inv()`
- `from_raw_value` → `ensures result@ == raw_addr as int && result.inv()`
