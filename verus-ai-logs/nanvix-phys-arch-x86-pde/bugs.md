# Bug Report — arch::x86::mem::paging::pde

None.

All in-scope functions (`PageDirectoryEntry::new`, `PageDirectoryEntryFlags::new`,
`PageDirectoryEntry::is_present`, `PageDirectoryEntryFlags::is_present`,
`PageDirectoryEntry::frame_address`) verify against their specifications with no
admit/assume/external_body and no code changes required. The overflow-freedom of
`frame_address` (frame index × FRAME_SIZE within `usize`) is discharged by
`lemma_frame_address` using the `FrameNumber` type-invariant bound.
