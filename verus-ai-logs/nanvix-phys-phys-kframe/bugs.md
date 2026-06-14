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

## Note (not a bug) — `KernelFrame::new` retains `external_body`

The automated *strip-external-body* step removed `#[verus_verify(external_body)]`
from `KernelFrame::new`, leaving the source non-building (`error!` macro →
"Unsupported constant type") and, more fundamentally, unverifiable: `new`'s body
calls `crate::mm::virt::identity_map_page`, whose precondition is the **global**
`identity_map_view().inv()`. That fact belongs to the `mm::virt` identity-map
ghost token (a parameter-free singleton-global like `phys_view()`), which is not
realized in `mm::phys` and cannot be derived from the sole available precondition
`base.inv()`. No lemma/axiom yields `identity_map_view().inv()` unconditionally
(the view is `uninterp`). The attribute was therefore restored; `new` is listed
in `verus-ai-logs/tcb-allowed.md`. Same cross-module global-token deferral as
`frame::book`/`frame::alloc`. The in-scope siblings `base` and `drop` verify
in-body (no `admit`/`assume`/`external_body`).
