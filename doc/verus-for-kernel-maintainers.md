# Verus for Kernel Maintainers

[Verus](https://github.com/verus-lang/verus) is a tool for verifying the correctness of
code written in Rust.

This guide explains how to read Verus contracts in the Nanvix kernel — no prior Verus experience is
required, and you will **not** need to write proofs or run the verifier.

The goal is to help you understand the *specification* side of verified code (what the code is
supposed to do) without needing to understand the *proof* side (why the code meets its
specification).

For a complete Verus tutorial, see the [Verus Tutorial and
Reference](https://verus-lang.github.io/verus/guide/).

---

## TL;DR

- `#[verus_spec(...)]` attributes attach formal contracts to regular functions — they are
  **erased** at compile time and have zero runtime cost.
- **`requires`** — precondition: what the caller must guarantee before calling.
- **`ensures`** — postcondition: what the function promises on return.
- `old(x)` is the pre-call state of any `&mut` parameter `x` (including `self`).
- `x@` is an abbreviation for `x.view()`, which means the abstract view of `x`.
- `inv()` is the data-structure invariant.
- `#[trigger]` tells the SMT solver *when* to apply a quantified fact — triggers do not
  change runtime behavior.
- Specs often use mathematical integers (`int`, `nat`) that never overflow — they are **not** the
  same as Rust's `i32` or `usize`. mathematical operators (e.g., +, *) upconvert their inputs to int
  (signed mathematical integer).
- Verified crates split into `lib.rs` (code + contracts), `lib.spec.rs` (specification
  material), and optionally `lib.proof.rs` (proof lemmas and helper facts).
- To understand a function, read its contract — you do **not** need to read the body or
  any proofs.

---

## Table of Contents

- [1. File Layout Convention](#1-file-layout-convention)
- [2. Reading `#[verus_spec(...)]` on Mainline Code](#2-reading-verus_spec-on-mainline-code)
  - [Example: `Slab::allocate()`](#example-slaballocate)
  - [Example: `RawArray::new()`](#example-rawarraynew)
- [3. Reading Loop Invariants](#3-reading-loop-invariants)
  - [Example: `Slab::from_raw_parts()`](#example-slabfrom_raw_parts)
  - [`decreases` — Loop and Recursion Termination](#decreases--loop-and-recursion-termination)
- [4. Understanding Triggers](#4-understanding-triggers)
- [5. Integer Types in Specifications](#5-integer-types-in-specifications)
  - [`int` and `nat`](#int-and-nat)
  - [Arithmetic Never Overflows in Ghost Code](#arithmetic-never-overflows-in-ghost-code)
- [6. Understanding Spec Files](#6-understanding-spec-files)
  - [`open spec fn` vs. `closed spec fn`](#open-spec-fn-vs-closed-spec-fn)
  - [`uninterp spec fn`](#uninterp-spec-fn)
  - [The View Pattern](#the-view-pattern)
  - [Invariant Functions (`inv()`)](#invariant-functions-inv)
  - [`recommends`](#recommends)
  - [`assume_specification`](#assume_specification)
- [7. Erased vs. Compiled Code](#7-erased-vs-compiled-code)
  - [`external_body`](#external_body)
  - [`external_type_specification`](#external_type_specification)
- [8. Common Proof Patterns](#8-common-proof-patterns)
  - [`assert ... by { ... }`](#assert--by---)
  - [`assert forall ... implies ... by`](#assert-forall--implies--by)
  - [Specialized Solvers](#specialized-solvers)
  - [`broadcast use`](#broadcast-use)
- [9. Common Verus Vocabulary](#9-common-verus-vocabulary)
- [10. Common Pitfalls for Readers](#10-common-pitfalls-for-readers)
  - [1. Confusing `int` with `i32` / `isize`](#1-confusing-int-with-i32--isize)
  - [2. Thinking `spec fn` runs at runtime](#2-thinking-spec-fn-runs-at-runtime)
  - [3. Confusing `requires` with `assert!`](#3-confusing-requires-with-assert)
  - [4. Misreading `old(x)` scope](#4-misreading-oldx-scope)
  - [5. Fearing `external_body`](#5-fearing-external_body)
  - [6. Overlooking `assume` in proofs](#6-overlooking-assume-in-proofs)
- [11. Navigating a Verified Crate](#11-navigating-a-verified-crate)

---

## 1. File Layout Convention

Verified crates **commonly** split their source into up to three files. Not every crate
has all of them — some omit `lib.proof.rs`, others add a `lib.test.rs` for Verus-only
tests.

When present, each file has a distinct purpose:

| File           | Purpose                                        | Must read?              |
|----------------|------------------------------------------------|-------------------------|
| `lib.rs`       | Implementation with `#[verus_spec]` attributes | **Yes**                 |
| `lib.spec.rs`  | Specification material                         | **Yes** (for contracts) |
| `lib.proof.rs` | Verification material (proofs, lemmas)         | When checking proofs    |

Spec and proof files are pulled into `lib.rs` with conditional includes:

```rust
// Include specifications (when present).
#[cfg(verus_keep_ghost)]
include!("lib.spec.rs");
// Include proofs (when present).
#[cfg(verus_keep_ghost)]
include!("lib.proof.rs");
```

The `verus_keep_ghost` cfg flag is only active during verification. In a normal
`cargo build` these files are not compiled at all, so they have zero impact on the
produced binary.

---

## 2. Reading `#[verus_spec(...)]` on Mainline Code

The `#[verus_spec(...)]` attribute attaches a formal contract to a regular `pub fn`.
A contract has two parts:

- **`requires`** — *Precondition*: what the caller **must guarantee** before calling
  the function. If the caller violates a `requires` clause, the function makes no
  promises.
- **`ensures`** — *Postcondition*: what the function **promises** when it returns,
  assuming all preconditions were met.

Three additional constructs appear inside contracts:

- `old(x)` — the value of any `&mut` parameter `x` (including `self`) *before* the method body
  executes. Used to compare pre-state with post-state.
- `@` (view operator) — accesses the abstract model of a concrete
  struct (see [Section 6](#6-understanding-spec-files)). For example,
  `self@` returns the `SlabView` of a `Slab`.
- `result` — refers to the return value in an `ensures` clause. Named
  with `result =>` at the start of the attribute.

### Example: `Slab::allocate()`

From `src/libs/slab/src/lib.rs`:

```rust
#[verus_spec(result =>
    requires
        old(self).inv(),
    ensures
        self.inv(),
        match result {
            Ok(ptr) => {
                let addr = ptr as usize;
                &&& old(self)@.free_addrs.contains(addr)
                &&& addr % self@.block_size == 0
                &&& self@ == SlabView {
                    allocated_addrs: old(self)@.allocated_addrs.insert(addr),
                    free_addrs: old(self)@.free_addrs.remove(addr),
                    ..old(self)@
                }
            },
            Err(_) => {
                &&& old(self)@.free_addrs == Set::<usize>::empty()
                &&& self@ == old(self)@
            },
        },
)]
pub fn allocate(&mut self) -> Result<*mut u8, Error>
```

Reading this contract:

1. **Requires** `old(self).inv()` — the slab must satisfy its invariant before the call.
2. **Ensures** `self.inv()` — the invariant is preserved after the call.
3. On `Ok(ptr)`:
   - The returned address **was** in the free set (`old(self)@.free_addrs.contains(addr)`).
   - The address is block-aligned (`addr % self@.block_size == 0`).
   - The new abstract state moves the address from `free_addrs` to `allocated_addrs`,
     leaving everything else unchanged (`..old(self)@`).
4. On `Err(_)`:
   - The free set was already empty — there was nothing to allocate.
   - The slab is unchanged (`self@ == old(self)@`).

The `deallocate()` contract mirrors `allocate()`, moving the pointer back from
`allocated_addrs` to `free_addrs`.

### Example: `RawArray::new()`

From `src/libs/raw-array/src/lib.rs`:

```rust
#[verus_verify(external_body)]
#[verus_spec(result =>
    requires
        len > 0,
        len < i32::MAX as usize,
        len * vstd::layout::size_of::<T>() + vstd::layout::align_of::<T>() - 1
            <= isize::MAX as usize,
    ensures
        match result {
            Ok(me) => {
                &&& me.inv()
                &&& me@.len() == len
                &&& forall|i: int| 0 <= i < len ==> is_zero(#[trigger] me@[i])
            },
            Err(e) => e.code == ErrorCode::OutOfMemory,
        },
)]
pub fn new(len: usize) -> Result<RawArray<T>, Error>
```

Reading this contract:

1. **Requires**: the length is positive, bounded, and the total byte size fits in memory.
2. **Ensures** on `Ok`: the invariant holds, the view has the requested length, and every
   element satisfies `is_zero` (all elements are zero-initialized).
3. **Ensures** on `Err`: the error code is `OutOfMemory`.

---

## 3. Reading Loop Invariants

A loop invariant is attached to a `for` loop with `#[verus_spec(invariant ...)]`. It
lists properties that hold **at the start of every iteration** (including the first
iteration and the point just after the loop ends).

### Example: `Slab::from_raw_parts()`

From `src/libs/slab/src/lib.rs`:

```rust
#[verus_spec(
    invariant
        index.inv(),
        index@.num_bits == index_len * u8_bits,
        index@.set_bits == Set::new(|j: int| num_data_blocks <= j < i),
)]
for i in num_data_blocks..(index_len * u8_bits) {
    index.set(i)?;
}
```

Reading this invariant:

1. `index.inv()` — the bitmap invariant is maintained throughout the loop.
2. `index@.num_bits == index_len * u8_bits` — the bitmap size does not change.
3. `index@.set_bits == Set::new(|j: int| num_data_blocks <= j < i)` — after processing
   loop index `i`, exactly the bits in range `[num_data_blocks, i)` have been set. This
   tracks the progress of marking upper bits as "in use".

The `verus_spec` attribute is active during verification and compiles away in normal
builds, so it does not need a `#[cfg_attr(verus_keep_ghost, ...)]` wrapper.

### `decreases` — Loop and Recursion Termination

Verus requires a `decreases` clause on loops and recursive functions to prove they
terminate. The clause names an expression that gets strictly smaller on every iteration
(or recursive call) and is bounded below — typically a natural number.

From `src/libs/bitmap/src/lib.rs` (simplified):

```rust
#[verus_spec(
    invariant
        0 <= i <= number_of_bits,
        // …
    decreases number_of_bits - i,
)]
while i < number_of_bits {
    // loop body
}
```

Reading this: the expression `number_of_bits - i` is a non-negative integer that shrinks
by at least 1 each iteration, so the loop must terminate. You can safely ignore
`decreases` when reading contracts — it is a proof obligation, not a functional property.

---

## 4. Understanding Triggers

Triggers help the verification of quantifier-related properties. When you are reading
contracts to understand what a function does, **you can safely ignore `#[trigger]` and
`#![auto]` annotations** — they do not change the *meaning* of the contract, only how the
SMT solver searches for a proof.

---

## 5. Integer Types in Specifications

Verus extends Rust's integer types with two specification-only types. Understanding them
is essential for reading contracts.

### `int` and `nat`

| Type                    | Range                                              | Ghost-only? |
|-------------------------|----------------------------------------------------|-------------|
| `int`                   | All mathematical integers ($-\infty$ to $+\infty$) | Yes         |
| `nat`                   | Non-negative integers ($0$ to $+\infty$)           | Yes         |
| `u8`, `u32`, `usize`, … | Fixed-width, bounded                               | No          |

`int` is the default type for specification-level arithmetic. Contracts and spec functions
use `int` liberally because it avoids overflow reasoning entirely.

### Arithmetic Never Overflows in Ghost Code

In executable Rust, `x + y` on two `u8` values can overflow. In ghost code (contracts,
spec functions), `+`, `-`, and `*` are **widened to `int`** and never overflow:

```rust
// In a requires clause, this addition is mathematical — no overflow.
requires
    x + y < 256,  // x: u8, y: u8, but x + y has type int here
```

This means you can compare values of different integer types freely inside contracts
(e.g., comparing a `u8` to a `usize`). The `as int` and `as nat` coercions convert
fixed-width values to mathematical integers explicitly when needed.

---

## 6. Understanding Spec Files

Spec files (`lib.spec.rs`) define the **abstract model** that contracts refer to via the
`@` operator. There are several key concepts.

### `open spec fn` vs. `closed spec fn`

- **`open spec fn`** — the body is *visible* outside the module (transparent).
  Anyone can reason about the body directly.
- **`closed spec fn`** — the body is *opaque* outside the module. Only code within
  the defining module can see inside. External callers must rely on lemmas to learn
  about the result.

### `uninterp spec fn`

An **uninterpreted** spec function has no body at all — it is purely abstract. The
verifier knows nothing about it except what is stated in lemmas and axioms.

From `src/libs/raw-array/src/lib.spec.rs`:

```rust
pub uninterp spec fn is_zero<T>(value: T) -> bool;
```

The function `is_zero` has no definition. Its meaning is established only through the
contracts that use it (e.g., `RawArray::new()` ensures every element satisfies `is_zero`).
This is useful when the concept is defined axiomatically rather than constructively.

### The View Pattern

The View pattern maps a concrete struct to an abstract model. Verus defines a `View`
trait; implementing it allows the `@` operator to work on your type.

For example, `Slab` has a corresponding `SlabView` (in `src/libs/slab/src/lib.spec.rs`):

```rust
#[verifier::ext_equal]
pub struct SlabView {
    pub block_size: usize,
    pub start_addr: usize,
    pub end_addr: usize,
    pub allocated_addrs: Set<usize>,
    pub free_addrs: Set<usize>,
}
```

Fields like `allocated_addrs: Set<usize>` and `free_addrs: Set<usize>` are *ghost*
collections. They exist only at the specification level and describe the abstract state of
the slab. Writing `self@` on a `Slab` returns this `SlabView`.

Other crates use simpler models — for instance, `RawArray<T>` maps to `Seq<T>`, so
`self@` is just a sequence of elements.

### Invariant Functions (`inv()`)

An `inv()` function expresses *what must always be true* about a data structure. Contracts
typically require `self.inv()` as a precondition and ensure it as a postcondition,
guaranteeing the structure is always in a valid state.

**Example:** `SlabView::inv()` from `src/libs/slab/src/lib.spec.rs`:

```rust
impl SlabView {
    pub open spec fn inv(&self) -> bool {
        &&& self.block_size > 0
        &&& self.start_addr % self.block_size == 0
        &&& self.end_addr % self.block_size == 0
        &&& self.end_addr > self.start_addr
        &&& forall|i| #[trigger] self.allocated_addrs.contains(i) ==> {
            &&& self.start_addr <= i < self.end_addr
            &&& i % self.block_size == 0
        }
        &&& forall|i| #[trigger] self.free_addrs.contains(i) ==> {
            &&& self.start_addr <= i < self.end_addr
            &&& i % self.block_size == 0
        }
        &&& self.allocated_addrs.disjoint(self.free_addrs)
    }
}
```

Reading this invariant: block size is positive; addresses are properly aligned and within
bounds; every address in either set is aligned and in range; and the two sets never
overlap.

### `recommends`

A `recommends` clause is a *soft precondition* on a spec function. Unlike `requires`
(which applies to exec and proof functions), `recommends` is only checked when a proof
fails — in that case the verifier generates a warning about any violated `recommends`
clauses to help diagnose the failure. It documents the intended domain of a spec function.

From `src/libs/raw-array/src/lib.spec.rs`:

```rust
pub open spec fn index(&self, i: int) -> T
    recommends
        0 <= i < self.len() as int,
```

Reading this: indexing outside `[0, len)` is not *prohibited* (spec functions are total),
but the result is meaningless. Think of `recommends` as "this function only makes sense
when these conditions hold."

### `assume_specification`

Spec files may contain `assume_specification` declarations. These provide trusted specs
for functions defined *outside* the crate (e.g., standard library pointer operations).
For example, in `src/libs/slab/src/lib.spec.rs`:

```rust
pub assume_specification<T: Sized> [ <*mut T>::add ] (p: *mut T, count: usize)
    -> (result: *mut T)
    requires
        p as usize + count * size_of::<T>() <= usize::MAX,
        count * size_of::<T>() <= isize::MAX,
    ensures
        result as usize == p as usize + count * size_of::<T>(),
;
```

This tells the verifier: "trust that `ptr.add(count)` returns the address `ptr + count *
size_of::<T>()`, provided the preconditions hold." It also tells the verifier to enforce
that any verified code calling this function must satisfy the preconditions.

---

## 7. Erased vs. Compiled Code

A key question when reading verified code: *does this affect the binary?*

| Construct                                       | Compiled? | Runtime cost     |
|-------------------------------------------------|-----------|------------------|
| Regular `fn` bodies                             | **Yes**   | Normal Rust cost |
| `spec fn` / `open spec fn` / `closed spec fn`   | No        | Zero — erased    |
| `proof fn`                                      | No        | Zero — erased    |
| `ghost` / `tracked` variables                   | No        | Zero — erased    |
| `#[verus_spec(...)]` attributes                 | No        | Zero — erased    |
| `proof! { ... }` / `proof_decl! { ... }` blocks | No        | Zero — erased    |
| `Set<T>`, `Seq<T>` (ghost collections)          | No        | Zero — erased    |

### `external_body`

When a function carries `#[verus_verify(external_body)]`, its body is *trusted* — the
verifier does not look inside. Instead, it assumes the attached `#[verus_spec(...)]`
contract is correct. You will see this on functions that call FFI, allocators, or standard
library routines that Verus cannot verify directly (see the `RawArray::new()` example in
[Section 2](#2-reading-verus_spec-on-mainline-code)).

### `external_type_specification`

Provides a Verus spec for a type defined outside the crate. For instance,
`RawArrayStorage` is defined without Verus, but a verification wrapper is declared:

```rust
#[verus_verify]
#[cfg(verus_keep_ghost)]
#[verus_verify(external_type_specification)]
#[verus_verify(external_body)]
pub struct ExRawArrayStorage<T>(RawArrayStorage<T>);
```

This exists only during verification (`cfg(verus_keep_ghost)`) and disappears in a normal
build.

---

## 8. Common Proof Patterns

You will encounter these patterns in `lib.proof.rs` files and occasionally in `lib.rs`.
You do not need to understand them deeply, but knowing what they *do* helps when scanning
verified code.

### `assert ... by { ... }`

An `assert(F) by { P }` proves fact `F` using the proof steps in `P`, then *discards* all
intermediate facts from `P` so they do not pollute the rest of the function. This is a
common pattern for keeping proofs modular:

```rust
assert(x * y <= 64) by {
    // proof steps here — only visible for proving x * y <= 64
};
// x * y <= 64 is known here, but nothing else from the block
```

### `assert forall ... implies ... by`

Proves a universal statement by bringing the quantified variable into scope:

```rust
assert forall|i: int| #![auto] self.set_bits.contains(i)
    implies full_range.contains(i) by {
    // proof that uses i — the solver assumes set_bits.contains(i)
}
```

Reading this: the block proves that for every `i` in `set_bits`, `i` is also in
`full_range`. The keyword `implies` replaces `==>` inside `assert forall`.

### Specialized Solvers

By default, the SMT solver is instructed not to reason about nonlinear arithmetic or
bitwise operations. Verus provides opt-in specialized solvers:

| Solver                | Use case                                                           |
|-----------------------|--------------------------------------------------------------------|
| `by(nonlinear_arith)` | Products, divisions, modular arithmetic with non-constant operands |
| `by(bit_vector)`      | Bitwise operations (`&`, `\|`, `^`, `<<`, `>>`)                    |
| `by(integer_ring)`    | Modular congruence equalities                                      |

Examples from the codebase:

```rust
// Nonlinear: proving a product bound
assert(num_index_blocks * block_size < total_num_blocks * block_size <= len)
    by (nonlinear_arith)
    requires
        num_index_blocks < total_num_blocks,
        block_size > 0,
        total_num_blocks * block_size <= len;

// Bit-vector: proving a bitmask property
assert((new_byte & (1u8 << shift)) != 0) by (bit_vector)
    requires
        0 <= shift < 8,
        new_byte == old_byte | (1u8 << shift);
```

The `requires` inside the `by(...)` block explicitly supplies context to the specialized
solver, which starts with an empty context (no ambient facts).

### `broadcast use`

A `broadcast use` declaration activates lemmas (proven facts) so they are automatically
available throughout the enclosing scope, without calling them explicitly:

```rust
broadcast use vstd::layout::layout_of_primitives, vstd::layout::align_of_u8;
```

Reading this: the listed lemmas are "always on" in the current module. You may encounter
this at the top of proof files.

---

## 9. Common Verus Vocabulary

Quick-reference table of terms you will encounter in verified crates:

| Term                           | Meaning                                                        |
|--------------------------------|----------------------------------------------------------------|
| `requires`                     | Precondition — what the caller must guarantee                  |
| `ensures`                      | Postcondition — what the function promises on return           |
| `old(x)`                       | The pre-call value of any `&mut` parameter `x`                 |
| `@` / `self@`                  | View operator — accesses the abstract model of a struct        |
| `result =>`                    | Refers to the return value as `result` in the postcondition    |
| `inv()`                        | Invariant (convention) — a spec fn that must hold              |
| `forall\|x\| P(x)`             | Universal quantifier — P holds for every x                     |
| `exists\|x\| P(x)`             | Existential quantifier — P holds for at least one x            |
| `choose\|x\| P(x)`             | Pick a witness x satisfying P (Hilbert choice operator)        |
| `#[trigger]`                   | Marks the trigger pattern for quantifier instantiation         |
| `#![auto]`                     | Accepts the auto-chosen trigger (suppresses diagnostic note)   |
| `Set<T>`, `Seq<T>`, `Map<K,V>` | Ghost collections — erased at runtime                          |
| `int` / `nat`                  | Mathematical integer types — ghost-only, never overflow        |
| `=~=`                          | Extensional equality — two collections are equal element-wise  |
| `&&&`                          | Conjunction in spec-mode (like `&&` for chaining conditions)   |
| `\|\|\|`                       | Disjunction operator used in spec-mode (like `\|\|`)           |
| `==>`                          | Implication operator (`A ==> B` means "if A then B")           |
| `implies`                      | Same as `==>`, used inside `assert forall` blocks              |
| `by (nonlinear_arith)`         | Delegates to a nonlinear arithmetic solver                     |
| `by (bit_vector)`              | Delegates to a bit-vector solver                               |
| `by (integer_ring)`            | Delegates to ring-theory solver (modular arithmetic)           |
| `assert ... by { ... }`        | Proves a fact using an isolated proof block                    |
| `assert forall ... by`         | Proves a universal statement with quantified variable in scope |
| `decreases`                    | Termination measure for loops and recursive functions          |
| `recommends`                   | Used for debugging proof errors                                |
| `broadcast use`                | Activates lemmas as ambient facts in the current scope         |
| `spec fn`                      | A specification function — erased at compile time              |
| `proof fn`                     | A function that only exists during verification (a lemma)      |
| `open spec fn`                 | Spec function whose body is visible outside the module         |
| `closed spec fn`               | Spec function whose body is opaque outside the module          |
| `uninterp spec fn`             | Uninterpreted spec function — no body, purely abstract         |
| `assume_specification`         | Provides a trusted spec for an external function               |
| `external_body`                | The function body is trusted, not verified                     |
| `external_type_specification`  | Provides a Verus spec for a type defined outside the crate     |
| `ghost` / `tracked`            | Variables that exist only during verification                  |
| `proof! { ... }`               | Block of proof code executed only during verification          |
| `proof_decl! { ... }`          | Declares erased variables inside an exec context               |
| `axiom`                        | A universally trusted fact — assumed correct without proof     |

---

## 10. Common Pitfalls for Readers

These are recurring sources of confusion based on community experience with Verus:

### 1. Confusing `int` with `i32` / `isize`

Contracts use `int` (unbounded mathematical integer) by default. When you see
`x + y < 256` in a `requires` clause, the addition is on `int` and **cannot overflow**.
Do not assume the same overflow semantics as Rust's `u8` or `u32`.

### 2. Thinking `spec fn` runs at runtime

Nothing in a spec file (`lib.spec.rs`) or proof file (`lib.proof.rs`) ever executes.
`spec fn`, `proof fn`, `ghost` variables, `Set<T>`, `Seq<T>` are all erased. The
`verus_keep_ghost` cfg gate ensures they are not even compiled in a normal build.

### 3. Confusing `requires` with `assert!`

`requires` is a **static** contract checked by the verifier at compile time. Rust's
`assert!()` is a **runtime** check. Verus's `assert()` (no `!`) is also static —
it asks the SMT solver to prove a fact, with zero runtime cost.

### 4. Misreading `old(x)` scope

`old(x)` captures any `&mut` parameter `x` (including `self`) at function entry, not at
the previous statement. In an `ensures` clause, `self` refers to the post-state and
`old(self)` to the pre-state. There is no "previous line" snapshot.

### 5. Fearing `external_body`

`external_body` does **not** mean the function is buggy or unverified. It means the
verifier trusts the body and relies on the attached contract. This is the standard
pattern for FFI, allocators, and I/O — the trust boundary is explicit and intentional.

### 6. Overlooking `assume` in proofs

`assume(P)` introduces fact `P` **without proof**. Treat it as a deliberate trust boundary:
check that the assumed fact is narrow, documented, and justified. (By contrast, `assert(P)`
is safe because the verifier must prove `P`.)

---

## 11. Navigating a Verified Crate

When you open a verified crate for the first time:

1. **Read `lib.rs`** — scan the public API and the `#[verus_spec]` contracts on each
   method. The contract (`requires` + `ensures`) is the authoritative specification;
   you do not need to read the function body.
2. **Open `lib.spec.rs`** — look at the View struct (e.g., `SlabView`) and, if present,
   its `inv()` function to understand what `self@` fields mean in contracts.
3. **Check for `uninterp spec fn`** — these define abstract concepts (like `is_zero`)
   with no body. Their meaning comes purely from the contracts that reference them.
4. **Open `lib.proof.rs` when needed** — especially for trust boundaries and helper lemmas.
5. **Grep for `assume(`** — each assumption should be narrow and justified.
