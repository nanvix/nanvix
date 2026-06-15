# Caller Analysis: `mm::virt::manager` (`VirtMemoryManager`)

## Script Output

Raw LSP caller-finding output: `/tmp/caller_script_out.txt` (regenerated via
`python scripts/find_callers_lsp.py src/kernel/src/mm/virt/manager.rs --project-dir .`).

Summary from the script (rust-analyzer semantic index, intra-crate `kernel`):

| Category | Count |
|----------|------:|
| Total exec functions | 16 |
| Public / trait-pub | 12 |
| Private | 4 |
| Types | 1 (`VirtMemoryManager`, 62 external refs) |

`VirtMemoryManager` is a **zero-sized singleton** (`struct VirtMemoryManager` with
unit body `Self`). All state it "manages" lives in the global
`PhysMemoryManager` and in the `Vmem` arguments callers pass in; the manager
itself is a stateless façade obtained via `VirtMemoryManager::get_mut()`.

### Scope

In scope for top-level `#[verus_spec(...)]` (caller-before-callee), exactly:

| Function | Line | External callers |
|----------|-----:|-----------------:|
| `new_vmem` | 238 | 2 |
| `link_user_pages` | 295 | 1 |
| `try_resolve_cow_fault` | 526 | 1 |
| `try_unmap_upage` | 567 | 5 |
| `alloc_upages` | 595 | 5 |
| `ctrl_upage` | 733 | 2 |
| `alloc_kpage` | 755 | 3 |
| `alloc_kpages` | 783 | 1 |
| `load_elf` | 816 | 1 |

Out of scope (not to be modified): `init`, `get`, `get_mut`, and the private
helpers `new`, `link_one_user_page`, `rollback_linked_pages`,
`make_uninitialized_array`.

### Notes on the script results

- **`get` has 0 external callers** — only `get_mut` (the `&mut` accessor) is used.
  `get` is dead/future API; out of scope regardless.
- One reported "external caller" each for `get_mut` / `alloc_kpage` at
  `mm/virt/vmem.rs:342-343` is a **false context line** (the LSP attributed a
  reference inside an unrelated multi-line call). The genuine call sites are
  listed below per function.
- No cross-crate callers exist: `kernel` is a leaf binary crate, so the full
  caller set is intra-crate and the analysis is complete.

## Trait Obligations

None. `VirtMemoryManager` implements no traits that drive these entry points;
every in-scope function is an inherent `pub fn`. There are no `Drop`,
`GlobalAlloc`, `Iterator`, or function-pointer/closure dispatch paths into the
in-scope functions. (`UserFrame`/`KernelPage` `Drop` releases frames, but that is
internal to the value types, not a caller obligation on these functions.)

## Caller Expectations

All callers reach the manager through
`let mm: &mut VirtMemoryManager = unsafe { VirtMemoryManager::get_mut() };`
(or receive `mm: &mut VirtMemoryManager` threaded down from there). The manager
carries no observable state of its own, so for every function:

- Callers care about the **effect on the `Vmem` argument** (and on the global
  physical-frame pool / refcounts) and the **`Result` discriminant**.
- Callers do **not** care about any field of `VirtMemoryManager` (it has none),
  nor about how page tables / PTE bits are laid out internally.

### `new_vmem(&self, vmem: &Vmem) -> Result<Vmem, Error>`

Callers: `pm/process/manager/mod.rs:571` (exec), `:806` (fork).

- Callers assume on `Ok(new)`: a fresh address space cloned from `vmem` that
  shares `vmem`'s **kernel** mappings and is immediately usable as a target for
  `load_elf` / `alloc_upages` / `link_user_pages`. `vmem` is read-only and left
  unchanged (`&self`, `&Vmem`).
- On `Err`, callers propagate with `?` and assume no new address space exists and
  `vmem` is untouched (no partial side effects to undo).
- Callers don't care: which kernel frame backs the new page directory, or how the
  clone copies kernel page tables.

### `link_user_pages(&mut self, parent: &mut Vmem, child: &mut Vmem) -> Result<(), Error>`

Caller: `pm/process/manager/mod.rs:812` (fork), called immediately after
`new_vmem`, on a freshly created `child` that has **no overlapping user mappings**.

- Caller assumes on `Ok(())`: every present user page of `parent` is now mapped in
  `child` at the same vaddr, sharing the same physical frame with copy-on-write
  semantics (logically-writable pages become CoW in both `parent` and `child`;
  genuinely read-only pages are shared read-only). First write by either side will
  later be resolved by `try_resolve_cow_fault`.
- Caller assumes on `Err`: **full rollback** — pages already linked into `child`
  are unmapped (shared refcounts released) and CoW marks installed on `parent`
  during this call are cleared, so `parent` and `child` are restored to entry
  state. The caller relies on this to safely abandon the fork.
- Precondition the caller must uphold: `child` must not already contain user
  mappings overlapping `parent`'s (relied on by the rollback heuristic).
- Caller doesn't care: chunking strategy, AVL/CoW bit encoding, iteration order.

### `try_resolve_cow_fault(&mut self, vmem: &mut Vmem, fault_addr: usize, error_code) -> Result<bool, Error>`

Caller: `pm/process/manager/mod.rs:2897` (`handle_cow_page_fault`), forwards a CPU
page-fault on the running process's `vmem`. `fault_addr` need not be aligned.

- `Ok(true)`: fault was a CoW write; a private writable frame was installed at the
  faulting page and the shared frame's refcount released. Caller resumes the
  faulting thread.
- `Ok(false)`: not a CoW fault (wrong error-code bits, non-user-space address, or
  page not CoW). Caller forwards the fault to the registered/default handler.
  Importantly, **invalid/non-user addresses yield `Ok(false)`, not `Err`** — the
  function inspects `error_code` and rejects before mutating `vmem`.
- `Err`: it *was* a CoW fault but resolution failed (e.g. out of frames). Caller
  treats as a hard fault.
- Caller doesn't care: how the copy is performed or how the PTE is rewritten.

### `try_unmap_upage(&mut self, vmem: &mut Vmem, vaddr) -> Result<bool, Error>`

Callers: `pm/process/manager/mod.rs:2388` (thread/stack teardown), `:2493`,
`:2604` (`rollback_mmap`), `:2624` (`munmap`); `pm/process/manager/unsafe.rs:406`;
plus internal use by `alloc_upages`'s rollback. `vaddr` is page-aligned by type.

- `Ok(true)`: page was present and is now unmapped; the backing user frame was
  released (refcount drop / free). `munmap` treats this as success.
- `Ok(false)`: page was **not** present — *not an error*. Teardown and rollback
  loops rely on this to skip never-mapped (demand-paged) pages silently; `munmap`
  maps it to `NoSuchEntry`.
- `Err`: unexpected failure; rollback callers log-and-continue, `munmap`
  propagates. Callers tolerate partial progress on `Err` in best-effort loops.
- Idempotency expectation: calling on an already-unmapped page returns
  `Ok(false)` (relied on by repeated rollback passes).

### `alloc_upages(&mut self, vmem, vaddr, access, clear, nframes, uframes: &mut Vec<UserFrame>) -> Result<(), Error>`

Callers: `mm/elf.rs:338` (segment load), `pm/process/manager/mod.rs:585,611,654`
(args/env/stack pages in exec), `:2566` (`do_mmap` batches).

- Caller-supplied-buffer contract (callers depend on it): `uframes` **must be
  empty** and have `capacity >= nframes`; violations return
  `InvalidArgument` without side effects. Callers pass `Vec::with_capacity(count)`
  or a reserved vec exactly for this. The vector is used only as scratch and is
  left empty/drained on return.
- On `Ok(())`: all `nframes` pages `[vaddr, vaddr + nframes*PAGE_SIZE)` are mapped
  with `access`; if `clear`, their contents are zeroed. The range is validated to
  lie entirely in user space and to be currently unmapped (`BadAddress` /
  `ResourceBusy` otherwise).
- On `Err`: **complete rollback** — any pages mapped during this call are unmapped
  and their frames freed, so `vmem` is unchanged from entry. Callers
  (`do_mmap`, exec) build their own higher-level rollback assuming each failed
  `alloc_upages` left nothing behind for that call.
- Caller doesn't care: physical frames chosen, mapping order, or how clearing is
  done.

### `ctrl_upage(&mut self, vmem, vaddr, access) -> Result<(), Error>`

Callers: `mm/elf.rs:304` (merge permissions for a page shared by two segments),
`pm/process/manager/mod.rs:2642` (`mctrl`).

- On `Ok(())`: the already-mapped page at `vaddr` now has permissions `access`.
- On `Err`: permissions unchanged; caller propagates. (ELF loader relies on this
  to widen permissions on an overlapping page without remapping.)
- Caller assumes the page is already mapped (it does not allocate).

### `alloc_kpage(&mut self, clear) -> Result<KernelPage, Error>`

Callers: `mm/kernel_vas.rs:148`, `pm/kcall/mcopy.rs:46` (scratch page).

- On `Ok(kpage)`: a single kernel page is returned; if `clear`, it is zeroed. The
  returned `KernelPage` owns its frame and **frees it on `Drop`** — `mcopy` relies
  on the scratch page being reclaimed when it goes out of scope.
- On `Err`: no page allocated; caller propagates with `?`.

### `alloc_kpages(&mut self, clear, count, kframes: &mut Vec<KernelFrame>) -> Result<(), Error>`

Caller: `mm/kstack.rs:95` (kernel stack frames).

- Same caller-supplied-buffer contract as `alloc_upages`: `kframes` must be empty
  with `capacity >= count` (else `InvalidArgument`).
- On `Ok(())`: `kframes` holds exactly `count` contiguous kernel frames; if
  `clear`, each is zeroed. The caller then maps them
  (`map(KernelPage::new)`) into a kernel stack.
- On `Err`: `kframes` is left empty (allocated frames dropped/freed); no leak.

### `load_elf(&mut self, vmem, elf: &Elf32Fhdr) -> Result<(VirtualAddress, PageAligned<VirtualAddress>), Error>`

Caller: `pm/process/manager/mod.rs:575` (exec), on a freshly created `vmem`.

- On `Ok((entry, args_vaddr))`: ELF segments are loaded/mapped into `vmem`;
  `entry` is the program entry point used to forge the user context, and
  `args_vaddr` is a page-aligned address where the caller then allocates a page
  for argv (via `alloc_upages`). Caller assumes both are valid user-space
  addresses for the loaded image.
- On `Err`: caller abandons the new `vmem` (`?`); partial mappings inside `vmem`
  are acceptable because the whole address space is discarded on failure.
- Caller doesn't care: segment layout, how `elf32_load` maps each PT_LOAD, or
  where `args_vaddr` is placed relative to segments.

## Abstract Resource

From the caller's perspective, `VirtMemoryManager` is a **stateless service
(singleton façade) for managing user/kernel page mappings inside a `Vmem` address
space**, drawing physical frames from the global physical-memory pool. Callers use
it to: create/clone address spaces (`new_vmem`), populate them
(`load_elf`, `alloc_upages`, `link_user_pages`), change/remove user mappings
(`ctrl_upage`, `try_unmap_upage`), resolve copy-on-write faults
(`try_resolve_cow_fault`), and obtain kernel-side pages/frames
(`alloc_kpage`, `alloc_kpages`).

## Key Invariants (caller perspective)

- **Transactional on failure.** Mapping operations either fully succeed or fully
  roll back: `alloc_upages`, `alloc_kpages`, and `link_user_pages` leave their
  target `Vmem` (and frame refcounts) as if never called when they return `Err`.
- **`Ok(false)` ≠ error.** For `try_unmap_upage` and `try_resolve_cow_fault`,
  `Ok(false)` denotes a benign "nothing to do / not applicable" outcome that
  callers branch on; only `Err` is a real failure.
- **Caller-owned scratch buffers** (`uframes`, `kframes`) must be empty with
  sufficient capacity; the manager validates this and never reallocates them.
- **Frame ownership flows through RAII.** Returned `KernelPage`/`UserFrame` (and
  those dropped during unmap/rollback) release physical frames on `Drop`; callers
  rely on scope-based reclamation, never manual frees.
- **User-space confinement.** User-page operations validate that addresses lie in
  the user region; out-of-range inputs yield `BadAddress`/`Ok(false)` rather than
  corrupting kernel mappings.
- **Manager statelessness.** No observable state lives in `VirtMemoryManager`
  itself; every effect is on the passed-in `Vmem` or the global frame pool, so the
  View must model the address space / frame ownership, not the (empty) manager.
