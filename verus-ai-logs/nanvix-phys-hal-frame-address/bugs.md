# Bugs — hal::mem::types::address::frame

## BUG-001 (auto-fixed): Duplicate `use ::vstd::prelude::*;` breaks normal build

- **File**: `src/kernel/src/hal/mem/types/address/frame.rs`
- **Symptom**: Under the normal (non-`verus_keep_ghost`) build with `-D warnings`,
  `./z build -- all-kernel` failed:
  `error: unused import: ::vstd::prelude::*` at line 22.
- **Root cause**: `frame.rs` had `use vstd::prelude::*;` at line 8 AND a redundant
  duplicate `use ::vstd::prelude::*;` at line 22. Sibling modules (`phys.rs`,
  `aligned/page.rs`) carry only the single line-8 import. The duplicate glob
  re-import is flagged as unused. Pre-existing (present at commit 38885545d,
  before this spec phase).
- **Fix**: Removed the redundant `use ::vstd::prelude::*;` (line 22). The single
  `use vstd::prelude::*;` at line 8 is retained (matches siblings; required for the
  cfg-gated Verus spec/proof includes).
- **Validation**: `./z build -- all-kernel` compiles clean; module re-verifies
  (6 verified, 0 errors).
