# Verification TODOs: bump-allocator

No remaining proof gaps.

All in-scope functions verify with `make verify-bump-allocator` (10 verified, 0
errors). No `admit()`, `assume()`, or unapproved `assume_specification` remain.
The only `external_body` items are `FixedSizeBumpAllocator::alloc` and
`FixedSizeBumpAllocator::alloc_as`, both explicitly allowed by
`verus-ai-logs/tcb-allowed.md`.
