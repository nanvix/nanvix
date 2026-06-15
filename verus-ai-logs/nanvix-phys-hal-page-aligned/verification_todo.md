# Verification TODOs — hal-page-aligned

These items are genuine, human-approved trust boundaries (recorded in
`verus-ai-logs/tcb-allowed.md`). They are blocked by **unverified upstream
dependencies** (`sys::mm::Address` trait and the `arch` `Alignment` encoding),
not by proof-strategy gaps in this module. No `admit()`/`assume()` is used.

## 1. `PageAligned::from_address` — `external_body` (page.rs)

Blocking facts (Verus front-end / unspecced upstream):

- The body validates alignment via `<T as Address>::is_aligned(PAGE_ALIGNMENT)`.
  The `sys::mm::Address` trait method `is_aligned` carries **no** `#[verus_spec]`
  contract (see `src/libs/sys/src/sys/mm/address/mod.rs:98`), so Verus knows
  nothing relating its `bool` result to `spec_addr(&addr) % spec_page_size()`.
- `spec_addr` is an `uninterp spec fn` (page.spec.rs), so the success arm
  `result@ == spec_addr(&addr)` cannot be discharged from the inner constructor.
- `PAGE_ALIGNMENT` is `arch::mem::PAGE_ALIGNMENT` (= `Alignment::Align4096`),
  an `arch` enum constant the Verus front-end cannot translate
  (`error: arch::x86::mem::constants::PAGE_ALIGNMENT is not supported`).

Body-verifying `from_address` would require writing a **new**
`assume_specification` for `<T as Address>::is_aligned` on the external `sys`
trait — a larger, unapproved external-bottom trust surface than the single
`external_body`. Per **verus-constraints** ("assume_specification — Human-Approved
Only"), this is out of scope until the `Address` trait and the `Alignment`
encoding are themselves verified. Listed in `tcb-allowed.md`.

## 2. `<PageAligned<T> as Address>::into_raw_value` — `assume_specification` (page.spec.rs)

Blocking fact (confirmed Verus front-end bug):

- Body-verifying this method requires marking the whole
  `impl Address for PageAligned<T>` block verified. Doing so triggers a Verus
  front-end panic, reproduced on this tree:

  ```
  thread 'rustc' panicked at vir/src/traits.rs:511:13:
  assertion failed: !method_impls.contains(&p)
  ```

- Even absent the panic, the inner `<T as Address>::into_raw_value` is unspecced,
  so `result as int == addr@` would still need an upstream `assume_specification`.

The `assume_specification` is the human-approved workaround (listed in
`tcb-allowed.md`); it is removed when the `sys::mm::Address` trait is verified.
This item is not counted by the cheating gate (it is neither `external_body`,
`admit`, nor `assume`).
