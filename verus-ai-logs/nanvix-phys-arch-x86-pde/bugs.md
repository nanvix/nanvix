# Bug Report — `src/libs/arch/src/x86/mem/paging/pde.rs`

## Summary

No bugs found during verification of the in-scope functions:

- `PageDirectoryEntryFlags::new`
- `PageDirectoryEntryFlags::is_present`
- `PageDirectoryEntry::new`
- `PageDirectoryEntry::is_present`
- `PageDirectoryEntry::frame_address`

All specifications verify with `make verify-arch` (48 verified, 0 errors) and the
full `make verify` (no regressions, 0 errors). The module contains no `admit()`,
`assume()`, or `external_body` constructs.

`frame_address` is overflow-free: the `FrameNumber` type invariant bounds the frame
index by `FrameNumber::spec_max()`, so the derived physical base
`frame * FRAME_SIZE` fits in `usize` (proved in `lemma_frame_address`).
