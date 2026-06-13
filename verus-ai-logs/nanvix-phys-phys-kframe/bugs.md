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
