# Caller Analysis: `x86/mem/paging/frame/number` (`FrameNumber`)

## Script Output

Source: `find_callers_lsp.py /home/ruize/nanvix-phy/src/libs/arch/src/x86/mem/paging/frame/number.rs --project-dir /home/ruize/nanvix-phy`

| Metric | Count |
|--------|------:|
| Total exec functions | 4 |
| Public / trait-pub | 2 (`from_raw_value`, `into_raw_value`) |
| Private | 2 (unit tests) |
| Types | 1 (`FrameNumber`) |

### Public API — external callers

- `FrameNumber::from_raw_value(value: usize) -> Option<Self>` — 2 external callers
  - `src/libs/arch/src/x86/mem/paging/pte.rs:304` — `frame: FrameNumber::from_raw_value(value as usize >> mem::FRAME_SHIFT)?`
  - `src/libs/arch/src/x86/mem/paging/pde.rs:303` — `frame: FrameNumber::from_raw_value(value as usize >> crate::mem::FRAME_SHIFT)?`
  - Internal: both unit tests (L83, L91).
- `FrameNumber::into_raw_value(self) -> usize` — 4 external callers
  - `src/libs/arch/src/x86/mem/paging/pte.rs:321` — `value |= (self.frame.into_raw_value() << FRAME_SHIFT) as PteWord;`
  - `src/libs/arch/src/x86/mem/paging/pte.rs:362` — `self.frame.into_raw_value() << FRAME_SHIFT` (`frame_address`)
  - `src/libs/arch/src/x86/mem/paging/pde.rs:320` — same pattern in `into_raw_value`
  - `src/libs/arch/src/x86/mem/paging/pde.rs:375` — same pattern in `frame_address`
  - Internal: both unit tests (L84, L92).

### Type `FrameNumber` — 12 external references

Re-exported via `paging/frame/mod.rs:14` and `paging/mod.rs:18`. Used as a struct
field (`frame: FrameNumber`) and parameter/return type in both `PageTableEntry`
and `PageDirectoryEntry` (`new`, `from_raw_value`, `into_raw_value`,
`frame_number`, `frame_address`).

## Trait Obligations

None. `FrameNumber` derives `Debug, Clone, Copy` only — no trait whose methods
impose external semantic contracts. There are no implicit/runtime-dispatched
callers (no `Drop`, `Iterator`, `GlobalAlloc`, etc.).

## Caller Expectations

### `from_raw_value(value: usize) -> Option<Self>`

- Callers assume:
  - Returns `Some(fn)` exactly when `value` is a valid frame number
    (`value <= FrameNumber::MAX`); returns `None` otherwise.
  - The `?` operator is used in both PTE/PDE `from_raw_value`, so a `None` here
    must cleanly abort entry construction and propagate as `None` to the entry's
    own `from_raw_value`. Callers depend on `None` meaning "out-of-range frame",
    never a silent truncation.
  - The value preserved on success round-trips: a frame built from
    `value >> FRAME_SHIFT` must yield that same number back from
    `into_raw_value()` (see PTE/PDE `into_raw_value`/`frame_address`).
- Callers don't care about:
  - The internal representation (a single `usize` newtype) or that the constructor
    is a trivial range check + wrap.
  - The exact numeric value of `MAX`, only that it bounds valid frames so that
    `into_raw_value() << FRAME_SHIFT` cannot overflow `usize`.

### `into_raw_value(self) -> usize`

- Callers assume:
  - Returns the exact raw frame number previously stored (round-trip identity with
    `from_raw_value`). No mutation, no clamping.
  - The result is bounded by `FrameNumber::MAX`, so the subsequent
    `<< FRAME_SHIFT` (= `* FRAME_SIZE`) and cast to `PteWord` does **not** overflow
    and reconstructs the physical address. This is the single most important
    guarantee: every caller immediately shifts the result left by `FRAME_SHIFT`.
  - It is total (never panics, no `Option`).
- Callers don't care about:
  - That it is a plain field read; any representation that preserves the value and
    bound is acceptable.

### Type `FrameNumber` (as a value)

- Callers (`PageTableEntry`/`PageDirectoryEntry`) store and pass it by value
  (`Copy`), treating it as an opaque, always-valid token. They never inspect or
  construct it except through `from_raw_value`/`into_raw_value`. The invariant
  "a `FrameNumber` is in `0..=MAX`" is assumed to hold for any value they hold.

## Abstract Resource

`FrameNumber` is an abstract, validated **physical page-frame index** — a `usize`
guaranteed to lie in `0..=MAX` (where `MAX = MAX_ADDRESS / FRAME_SIZE - 1`). It is
the bounded numerator of a physical address: `address = frame * FRAME_SIZE`. The
module's only job is to enforce that bound at construction so consumers can shift
it into a physical address without overflow.

## Key Invariants (caller perspective)

- **Range bound:** every `FrameNumber` satisfies `value <= MAX`
  (`MAX = usize::MAX / FRAME_SIZE - 1`).
- **Overflow safety:** the bound guarantees `into_raw_value() << FRAME_SHIFT`
  (i.e. `value * FRAME_SIZE`) does not overflow `usize` — directly relied on by
  `PageTableEntry`/`PageDirectoryEntry` `into_raw_value` and `frame_address`.
- **Round-trip identity:** `from_raw_value(v).map(|f| f.into_raw_value()) == Some(v)`
  for all `v <= MAX`; `from_raw_value(v) == None` for all `v > MAX`.
- **Totality of `into_raw_value`:** never fails, never panics, value-preserving.

## Pre-existing Specs (from upstream verification)

- `number.spec.rs` and `number.proof.rs` exist but are **empty** (`verus! { }`).
- Functions with specs: none. Functions WITHOUT specs: `from_raw_value`,
  `into_raw_value`.
- View type: does not exist yet.
- No `#[verus_spec]` annotations present in `number.rs`.

### Assessment
- Coverage: none — View/specs must be designed from scratch.
- Recommended View: a single abstract `nat`/`int` frame index with the invariant
  `view <= MAX`, sufficient to express both caller guarantees (round-trip and
  overflow-safe shift). Mirroring the `usize` field directly is acceptable here
  since the value *is* the abstraction, but the View spec must carry the
  `<= MAX` bound so callers can discharge the no-overflow shift.
