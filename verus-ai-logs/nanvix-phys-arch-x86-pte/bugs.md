# Bug Report — x86::mem::paging::pte

Module: `src/libs/arch/src/x86/mem/paging/pte.rs`

Functions in scope: `PageTableEntry::new`, `PageTableEntryFlags::new`,
`PageTableEntry::is_present`, `PageTableEntryFlags::is_present`.

None.

All four in-scope functions verified cleanly (`6 verified, 0 errors`, `admit=0`,
`assume=0`) with no proof gaps and no code defects. The `PageTableEntry::new`
postcondition `result.inv()` (frame bound `0 <= frame <= FrameNumber::spec_max()`)
is discharged from the `FrameNumber` type invariant via the existing
`use_type_invariant(frame)` proof block — no overflow, off-by-one, or unchecked
cast was found.
