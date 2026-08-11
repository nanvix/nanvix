# Nanvix Virtual-Memory Environment Interface

## Goal

The task is to add low-level Verus specifications for the boundary between Nanvix virtual-memory
code and the MMU. The specifications should eventually permit verification of Nanvix while treating
the MMU contract as trusted. An explicit executable or ghost model of the MMU is deferred.

The investigation begins at `src/kernel/src/mm/virt/vmem.rs` and follows all reachable virtual
memory paths.

## Informal MMU Model

The MMU:

- reads paging structures rooted at the active translation-control register;
- interprets physical targets, presence, permissions, page size, and caching fields;
- may set architecturally managed accessed and dirty bits;
- caches translations in the TLB;
- may continue using a cached translation after a paging-memory write;
- observes replacement mappings after the required invalidation or root change;
- raises faults for absent, reserved, or disallowed translations.

Nanvix:

- constructs architecturally valid paging entries;
- publishes initialized child tables before making parent entries present;
- keeps reachable paging structures alive;
- updates mappings and permissions;
- invalidates stale translations separately;
- handles faults and hardware-managed entry-bit changes.

Ordinary instruction fetches and ordinary loads or stores are outside the interaction inventory.
The boundary contains explicit paging-memory, control-register, fault-state, and TLB operations.

## Architecture Distinction

On x86, `PageDirectory` and `PageTable` are hardware-walked paging structures.

On x86_64, these two-level structures are mostly Nanvix bookkeeping. Hardware walks the separate
PML4/PDPT/PD/PT hierarchy implemented in
`src/kernel/src/hal/arch/x86_64/mem/mmu/hwpt.rs`.

Specifications must not assume that a logical Nanvix table is the hardware table on every target.

## Interaction Inventory

The broad investigation is recorded in:

- `virtual-memory-interactions.md`
- `virtual-memory-interactions-specific-lines.md`

The terminal interactions are:

### Paging-structure memory

- raw PDE reads, writes, and clearing in `page_directory.rs`;
- raw PTE reads, writes, scans, fills, and clearing in `page_table.rs`;
- volatile generic table reads and writes in
  `src/libs/arch/src/x86/mem/paging/table.rs`;
- volatile x86_64 hardware hierarchy reads, writes, and reused-page clearing in `hwpt.rs`.

### Translation control

- CR0 reads and writes used to enable paging;
- CR3 reads and writes used to inspect or replace the active root;
- CR3 changes that also invalidate applicable cached translations.

### TLB state

- `invlpg` in the shared x86 paging library;
- the local x86_64 `invlpg` implementation.

### Fault state

- CR2 reads in architecture hook assembly.

The user later excluded both `hooks.rs` files from modification. Their interactions remain part of
the inventory but are not wrapped.

## Isolation Work

Every modified terminal operation was moved behind an `env_interaction_*` wrapper. For an original
file `x.rs`, new wrappers were placed in `x.spec.rs`, which is included by `x.rs`. Original
interaction lines were commented rather than deleted.

Created specification files:

- `src/kernel/src/hal/arch/shared/mem/mmu/page_directory.spec.rs`
- `src/kernel/src/hal/arch/shared/mem/mmu/page_table.spec.rs`
- `src/kernel/src/hal/arch/x86/mem/mmu/mod.spec.rs`
- `src/kernel/src/hal/arch/x86_64/mem/mmu/hwpt.spec.rs`
- `src/libs/arch/src/x86/cpu/cr3.spec.rs`
- `src/libs/arch/src/x86/mem/paging/mod.spec.rs`
- `src/libs/arch/src/x86/mem/paging/table.spec.rs`

The wrappers currently cover:

- PDE read, write, and clear;
- PTE read, indexed read, write, and clear;
- typed volatile paging-entry read and write;
- x86_64 hardware-entry read, write, and zero;
- CR0 and CR3 access;
- TLB invalidation.

Most wrappers still have no Verus contract. The current detailed work focuses only on
`PageDirectory::env_interaction_write_page_directory_entry`.

## PDE Write Contract

The English contract is recorded in `virtual-memory-contract.md`. Its essential local obligations
are:

### Nanvix assumptions

- the MMU interprets the active x86 paging format;
- the MMU may cache the old translation until invalidation;
- the MMU may update architecturally managed bits.

### Nanvix guarantees

- the written word is interpretable as a standard-page-table PDE;
- the selected PDE is replaced with the requested value;
- other PDEs are unchanged;
- the paging structure retains the expected size;
- separate code handles publication, lifetime, and TLB invalidation.

### MMU guarantees relied upon

- non-present entries are not followed;
- permissions and invalid encodings cause the architectural fault behavior;
- invalidation prevents later use of the superseded cached translation;
- hardware changes only architecturally managed fields.

The trusted wrapper should express only state Nanvix can observe. Earlier drafts introduced an
`MmuState`, TLB maps, live-table sets, and an atomic invariant containing MMU state. That approach
was rejected for the current stage because it begins implementing an environment model. Such state
may be useful later, but it must not be part of the present trusted contract.

## Why a PDE/PTE-Specific Token Interface

The original specification used `&mut [PteWord]`. This was rejected because the MMU may also modify
the underlying entry, especially accessed and dirty bits. A mutable reference models exclusive
access and is therefore too strong.

An intermediate design used raw `PointsTo<PteWord>` tokens. That design was also rejected as the
current interface because initialized `PointsTo` retains persistent exact knowledge that becomes
stale after an MMU-managed update. Making it private does not weaken that claim.

The selected interface is intentionally independent of Verus ownership machinery. Each abstract
Nanvix token exposes only:

- `ptr()`, identifying one paging entry;
- initialization state;
- `expected()`, the baseline most recently established by Nanvix; and
- `admits(value)`, describing values that may currently be observed.

The implementation may later use Verus permissions, invariants, state machines, or another
mechanism, but the interface must not expose persistent exact memory contents.

## Refining Permission Knowledge for MMU-Managed Bits

The earlier use of an initialized `PointsTo<PteWord>` as persistent exact knowledge was too strong
for hardware-walked paging memory. The MMU may update an entry after Nanvix's last write and before
Nanvix's next read. Making the permission private prevents callers from inspecting stale knowledge,
but does not make the exclusive `PointsTo` claim sound.

For a standard x86 PDE, where `PageSizeFlag::Standard` means that bit 7 is clear, the flag ownership
is:

| Nanvix flag | Bit | MMU behavior |
|---|---:|---|
| `PresentFlag` | 0 | Stable across MMU activity. |
| `ReadWriteFlag` | 1 | Stable across MMU activity. |
| `UserSupervisorFlag` | 2 | Stable across MMU activity. |
| `PageWriteThroughFlag` | 3 | Stable across MMU activity. |
| `PageCacheDisableFlag` | 4 | Stable across MMU activity. |
| `AccessedFlag` | 5 | The MMU may set it during a page walk. |
| `DirtyFlag` | 6 | Ignored by hardware for a standard non-leaf PDE. |
| `PageSizeFlag` | 7 | Stable and required to remain clear by the current contract. |

The page-table frame field is also stable across MMU activity. The dirty bit becomes
hardware-managed for a PTE or a large-page PDE, but not for the standard PDE currently specified.

Consequently, a PDE read must not generally guarantee equality with the exact value last recorded
by Nanvix. It should guarantee:

- the returned value is a valid standard PDE;
- all stable fields equal the last value established or observed by Nanvix; and
- the accessed bit differs only by an architecturally permitted MMU transition, normally from clear
  to set.

### Selected Compatibility Rules

- A standard non-leaf PDE permits only a monotonic accessed-bit update; its dirty bit is not
  MMU-managed.
- A leaf PTE permits independent monotonic accessed- and dirty-bit updates.
- All remaining fields must equal the Nanvix-established baseline.
- A read returns an admitted observation, not `expected()` itself.
- A write replaces `expected()`; it does not promise persistent exact equality afterward.
- Postconditions derivable from the open `admits()` definition are omitted.

No MMU-side token is introduced at this trusted-interface stage.

### Immediate Validity of a Present PDE Target

A present standard PDE may be followed immediately, so its write precondition includes a proof-only
`Option<&PageTable<PageTableStorage>>`. The two valid cases correspond exactly:

```text
present PDE     <=> Some(page table)
non-present PDE <=> None
```

For the present case, the page table must satisfy its invariant, contain exactly
`PAGE_TABLE_LENGTH` entries, have every entry initialized with a valid PTE baseline, and have a
physical base equal to the address encoded in the PDE. A zeroed table satisfies this requirement
because zero is a valid non-present PTE.

The page table therefore tracks virtual storage identity separately from its proof-only physical
base. This precondition proves readiness at the PDE write; keeping the table alive while reachable
remains a distinct lifetime obligation.

## Current `PageDirectory` Proof State

`PageDirectory<T>` retains its executable storage and one proof-only token map:

```rust
entries: T
permissions: Tracked<Map<nat, NanvixPdeToken>>,
```

The permission field is gated with `#[cfg(verus_keep_ghost_body)]`. The earlier
`entries_base: Ghost<*mut PteWord>` field was removed.

The generic storage bound is now:

```rust
T: DerefMut<Target = [PteWord]> + GetPageDirectoryStorage
```

`GetPageDirectoryStorage` is declared in `src/kernel/src/mm/virt/mod.rs`. Its spec method returns a
reference to the existing `PageDirectoryStorage` enum. `PageDirectoryStorage::base_address()` then
exposes the backing address in spec mode:

- the BSS variant stores a proof-only `Ghost<usize>` address captured when the executable reference
  is constructed;
- the `KernelPage` variant derives the address through spec accessors on `KernelPage`,
  `KernelFrame`, and `FrameAddress`.

This avoids calling executable `as_ptr()` or `len()` functions from specification expressions.
The permission-to-storage relationship uses `PageDirectoryEntry::SIZE` rather than an executable
`size_of` call.

The invariant is split according to the data-structure idiom:

```rust
pub open spec fn wf(&self) -> bool;
pub closed spec fn internal_inv(&self) -> bool;
pub open spec fn inv(&self) -> bool {
    self.wf() && self.internal_inv()
}
```

`wf()` contains public sanity and value-validity facts:

- the permission domain length is `PAGE_TABLE_LENGTH`;
- the domain is exactly the range `0..PAGE_TABLE_LENGTH`;
- every permission is either uninitialized or initialized with a value satisfying
  `valid_standard_pde`.

`internal_inv()` contains the representation-dependent address correspondence. For each index, the
permission address equals the storage base address plus `index * PageDirectoryEntry::SIZE`.
`permissions_match_storage()` is private because clients do not need this implementation detail.

The constructor keeps its ordinary Rust signature:

```rust
PageDirectory::new(entries)
```

Its `#[verus_spec(with ...)]` receives the allocator-provided permission map only during
verification. `proof_with!` injects that map into the proof-only field. The constructor requires the
map to have the expected domain and requires each permission address to match the concrete storage
base.

The BSS construction path in `Vmem::new` now:

1. allocates the executable entry array;
2. computes its executable base address;
3. supplies that address to the proof-only BSS field with `proof_with!`; and
4. constructs `PageDirectoryStorage::Bss`.

The permission map is not yet produced by the allocator or supplied at the `PageDirectory::new`
call site. The runtime `KernelPage` construction path uses its address wrapper but likewise does not
yet receive allocator-originated entry permissions.

## Current Page-Directory Interaction Specifications

All three wrappers are private `PageDirectory` methods outside `verus!`, preserving their runtime
behavior and giving their contracts direct access to `self.permissions`.

### Write

`env_interaction_write_page_directory_entry` reproduces the original indexed assignment:

```rust
&mut self,
index: usize,
value: PteWord,
```

- `old(self).inv()`;
- explicit lower and upper index bounds;
- the new value satisfies `valid_standard_pde`; and
- PDE presence exactly matches the optional page-table argument, with present targets ready for an
  MMU walk and physically matching the encoded address.

- `final(self).inv()`;
- the selected token pointer is unchanged;
- the selected token is initialized with `expected() == value`; and
- every other token is unchanged.

The whole-storage equality postcondition currently present on the write wrapper should be reviewed.
As learned from the clear wrapper, equality of an owning storage object may be stronger than the
required storage-identity frame condition and may accidentally constrain contents.

### Clear

`env_interaction_clear_page_directory` reproduces the original loop over `self.entries`. It:

- requires `old(self).inv()`;
- ensures `final(self).inv()`;
- preserves every permission pointer; and
- initializes every permission with zero.

An earlier whole-storage equality clause was removed because it was stronger than necessary.

### Read

`env_interaction_read_page_directory_entry` reproduces the original indexed read. Its current
contract:

- requires `self.inv()`;
- requires explicit lower and upper index bounds;
- requires the selected token to be initialized; and
- returns a value admitted by that token.

It intentionally does not guarantee equality with `expected()`, because the MMU may set the
accessed bit between Nanvix operations.

The current architectural validity predicate only checks that the page-size bit is clear:

```rust
value & 0x80 == 0
```

This is intentionally incomplete. Before generalizing the approach, confirm the exact x86 rules
for standard PDE reserved, ignored, software-available, physical-address, and feature-dependent
bits. The predicate should be strengthened without rejecting architecturally permitted fields.

## Permission Origin and Threading

The agreed ownership flow is:

```text
page-directory allocator -> PageDirectory::new -> PageDirectory.permissions
                           -> environment interaction
```

The executable constructor remains `PageDirectory::new(entries)`. The allocator must eventually
return a permission map alongside its executable storage. Verified construction will then supply it
with:

```rust
proof_with! { Tracked(permissions) }
PageDirectory::new(entries)
```

There are two backing sources:

1. BSS slots from `PAGE_TABLE_ALLOCATOR.alloc_as()` in `Vmem::new`;
2. runtime `KernelPage` storage used by `Vmem::clone`.

Their allocator specifications must transfer the permission map and retain no duplicate permission.
The existing BSS ghost base address is storage identity, not a replacement for memory permissions.

## Important Incomplete Work

Ordinary formatting and kernel compilation succeed, but Verus verification has not run because the
configured Verus binary is unavailable.

The next blockers are:

- determine which Verus machinery can soundly implement the abstract PDE and PTE tokens;
- thread allocator-originated tokens and physical-base facts through BSS and `KernelPage` paths;
- supply the proof-only page-table argument at every present and non-present PDE write;
- prove continued page-table lifetime separately from immediate write-time readiness;
- run targeted Verus verification and correct attribute-mode issues; and
- refine `valid_standard_pde` and `valid_pte` from the architecture definition.

## Generalizing to Remaining Interactions

### Page table

`PageTable<T>` now mirrors the page-directory token pattern and tracks virtual and physical base
facts. Allocator threading remains incomplete. Its interactions specify:

- clear: every permission becomes initialized with zero;
- read: returned value equals the selected permission value;
- write: selected permission becomes `value`, others remain unchanged;
- scans and iteration: observations correspond to the relevant permission values;
- bulk fill: exactly the target range changes;
- PTE validity: present, permission, caching, physical-target, software COW, and reserved bits.

### Generic volatile `Table<E>`

Relate the typed table pointer to raw `PointsTo<PteWord>` permissions. Reads must decode the stored
word; writes must store `entry.raw()`. Preserve volatility as executable behavior, while the
contract reasons about memory contents.

### x86_64 hardware tables

Store or thread permissions for PML4, PDPT, PD, and PT pages allocated from `PT_POOL`. Specify
64-bit architectural entry validity and the ownership transfer between active, detached, and free
list states. The direct zeroing path must use the same permission model as `write_entry`.

### CR0 and CR3

These interactions do not use `PointsTo`. Introduce trusted contracts over system-visible register
values:

- CR0 read returns the current control value;
- CR0 write installs the requested legal value;
- CR3 read returns the current root and caching controls;
- CR3 write installs an aligned, architecturally valid root and has the architectural TLB effect.

Do not introduce a full MMU state object until the environment model stage.

### TLB invalidation

At the trusted-contract stage, specify the architectural guarantee relied upon after `invlpg` or a
CR3 write. When an explicit MMU model is added, connect these functions to modeled cached
translations.

## Constraints to Preserve

- Do not modify either `hooks.rs`.
- Keep original interaction lines commented beside replacements.
- Put new wrappers for `x.rs` in `x.spec.rs`.
- Do not remove or rewrite imports that predated this work without necessity.
- Keep proof state erased from ordinary Rust builds.
- Do not pass `Tracked` or `Ghost` values as ordinary executable arguments.
- Do not introduce `MmuState` or another environment-owned state in the current trusted contracts.
- Do not use `&mut` as the memory-content model for hardware-shared paging entries.
- Derive permissions from allocation ownership and store them with the paging structure.

## Recommended Next Session

1. Choose Verus implementations for the already-defined PDE and PTE token semantics.
2. Thread allocator-originated tokens and physical-base facts through both storage paths.
3. Thread `Some(page_table)` for present PDE writes and `None` for non-present writes.
4. Add a separate ownership protocol for keeping referenced tables alive.
5. Run targeted Verus verification.
6. Refine the PDE and PTE architectural validity predicates.
7. Continue through volatile hardware tables, registers, and TLB operations.
