# Ongoing x86_64 Hardware Paging Interface Design

## Goal

Map the token and compatibility design used for `PageDirectory` and `PageTable` onto the x86_64
hardware hierarchy in `hwpt.rs`:

```text
PML4 -> PDPT -> PD -> PT
```

The runtime implementation should continue using physical addresses and volatile entry operations.
Proof-only state should describe entry knowledge, hierarchy validity, parent-child relationships,
and the lifecycle of pages allocated from `PT_POOL`.

## Page-Level Abstraction

Model complete hardware paging pages rather than isolated pointers:

```rust
enum HwPagingLevel {
    Pml4,
    Pdpt,
    Pd,
    Pt,
}

struct NanvixHwPageToken {
    physical_base: u64,
    level: HwPagingLevel,
    entries: Map<nat, NanvixHwEntryToken>,
}
```

A valid page token should establish:

- a 4 KiB-aligned physical base;
- exactly 512 entries;
- contiguous entry-pointer correspondence with the page;
- initialized entry tokens; and
- baselines valid for the page's assigned level.

The executable representation remains a physical `u64`. These tokens are proof-only.

## Minting from Raw Memory Authority

Custom hardware tokens originate from Verus raw-memory permissions, not from addresses alone.
Trusted proof functions perform one-way, linear conversion:

```text
PointsTo<u64> -> NanvixHwEntryToken
Map<nat, PointsTo<u64>> -> NanvixHwPageToken
```

The conversion consumes the source permissions. Its preconditions require:

- the complete entry domain `0..ENTRIES_PER_TABLE`;
- initialized raw permissions for hardware pages already containing readable entries;
- exact pointer-to-page offset correspondence;
- a page-aligned physical base; and
- entry values valid for the assigned hierarchy level.

Its postconditions preserve the pointer and current value as the token baseline, establish the page
level and physical base, and return a ready page token. No raw `PointsTo` permission remains
alongside the Nanvix token.

For zeroed fresh pages, the raw permissions carry zero and the minted token is immediately ready.
For reused pages, existing token authority must be recovered from the free pool, zeroed through the
same interaction model, and reassigned to the required level. Reuse must not mint a second token
from newly assumed authority while the old token still exists.

The current stage may assume that raw permissions are passed proof-only at construction boundaries.
Connecting those assumptions to the BSS allocator is separate allocator-verification work.

## Entry Tokens and Compatibility

Each entry token should expose only:

```text
ptr()
is_init()
expected()
admits(observed)
```

Compatibility is level-sensitive:

| Entry kind | MMU-managed fields |
|---|---|
| PML4E | Accessed |
| Non-leaf PDPTE | Accessed |
| Non-leaf PDE (`PS = 0`) | Accessed |
| Large-page PDPTE/PDE | Accessed and dirty |
| Leaf PTE | Accessed and dirty |

All stable fields must equal the Nanvix-established baseline. The compatibility relation should
therefore be parameterized by the entry level and leaf kind:

```rust
compatible_hw_entry(level, expected, actual)
```

Reads return admitted observations rather than persistent equality with `expected()`. Writes replace
the baseline but do not promise that accessed or dirty remains unchanged afterward.

## Architectural Validity

Define separate predicates:

```rust
valid_pml4e(value)
valid_pdpte(value)
valid_pde(value)
valid_pte(value)
valid_hw_entry(level, value)
```

They must distinguish:

- absent entries;
- present non-leaf entries;
- large-page leaves;
- ordinary 4 KiB leaves;
- level-specific reserved and feature-dependent bits;
- physical-address alignment; and
- legal flag combinations.

Boot-provided entries must be interpreted at their actual hierarchy level.

## Page Readiness

Define:

```rust
page.ready_for_mmu(level)
```

to require:

- the page invariant;
- the assigned level equals `level`;
- exactly 512 entries;
- every entry is initialized; and
- every baseline is valid for that level.

A zeroed page may initially be assigned any level because every zero entry is non-present.

## Parent-Child Correspondence

For entries that may reference another paging page, require exact correspondence:

```rust
present_nonleaf(level, value) == child.is_some()
```

For `Some(child)`:

```rust
child.ready_for_mmu(next_level(level))
child.physical_base() == entry_target_address(value)
```

The intended cases are:

- present PML4E -> `Some(PDPT)`;
- present non-leaf PDPTE -> `Some(PD)`;
- present non-leaf PDE -> `Some(PT)`;
- absent entry -> `None`;
- large-page PDPTE/PDE -> `None`;
- leaf PTE -> `None`.

Large-page and PTE entries target payload memory, not child paging structures.

## Terminal Interaction Contracts

### Read

Conceptually:

```rust
requires
    page.ready_for_mmu(level),
    0 <= index < ENTRIES_PER_TABLE,
ensures
    page.entries[index].admits(result),
```

### Write

Conceptually:

```rust
with
    child: Option<&NanvixHwPageToken>,
requires
    old(page).ready_for_mmu(level),
    0 <= index < ENTRIES_PER_TABLE,
    valid_hw_entry(level, value),
    valid_child_target(level, value, child),
ensures
    final(page).ready_for_mmu(level),
    final(page).entries[index].ptr()
        == old(page).entries[index].ptr(),
    final(page).entries[index].expected() == value,
    all other entry tokens are unchanged,
```

The executable wrapper may continue taking only `ptr` and `value`; page identity, level, index, and
child information may be proof-only arguments.

### Zero

Zeroing a page must establish:

```rust
forall|i| page.entries[i].expected() == 0
```

A reused page becomes ready for level assignment only after all 512 entries have been zeroed.

## Pool and Free-List Lifecycle

Unlike the owning 32-bit structures, hardware paging pages are recycled. Model transitions:

```text
free or unallocated
    -> allocated and zeroed
    -> assigned a level
    -> linked into a hierarchy
    -> detached and unreachable
    -> free
```

`alloc_pt_page()` should return:

- the executable physical address;
- the unique page token transferred from fresh allocation or the free pool; and
- all entry baselines initialized to zero.

The reused-page zeroing loop must update every token baseline before allocation completes.

`free_pt_page()` should consume the page token and require that the page is no longer:

- referenced by a present parent;
- installed as an active root; or
- reachable through a hierarchy the MMU may walk.

The allocator and free list must not retain duplicate usable authority.

## Ownership Partition

Hardware paging pages do not all have the same owner.

### Per-address-space ownership

Each non-kernel `Vmem` has a distinct PML4, private PDPT, and process-private PD/PT pages. Store
their tokens directly in that `Vmem`:

```rust
#[cfg(verus_keep_ghost_body)]
hw_pages: Tracked<Map<u64, NanvixHwPageToken>>
```

The map key is the physical page address. Its invariant requires every token to match its key and
belong to the hierarchy rooted at `hw_pml4`. This is unique state and does not need shared interior
mutability.

`Vmem::clone()` receives newly allocated private tokens rather than cloning the source map.
`Vmem::drop()` must detach and return every private page token before the executable page is added
to `PT_FREELIST`.

### Shared manager ownership

Two categories cannot be copied into every `Vmem`:

- boot PML4/PDPT/PD pages shared by all address spaces; and
- unallocated or free `PT_POOL` page authority.

Keep these in one manager-level proof state. A process `Vmem` receives a proof-only witness for a
shared boot child when publishing `PDPT[0]`; it does not acquire ownership and must never free that
page. Allocation transfers a token from manager state into `Vmem::hw_pages`; reclamation transfers
it back.

The root kernel `Vmem`, whose executable `hw_pml4` is zero, uses the boot hierarchy and therefore
does not own a private root token.

### Shared-handle guidance

Do not put all paging tokens in one shared container merely to simplify lookup. Prefer direct
per-`Vmem` ownership for private pages. If manager-level boot/pool state needs duplicable proof-only
access, use an explicitly specified Verus shared container; do not introduce executable
`Rc<RefCell<_>>` or change hardware-paging runtime APIs.

Single-threaded execution avoids concurrent kernel mutation, but it does not permit duplicate
linear tokens. Shared boot and pool authority still require one logical owner.

## Mapping Existing Operations

### `ensure_table()`

For an absent parent entry:

1. Allocate a zeroed child.
2. Assign the next hierarchy level.
3. Establish child readiness.
4. Write the present parent entry with `Some(child)`.

For an existing entry, its admitted observation identifies the child. Updating the user bit keeps
the same child-target witness.

### `split_2m_entry()`

1. Read and validate the large-page PDE.
2. Allocate and assign a PT page.
3. Initialize all 512 leaf PTEs.
4. Establish `pt.ready_for_mmu(Pt)`.
5. Replace the large-page PDE with a non-leaf PDE and `Some(pt)`.

This captures initialization-before-parent-publication.

### `create_user_pml4()`

- Allocate zeroed PML4 and PDPT pages.
- Assign their hierarchy levels.
- Write `PDPT[0]` with a borrowed boot-PD witness.
- Write `PML4[0]` with the new PDPT witness.
- Return the executable PML4 address and transfer the private page-token map into the new `Vmem`.

### `map_in()`

Borrow the corresponding token from the owning `Vmem::hw_pages` map through every private hierarchy
step. Allocation obtains a fresh or recycled token from manager state and inserts it into that map.
The final PTE write has no child witness because it maps payload memory.

### `unmap_in()` and `protect_user()`

Reads return admitted observations. Leaf writes replace the baseline while permitting later
accessed and dirty updates. TLB invalidation remains a separate interaction.

### `destroy_user_pml4()`

Require that the PML4 is not active and is no longer MMU-reachable. Recover child tokens while
detaching the hierarchy, remove them from `Vmem::hw_pages`, then transfer them to manager free-pool
state as `free_pt_page()` updates the executable free list.

## Boot-Owned Hierarchy

The boot PML4, PDPT, and `BOOT_PD0` do not originate from `PT_POOL`. `init()` needs trusted
proof-only inputs establishing:

- the CR3 root matches the boot PML4 token;
- traversed entries are initialized and valid;
- entry targets match their child tokens; and
- the relevant pages are identity-accessible.

These tokens should enter once through the boot-environment boundary rather than being invented at
individual reads. They remain manager-owned and are only borrowed as child witnesses by private
address spaces.

## Resulting File Responsibilities

### `hwpt.rs`

Retain:

- runtime globals and physical-address APIs;
- volatile entry operations;
- allocation, traversal, mapping, and destruction algorithms;
- proof-only state gated from ordinary builds; and
- proof-only arguments supplied without changing runtime APIs or behavior.

### `hwpt.spec.rs`

Contain:

1. hierarchy-level and page-lifecycle definitions;
2. abstract page and entry tokens;
3. level-specific validity predicates;
4. accessed/dirty compatibility predicates;
5. page-readiness predicates;
6. exact parent-child target correspondence;
7. read, write, and zero interaction contracts;
8. allocator and free-list transition contracts;
9. boot-hierarchy assumptions; and
10. separate TLB and CR3 interaction wrappers.

### `Vmem`

Own the token map for its private hardware hierarchy. Pass proof-only references to that map into
`hwpt` calls while preserving existing executable `u64` arguments. Do not store manager-owned boot
or free-pool tokens in each `Vmem`.

## Main Difference from the 32-bit Design

x86_64 requires a level-aware page abstraction. Entry interpretation changes between non-leaf
pointers, large-page leaves, and ordinary PTE leaves. The static pool also requires explicit
lifecycle tracking because hardware paging pages are detached, reclaimed, zeroed, and reused.

## Remaining Work

The current implementation establishes the level-aware token vocabulary, trusted raw-permission
minting functions, and contracts only `read_entry()` and `write_entry()`. The following work
remains:

1. **Verify token minting.**
   - Confirm that Verus accepts the proof-only immutable and mutable page-token references used by
     the read and write contracts.
   - Verify that minting consumes complete `PointsTo` maps and establishes pointer, level, physical
     base, and baseline correspondence.
   - Run targeted Verus verification when the configured Verus binary is available.

2. **Add per-`Vmem` private ownership.**
   - Add an erased tracked map for private PML4/PDPT/PD/PT tokens.
   - Ensure cloning allocates a distinct map rather than cloning token authority.
   - Supply the correct private page token, level, and optional child token at every `read_entry()`
     and `write_entry()` call.
   - Preserve unique authority while discovering and retaining child tokens during walks.
   - Keep all executable function signatures and control flow unchanged.

3. **Add manager boot/pool ownership.**
   - Retain boot PML4/PDPT/PD tokens in one manager-level state.
   - Retain unallocated and free `PT_POOL` authority in that state.
   - Transfer tokens between manager state and per-`Vmem` maps at allocation and reclamation.
   - Provide controlled boot-child witnesses without transferring or duplicating ownership.

4. **Specify allocation and reuse.**
   - Make `alloc_pt_page()` return the unique token for the selected pool page.
   - Connect BSS addresses to physical addresses using the identity-mapping assumption.
   - Specify the reused-page zeroing loop so all 512 baselines become zero before allocation
     completes.
   - Assign a hierarchy level only after the page is initialized.

5. **Specify detachment and reclamation.**
   - Define page lifecycle states for allocated, linked, detached, and free pages.
   - Require `free_pt_page()` to consume a detached, unreachable page token.
   - Prove that a freed page is not referenced by a present parent and is not an active CR3 root.
   - Prevent duplicate authority between active pages, the free list, and the unallocated pool.

6. **Import the boot hierarchy.**
   - Introduce trusted boot tokens for the CR3 PML4, its PDPT, and `BOOT_PD0`.
   - Validate the entries traversed by `init()` and connect their encoded targets to child tokens.
   - Keep boot-owned pages distinct from pages allocated from `PT_POOL`.

7. **Strengthen architectural validity.**
   - Add complete reserved-bit and feature-dependent rules for PML4E, PDPTE, PDE, and PTE values.
   - Account for the implemented physical-address width rather than relying only on address masks.
   - Specify NX, caching, software-available, PAT, 1 GiB page, and 2 MiB page rules.
   - Confirm the architectural conditions under which accessed and dirty may change, especially for
     non-present entries and large pages.

8. **Specify hierarchy-changing operations.**
   - Specify initialization-before-publication in `ensure_table()` and `split_2m_entry()`.
   - Specify root construction in `create_user_pml4()`.
   - Specify token flow through `map_in()`, `unmap_in()`, and `protect_user()`.
   - Specify token recovery and consumption in `destroy_user_pml4()`.

9. **Specify translation-control effects separately.**
   - Connect `invlpg` to the relied-upon invalidation guarantee.
   - Specify CR3 root reads and writes after root-token ownership and lifetime are available.
   - Do not fold TLB or active-root effects into paging-memory write contracts.

10. **Review the trusted boundary.**
   - Decide whether `read_entry()` and `write_entry()` should remain the `external_body` boundary or
     whether trust should move down to the volatile wrappers after token threading is implemented.
   - Keep only one trusted contract per terminal operation and avoid duplicate guarantees.
