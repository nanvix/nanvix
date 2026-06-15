# Caller Analysis: `mm::virt::vmem` (`Vmem`)

## Script Output
See: `./find_callers_lsp_output.md` (raw `find_callers_lsp.py` output).

- **Source file:** `src/kernel/src/mm/virt/vmem.rs`
- **Crate:** `kernel` (intra-crate only; no external crate depends on it)
- **Module summary:** 35 exec functions — 26 pub/trait-pub, 9 private, 1 type (`Vmem`)
- **Implicit caller:** `impl Drop for Vmem` (compiler-dispatched on scope exit)

All callers are *inside the `kernel` crate*. The dominant clients are:
- `mm/virt/manager.rs` (`VirtMemoryManager`) — fork/CoW linking, demand paging,
  page-fault resolution, unmapping, permission control.
- `pm/process/manager/mod.rs` — process creation, argv copy, CoW pre-resolution,
  cross-process copy, address translation, kernel-page permission control.
- `pm/process/state/mod.rs` — thin pass-through for user/kernel copy helpers.
- `pm/kcall/*` and `pm/process/manager/mod.rs` — `is_user_addr` / `is_user_region`
  argument validation for syscalls.
- `mm/elf.rs` — ELF segment loading via `copy_to_user_unaligned_unchecked`.
- `mm/kernel_vas.rs` — `map_kpage` during kernel VAS construction.

## Trait Obligations
- Trait: `Drop` for `Vmem` — on scope exit the address space must release every
  resource it owns: user frames (refcount decrement / free), kernel page-table
  allocations, and the page-directory page. Callers (`VirtMemoryManager::new_vmem`,
  process teardown) rely on simply dropping a `Vmem` to reclaim all backing
  memory with no leak and no double-free. The View must therefore model
  *ownership* of the mapped frame/page-table set, not just the address→PTE map.

## Caller Expectations

### `Vmem::new(kernel_pages, kernel_page_tables) -> Result<Self, Error>` (L103)
- **Caller:** `VirtMemoryManager::new` (manager.rs L218), immediately followed by
  `root.load()`.
- Callers assume: on `Ok`, a fully-formed root address space whose kernel half is
  populated from the supplied kernel pages/tables and whose user half is empty;
  the page directory is clean/consistent and loadable. Consumes both input lists.
- Callers don't care about: linked-list layout, allocation order, identity-mapped
  vs. listed kernel pages.

### `Vmem::clone(from: &Vmem, pgdir_page: KernelPage) -> Result<Vmem, Error>` (L177)
- **Caller:** `VirtMemoryManager::new_vmem` (manager.rs L250); the supplied
  `pgdir_page` is a freshly allocated kernel frame.
- Callers assume: on `Ok`, a new independent `Vmem` sharing the source's *kernel*
  mappings but with its own page directory (the new `pgdir().physical_address()`
  differs from the source — see L254/L255). User mappings are NOT copied here;
  user sharing is done separately via `for_each_user_mapping`/`map`/`mark_cow`.
- Callers don't care about: how kernel page tables are re-referenced internally.

### `Vmem::load(&self) -> Result<(), Error>` (L226)
- **Callers:** `VirtMemoryManager::new` (manager.rs L221); internally `map_kpage`
  (L319). Also conceptually paired with `pgdir().physical_address()` used to set
  CR3 at context switch (process/manager L269).
- Callers assume: after `Ok`, this address space is the active one (CR3 points at
  its page directory) and subsequent memory accesses use its translations.
- Callers don't care about: the exact CR3-write mechanism.

### `Vmem::pgdir(&self) -> &PageDirectory<...>` (L233)
- **Callers:** manager.rs L254/L255 (trace), process/manager L269
  (`pgdir().physical_address()` → CR3 value).
- Callers assume: returns the live page directory; its `physical_address()` is the
  stable physical base used to program the MMU. Pure read-only borrow, no mutation.
- Callers don't care about: page-directory storage representation.

### `Vmem::map_kpage(&mut self, kpage, vaddr) -> Result<(), Error>` (L250)
- **Caller:** `kernel_vas.rs` L151 (building the kernel VAS).
- Callers assume: on `Ok`, the kernel page `kpage` is mapped at page-aligned
  `vaddr` in kernel space; the page table is allocated on demand if needed; the
  `Vmem` now owns `kpage`.
- Callers don't care about: whether a new kernel page table was allocated.

### `Vmem::map(&mut self, uframe, vaddr, access) -> Result<(), Error>` (L366)
- **Callers:** `link_one_user_page` (manager.rs L386) during fork; `alloc_upages`
  (manager.rs L673) during demand allocation.
- Callers assume: `vaddr` must lie in user space (else `BadAddress`); on `Ok` the
  user frame is mapped at `vaddr` with the requested `access`, the `Vmem` takes
  ownership of `uframe`, and a backing user page table is created if absent. On
  `Err`, `uframe` is dropped (its refcount released) and no mapping persists —
  callers rely on this to keep refcounts balanced on the fork/alloc rollback paths.
- Callers don't care about: page-table list ordering / lookup caching.

### `Vmem::is_user_page_mapped(&self, vaddr) -> Result<bool, Error>` (L443)
- **Callers:** `rollback_linked_pages` (manager.rs L468), `alloc_upages` range
  pre-check (manager.rs L634).
- Callers assume: `Ok(true)` iff a present user page currently backs `vaddr`;
  `Ok(false)` if unmapped (and, per impl, if `vaddr` is not in user space). Pure
  query — does not mutate. Used both to skip already-unmapped pages and to reject
  ranges that overlap existing mappings (`ResourceBusy`).
- Callers don't care about: the PTE contents, only presence.

### `Vmem::is_user_addr(virt_addr) -> bool` (L453)
- **Callers (7 ext):** `create_thread.rs` L82/L111, `set_thread_data_area.rs` L66,
  `process/manager/mod.rs` L260/L321/L745/L759; plus heavy internal use.
- Callers assume: pure, total, side-effect-free predicate — `true` iff the address
  is within the user-space window. Used as a guard before syscall arguments are
  trusted (`debug_assert!` and explicit `BadAddress` rejection).
- Callers don't care about: implementation constants — only the user/kernel split.

### `Vmem::is_user_region(start, size) -> bool` (L471)
- **Callers (6 ext):** `create_thread.rs` L62/L89, `duplicate.rs` L66,
  `process/manager/mod.rs` L259/L750, `manager.rs` L624; plus internal copy paths.
- Callers assume: pure predicate — `true` iff the entire `[start, start+size)`
  range lies in user space, with correct handling of zero size and end-overflow
  (must NOT report a wrapping/overflowing region as user). Foundational guard for
  every copy/validation path.
- Callers don't care about: how the bound check is computed.

### `Vmem::is_physical_region(start, size) -> bool` (L530)
- **External callers: 0.** Internal only — `copy_to_user_unaligned_unchecked`
  (L1287, L1318) validates that resolved physical frame ranges lie within physical
  memory before a physical-alias copy.
- Callers assume: pure predicate guarding raw physical-memory copies; `true` iff
  the whole physical range is in valid physical memory. Not dead code — it is a
  safety gate on the unchecked copy path. Treat as part of the public surface
  (it is `pub`) but spec it from the internal safety-contract perspective.

### `Vmem::try_find_user_pte(&self, vaddr) -> Result<Option<PageTableEntry>, Error>` (L736)
- **Caller (pub(crate)):** `link_user_pages` (manager.rs L312) to detect whether
  the child already has a mapping; internally `resolve_cow_at` (L947).
- Callers assume: `Ok(Some(pte))` is a *decoded copy* of the backing PTE when the
  page is present; `Ok(None)` when page table or page is absent. Read-only — does
  not mutate. Callers read `frame_number`, `flags().is_writable()`, `is_cow()`.
- Callers don't care about: the page-walk mechanics.

### `Vmem::for_each_user_mapping(&self, f) -> Result<(), Error>` (L777)
- **Callers:** `link_user_pages` (manager.rs L311) and `rollback_linked_pages`
  (manager.rs L464).
- Callers assume: `f` is invoked once per *present* user page with a page-aligned
  user `vaddr` and a decoded PTE copy; an `Err` from `f` short-circuits and
  propagates. Crucially, callers tolerate (and design around) the iteration
  revisiting entries and being safe to call while concurrently mutating CoW bits
  via `mark`/`unmap` between chunks (the chunked buffer pattern). Iteration order
  is "internal list order" — callers do not depend on a specific order, only on
  complete coverage of present mappings.
- Callers don't care about: the page-table list structure or traversal order.

### `Vmem::mark_user_page_cow(&mut self, vaddr) -> Result<(), Error>` (L827)
- **Callers:** `link_one_user_page` (manager.rs L395 parent, L405 child).
- Callers assume: precondition — the page is present AND writable (callers ensure
  this; re-marking an already-CoW page is *avoided* because it would fail). On
  `Ok`, the PTE is read-only with the AVL CoW bit set. On `Err`, callers run
  explicit rollback (`unmap`/`unmark`), relying on no partial half-marked state
  being silently left that breaks the inverse.
- Callers don't care about: which AVL bit encodes CoW.

### `Vmem::unmark_user_page_cow(&mut self, vaddr) -> Result<(), Error>` (L857)
- **Callers:** `link_one_user_page` rollback (manager.rs L409); internally
  `resolve_cow_at` (L966).
- Callers assume: exact inverse of `mark_user_page_cow` — restores the writable
  bit and clears the CoW bit on a present page. Used to undo a just-applied mark.
- Callers don't care about: bit layout.

### `Vmem::resolve_cow_at(&mut self, vaddr) -> Result<bool, Error>` (L936)
- **Caller:** page-fault path `attempt_cow` (manager.rs L548); internally
  `resolve_cow_for_region` (L1039).
- Callers assume: `Ok(true)` ⇒ a CoW mapping existed at `vaddr` and was resolved
  (private frame allocated, contents copied, PTE repointed + writable, old shared
  reference dropped). `Ok(false)` ⇒ page absent or not CoW (caller then forwards
  the fault to the registered handler). `Err` ⇒ resolution failed (e.g. OOM) with
  no corruption. Idempotency: a second call returns `Ok(false)`.
- Callers don't care about: frame allocation/copy details — only the boolean
  "was a CoW resolved" and that on success the page is now privately writable.

### `Vmem::resolve_cow_for_region(&mut self, addr, size) -> Result<(), Error>` (L1008)
- **Caller:** `process/manager/mod.rs` L2171, immediately before
  `copy_user_to_user`; internally `copy_to_user_unaligned_unchecked` (L1262).
- Callers assume: after `Ok`, *every* CoW page overlapping `[addr, addr+size)` is
  privately owned and writable, so a subsequent kernel-side write via the physical
  alias cannot silently mutate a frame still shared with another address space.
  Zero size is a no-op; pages outside user space / non-CoW are left untouched.
- Callers don't care about: per-page iteration — only the post-condition over the
  whole range.

### `Vmem::user_vaddr_to_paddr(&self, vaddr) -> Result<usize, Error>` (L1065)
- **Caller:** `process/manager/mod.rs` L2202 (under `stdio` feature).
- Callers assume: on `Ok`, the guest physical address for a *present* user page,
  including the intra-page offset of `vaddr`. Read-only walk. `Err` if unmapped /
  not user.
- Callers don't care about: walk internals.

### `Vmem::copy_from_user_unaligned(&self, dst, src, size) -> Result<(), Error>` (L1098)
- **Caller:** `ProcessState::copy_from_user_unaligned` pass-through (state L347).
- Callers assume: copies `size` bytes from user `src` into kernel `dst`, no
  alignment requirement. Errors: `InvalidArgument` if `size==0`; `BadAddress` if
  `src` not entirely user OR `dst` not entirely kernel. On `Ok`, `dst` holds the
  user data; on `Err`, partial-copy state is unspecified but no safety violation.
- Callers don't care about: per-page slicing.

### `Vmem::copy_to_user_unaligned_unchecked(&mut self, dst, src, size, dry_run) -> Result<(), Error>` (L1210)
- **Caller:** `elf.rs` L362 (ELF segment load; called once with `dry_run=true`
  then once with `false`); internally from `copy_to_user_unaligned`.
- Callers assume: with `dry_run=true`, validates the whole operation
  (user/kernel/physical-region checks) WITHOUT writing, so a later real run with
  the same args cannot fail on validation. With `dry_run=false`, performs the
  physical-alias copy and **panics** on a mid-copy error (callers accept this only
  after a successful dry run). Errors: `InvalidArgument` (size 0), `BadAddress`
  (src not kernel / dst not user / either physical range out of bounds).
- Callers don't care about: physical aliasing mechanics — only the dry-run /
  real-run contract.

### `Vmem::copy_to_user_unaligned(&mut self, dst, src, size) -> Result<(), Error>` (L1367)
- **Callers:** `ProcessState` pass-through (state L356); argv/NUL writes during
  process creation (`process/manager/mod.rs` L480, L484).
- Callers assume: a *checked* kernel→user copy — performs an internal dry run
  first, so either the whole copy succeeds or it returns `Err` having written
  nothing observable (no panic). Same address-space requirements as the unchecked
  variant. This all-or-nothing guarantee is what lets callers use it on
  untrusted destination addresses safely.
- Callers don't care about: the dry-run-then-copy implementation.

### `Vmem::copy_user_to_user(src_vmem, src, dst_vmem, dst, size) -> Result<(), Error>` (L1409)
- **Caller:** `process/manager/mod.rs` L2178 (after `resolve_cow_for_region` on
  the destination).
- Callers assume: copies `size` bytes from `src_vmem`'s user space to `dst_vmem`'s
  user space page-by-page via physical frames, bypassing kernel space. Both ranges
  must be user-space and mapped. Errors: `InvalidArgument` (size 0),
  `BadAddress` (either range not user), `NoSuchEntry` (a page unmapped). Callers
  are responsible for CoW pre-resolution on the destination before calling.
- Callers don't care about: the two-vmem frame-walk details.

### `Vmem::memset(&mut self, dst, value) -> Result<(), Error>` (L1508)
- **Caller:** `alloc_upages` (manager.rs L679) to zero a freshly mapped page.
- Callers assume: fills the page at page-aligned user `dst` with `value`; `dst`
  must be a present user page. On `Ok`, the whole page holds `value`.
- Callers don't care about: how the write reaches the frame (physical alias).

### `Vmem::unmap(&mut self, vaddr) -> Result<Option<UserFrame>, Error>` (L1538)
- **Callers:** fork rollback in `link_one_user_page` (manager.rs L396, L416),
  `rollback_linked_pages` (L489), `try_unmap_upage` (L573).
- Callers assume: `Ok(Some(frame))` ⇒ a present page was removed and its
  `UserFrame` handed back (dropping it frees/derefs the physical frame — callers
  rely on this to reclaim memory). `Ok(None)` ⇒ page was absent, treated as a
  benign no-op (suitable for cleaning up lazily-allocated regions). `vaddr` must
  be user space. Errors are logged but rollback callers continue best-effort.
- Callers don't care about: page-table list maintenance after unmap.

### `Vmem::uctrl(&mut self, vaddr, access) -> Result<(), Error>` (L1619)
- **Caller:** `ctrl_upage` (manager.rs L739).
- Callers assume: changes the access permissions of the present user page at
  `vaddr` to `access`. On `Ok`, future accesses honor the new permission.
- Callers don't care about: PTE flag encoding.

### `Vmem::kctrl(&mut self, vaddr, access, dry_run) -> Result<(), Error>` (L1690)
- **Caller:** `process/manager/mod.rs` L2667/L2673/L2679 — uses the same
  dry-run-then-commit pattern (commit, validate, commit).
- Callers assume: `dry_run=true` validates that the permission change on a kernel
  page would succeed (no PTE mutation); `dry_run=false` applies it. `vaddr` must be
  kernel space (else `BadAddress`). Errors: `TryAgain` (PDE read), `NoSuchEntry`
  (page table / PTE absent). The dry-run guarantees a subsequent real call with the
  same args won't fail validation — callers depend on this for atomic multi-page
  permission updates.
- Callers don't care about: kernel page-table lookup/caching internals.

## Internal Call Graph (private helpers in scope's support)
- `replace_user_page_cow_frame` ← `resolve_cow_at`
- `find_user_frame` ← `user_vaddr_to_paddr`, `copy_from_user_unaligned`,
  `copy_to_user_unaligned_unchecked`, `copy_user_to_user` (×2), `memset`
- `try_find_user_frame` ← `is_user_page_mapped`, `unmap`
- `lookup_user_page_table` ← `map`, `mark_user_page_cow`, `unmark_user_page_cow`,
  `replace_user_page_cow_frame`, `unmap`, `uctrl`
- `is_kernel_addr` ← `is_kernel_region`, `kctrl`
- `is_kernel_region` ← `copy_from_user_unaligned`, `copy_to_user_unaligned_unchecked`
- `allocate_user_page_table` ← `map`
- `allocate_kernel_page_table` ← `map_kpage`
- `lookup_kernel_page_table` ← `kctrl`

These reveal three families the View must model: (1) the **user
address→PTE/frame** map (lookup/alloc user page tables, find user frame),
(2) the **kernel address→PTE** map (kernel page tables, kctrl), and (3) the
**CoW state** carried in PTEs (mark/unmark/replace/resolve).

## Abstract Resource
A `Vmem` is an **owned virtual address space**: a mapping from page-aligned
virtual addresses to backing physical frames plus per-page access/CoW state,
split into a *kernel half* (shared across address spaces, mapped via kernel page
tables/pages) and a *user half* (per-process, owning its `UserFrame`s and user
page tables). It is the unit the kernel loads into the MMU (via its page
directory's physical address → CR3) and the unit of fork-time copy-on-write
sharing.

## Key Invariants (caller perspective)
- **User/kernel partition is total and stable:** every virtual address is
  classified user xor kernel; `is_user_addr` / `is_user_region` /
  `is_physical_region` are pure predicates that all guard paths trust. Region
  predicates must reject zero-size and overflow/wrapping ranges correctly.
- **Ownership balance (no leak / no double-free):** `map`/`map_kpage` take
  ownership of the supplied frame/page; `unmap` returns it; `map` on error
  releases the frame it was given; `Drop` releases everything. Fork rollback
  depends on these being exactly balanced.
- **CoW round-trip:** `mark_user_page_cow` (requires present+writable) and
  `unmark_user_page_cow` are inverses; `resolve_cow_at` privatizes a CoW page and
  is idempotent (`true` once, then `false`); after `resolve_cow_for_region` no CoW
  page remains in the range.
- **Presence semantics of queries:** `is_user_page_mapped`, `try_find_user_pte`,
  `user_vaddr_to_paddr`, and `for_each_user_mapping` observe only *present* user
  mappings and never mutate; `unmap` of an absent page is `Ok(None)`.
- **Dry-run ⇒ commit safety:** a successful `dry_run` of
  `copy_to_user_unaligned_unchecked` / `kctrl` guarantees the matching real run
  passes validation; `copy_to_user_unaligned` exposes this as an all-or-nothing
  (no-panic) checked copy.
- **Address-space activation:** after `load`, this `Vmem`'s page directory is the
  active translation context; `pgdir().physical_address()` is the value programmed
  into CR3, so it must stay consistent with the live mappings.

## Pre-existing Specs (from upstream verification)
- `src/kernel/src/mm/virt/vmem.spec.rs` exists but is **empty** (`verus! { }`).
- A `vmem.proof.rs` is `include!`d but no `#[verus_spec]` annotations or `View`
  type are present on any in-scope function (`grep` found none).
- **View type:** does not yet exist.
- **Assessment:** clean slate — no upstream bias. Design the `View` from the
  abstract resource above (a partitioned address→(frame, perms, cow) map with
  ownership), not by mirroring the four `LinkedList`/`Rc<RefCell<...>>` fields.
