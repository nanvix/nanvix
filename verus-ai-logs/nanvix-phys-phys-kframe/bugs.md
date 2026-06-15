# Bugs — `mm::phys::kframe`

No correctness/logic bugs found in the in-scope functions
(`KernelFrame::new`, `KernelFrame::base`, `KernelFrame::drop`).

## Build-hygiene fix (auto-fixed) — duplicate import (kframe.rs)

**What**: `kframe.rs` imported `vstd::prelude::*` twice — `use vstd::prelude::*;`
(top, required so Verus attribute macros parse/erase correctly) and a redundant
`use ::vstd::prelude::*;` lower down.

**Why**: In the normal (non-Verus) `cargo build`, the Verus constructs are erased,
so the glob import is otherwise unused. With two identical glob imports, rustc
flags the second as `unused import`, which is promoted to a hard error under the
crate's `-D warnings`. This broke dual compilation (`./z build -- all-kernel`).
The sibling `mm::phys::upool` has only the single top-of-file import and builds
cleanly.

**Verification Failure**: Not a Verus failure (Verus build passed). Surfaced by
`./z build -- all-kernel`: `error: unused import: ::vstd::prelude::*`.

**How Verus Helped**: The dual-compilation requirement (Verus build + normal
build) exposed a latent redundant import that a Verus-only or normal-only build
in isolation would not necessarily flag the same way.

**Severity**: cosmetic / build-hygiene (no runtime effect).

**Suggested Fix / Auto-Fixed**: Yes — removed the redundant
`use ::vstd::prelude::*;`, keeping the single top-of-file `use vstd::prelude::*;`
(matching `upool.rs`). Both Verus verification and normal build pass afterwards.

## Proving-phase note — `KernelFrame::new` retains sanctioned `external_body`

**What**: The `strip-external-body` proving step removed `#[verus_verify(external_body)]`
from `KernelFrame::new`, which broke compilation (Verus "Unsupported constant type"
on the `error!` macro) and then required a spec for `PageAligned::from_raw_value`.

**Why it must stay `external_body`**: `new`'s body calls
`crate::mm::virt::identity_map_page`, whose precondition is `identity_map_view().inv()`
— a global well-formedness invariant of the **not-yet-verified** `mm::virt` module
(`identity_map_view()` is `uninterp`; its `inv()` is not a tautology and `mm::virt`
itself still carries `admit()`). This precondition cannot be discharged within
`mm::phys` without (a) `admit()` (forbidden), (b) adding a `requires` to `new`
(spec weakening — restricts the manager callers, forbidden), or (c) touching the
out-of-scope `mm::virt` layer. `new` is explicitly listed in `tcb-allowed.md`
("Cross-module dependencies marked external_body"), so `external_body` is the sound,
rule-compliant choice until `mm::virt` is verified, at which point the trusted bridge
for `identity_map_view().inv()` (mirroring `frame::instance()` pinning `phys_view()`)
becomes available and the `external_body` can be eliminated.

**Resolution**: Restored `external_body` on `new` (matching the spec-phase design and
the prior known-good PASS). `KernelFrame::base` and `KernelFrame::drop` are fully
verified in-body (0 `admit`, 0 `assume`). Not a code bug.
