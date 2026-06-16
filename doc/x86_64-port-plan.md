# x86_64 Kernel Port — Re-implementation Plan (on top of `dev`)

Status: design plan. Companion to `doc/x86_64-port.md` (the original port's design
notes). This document describes the **cleanest path** to re-land a working x86_64
kernel on the current `dev` line, driven by the implementation that already exists on
the `feature-kernel-x64` branch.

## 1. Context and motivation

`feature-kernel-x64` is the long-lived x86_64 branch; `dev` is merged into it
periodically (`.github/workflows/sync-x64-branch.yml`). The branch has now been merged
up to current `dev` (commit `48859387b`). During that merge the kernel paging layer was
resolved in favor of **dev's** design, which has a critical consequence:

- **dev's x86_64 kernel is scaffolding only.** `src/kernel/src/hal/arch/x86_64/mem/mmu/hwpt.rs`
  implements a minimal 4-level mapper but is **never called** — nothing wires it into the
  memory manager. `src/kernel/src/mm/virt/vmem.rs` is entirely x86/2-level (no `target_arch`
  dispatch). So `TARGET=x86_64` does not build/boot.
- **The branch already contains a complete, previously-working x86_64 implementation** (22
  commits, `f2b557869..f75c6d0af`). Most of it survived the merge as additive code; the
  kernel **paging/VM** portion was dropped.

This plan re-introduces the dropped paging/VM portion using the branch's own work, in a
layering that keeps dev's evolved `Vmem` features (CoW, demand paging, copy-to-user)
intact and keeps the x86 path unchanged.

## 2. Key design decision: the `PageMap` abstraction (and delete `hwpt`)

dev split paging into two incompatible halves:

- `PageDirectory<T>` / `PageTable<T>` — a **2-level**, x86-only abstraction that `Vmem`
  is built on directly.
- `hwpt` — a **separate**, minimal 4-level mapper for x86_64 that nothing uses.

The branch's design is cleaner and is the one to adopt: a per-arch **`PageMap`** type that
*owns the whole hardware hierarchy* and exposes a single, arch-agnostic API. `Vmem` (and
every caller) talks only to `PageMap`; the 2-level vs 4-level difference lives entirely
inside `PageMap`.

Reference: `feature-kernel-x64:src/kernel/src/hal/arch/x86_64/mem/mmu/page_map.rs`
exposes exactly the surface `Vmem` needs:

```
PageMap::new_boot(...)        PageMap::new_clone(from, pages)
PageMap::cr3_value()          unsafe PageMap::load()
unsafe PageMap::copy_from_user(...)   unsafe PageMap::copy_to_user(...)
unsafe PageMap::memset_user(...)
PageMap::map_user_page(vaddr, ...)    PageMap::unmap_user_page(vaddr)
```

x86 gets a sibling `src/kernel/src/hal/arch/x86/mem/mmu/page_map.rs` that wraps the
existing 2-level `PageDirectory`/`PageTable` behind the same API, selected with the
existing `#[path]` arch-alias pattern (see `doc/x86_64-port.md` §"Isolation Pattern").

**Action:** delete `hwpt.rs` and its references; it is a dead-end that only adds
confusion.

### Target layering

```
            Vmem  (arch-agnostic: CoW, demand paging, copy-user, page tracking)
              |
          PageMap  (arch-agnostic API; #[path]-selected per target_arch)
          /      \
  x86 PageMap     x86_64 PageMap
  PageDirectory   PML4 -> PDPT -> PD -> PT
  + PageTable     (hierarchy owned + walked internally)
              |
   libs/arch paging primitives (entries, indices, flags, PteWord = u32|u64)
```

## 3. What is already in place vs. what must be re-landed

Already merged on the branch (keep — do **not** redo):

| Area | Location | Source commit |
|---|---|---|
| LP64 wire format / `repr(C)` | `src/libs/sysapi/**` | `f5ff75056`, `c0c61e6c2` |
| 64-bit portability fixes | event mgr, `libc_stdlib`, `sorted-vec`, `syscall`, `linuxd`, `c-bindings` | `b230bd584`, `aed88c5ea`, `9cf9f93e3`, `d4eff40c5`, `4b0e7ca9e`, `98bd9a3e3` |
| User kernel-call ABI stubs | `src/libs/sys/src/sys/kcall/arch/x86_64.rs` | `9cf2fdc14` |
| Thread start/exit (x86_64) | `src/libs/sys/src/sys/kcall/pm_x86_64.rs` | `c1a5aed0f` |
| CRT0 entry point | `src/libs/nvx-crt0/src/crt0_x86_64.rs` (relocated into `nvx-crt0`) | `9ce427193` |
| UserVM x86_64 boot | `src/uservm/src/{elf64.rs, vmm/microvm/kvm/vcpu/reset64.rs}`, `guest.rs` `is_64bit` | `bcaf1486a` |
| x86_64 IDT (platform-storage design) | `src/kernel/src/hal/arch/x86_64/cpu/idt.rs` | `04604f60f` (this branch) |
| process-mgr usize-width context | `src/kernel/src/pm/process/manager/mod.rs` | `04604f60f` |
| Integration test configs | `test/test-standalone-x86_64.toml`, `test/test-*-x86_64.toml` | `1a6baa927`, `d064bd3b1` |
| x86_64 CI workflow | `.github/workflows/ci-x86_64.yml` | `d064bd3b1` |
| Registry removal | (superseded — dev removed the `nanvix-registry` crate) | `b719b2436` |

Must be re-landed (dropped during the merge; pull from the branch's history):

| Area | Branch source (files / commit) |
|---|---|
| libs/arch paging primitives | `src/libs/arch/src/x86_64/mem/paging/{pml4e.rs,pdpte.rs,table.rs}`; constants `PGTAB_SHIFT/PGTAB_ALIGNMENT/PHYS_ADDR_MASK/NUM_HIERARCHY_PAGES`, `PteWord` (`26992c110`, `4c9b9af08`) |
| `PageMap` abstraction (both arches) | `src/kernel/src/hal/arch/{x86,x86_64}/mem/mmu/page_map.rs`, `x86_64/.../{pml4.rs,pdpt.rs}` (`7190ddc6a`) |
| Kernel page address types | `src/kernel/src/hal/mem/types/address/{pt.rs,pml4.rs,pdpt.rs}` (`a8cb4881a`) |
| `Vmem` on `PageMap` + ELF64 loader + `ElfClass` | `src/kernel/src/mm/virt/vmem.rs`, `src/kernel/src/mm/{elf.rs,elf64.rs,mod.rs}`, shared `page_directory.rs`/`page_table.rs` (`9801c25ad`) |
| x86_64 boot trampoline / boot sequence | `src/kernel/src/hal/arch/x86_64/asm/start16.rs` + boot path (`3baf69eea`); kernel linker `build/kernel/linker/x86_64/kernel.ld.in` trampoline section |
| Remove `hwpt` | delete `src/kernel/src/hal/arch/x86_64/mem/mmu/hwpt.rs` |

## 4. Phased implementation plan

Each phase is independently buildable and has an explicit validation gate. **x86 must
stay green at every gate** (`./z build -- all TARGET=x86 && ./z build -- run-nanvix-tests
TARGET=x86`). The recurring x86_64 gate is `./z build -- check-kernel TARGET=x86_64`
until Phase 6, then full build + tests.

### Phase 0 — Baseline & cleanup (low risk)
- Delete `hwpt.rs`; remove its module declaration and the unused `microvm`
  `get_kstack_top`/`kstack` statics it pulled in.
- Re-land the libs/arch x86_64 paging primitives (`pml4e`, `pdpte`, `table`) and the
  arch-conditional constants/`PteWord`. These are leaf modules with no kernel deps.
- **Gate:** `cargo check` of the `arch` crate for both targets; x86 unaffected.

### Phase 1 — Arch-generic shared paging (`PageDirectory`/`PageTable`)
- Reconcile the shared `page_directory.rs`/`page_table.rs` to the branch's
  `Table<Entry>`-based form **without** changing observable behavior on x86. Where dev
  added fixes after the merge-base (CoW flag plumbing, PTE accessors), fold those into the
  branch's versions rather than discarding them.
- Re-land kernel page address types (`pt.rs`, `pml4.rs`, `pdpt.rs`) and wire them through
  `hal/mem/types/address/mod.rs`.
- **Gate:** x86 builds + standalone tests pass (no regression). x86_64 paging primitives
  compile.

### Phase 2 — `PageMap` abstraction layer
- Introduce `PageMap` for both arches behind the `#[path]` alias:
  - x86 `PageMap` wraps the existing 2-level `PageDirectory`+`PageTable`.
  - x86_64 `PageMap` owns `PML4 -> PDPT -> PD -> PT` (port `pml4.rs`/`pdpt.rs`).
- Define the uniform API once (see §2). Keep `cr3_value()`/`load()` arch-specific
  (CR3 = PD paddr on x86, PML4 paddr on x86_64).
- **Gate:** both `PageMap` impls compile for their target; API parity asserted by a small
  `trait`-or-doc contract.

### Phase 3 — Port `Vmem` onto `PageMap`
- This is the integration crux. Replace `Vmem`'s direct `PageDirectory` field/uses with a
  `PageMap`, mapping each `Vmem` operation to the `PageMap` API:
  `new`→`new_boot`, `clone`→`new_clone`, `load`→`load`, `map`/`unmap`→
  `map_user_page`/`unmap_user_page`, `copy_*_user`→`PageMap::copy_*_user`,
  translation→a `PageMap::translate(vaddr)` helper.
- **Preserve dev's evolution**: CoW (`mark_user_page_cow`/`resolve_cow_at`), demand
  paging, region copy helpers. Express their page-table touches through `PageMap`
  (add `PageMap` hooks for AVL/CoW bits as needed — the x86_64 PTE already carries them).
- Update the handful of `Vmem` callers whose signatures changed
  (`VirtMemoryManager::new`, `identity_map::init`, page-fault handler) so they are
  arch-agnostic.
- **Gate:** x86 builds + standalone tests pass (this is the regression-sensitive step);
  x86_64 kernel `check` is clean.

### Phase 4 — ELF64 loading
- Re-land `mm/elf64.rs` and the `ElfClass` dispatcher in `mm/elf.rs`; thread the class
  through `VirtMemoryManager::load_elf` (dev already has the `Elf64*` types in
  `src/libs/elf`). Keep the `build_user_image` refactor dev introduced.
- **Gate:** kernel builds for x86_64; ELF32 load path on x86 unchanged.

### Phase 5 — x86_64 CPU + boot bring-up
- Verify/complete the x86_64 CPU subsystems against dev's platform-init contract
  (`hal/platform/microvm`): GDT/TSS already use `set_backing_storage`; IDT done in
  `04604f60f`; confirm exception/interrupt controllers and context switch
  (`context.rs` `cr3/rsp/rsp0: u64`).
- Re-land the boot trampoline (`start16.rs`) **or** confirm direct long-mode entry via the
  UserVM `reset64` path; pick one and make the kernel linker
  (`build/kernel/linker/x86_64/kernel.ld.in`) match (trampoline section iff `start16` is
  used). The branch boots **directly in long mode** via `reset64` (see
  `doc/x86_64-port.md` §"Boot Flow"), so `start16` is only needed for SMP AP bring-up and
  can be deferred.
- **Gate:** kernel boots to `kmain` on x86_64 under KVM (kernel log reaches user-mode
  transition). Use `LOG_LEVEL=trace` and the GDB workflow (`.github/skills/gdb-debugging`).

### Phase 6 — User-mode + kernel calls
- Confirm the already-merged user path links and runs: `crt0_x86_64` → `_start` →
  `__nanvix_libc_start_main`; `kcall0..4` (`int 0x80`, SysV regs); `pm_x86_64` thread
  start/exit; TDA via `%fs`.
- Run one user program end-to-end (`hello-rust-nostd`).
- **Gate:** a single x86_64 user program prints and exits cleanly.

### Phase 7 — Tests + CI
- Run `./z build -- run-nanvix-tests TARGET=x86_64` against
  `test/test-standalone-x86_64.toml`; fix failures iteratively.
- Ensure `.github/workflows/ci-x86_64.yml` exercises build + standalone tests; keep the
  hourly `sync-x64-branch.yml` merge model (no history rewrite).
- **Gate:** all x86_64 standalone integration tests pass; x86 still green.

## 5. Risks and mitigations

- **CoW reconciliation (Phase 3)** is the highest-risk step: dev's CoW and the branch's
  CoW differ. Mitigation: keep dev's CoW control flow in `Vmem`; only re-express the
  low-level PTE bit operations through `PageMap`. Add a `PageMap` unit test that round-
  trips map → mark-cow → resolve-cow on both arches.
- **Boot bring-up (Phase 5)** is hard to unit test. Mitigation: bisect with `LOG_LEVEL=
  trace` + GDB; validate paging by mapping/translating a known page before enabling user
  mode.
- **x86 regression.** Mitigation: the x86 gate runs at every phase; treat any x86 test
  failure as a blocking regression.
- **Address-space window assumption.** The branch confines 4-level setup to boot and lets
  `Vmem` operate within a bounded window. Document the supported guest-memory size and
  assert it at `new_boot`.

## 6. Validation strategy (summary)

| Gate | Command |
|---|---|
| x86 build | `./z build -- all TARGET=x86` |
| x86 tests | `./z build -- run-nanvix-tests TARGET=x86` |
| x86_64 type-check | `./z build -- check-kernel TARGET=x86_64` |
| x86_64 build | `./z build -- all TARGET=x86_64` |
| x86_64 tests | `./z build -- run-nanvix-tests TARGET=x86_64` |
| Full pipeline | `./scripts/pipeline.sh` |

## 7. Sequencing note

Phases 0–2 are mechanical and low-risk (re-landing leaf modules + the `PageMap` layer).
Phase 3 is the real work and should be its own reviewed PR. Phases 4–7 are incremental and
each independently verifiable. Land each phase as a focused commit so the
`sync-x64-branch` automation keeps testing the result.
