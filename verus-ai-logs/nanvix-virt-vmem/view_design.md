# View Design: `mm::virt::vmem` (`Vmem`)

## Abstract Resource

A `Vmem` is an **owned virtual address space**: a partial map from page-aligned
virtual addresses to backing physical frames, carrying per-page access
permissions and copy-on-write (CoW) state, split into a **kernel half** (shared
across address spaces) and a **user half** (per-process, privately owning its
`UserFrame`s). It is the unit the kernel loads into the MMU — its page
directory's physical address is the value programmed into CR3 — and the unit of
fork-time CoW sharing.

The View models the **structure** of this address space (which addresses map to
which frames, with which permissions and CoW marks, plus the page-directory
base). It deliberately does **not** model byte-level memory *contents* (see
*Rejected Alternatives*).

---

## View Struct

```rust
verus! {

/// Abstract access permissions of a mapped page, as observed by callers
/// (e.g. through `PageTableEntry::flags().is_writable()`).
pub struct PagePerms {
    pub read: bool,
    pub write: bool,
    pub exec: bool,
}

/// Abstract state of a single user-space mapping.
pub struct UserPageView {
    /// Physical base address of the backing user frame.
    pub frame: nat,
    /// Access permissions currently in effect for the page.
    pub perms: PagePerms,
    /// Whether the page is marked copy-on-write.
    pub cow: bool,
}

/// Abstract state of a single kernel-space mapping.
pub struct KernelPageView {
    /// Physical base address of the backing kernel page/frame.
    pub frame: nat,
    /// Access permissions currently in effect for the page.
    pub perms: PagePerms,
}

/// Abstract state of an entire virtual address space.
pub struct VmemView {
    /// User half: page-aligned user vaddr -> mapping. Domain = present user pages.
    pub user: Map<nat, UserPageView>,
    /// Kernel half: page-aligned kernel vaddr -> mapping. Domain = present kernel pages.
    pub kernel: Map<nat, KernelPageView>,
    /// Physical base address of the page directory (the value loaded into CR3).
    pub pgdir: nat,
}

} // verus!
```

`view()` is exposed as `pub closed spec fn view(&self) -> VmemView` on `Vmem`
(public so callers can reference abstract state; closed so the mapping from the
four internal `LinkedList`/`Rc<RefCell<…>>` fields does not leak).

---

## Address-partition spec helpers

The classification predicates (`is_user_addr`, `is_user_region`,
`is_physical_region`) are **pure, total, instance-independent** functions of the
address only — they take no `&self`. They are therefore modeled as free spec
functions over architectural constants, **not** as View fields.

```rust
verus! {

// Architectural constants (correspond to `sys::config` memory-layout values).
pub spec const USER_BASE: nat;        // first user-space address
pub spec const USER_END: nat;         // one-past-last user-space address
pub spec const PHYS_MEM_SIZE: nat;    // size of valid guest physical memory
pub spec const PAGE_SIZE: nat;        // == arch::mem::PAGE_ALIGNMENT

pub open spec fn is_page_aligned(a: nat) -> bool { a % PAGE_SIZE == 0 }

pub open spec fn spec_is_user_addr(a: nat) -> bool {
    USER_BASE <= a < USER_END
}

pub open spec fn spec_is_kernel_addr(a: nat) -> bool {
    // Total partition over the addressable space: kernel == not user.
    !spec_is_user_addr(a)
}

/// `[start, start+size)` lies entirely in user space; rejects zero size and
/// any range whose end exceeds `USER_END` (overflow/wrap is impossible in `nat`,
/// so an exec range that would wrap maps to one that fails `end <= USER_END`).
pub open spec fn spec_is_user_region(start: nat, size: nat) -> bool {
    size > 0 && USER_BASE <= start && start + size <= USER_END
}

pub open spec fn spec_is_kernel_region(start: nat, size: nat) -> bool {
    size > 0 && start + size <= USER_BASE  // wholly below the user window
}

/// `[start, start+size)` lies entirely within valid physical memory.
pub open spec fn spec_is_physical_region(start: nat, size: nat) -> bool {
    start + size <= PHYS_MEM_SIZE
}

/// Page-aligned base of the page containing `a`, and intra-page offset.
pub open spec fn page_base(a: nat) -> nat { a - (a % PAGE_SIZE) }
pub open spec fn page_offset(a: nat) -> nat { a % PAGE_SIZE }

} // verus!
```

These satisfy the partition invariant `spec_is_user_addr(a) <==>
!spec_is_kernel_addr(a)`, which guarantees the user/kernel key sets in a
`VmemView` are disjoint.

---

## Well-formedness Invariant

```rust
verus! {
impl VmemView {
    pub open spec fn inv(self) -> bool {
        // User keys are page-aligned user addresses.
        &&& forall|v: nat| #[trigger] self.user.contains_key(v)
                ==> spec_is_user_addr(v) && is_page_aligned(v)
        // Kernel keys are page-aligned kernel addresses.
        &&& forall|v: nat| #[trigger] self.kernel.contains_key(v)
                ==> spec_is_kernel_addr(v) && is_page_aligned(v)
        // User frames are valid, page-aligned, and CoW implies read-only.
        &&& forall|v: nat| #[trigger] self.user.contains_key(v) ==> {
                let p = self.user[v];
                &&& is_page_aligned(p.frame)
                &&& spec_is_physical_region(p.frame, PAGE_SIZE)
                &&& (p.cow ==> !p.perms.write)
            }
        // Kernel frames are valid and page-aligned.
        &&& forall|v: nat| #[trigger] self.kernel.contains_key(v) ==> {
                let p = self.kernel[v];
                &&& is_page_aligned(p.frame)
                // In physical memory, OR an identity-mapped MMIO frame (frame == v).
                &&& (spec_is_physical_region(p.frame, PAGE_SIZE) || p.frame == v)
            }
        // The page directory has a valid, page-aligned physical base.
        &&& is_page_aligned(self.pgdir)
        &&& spec_is_physical_region(self.pgdir, PAGE_SIZE)
    }
}
} // verus!
```

`inv()` is the open, caller-visible predicate (placed on the `VmemView` and
mirrored by `Vmem::inv(&self) == self@.inv()`). The CoW invariant
`cow ==> !write` is the key abstraction-level property fork/page-fault callers
rely on.

> **No global uniqueness invariant.** Frames are intentionally *not* required to
> be uniquely owned: CoW sharing and shared kernel pages mean the same physical
> frame may back pages in several address spaces. Cross-`Vmem` ownership balance
> is a global/`Drop` concern, not a single-View property.

---

## Spec Transition Functions

All mutators preserve unchanged fields via `..self` (frame condition).

```rust
verus! {
impl VmemView {
    pub open spec fn user_mapped(self, v: nat) -> bool { self.user.contains_key(v) }

    pub open spec fn spec_map(self, v: nat, frame: nat, perms: PagePerms) -> VmemView {
        VmemView { user: self.user.insert(v, UserPageView { frame, perms, cow: false }), ..self }
    }

    pub open spec fn spec_unmap(self, v: nat) -> VmemView {
        VmemView { user: self.user.remove(v), ..self }
    }

    pub open spec fn spec_map_kpage(self, v: nat, frame: nat, perms: PagePerms) -> VmemView {
        VmemView { kernel: self.kernel.insert(v, KernelPageView { frame, perms }), ..self }
    }

    pub open spec fn spec_mark_cow(self, v: nat) -> VmemView {
        let p = self.user[v];
        VmemView {
            user: self.user.insert(v, UserPageView { perms: PagePerms { write: false, ..p.perms },
                                                      cow: true, ..p }),
            ..self
        }
    }

    pub open spec fn spec_unmark_cow(self, v: nat) -> VmemView {
        let p = self.user[v];
        VmemView {
            user: self.user.insert(v, UserPageView { perms: PagePerms { write: true, ..p.perms },
                                                     cow: false, ..p }),
            ..self
        }
    }

    // `new_frame` is the freshly allocated private frame (an implementation
    // allocation choice; supplied existentially by the ensures clause).
    pub open spec fn spec_resolve_cow(self, v: nat, new_frame: nat) -> VmemView {
        let p = self.user[v];
        VmemView {
            user: self.user.insert(v, UserPageView { frame: new_frame,
                                                     perms: PagePerms { write: true, ..p.perms },
                                                     cow: false }),
            ..self
        }
    }

    pub open spec fn spec_uctrl(self, v: nat, perms: PagePerms) -> VmemView {
        let p = self.user[v];
        VmemView { user: self.user.insert(v, UserPageView { perms, ..p }), ..self }
    }

    pub open spec fn spec_kctrl(self, v: nat, perms: PagePerms) -> VmemView {
        let p = self.kernel[v];
        VmemView { kernel: self.kernel.insert(v, KernelPageView { perms, ..p }), ..self }
    }

    // `kctrl` on an *absent* kernel page identity-maps it (frame == v) with `perms`
    // (the MMIO mapping-creation branch; see SB-1). `spec_kctrl` assumes the key present.
    pub open spec fn spec_kctrl_create(self, v: nat, perms: PagePerms) -> VmemView {
        VmemView { kernel: self.kernel.insert(v, KernelPageView { frame: v, perms }), ..self }
    }

    /// Every CoW user page overlapping `[start, start+size)` is privatized.
    pub open spec fn region_cow_resolved(self, start: nat, size: nat) -> bool {
        forall|v: nat| #[trigger] self.user.contains_key(v)
            && page_base(start) <= v < start + size
            ==> !self.user[v].cow
    }
}
} // verus!
```

---

## Spec sketches per top-level entry point

(Contracts are written in a later phase; sketches show how each function maps
onto the View, demonstrating every field is used.)

- **`new`** → `Ok` ⟹ `self@.user == Map::empty()`, `self@.kernel` populated from
  the supplied kernel pages/tables, `self@.pgdir` a fresh valid base, `inv()`.
- **`clone`** → `Ok(new)` ⟹ `new@.kernel == from@.kernel`,
  `new@.user == Map::empty()`, `new@.pgdir != from@.pgdir`.
- **`load`** → `&self`; `self@` unchanged. (Activation sets CR3 = `self@.pgdir`;
  that CR3 effect is *global* MMU state, outside this View — see Rejected.)
- **`pgdir`** → `result.physical_address()@ == self@.pgdir`.
- **`map_kpage`** → req. `spec_is_kernel_addr(v)`; `Ok` ⟹
  `self@ == old@.spec_map_kpage(v, frame, perms)`; `Err` ⟹ `self@ == old@`.
- **`map`** → req. `spec_is_user_addr(v)` (else `BadAddress`); `Ok` ⟹
  `self@ == old@.spec_map(v, frame, perms)`; `Err` ⟹ `self@ == old@`
  (frame dropped, no mapping persists).
- **`is_user_page_mapped`** → `result == Ok(self@.user.contains_key(v))`
  (and `Ok(false)` when `v` not user). `self@ == old@`.
- **`is_user_addr`** → `result == spec_is_user_addr(virt_addr)`.
- **`is_user_region`** → `result == spec_is_user_region(start, size)`.
- **`is_physical_region`** → `result == spec_is_physical_region(start, size)`.
- **`try_find_user_pte`** → `Ok(Some(pte))` iff `self@.user.contains_key(v)`,
  with `pte.frame_number == self@.user[v].frame / PAGE_SIZE`,
  `pte.flags().is_writable() == self@.user[v].perms.write`,
  `pte.is_cow() == self@.user[v].cow`; `Ok(None)` otherwise. `self@ == old@`.
- **`for_each_user_mapping`** → invokes `f` exactly on `self@.user.dom()`
  (each page-aligned user `v`), short-circuiting on the first `Err`.
  `self@ == old@`.
- **`mark_user_page_cow`** → req. `self@.user.contains_key(v) &&
  self@.user[v].perms.write`; `Ok` ⟹ `self@ == old@.spec_mark_cow(v)`.
- **`unmark_user_page_cow`** → `Ok` ⟹ `self@ == old@.spec_unmark_cow(v)`
  (exact inverse of `spec_mark_cow` on a writable origin).
- **`resolve_cow_at`** → `Ok(true)` ⟹ `old@.user.contains_key(v) &&
  old@.user[v].cow` and `exists f. valid(f) && self@ == old@.spec_resolve_cow(v, f)`;
  `Ok(false)` ⟹ `(!contains_key(v) || !old@.user[v].cow)` and `self@ == old@`
  (hence idempotent: a second call returns `Ok(false)`).
- **`resolve_cow_for_region`** → `Ok` ⟹ `self@.region_cow_resolved(addr, size)`
  and pages outside the range / non-CoW are unchanged. `size == 0` ⟹ `self@ == old@`.
- **`user_vaddr_to_paddr`** → `Ok(p)` ⟹ `self@.user.contains_key(page_base(v)) &&
  p == self@.user[page_base(v)].frame + page_offset(v)`. `self@ == old@`.
- **`copy_from_user_unaligned`** → `&self`; errors keyed on
  `!spec_is_user_region(src, size)` / `!spec_is_kernel_region(dst, size)` /
  `size == 0`. `self@ == old@`.
- **`copy_to_user_unaligned_unchecked`** → error predicate is a pure function of
  `(self@, dst, src, size)` (user/kernel/physical-region + size==0). A dry run does
  not mutate (`dry_run ==> self@ == old@`); the real run eagerly resolves CoW in the
  destination range, so `Ok && !dry_run ==> self@.region_cow_resolved(dst, size)`
  (SB-3b); `Err ==> self@ == old@`. `inv()` preserved.
- **`copy_to_user_unaligned`** → all-or-nothing checked copy: `Ok ==>
  self@.region_cow_resolved(dst, size)` (SB-3b); on `Err` no observable change,
  `self@ == old@`; never panics.
- **`copy_user_to_user`** → both ranges user-mapped; errors:
  `size==0`/`!user_region`/page-not-mapped (`NoSuchEntry`). Neither `self@` changes.
- **`memset`** → req. `self@.user.contains_key(dst)`; `self@ == old@`
  (content-only effect).
- **`unmap`** → `Ok(Some(frame))` ⟹ `old@.user.contains_key(v)`,
  returned frame address `== old@.user[v].frame`, `self@ == old@.spec_unmap(v)`;
  `Ok(None)` ⟹ `!old@.user.contains_key(v)` and `self@ == old@`.
- **`uctrl`** → req. `self@.user.contains_key(v)`; `Ok` ⟹
  `self@ == old@.spec_uctrl(v, access_perms(access))`.
- **`kctrl`** → req. `spec_is_kernel_addr(v)`; `dry_run == true` ⟹ validates,
  `self@ == old@`; `dry_run == false && Ok` ⟹ if the page is present,
  `self@ == old@.spec_kctrl(v, access_perms(access))`, else (absent PTE, MMIO create)
  `self@ == old@.spec_kctrl_create(v, access_perms(access))` (SB-1). `inv()` preserved
  (TYPE-5 admits identity-mapped MMIO frames).

(`access_perms(access: AccessPermission) -> PagePerms` is a small spec mapping
from the exec permission type to the abstract triple.)

---

## Design Rationale (per field — substitution test)

| Field | Why callers need it | Substitution test |
|-------|--------------------|-------------------|
| `user: Map<nat, UserPageView>` | The user half: presence (`is_user_page_mapped`), decode (`try_find_user_pte`), iterate (`for_each_user_mapping`), translate (`user_vaddr_to_paddr`), map/unmap, CoW mark/unmark/resolve, `uctrl`. The single most-used field. | ✅ Any address space, however implemented (linked list, radix tree, hash map), maps user vaddrs to frames. |
| `UserPageView.frame: nat` | `user_vaddr_to_paddr`, the copy paths, and `unmap`'s returned frame all observe the backing physical frame. | ✅ Every mapping is backed by *some* physical frame; the page-walk mechanism is irrelevant. |
| `UserPageView.perms: PagePerms` | `uctrl` sets it; `try_find_user_pte().flags().is_writable()` reads `write`; the CoW invariant hinges on `write`. | ✅ Permissions are an architectural property of a mapping, not of any storage layout. |
| `UserPageView.cow: bool` | `mark`/`unmark`/`resolve` toggle it; `try_find_user_pte().is_cow()` reads it; fork relies on it. | ✅ CoW is a semantic state any implementation must track, independent of *which* PTE bit encodes it. |
| `kernel: Map<nat, KernelPageView>` | Kernel half: `map_kpage` inserts, `kctrl` mutates perms / needs presence (`NoSuchEntry`). `clone` shares it. | ✅ The kernel↦frame mapping exists regardless of `LinkedList<Rc<RefCell<…>>>` storage. |
| `KernelPageView.{frame,perms}` | `kctrl` changes perms and depends on presence; `map_kpage` records the owned page; `Drop` releases the frame. | ✅ Kernel mappings carry a frame and permissions in any design. |
| `pgdir: nat` | `pgdir().physical_address()` is the CR3 value (`load`/context switch); `clone` guarantees a *distinct* base. | ✅ Every address space has a root table with a physical base; representation is irrelevant. |

**Why abstract types:** `Map`/`nat`/`bool` instead of `LinkedList`,
`Rc<RefCell<…>>`, `usize`, raw PTE words — the View lives in spec world and
must survive a complete reimplementation. The four internal fields
(`pgdir`, `kernel_page_tables`, `kernel_pages`, `user_page_tables`) are *not*
mirrored: their list/refcount structure is pure bookkeeping.

**Why the partition predicates are free spec fns, not fields:** `is_user_addr`,
`is_user_region`, `is_physical_region` are associated functions with no `&self`
— they depend only on architectural constants, so encoding them as View state
would be wrong. They live as `spec_*` helpers over `USER_BASE`/`USER_END`/
`PHYS_MEM_SIZE`.

---

## Rejected Alternatives

1. **Byte-level memory contents** (`Map<nat, u8>` of physical memory).
   *Rejected.* The in-scope verified obligations are structural/safety
   properties: valid address-range classification, CoW pre-resolution,
   ownership balance, dry-run⇒commit determinism, all-or-nothing/no-panic.
   Proving byte equality for the copy/`memset` paths would require a *global*
   physical-memory model shared across all `Vmem`s plus a faithful
   physical-aliasing semantics — a far larger abstraction orthogonal to every
   safety property the callers actually depend on. Copy/`memset` are therefore
   specified as content-only (structurally `self@ == old@`) with rigorous
   address-validation error specs. (Deferrable to a future content-aware View.)

2. **A global "is this `Vmem` active / CR3 register" field.**
   *Rejected.* "Active" is a property of the *global* MMU (one CR3 for the whole
   machine), not of an individual `Vmem`. Modeling it inside `VmemView` would be
   a category error. `load`'s only View-level fact is `self@ == old@`; the
   activation effect (`CR3 := self@.pgdir`) belongs to a separate global/HAL
   abstraction. `clone` still captures the observable consequence (distinct
   `pgdir`).

3. **Mirroring the four internal fields** (`pgdir`,
   `kernel_page_tables: LinkedList<Rc<RefCell<…>>>`, `kernel_pages`,
   `user_page_tables`). *Rejected.* These fail the substitution test outright —
   the LRU move-to-front cache in `lookup_user_page_table`, the `Rc<RefCell>`
   sharing of kernel tables, and list ordering are all implementation strategy.
   `for_each_user_mapping`'s docstring explicitly tells callers *not* to depend
   on iteration order. Collapsing them into two `Map`s + one `nat` keeps specs
   minimal and rewrite-proof.

4. **Separate `writable: bool` field instead of `perms.write`.**
   *Considered.* `write` is the only permission bit any caller currently reads
   (`is_writable`). But `uctrl(access)` sets the *full* `AccessPermission`, so a
   bare `writable` bool would make `uctrl`'s postcondition lossy. `PagePerms`
   `{read, write, exec}` gives `uctrl` a complete postcondition while still
   exposing `write` for the CoW invariant and PTE decode. The `read`/`exec`
   bits are the minimal honest completion of the permission triple `uctrl`
   writes; if a future reviewer judges them unused, they can be pruned to just
   `write` without affecting any other clause.

5. **Modeling per-page *ownership tokens* (refcounts) in the View.**
   *Rejected.* Ownership balance (no leak / no double-free) is enforced by
   Rust's type system (`UserFrame` by-value transfer, `Drop`) and is reflected
   *observably* by domain membership: `map` adds `v` to `user`, `unmap` removes
   it and returns the frame, `map`-on-`Err` leaves `user` unchanged (frame
   dropped). An explicit refcount field would duplicate what `Drop` already
   guarantees and would couple the View to the `Rc` strategy.

6. **A combined `Map<nat, Either<UserPageView, KernelPageView>>`.**
   *Rejected.* The user/kernel split is a first-class, caller-visible
   distinction (different ownership, different mutators, `clone` shares one half
   and empties the other). Two typed maps make every spec read directly and let
   `inv()` state the partition cleanly; a tagged union would force constant
   case-splitting with no benefit.
