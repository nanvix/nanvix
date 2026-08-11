# Page-Directory Entry Write Contract

This contract applies to
`env_interaction_write_page_directory_entry(entries, index, value)`.

## Nanvix Assumptions About the MMU

- The MMU interprets a PDE using the active x86 paging format.
- A PDE with the page-size bit clear is interpreted as a reference to a standard page table.
- The MMU may cache a translation derived from the previous PDE until Nanvix performs the
  required invalidation.
- A later page walk may observe the new PDE after the write and required invalidation.
- The MMU may set architecturally hardware-managed bits in reachable entries.

## Nanvix Guarantees to the MMU

- The written value is a valid standard-page-table PDE encoding: the page-size bit is clear.
- The selected entry contains exactly `value` after the operation.
- Every other page-directory entry is unchanged by this operation.
- The page-directory length is unchanged.
- Nanvix publishes child page-table contents before writing a present parent PDE.
- Nanvix keeps a referenced page table available while the MMU may reach it.
- Nanvix performs any required TLB invalidation separately before relying on the new mapping.

## MMU Assumptions About Nanvix

- A present PDE has an architecturally valid encoding.
- A present non-leaf PDE refers to the intended initialized page table.
- Referenced paging structures remain available while reachable through a page walk or stale
  translation.
- Nanvix does not rely on the PDE memory write itself to invalidate cached translations.
- Nanvix accounts for possible hardware-managed-bit updates when replacing a whole entry.

## MMU Guarantees to Nanvix

- Future page walks interpret the written PDE according to the architecture.
- A non-present PDE is not followed.
- A present PDE with insufficient permissions causes a protection fault rather than an
  unauthorized target access.
- After the appropriate invalidation, the MMU does not use a superseded cached translation.
- The MMU modifies only architecturally hardware-managed fields.

## Specification Boundary

The function directly observes only the page-directory slice, index, and raw value. Its Verus
contract therefore captures:

1. validity of the raw standard-page-table PDE encoding;
2. replacement of exactly one entry; and
3. preservation of every other entry.

Page-table initialization, target lifetime, MMU concurrency, and TLB state are obligations of the
callers and the separate environment-interaction functions that publish tables or invalidate
translations.
