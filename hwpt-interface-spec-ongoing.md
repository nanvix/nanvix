# Ongoing x86_64 Hardware Paging Interface Work

## Purpose

This file tracks only unfinished work for the x86_64 PML4/PDPT/PD/PT interface. The completed design
and cross-architecture rules are recorded in
`nanvix-virt-mem-environment-interface-specification.md`.

## Completed

### Token model

- `NanvixHwEntryToken` records entry identity and the Nanvix-established baseline.
- `NanvixHwPageToken` records physical base, hierarchy level, and exactly 512 entry tokens.
- Compatibility distinguishes non-leaf accessed updates from leaf accessed/dirty updates.
- Parent writes require exact correspondence between a present non-leaf encoding and a ready child
  token at the next level.

### Authority origin and ownership

- Boot and pool raw permissions are consumed once by `mint_hwpt_manager()`.
- The shared manager owns boot pages and available pool pages.
- Each non-kernel `Vmem` owns only its private hierarchy map.
- Shared handles provide access to one manager invariant without duplicating linear page tokens.
- Allocation transfers a zeroed token from manager state to the private owner map.
- Reclamation removes a detached token from the owner map and returns it to manager state.

### Threading

Tracked authority reaches all hardware-entry reads and writes through centralized owned-entry
helpers. Token flow is implemented through:

- `ensure_table()`;
- `split_2m_entry()`;
- `create_user_pml4()`;
- mapping and unmapping;
- permission protection; and
- hierarchy destruction.

Private user paths are constrained away from the shared low-memory edge. The explicit exception in
the private hierarchy invariant permits `PDPT[0]` to reference manager-owned boot `PD0`.

### Teardown

`destroy_user_pml4()` clears parent entries before returning child tokens. This makes the
detach-before-free order executable as well as proof-visible. It is the only intentional runtime
behavior change introduced by HWPT ownership threading.

### Validation

- ordinary x86_64 kernel compilation passes;
- pre-commit checks passed for the committed HWPT work; and
- full x86_64 Verus progresses into unrelated slab-proof omissions.

## Current Trusted Boundaries

The current implementation deliberately trusts more than the eventual ideal:

- raw `PointsTo<u64>` conversion into entry/page tokens;
- one-time boot/pool manager import;
- mutable-static accessors for initialization state and boot addresses;
- allocation transfer from the executable static pool/free list;
- `free_pt_page()` correspondence with the executable free list;
- boot hierarchy discovery in `init()`;
- terminal hardware-entry reads and writes;
- `invlpg`; and
- the CR3 read's architectural validity guarantee and boot-root discovery.

The isolated CR3 read now guarantees that the observed value has a non-null, representable PML4
address. `cr3_root_address()` states the exact masking performed by `init()`, and
`valid_cr3_root()` relates such a value to the existing ready `NanvixHwPageToken` abstraction.
This deliberately does not introduce a CR3 state token or claim that later context-switch writes
are already connected to the owning `Vmem`.

The entry read/write contracts express baseline compatibility and child readiness. The raw
`env_interaction_*` wrappers beneath them preserve the original volatile or assembly operation but
do not independently duplicate those semantic contracts.

## Remaining Work

### 1. Complete architectural validity

Strengthen `valid_pml4e()`, `valid_pdpte()`, `valid_pde()`, and `valid_pte()` to cover:

- implemented physical-address width;
- reserved and ignored fields;
- NX and feature enablement;
- PAT and caching encodings;
- software-available bits;
- 1 GiB and 2 MiB alignment and legality; and
- precise accessed/dirty behavior for present, absent, leaf, and non-leaf entries.

`valid_pte()` currently accepts every `u64`; the other predicates are structural rather than
complete architectural checks.

### 2. Tie manager state to executable allocator state

Prove or narrowly specify:

- `PT_POOL_NEXT` corresponds to unallocated manager tokens;
- `PT_FREELIST[..PT_FREELIST_LEN]` corresponds exactly to available reclaimed tokens;
- allocation selects the same address whose token is transferred;
- reused-page zeroing updates all 512 token baselines before reassignment; and
- no address appears simultaneously as live, free, and unallocated authority.

The current `alloc_owned_pt_page()` is an `external_body` transfer boundary and does not verify this
correspondence internally.

### 3. Strengthen boot import

Connect the imported manager state to:

- the CR3 root observed by `init()`;
- the discovered boot PDPT and `BOOT_PD0`;
- actual hierarchy levels and encoded parent-child targets; and
- identity accessibility of every imported page.

The one-time minting location is correct, but its postcondition exposes little information through
the shared handle and `init()` remains trusted.

### 4. Finish lifetime and active-root reasoning

Extend detachment beyond private-map references:

- prove the hierarchy being destroyed is not active in CR3;
- prove no manager-owned or external root reaches a reclaimed private page;
- preserve the hierarchy invariant after every parent clear and token return; and
- show that the private token map is empty after destruction.

The current order prevents private-parent dangling edges, but active-root and external reachability
remain trusted preconditions rather than a complete environment model.

### 5. Specify translation control

Complete semantic contracts for:

- context-switch root installation and replacement;
- `invlpg`; and
- the relied-upon TLB invalidation effects.

Do not fold these guarantees into a paging-entry write contract.

### 6. Reduce broad `external_body` use

Verify non-terminal ownership transfers and hierarchy algorithms where supported. Keep trust at:

- true authority-import boundaries;
- terminal volatile or assembly interactions; and
- narrowly documented unsupported runtime primitives.

External type specifications for containers or mutable statics should not become permanent
substitutes for ownership proofs.

### 7. Complete x86_64 verification

Resolve or separately isolate the unrelated x86_64 slab match omissions, then run the complete
target verification. Re-audit every direct entry read, write, zero, allocation, and free operation
afterward to ensure there is exactly one token path and no bypass.

## Non-Goals for This Stage

- Do not introduce a full executable MMU model.
- Do not duplicate boot or pool tokens into each address space.
- Do not replace proof-only sharing with runtime `Rc<RefCell<_>>`.
- Do not treat single-threaded execution as permission duplication.
- Do not change runtime signatures merely to carry proof state.
