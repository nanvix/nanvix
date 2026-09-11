# Nanvix Virtual-Memory Environment Interface

## Goal and Scope

This document records the trusted interface between Nanvix virtual-memory code and the x86 MMU,
plus the proof-ownership discipline used to reach that interface. Nanvix is verified against these
contracts; an explicit MMU implementation or ghost state machine is deferred.

The investigation starts at `src/kernel/src/mm/virt/vmem.rs` and follows reachable paging-memory,
translation-control, TLB, and fault-state operations. Ordinary instruction fetches and ordinary
loads and stores are not terminal interactions for this inventory.

The MMU:

- walks paging structures rooted at the active CR3 value;
- interprets presence, permissions, page size, caching, and physical targets;
- may set architecturally managed accessed and dirty bits;
- may retain cached translations after paging-memory writes;
- observes replacement mappings after the required invalidation or root change; and
- faults on absent, reserved, or disallowed translations.

Nanvix:

- constructs valid paging entries;
- initializes child paging structures before publishing present parent entries;
- keeps every reachable paging structure alive;
- updates mappings and permissions;
- detaches paging pages before reclamation; and
- performs TLB invalidation and root replacement as separate interactions.

## Architecture Distinction

On x86, `PageDirectory` and `PageTable` are hardware-walked structures.

On x86_64, those two-level structures are primarily Nanvix bookkeeping. Hardware walks the
PML4/PDPT/PD/PT hierarchy in
`src/kernel/src/hal/arch/x86_64/mem/mmu/hwpt.rs`.

Proof ownership and architectural interpretation must therefore be target-specific even where the
high-level `Vmem` operations are shared.

## Interaction Inventory

### Paging-structure memory

- PDE read, write, and clearing in `page_directory.rs`;
- PTE read, write, iteration, fill, and clearing in `page_table.rs`;
- generic typed volatile entry access in `src/libs/arch/src/x86/mem/paging/table.rs`; and
- x86_64 hardware-entry read, write, and reused-page zeroing in `hwpt.rs`.

### Translation and cached state

- CR0 reads and writes used to enable paging;
- CR3 reads and writes used to inspect or replace the active root;
- `invlpg` on both x86 targets; and
- CR2 reads in fault-entry assembly.

The two `hooks.rs` files remain intentionally unmodified. Their context-switch CR3 interactions
are still part of the inventory and are not covered by the Rust-wrapper contracts described below.

Terminal operations are isolated in `env_interaction_*` wrappers. The detailed PDE/PTE and x86_64
entry accesses have trusted contracts. Isolated CR3 reads now guarantee a valid current
configuration, and the x86 root-loading write requires a ready page directory at the encoded
physical address. Generic table, CR0, TLB, fault, and context-switch CR3 interactions still lack
complete semantic contracts.

## Shared Paging-Memory Knowledge

An initialized `PointsTo<PteWord>` carries persistent exact knowledge. That is too strong for an
entry the MMU may change after Nanvix's last write. Privacy does not make stale exact knowledge
sound.

Nanvix therefore converts allocator-provided raw permissions into protocol tokens exposing:

- `ptr()`, the entry identity;
- initialization state;
- `expected()`, the baseline most recently established by Nanvix; and
- `admits(value)`, the values the MMU may currently have produced.

The conversion is trusted and linear:

```text
allocator PointsTo -> Nanvix entry token -> owning paging object -> interaction
```

It consumes the source `PointsTo`; no duplicate exact permission remains. The token is conservative
knowledge at the trusted-interface stage, not an implementation of an MMU-side owner.

### Compatibility rules

- A standard non-leaf x86 PDE permits a monotonic accessed-bit update.
- A leaf x86 PTE permits monotonic accessed- and dirty-bit updates.
- A present x86_64 non-leaf entry permits an accessed-bit update.
- A present x86_64 leaf permits accessed- and dirty-bit updates.
- Stable fields remain equal to the Nanvix baseline.
- A non-present x86_64 entry is currently modeled as unchanged.
- A read returns an admitted observation, not necessarily `expected()`.
- A write replaces `expected()` but does not promise persistent exact equality.

These rules remain intentionally conservative. Complete reserved-bit and feature-dependent
architectural validity is unfinished.

## x86 Page Directory and Page Table

### Ownership

`PageDirectory<T>` owns a tracked map of `NanvixPdeToken`; `PageTable<T>` owns a tracked map of
`NanvixPteToken`. Each map has the exact hardware length and ties every token pointer to the
corresponding executable storage offset.

Raw permissions now originate at both real storage sources:

1. `allocate_page_table_slot()` returns a BSS slot and
   `PageTableSlotPermissions`.
2. `KernelFrame::allocate_page_table()` returns a frame and a raw per-entry permission map.

The boot page tables, root page directory, cloned page directories, and dynamically allocated
kernel and user page tables all thread these permissions into `PageDirectory::new()` or
`PageTable::new()`. The constructors consume them through `mint_nanvix_pde_tokens()` or
`mint_nanvix_pte_tokens()` and retain only specialized tokens.

Allocator minting is a trusted authority-origin boundary. Constructor conversion is a trusted
representation boundary. The intervening transfer is proof-only and leaves runtime signatures and
layouts unchanged.

### Terminal contracts

The following six trusted interaction contracts were not changed by the allocator-threading work:

- `env_interaction_clear_page_directory`;
- `env_interaction_read_page_directory_entry`;
- `env_interaction_write_page_directory_entry`;
- `env_interaction_clear_page_table`;
- `env_interaction_read_page_table_entry`; and
- `env_interaction_write_page_table_entry`.

They preserve pointer identity and unaffected tokens, return admitted observations on reads, and
replace the selected baseline on writes. A present PDE write receives an erased witness for the
actual child page table and requires exact encoded-address correspondence and child readiness.

### Validation status

- formatting passes;
- ordinary kernel checks pass for x86 and x86_64; and
- Verus verification passes for `TARGET=x86`.

### Remaining x86 work

- Strengthen `valid_standard_pde()` beyond the current page-size-bit check.
- Strengthen `valid_pte()`, which currently accepts every `u32`.
- Prove continued child lifetime and reachability, not only readiness at publication.
- Restore constructor guarantees weakened during proof refactoring:
  - both constructors previously guaranteed every baseline was exactly zero after cleaning;
  - `PageTable::new()` also previously guaranteed `nmapped == 0`;
  - `ready_for_mmu()` currently guarantees initialization and validity, not those stronger facts.
- Review and minimize external type specifications and broad `external_body` boundaries where the
  frontend can support direct verification.

The constructor issue is not a change to the six environment-interaction contracts, but it is a
real specification regression and should be corrected.

## x86_64 Hardware Paging Hierarchy

### Ownership partition

Each non-kernel `Vmem` owns a tracked map containing its private PML4, PDPT, PD, and PT page tokens.
The root kernel `Vmem` uses the boot hierarchy and does not own a private root.

One shared proof manager owns:

- boot PML4/PDPT/PD pages shared by address spaces; and
- unallocated or reclaimed `PT_POOL` pages.

The manager uses a duplicable handle to one `LocalInvariant`; cloning the handle does not clone the
linear page tokens. Allocation transfers one page token into a `Vmem`; reclamation transfers it
back. A private address space borrows a witness for shared `PDPT[0]` rather than owning or freeing
the boot PD.

The manager is minted once at virtual-memory-manager initialization from boot and pool permission
maps. Minting authority at individual reads or allocations is prohibited.

### Hierarchy invariants

Private page maps tie keys to physical page identities and levels. Present non-leaf entries must
target a ready owned child at the next level, except for the explicit shared boot edge. Child
publication requires exact optional-witness correspondence:

```text
present non-leaf <=> Some(child)
leaf or absent    <=> None
```

`ensure_table()`, `split_2m_entry()`, `create_user_pml4()`, mapping, unmapping, and protection
thread the corresponding tokens through the trusted entry read/write boundary.

### Reclamation

`destroy_user_pml4()` now clears each parent entry before returning the child page token to the
manager. This enforces detached-before-free order and preserves the ownership invariant during
teardown.

This is a runtime hardening relative to the earlier implementation. Under the existing precondition
that the hierarchy is not active in CR3, the old order was not known to cause an observable failure,
but it did not establish the proof-relevant absence of environment-reachable dangling edges.

### Validation and trust status

Ordinary x86_64 kernel compilation passes. Full x86_64 Verus verification reaches unrelated
x86_64 slab-proof omissions, so the repository does not yet have an end-to-end successful
x86_64 verification run.

The current HWPT proof boundary is broader than the ideal terminal-operation-only design:

- boot and pool permission import is trusted;
- allocation transfer and several mutable-static accessors are `external_body`;
- `init()` remains a trusted boot-discovery boundary;
- entry read and write are trusted transitions over the page tokens; and
- the boot CR3 read validates the observed encoding and extracts its PML4 address, while TLB and
  context-switch CR3 effects do not yet carry complete semantic contracts.

## Remaining Environment-Interface Work

1. **Architectural validity**
   - Complete x86 and x86_64 reserved-bit, physical-width, NX, PAT, caching, software-bit, and
     large-page rules.
   - Confirm exactly when accessed and dirty may change, including non-present entries.

2. **Translation control**
   - Connect context-switch CR3 writes to root readiness and active-root lifetime.
   - Specify CR0 reads and writes over system-visible register values.
   - Connect CR3 replacement to architectural TLB effects.

3. **TLB invalidation**
   - State the guarantee relied upon after `invlpg` and CR3 replacement.
   - Keep invalidation separate from paging-memory writes.

4. **Lifetime and reachability**
   - Prove that every published child remains allocated while reachable.
   - Connect detached page reclamation to active-root exclusion, not only the private owner map.
   - Relate executable free-list contents to manager token ownership.

5. **Generic and fault interactions**
   - Add contracts for generic volatile `Table<E>` access.
   - Complete the fault-state boundary without modifying the excluded hook files.

6. **Trusted-boundary reduction**
   - Narrow allocator, boot-import, mutable-static, and container `external_body` annotations where
     supported.
   - Keep authority minting distinct from MMU interaction transitions.

7. **Verification and regression review**
   - Restore the x86 constructor postconditions described above.
   - Resolve or isolate the unrelated x86_64 slab proof omissions, then run full x86_64 Verus.
   - Compare trusted interaction contracts textually whenever ownership threading changes.

## Constraints

- Do not introduce a full `MmuState` during the trusted-contract stage.
- Do not model hardware-shared paging memory solely with `&mut`.
- Do not retain raw `PointsTo` after minting a specialized token.
- Do not duplicate shared boot or free-pool authority across `Vmem` objects.
- Do not return a page token to the manager before all environment-visible incoming edges are
  removed.
- Keep proof state erased and ordinary runtime signatures unchanged unless a runtime correction is
  independently justified.
- Keep original terminal operations auditable against their wrappers.
