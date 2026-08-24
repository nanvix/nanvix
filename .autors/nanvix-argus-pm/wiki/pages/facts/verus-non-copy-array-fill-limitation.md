---
id: verus-non-copy-array-fill-limitation
type: fact
status: stable
title: Verus rejects non-Copy array repeat fills
sources: []
created_at: 2026-08-24
last_reviewed_at: 2026-08-24
---

Verus `0.2026.08.23.fbbbbcf` rejects an array repeat expression whose element
type is not `Copy` during frontend lowering:

```text
The verifier does not yet support the following Rust feature:
array-fill expresion with non-copy type
```

The rejection applies to legal Rust such as `[const { None }; N]` for
`[Option<Record>; N]` when `Record` is not `Copy`. Verus accepts ordinary
`[value; N]` repetition for `Copy` element types. Its own
`test_array_repeat_non_copy_const` test records the non-Copy diagnostic as an
expected error, while `test_array_repeat` covers accepted `Copy` forms.

`core::array::from_fn` is not an accepted workaround in this Verus version.
A direct single-file check rejects the call as unsupported and suggests adding
an `assume_specification`, which would add forbidden trust rather than discharge
the limitation.

The Nanvix PM occurrence is
`src/kernel/src/pm/process/manager/delivery.rs::LifecycleQueueChunk::new`.
Its array element is
`Option<(DeliverySequence, LifecycleNotification)>`.
`LifecycleNotification` owns creation or termination capacity credits that are
intentionally non-`Copy` linear resources; making the element `Copy` would allow
credit duplication and invalidate reservation accounting.

The 66 PM files contain 12 array repeat initializers: 11 have `Copy` elements,
and the delivery queue initializer is the sole non-`Copy` occurrence. The
minimized current-form, `from_fn`, `Copy`-contrast, and legal-Rust cases are
under `/home/ruize/argus-pm-artifacts-20260824/reproducer/`.
