# C Header Generator

`gen-headers` combines Rust ABI declarations with TOML header specifications and writes the public
headers under `include/`. Header specifications remain responsible for include order, guards,
section order, C-only declarations, and the exact set of Rust symbols exported to C.

## Rust Exports

Rust exports are opt-in. A public Rust item is never emitted unless its fully qualified module path
appears in a header specification.

Constants use array tables:

```toml
[[rust_constants]]
path = "pthread::PTHREAD_CREATE_JOINABLE"
comment = "Thread is created joinable."
```

Constant exports support the following metadata:

- `c_name`: overrides the Rust identifier used in C.
- `comment`: overrides the Rust documentation summary.
- `guard`: wraps the macro in `#ifndef`.
- `c_value`: preserves an exact C replacement spelling while retaining the Rust source binding.
- `c_declaration`: preserves a complete exact C declaration while retaining the Rust binding.
- `c_bindings`: maps additional C names inside an exact declaration to public Rust paths.
- `unchecked_c_value_reason`: explains a C value representation that cannot be compared
  mechanically with the Rust expression.
- `c_only_reason`: classifies genuinely C-only extra names inside an exact declaration.
- `no_comment`: suppresses Rust documentation for one export.
- `emit = false`: registers a source/dependency binding without emitting another declaration.
- `group` and `section_title`: place constants in named `rust_constants:<group>` sections.

`rust_constant_docs = false` disables implicit Rust documentation for an entire header. Constant
expressions support integer, character, and string literals; unary, binary, bitwise, and
parenthesized expressions; aliases to other constants exported by the same header; and casts to
known C types. Rust integer suffixes are removed, octal literals are translated to C syntax, and
unsupported expressions fail with a diagnostic.

Types also use array tables:

```toml
[[rust_types]]
path = "sys_types::pthread_mutexattr_t"
field_renames = { type_ = "type" }
packed = false
```

The generator accepts public aliases and public `repr(C)` or `repr(C, packed)` structs and unions.
Parameterized packing such as `repr(packed(2))` and unsupported representation modifiers fail with
an actionable diagnostic instead of being approximated in C. Explicit `repr(align(N))` is indexed
and emitted as a C alignment attribute for direct declarations.
The following type options are available:

- `c_name`: overrides the Rust identifier used in C.
- `style`: selects `typedef` (default), `tag`, or `typedef_tag` output.
- `tag_name`: sets the tag used by `typedef_tag`.
- `comment`: overrides the Rust documentation summary.
- `field_renames`: maps Rust field identifiers to C identifiers.
- `field_comments`: overrides field documentation summaries.
- `c_type`: preserves an exact C spelling for an alias's underlying type.
- `c_declaration`: preserves a complete C declaration while binding it to the selected Rust type.
- `c_bindings`: maps additional C type names inside an exact declaration to public Rust paths.
- `c_only_reason`: classifies genuinely C-only extra names such as typedef guards or member aliases.
- `field_c_declarations`: supplies exact C declarators for fields whose public C view intentionally
  differs from the Rust spelling.
- `declarator_suffix`: appends syntax such as `[1]` to a typedef declarator.
- `guard`: wraps the declaration in `#ifndef`.
- `no_comment`: suppresses Rust documentation for one type.
- `emit = false`: registers a source/dependency binding without emitting another declaration.
- `group` and `section_title`: place types in named `rust_types:<group>` sections.
- `packed`: controls whether the C declaration spells `__attribute__((packed))`; by default it
  follows Rust.
- `forward_declare`: emits a tagged forward declaration and permits pointer cycles.

`rust_type_docs = false` disables implicit Rust type and field documentation for a header.

Selected types are emitted in dependency order. Type aliases, scalar fields, pointers,
pointer-to-pointer fields, arrays, function pointers, and nested type paths use the fixed i686
Nanvix guest ABI. `Option<extern "C" fn>` is accepted as a nullable C function pointer; other
`Option<T>` forms are rejected because their Rust layout is not generally a C declaration. Target-
specific Rust definitions are selected as 32-bit x86, little-endian, `target_os = "nanvix"`, and
`target_family = "unix"` declarations rather than using the host ABI.

Use `rust_constants`, `rust_types`, `rust_constants:<group>`, and `rust_types:<group>` in
`content_order` to place generated declarations alongside `macros`, `types`, `section:N`, and
`raw:N` blocks. `symbol_crates` adds supplemental Rust crates under qualified paths such as
`sysapi::time::timespec`; the owning crate remains unqualified.

## C Escape Layer

Keep declarations that do not have a direct Rust constant or type equivalent in TOML:

- `macros` for C-only object-like macros;
- `types` for raw C type declarations and conditional typedef guards;
- `raw_sections` for function-like macros, aggregate initializers, feature-test machinery, and
  include-cycle handling;
- `overrides` for C function declarations that cannot be recovered exactly from Rust signatures;
- `trailer` for text that must appear outside the normal generated body.

Raw object-like macros and C type declarations are audited, including trailers and exact declaration
blocks. They must either have matching `rust_constants`/`rust_types` bindings (often `emit = false`
for exact legacy presentation) or the containing block/structured macro must provide a concrete
`c_only_reason`. Additional names in exact blocks require explicit `c_bindings`. Source-bound raw
macro values are compared with translated Rust expressions or explicit `c_value` spellings; an
`unchecked_c_value_reason` is required for non-comparable pointer or aggregate representations.
Typical C-only reasons are function-like macros, compiler builtins, aggregate initializers,
lvalue/member aliases, feature-test conditions, and global declaration mechanics. This prevents
duplicated ABI constants and types from silently returning to TOML.

An unsupported Rust expression or type must use one of these explicit C representations. The
generator does not evaluate `const fn` calls, associated constants, aggregate initializers, or
layout expressions such as `size_of::<T>()`.

## Commands

Regenerate all headers:

```bash
cargo run --locked --quiet -p gen-headers
```

Check that committed headers are current:

```bash
cargo run --locked --quiet -p gen-headers -- --check
```

Pass `--header pthread.h` to restrict writes or comparisons to one output header.
