# View Design: `mm::address::mod` (`Address` trait)

## Abstract Resource

To the outside world an `Address` is **a single machine address modeled as a
mathematical integer**. A caller treats any implementor (`VirtualAddress`,
`PhysicalAddress`, `PageAligned<T>`, `PageTableAligned<T>`) purely as the
integer value it denotes, plus range/alignment predicates over that integer.
Nothing else about an address is caller-observable.

## View Type

The `Address` trait already fixes its abstraction at the type level:

```rust
pub trait Address
where
    Self: ... + View<V = int>,
{ ... }
```

So **`View = int`**. The view function is the standard

```rust
// supplied by `impl View for <Implementor>`; closed so the usize→int mapping
// does not leak.
spec fn view(&self) -> int;   // written `self@`
```

`self@` is the integer address. This is the entire abstract state — there is no
auxiliary bookkeeping a caller can observe, so **no `View` struct is
introduced**. Wrapping a lone integer in a one-field struct would add ceremony
without adding a caller-visible concept (see *Rejected Alternatives*).

### Why a bare `int` (substitution test on the single "field")

> If the implementation were completely rewritten with a different algorithm,
> would the integer address value still make sense?

Yes. Every conceivable implementation of `Address` — a `usize` newtype, a
bit-packed canonical-form pointer, a tagged representation, a wrapper that
forwards to an inner address (`PageAligned<T>`) — still *denotes one integer
machine address*. The integer survives any rewrite. ✅ All caller expectations
in the analysis (round-trip, range, alignment, ordering) are statements about
this one integer.

## Well-formedness Invariant

An address value is well-formed exactly when it lies in the representable range
`[0, max_addr]` for its type. The upper bound is a *type-level constant*
(`Self::max_addr()`), modeled by the spec companion `spec_max_addr::<Self>()`,
not per-value state. Because `View = int` is a primitive (we cannot hang methods
on `int`), the invariant is stated as a free predicate parameterized by the
implementor type:

```rust
// Abstract well-formedness of an address value of type `T`.
pub open spec fn addr_wf<T: Address>(v: int) -> bool {
    0 <= v <= spec_max_addr::<T>()
}
// Used as: addr_wf::<Self>(self@)
```

Rationale:
- `0 <= v` — a raw value originates from `usize` (`from_raw_value`), which is
  non-negative; in spec world `int` could be negative, so the lower bound is
  made explicit.
- `v <= spec_max_addr::<T>()` — the range contract every implementor enforces in
  `from_raw_value` and every test pins (`from_raw_value(max_addr()) == Ok`,
  `from_raw_value(max_addr()+1) == Err(BadAddress)`).

This is the abstraction-level invariant the skill asks for: it is visible to
callers and would hold under any reimplementation.

## Spec Transition Functions

`Address` is an **immutable value type** — none of the in-scope functions mutate
an existing address, so there are no `..self` state transitions. The View
instead needs deterministic spec *helpers* that the three target contracts
reference. They are pure functions of the integer value (and inputs), one per
caller-observable concept:

```rust
// 1. Validity of a raw value as an address of type T (drives from_raw_value).
//    Bidirectional: success iff in range.
pub open spec fn spec_addr_valid<T: Address>(raw: int) -> bool {
    0 <= raw <= spec_max_addr::<T>()
}

// 2. Alignment predicate over an address value (drives is_aligned).
//    `spec_align_value` is the existing spec companion of `Alignment`.
pub open spec fn spec_addr_is_aligned(v: int, align: Alignment) -> bool {
    v % crate::mm::spec_align_value(align) == 0
}
```

`into_raw_value` needs no helper: its contract is the identity projection
`result as int == self@`, already on the trait.

### How the in-scope contracts read against this View

- **`from_raw_value(raw: usize) -> Result<Self, Error>`** (currently *unspec'd*
  — the gap the analysis flags):
  ```text
  ensures
    result matches Ok(a)  ==> a@ == raw as int && spec_addr_valid::<Self>(raw as int),
    result matches Err(e) ==> e == Error::BadAddress && !spec_addr_valid::<Self>(raw as int),
  ```
  The success arm gives callers the round-trip fact (`a@ == raw`) plus
  well-formedness; the error arm is the bidirectional failure condition
  (out-of-range ⇔ `BadAddress`) the tests and blanket-impl `?`-propagation rely
  on.

- **`into_raw_value(self) -> usize`** (already spec'd, keep):
  ```text
  ensures result as int == self@
  ```
  Exact, total, lossless projection — `MemoryRegion::new` and the
  `PageAligned`/`PhysicalAddress` round-trips depend on this equality.

- **`is_aligned(&self, align: Alignment) -> Result<bool, Error>`** (already
  spec'd; restate via the helper):
  ```text
  ensures result matches Ok(b) && b == spec_addr_is_aligned(self@, align)
  ```
  Drives `PageAligned`/`PageTableAligned` construction correctness.

The supertrait `Ord`/`Eq` agreement with `@` (needed by `MemoryRegion`,
`PageAligned`) is a property of the `int` view itself and needs no extra View
machinery.

## Design Rationale

| Decision | Reason / substitution-test result |
|----------|------------------------------------|
| `View = int` (no struct) | Fixed by the trait's `View<V = int>` supertrait and matches the only thing callers observe — the integer address. Survives any rewrite. ✅ |
| Invariant `0 <= self@ <= spec_max_addr::<Self>()` | The single caller-visible well-formedness fact; every implementor enforces it in `from_raw_value`; tests pin both bounds. ✅ |
| `spec_addr_valid` as a free predicate | Lets `from_raw_value`'s success/error arms be stated symmetrically at the interface level (range), not as a list of internal checks. ✅ |
| `spec_addr_is_aligned` wraps existing `spec_align_value` | Keeps `is_aligned`'s contract declarative (`v % k == 0`) and reuses the already-trusted alignment spec; independent of how alignment is computed. ✅ |
| Helpers parameterized by `T`, not methods on `int` | `int` is primitive; per-type bound `max_addr` must be threaded through a type parameter. |

Every helper/invariant is referenced by at least one in-scope contract
(`spec_addr_valid` → `from_raw_value`; `spec_addr_is_aligned` → `is_aligned`;
`addr_wf`/range → `from_raw_value`), satisfying the *minimal* and *no floating
spec* criteria.

## Rejected Alternatives

1. **A `struct AddressView { raw: int }`** — a one-field wrapper around the
   integer. Rejected: adds no caller-observable concept beyond the integer, and
   conflicts with the trait's mandated `View<V = int>`. Violates *minimal*.

2. **`struct AddressView { raw: int, max: int }`** (carry the per-type maximum
   in the view) — Rejected: `max_addr` is a *type-level constant*, identical for
   all values of a given implementor; storing it per value is redundant and
   would make two equal addresses with mismatched `max` representable.
   `spec_max_addr::<T>()` models it instead. Fails the *minimal* test.

3. **`usize` (or the raw newtype) as the view** — Rejected: machine type leaks
   the representation and reintroduces overflow reasoning in specs. The skill
   mandates abstract types (`int`). Fails *caller-only* / *no code-as-spec*.

4. **Adding an `aligned: Set<int>` / alignment cache to the view** — Rejected:
   alignment is *computed* from `self@` on demand (`self@ % k == 0`), never
   stored; modeling it as state would mirror a hypothetical implementation
   choice. Fails the substitution test.

5. **A boolean `valid` flag in the view** — Rejected: validity is fully
   determined by `0 <= self@ <= spec_max_addr::<T>()`; a separate flag could
   contradict the integer and is derivable. Fails *minimal* and risks
   inconsistency.

## Quality Review (skill Step 4)

| Criterion | Verdict |
|-----------|---------|
| Substitution | `int` value survives any reimplementation. ✅ |
| Caller-only | Callers reason in integers + alignment/range predicates; no impl detail. ✅ |
| Complete | Round-trip, range/validity, alignment, ordering all expressible over `self@`. ✅ |
| Minimal | Single integer; both helpers and the invariant feed an in-scope contract. ✅ |
| No code-as-spec | Helpers state *what* (in-range, divisible), not *how*. ✅ |

## Notes / Constraints

- In scope only: `is_aligned`, `into_raw_value`, `from_raw_value`. Other trait
  methods (`align_up`, `align_down`, `max_addr`, `clone_address`, `as_ptr`,
  `as_mut_ptr`) are untouched.
- No `external_body` introduced (none listed in `verus-ai-logs/tcb-allowed.md`
  for this module).
- `spec_max_addr::<T>()` is assumed to be the existing spec companion of
  `Address::max_addr()`; if absent it is the one spec helper the proving phase
  must surface (a pure type-level constant), referenced by the range invariant.

## Specification-phase update (deviation recorded)

While binding contracts to exec code, the proposed `spec_max_addr::<T>()` /
`spec_addr_valid::<T>` / `addr_wf` machinery and the bidirectional range arm
for `from_raw_value` (`Err ⇔ raw > max_addr`) were **dropped**. Reasons:

1. **Untruthful across implementors.** `PhysicalAddress::from_raw_value`
   (kernel `phys.rs`) validates via `is_valid_physical_address` to support
   *sparse* physical memory — it can reject `raw` values that are `<= max_addr`.
   So `Err ⇔ raw > spec_max_addr` is false; encoding it on the trait would be a
   wrong contract. Whether `from_raw_value` succeeds depends on dynamic /
   per-platform validity, which is not expressible as a uniform caller-visible
   predicate (per spec-design: dynamic info → keep the `Err` arm, do not turn it
   into a `requires`/range predicate).
2. **Out of scope to surface `spec_max_addr`.** A per-type spec maximum would
   require either a new trait spec method (forcing every implementor — incl.
   out-of-scope `max_addr` impls — to change) or an `ensures` on the
   out-of-scope `max_addr`. Both violate "do not touch unlisted functions".

### Contracts actually bound (final)

- `from_raw_value(raw_addr) -> Result<Self, Error>` (newly specified):
  ```text
  ensures match result {
      Ok(a)  => a@ == raw_addr as int,                 // round-trip
      Err(e) => e.code == ErrorCode::BadAddress,       // error code pinned
  }
  ```
  Covers the test/caller expectations that are uniform across all implementors
  (round-trip on success; `BadAddress` on failure, used by `?`-propagation in
  the `PageAligned`/`PageTableAligned` blanket impls and the kernel tests).
- `into_raw_value` — unchanged (`result as int == self@`).
- `is_aligned` — same predicate, now via helper
  `spec_addr_is_aligned(self@, align)` defined in `mod.spec.rs`.

`spec_addr_is_aligned(v, align) := v % spec_align_value(align) == 0` is the only
View helper retained; it is referenced by `is_aligned`'s `ensures`.

### Incidental fix

`mod.rs` carried a redundant duplicate `use ::vstd::prelude::*;` (in addition to
the conventional `use vstd::prelude::*;`), pre-existing and identical in the
pre-spec commit. It broke the non-Verus `cargo build` under the workspace's
`warnings = "deny"`. Removed the duplicate to restore dual compilation; matches
the single-import pattern of the sibling `virt.rs`. Verus verification unaffected.
