# Ongoing Work: PDE Permission Design

## Context

Nanvix currently represents ownership and knowledge of each page-directory entry with a Verus
`PointsTo<PteWord>` token. This is too strong for a hardware-walked page directory.

An initialized `PointsTo<PteWord>` gives its owner persistent exact knowledge of the word stored at
the associated address. The MMU may independently set architecturally managed bits in that word,
especially the accessed bit. Therefore, retaining an initialized `PointsTo<PteWord>` on the Nanvix
side would leave the verifier with stale exact knowledge after MMU activity.

Making the token private does not solve the problem. Privacy prevents clients from inspecting the
token, but it does not weaken the semantic claim that the token contains exact current knowledge.

The immediate goal is to design a small PDE-specific permission API that plays the same role as
`PointsTo`, while expressing the actual contract between Nanvix and the MMU.

This is currently an interface design. The implementation in terms of Verus primitives will be
considered later.

## Scope

The initial design covers a standard, non-leaf x86 page-directory entry:

- the page-size bit is clear;
- Nanvix controls the entry's stable fields;
- the MMU may set the accessed bit;
- the MMU may not clear the accessed bit;
- the dirty bit is not MMU-managed for this PDE kind; and
- TLB state and invalidation are separate interactions.

The architectural validity predicate is still incomplete. The current sketch retains the existing
check that bit 7 is clear. Reserved, ignored, software-available, physical-address, and
feature-dependent fields must be refined from the architecture definition.

## Design Direction

The first proposal included lifecycle states, generations, retirement operations, exact private
ownership, observations, and explicit publication transitions. Those concepts may eventually be
useful, but they are too complex for the current task.

The selected direction deliberately mirrors the small `PointsTo` API:

- associate a token with one PDE pointer;
- record whether the entry is initialized;
- record the baseline value most recently established by Nanvix;
- expose a predicate describing possible current values;
- specify reads as observations compatible with the baseline; and
- specify Nanvix writes as replacing the baseline.

The token must not expose an exact persistent `value()` for shared paging memory. Instead, it
exposes `expected()`, which is the Nanvix-established baseline. A returned read value is exact only
at the read's interaction point.

## PDE Compatibility

Let `expected` be the value most recently established by Nanvix and `actual` be a value observed
from memory.

For the current standard-PDE model, `compatible_pde(expected, actual)` means:

1. `expected` and `actual` are valid standard PDEs.
2. Every MMU-stable field in `actual` equals the corresponding field in `expected`.
3. If the accessed bit is set in `expected`, it must be set in `actual`.
4. If the accessed bit is clear in `expected`, it may be either clear or set in `actual`.

Thus, the MMU may perform the monotonic transition:

```text
accessed: 0 -> 1
```

It may not clear accessed or modify another field.

The minimal design intentionally does not remember that a previous read observed accessed set. If
the baseline has accessed clear, every later read may independently return either clear or set.
This loses proof information but remains sound and keeps the token API small.

One unresolved architectural question is whether the MMU may set accessed on a non-present PDE.
The final compatibility predicate must encode the architectural rule.

## Proposed Token API

The following is a Rust-like Verus interface sketch. It is not expected to compile until the token
representation and shared-state mechanism are selected.

```rust
use ::vstd::prelude::*;

type PteWord = u32;

const ACCESSED_BIT: PteWord = 1 << 5;

verus! {

/// Returns `true` if `value` is a valid standard PDE.
pub open spec fn valid_standard_pde(value: PteWord) -> bool {
    value & 0x80 == 0
}

/// Returns the fields that the MMU cannot modify.
pub open spec fn stable_pde_fields(value: PteWord) -> PteWord {
    value & !ACCESSED_BIT
}

/// Returns `true` if `actual` is a possible current value after Nanvix
/// established `expected`.
pub open spec fn compatible_pde(expected: PteWord, actual: PteWord) -> bool {
    &&& valid_standard_pde(expected)
    &&& valid_standard_pde(actual)
    &&& stable_pde_fields(actual) == stable_pde_fields(expected)
    &&& (expected & ACCESSED_BIT != 0 ==> actual & ACCESSED_BIT != 0)
}

/// Nanvix's permission to access one PDE.
///
/// `expected()` is the value most recently established by Nanvix. It is not
/// necessarily the exact current memory value.
pub struct NanvixPdeToken {
    // Abstract representation.
}

/// The MMU's permission to access the same PDE.
///
/// This token authorizes only architecturally permitted MMU operations.
pub struct MmuPdeToken {
    // Abstract representation.
}

impl NanvixPdeToken {
    /// Returns the address of the associated PDE.
    pub spec fn ptr(&self) -> *mut PteWord;

    /// Returns `true` if the PDE has been initialized.
    pub spec fn is_init(&self) -> bool;

    /// Returns `true` if the PDE is uninitialized.
    pub open spec fn is_uninit(&self) -> bool {
        !self.is_init()
    }

    /// Returns the value most recently established by Nanvix.
    ///
    /// This is a baseline value, not necessarily the current memory value.
    pub spec fn expected(&self) -> PteWord
        recommends
            self.is_init();

    /// Returns `true` if `value` is a possible current PDE value.
    pub open spec fn admits(&self, value: PteWord) -> bool {
        self.is_init() && compatible_pde(self.expected(), value)
    }
}

impl MmuPdeToken {
    /// Returns the address of the associated PDE.
    pub spec fn ptr(&self) -> *mut PteWord;

    /// Returns `true` if the PDE has been initialized.
    pub spec fn is_init(&self) -> bool;

    /// Returns the Nanvix-established baseline for the associated PDE.
    ///
    /// This observes shared protocol state, not an exact private copy.
    pub spec fn expected(&self) -> PteWord
        recommends
            self.is_init();

    /// Returns `true` if `value` is a value the MMU may observe.
    pub open spec fn admits(&self, value: PteWord) -> bool {
        self.is_init() && compatible_pde(self.expected(), value)
    }
}

/// Returns `true` if these are the Nanvix and MMU tokens for the same PDE.
pub spec fn paired(
    nanvix: &NanvixPdeToken,
    mmu: &MmuPdeToken,
) -> bool;

} // verus!
```

The abstract implementation must ensure that paired tokens observe the same initialization state,
pointer, and Nanvix-established baseline. The design intentionally defers whether that relationship
is implemented with an invariant, a custom state machine, or another Verus mechanism.

## Nanvix Interaction API

### Read

Nanvix must provide an initialized token whose baseline is a valid standard PDE. The returned word
must be compatible with the baseline.

The token is unchanged. Nanvix learns the exact value returned by this particular read, but it
cannot conclude that the value remains in memory afterward.

```rust
/// Reads a PDE on behalf of Nanvix.
#[verus_verify(external_body)]
#[verus_spec(result =>
    requires
        token.is_init(),
        valid_standard_pde(token.expected()),
    ensures
        token.admits(result),
)]
fn nanvix_read_pde(token: &NanvixPdeToken) -> PteWord {
    unsafe { token.ptr().read_volatile() }
}
```

### Write

Nanvix must provide an architecturally valid standard PDE. The write stores exactly the requested
word at its interaction point and establishes it as the token's new baseline.

Persistent exact equality is not promised. The MMU may set accessed immediately after the write,
so future reads are guaranteed only to return a compatible value.

```rust
/// Writes a PDE on behalf of Nanvix.
#[verus_verify(external_body)]
#[verus_spec(
    requires
        valid_standard_pde(value),
    ensures
        final(token).ptr() == old(token).ptr(),
        final(token).is_init(),
        final(token).expected() == value,
)]
fn nanvix_write_pde(token: &mut NanvixPdeToken, value: PteWord) {
    unsafe {
        token.ptr().write_volatile(value);
    }
}
```

Writing accessed clear resets the baseline to clear. That does not establish persistent knowledge
that the current accessed bit remains clear.

## MMU Interaction API

### Read

The MMU may read an initialized PDE. Its observation must be compatible with the baseline
established by Nanvix.

```rust
/// Reads a PDE during an MMU page walk.
#[verus_verify(external_body)]
#[verus_spec(result =>
    requires
        token.is_init(),
        valid_standard_pde(token.expected()),
    ensures
        token.admits(result),
)]
fn mmu_read_pde(token: &MmuPdeToken) -> PteWord {
    unsafe { token.ptr().read_volatile() }
}
```

### Set Accessed

The MMU may set accessed while preserving every stable field. It may not perform an unrestricted
write.

The following sketch passes the immediately preceding observation explicitly. A real MMU model
would likely represent this as an atomic read-modify-write transition.

```rust
/// Sets the accessed bit.
#[verus_verify(external_body)]
#[verus_spec(result =>
    requires
        token.is_init(),
        token.admits(observed),
    ensures
        result == observed | ACCESSED_BIT,
        token.admits(result),
        stable_pde_fields(result) == stable_pde_fields(observed),
        result & ACCESSED_BIT != 0,
)]
fn mmu_set_accessed(
    token: &MmuPdeToken,
    observed: PteWord,
) -> PteWord {
    let value: PteWord = observed | ACCESSED_BIT;

    unsafe {
        token.ptr().write_volatile(value);
    }

    value
}
```

There should be no general MMU operation equivalent to:

```rust
fn mmu_write_pde(token: &MmuPdeToken, value: PteWord);
```

The MMU API should expose only named architectural transitions.

## Proposed Page-Directory Write Specification

Assume that `PageDirectory.permissions` becomes:

```rust
permissions: Tracked<Map<nat, NanvixPdeToken>>,
```

The current wrapper specification could then become:

```rust
// Equivalent to direct assignment because it writes the same raw value to the same PDE.
#[verus_verify(external_body)]
#[verus_spec(
    requires
        old(self).inv(),
        0 <= index < ::arch::mem::PAGE_TABLE_LENGTH,
        valid_standard_pde(value),
    ensures
        final(self).inv(),

        final(self).permissions[index as nat].ptr()
            == old(self).permissions[index as nat].ptr(),

        final(self).permissions[index as nat].is_init(),

        // Nanvix establishes this value as the new baseline.
        final(self).permissions[index as nat].expected() == value,

        // The MMU may subsequently set accessed, so future reads are
        // guaranteed to return a value compatible with this baseline.
        forall|observed: PteWord|
            final(self).permissions[index as nat].admits(observed)
                <==> compatible_pde(value, observed),

        // No other entry permission changes.
        forall|i: nat|
            0 <= i < ::arch::mem::PAGE_TABLE_LENGTH && i != index as nat
                ==> final(self).permissions[i] == old(self).permissions[i],
)]
fn env_interaction_write_page_directory_entry(
    &mut self,
    index: usize,
    value: PteWord,
) {
    self.entries[index] = value;
}
```

This intentionally removes the existing whole-storage equality condition:

```rust
final(self).entries.get_storage() == old(self).entries.get_storage()
```

That condition may be stronger than the required storage-identity frame condition and may
accidentally constrain contents. The proposed contract instead preserves the selected token's
pointer and leaves every other token unchanged.

The quantified `admits()` postcondition may be redundant if `admits()` is an open definition based
only on `is_init()`, `expected()`, and `compatible_pde()`. It is included in the sketch to make the
intended persistent memory guarantee explicit. It can be removed if Verus proves it directly from
the preceding postconditions.

## Contract Summary

The replacement for the exact `PointsTo::value()` model is:

```text
token.expected()
```

for the value established by Nanvix, together with:

```text
token.admits(observed)
```

for values that may currently be observed.

The intended guarantees are:

- a Nanvix read returns an admitted value;
- a Nanvix write replaces the baseline with the requested valid value;
- an MMU read returns an admitted value;
- an MMU update may set accessed while preserving every stable field; and
- neither side receives persistent exact knowledge of shared PDE memory.

## Open Questions

1. How should paired Nanvix and MMU tokens share the baseline without introducing unsound duplicate
   authority?
2. Can existing Verus invariants or state-machine machinery implement this API, or is another
   abstraction required?
3. Should the trusted-interface stage expose an MMU token at all, or should it exist only in the
   later explicit MMU model?
4. What are the complete architectural validity rules for a standard x86 PDE?
5. Under what conditions may the MMU set accessed on a non-present PDE?
6. Does the MMU's accessed update need an explicit atomic read-modify-write specification to avoid
   overwriting a concurrent Nanvix replacement?
7. Should `admits()` remain an open derived predicate, or should it be part of an opaque token
   interface?
8. How are the tokens created by the page-directory allocator and transferred into
   `PageDirectory::new` without changing the executable API?

No source specification has been changed as part of this design note.
