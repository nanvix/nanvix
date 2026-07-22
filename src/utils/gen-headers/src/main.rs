// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
//! Generates C headers from Nanvix `libc_*` Rust crate sources.
//!
//! Each `libc_*` crate that carries a `header.toml` specification owns exactly one C header under
//! `include/`. This tool parses `extern "C"` function signatures and explicitly selected constants
//! and C-compatible types with `syn`, combines them with the specification's C-only declarations
//! and layout, and renders the header. Output is byte-for-byte reproducible.
//==================================================================================================

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::{
    bail,
    Context,
    Result,
};
use ::clap::Parser;
use ::serde::Deserialize;
use ::std::{
    collections::{
        BTreeMap,
        BTreeSet,
        HashMap,
    },
    fs,
    path::{
        Path,
        PathBuf,
    },
};

//==================================================================================================
// CLI Definition
//==================================================================================================

/// Command-line flags accepted by the header generator.
#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Generate C headers from Nanvix libc_* Rust crate sources",
    long_about = None
)]
struct Cli {
    /// # Description
    /// Check that committed headers are up to date instead of writing them.
    #[arg(long)]
    check: bool,

    /// # Description
    /// Generate only the named header (e.g. `stdio.h`).
    #[arg(long, value_name = "NAME")]
    header: Option<String>,
}

fn default_extern_c() -> bool {
    true
}

fn default_emit() -> bool {
    true
}

fn default_rust_docs() -> bool {
    true
}

//==================================================================================================
// Specification Data Structures
//==================================================================================================

/// Complete specification for a single generated header.
#[derive(Debug, Deserialize)]
struct HeaderSpec {
    /// Output header path, relative to `include/`.
    file: String,
    /// One-line `@brief` for the `@file` documentation block.
    brief: String,
    /// Multi-line `@file` description.
    description: String,
    /// System headers to `#include`.
    #[serde(default)]
    includes: Vec<String>,
    /// Include guard macro name.
    guard: String,
    /// Whether to wrap declarations in an `extern "C"` block.
    #[serde(default = "default_extern_c")]
    extern_c: bool,
    /// Explicit ordering of content blocks. Defaults when absent.
    #[serde(default)]
    content_order: Option<Vec<String>>,
    /// `#define` constants.
    #[serde(default)]
    macros: Vec<Macro>,
    /// Constants selected from Rust sources and emitted as `#define` entries.
    #[serde(default)]
    rust_constants: Vec<RustConstantExport>,
    /// Whether Rust documentation summaries are emitted for constants by default.
    #[serde(default = "default_rust_docs")]
    rust_constant_docs: bool,
    /// Raw C type definitions.
    #[serde(default)]
    types: Vec<RawText>,
    /// C-compatible Rust types selected for export.
    #[serde(default)]
    rust_types: Vec<RustTypeSpec>,
    /// Whether Rust documentation summaries are emitted for types and fields by default.
    #[serde(default = "default_rust_docs")]
    rust_type_docs: bool,
    /// Titled groups of function declarations.
    #[serde(default)]
    sections: Vec<Section>,
    /// Titled blocks of raw C text.
    #[serde(default)]
    raw_sections: Vec<RawSection>,
    /// Manual declaration overrides, keyed by function name.
    #[serde(default)]
    overrides: HashMap<String, String>,
    /// Raw block emitted outside the include guard.
    #[serde(default)]
    trailer: Option<RawText>,
    /// Extra crates (directory names under `src/libs`) whose `src/**` trees are
    /// scanned *recursively* for the `extern "C"` signatures this header
    /// declares. Used by the POSIX headers, whose functions live in the
    /// `syscall`, `posix`, and `libc_*` crates rather than next to the spec.
    /// When empty, the spec's own crate `src/` directory is scanned (the
    /// `libc_*` convention).
    #[serde(default)]
    scan_crates: Vec<String>,
    /// Supplemental crates whose public Rust constants and types may be selected.
    /// Their symbols are qualified by the crate directory name.
    #[serde(default)]
    symbol_crates: Vec<String>,
}

/// A single `#define` constant.
#[derive(Debug, Deserialize)]
struct Macro {
    /// Macro name.
    name: String,
    /// Macro replacement value.
    value: String,
    /// Optional documentation comment.
    #[serde(default)]
    comment: Option<String>,
    /// Optional `#ifndef` redefinition guard.
    #[serde(default)]
    guard: Option<String>,
    /// Exact C declaration override.
    #[serde(default)]
    declaration: Option<String>,
    /// Why this object-like macro cannot be sourced from a Rust constant.
    #[serde(default)]
    c_only_reason: Option<String>,
}

/// A Rust constant selected for export, either by path alone or with C-specific overrides.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RustConstantExport {
    /// Fully qualified Rust path with default C spelling and documentation.
    Path(String),
    /// Fully qualified Rust path with explicit C metadata.
    Detailed(Box<RustConstantSpec>),
}

impl RustConstantExport {
    /// Returns the fully qualified Rust path of this export.
    fn path(&self) -> &str {
        match self {
            Self::Path(path) => path,
            Self::Detailed(spec) => &spec.path,
        }
    }

    /// Returns the C name of this export.
    fn c_name(&self) -> &str {
        match self {
            Self::Path(path) => path.rsplit("::").next().unwrap_or(path),
            Self::Detailed(spec) => spec
                .c_name
                .as_deref()
                .unwrap_or_else(|| spec.path.rsplit("::").next().unwrap_or(&spec.path)),
        }
    }

    /// Returns an optional C documentation override.
    fn comment(&self) -> Option<&str> {
        match self {
            Self::Path(_) => None,
            Self::Detailed(spec) => spec.comment.as_deref(),
        }
    }

    /// Returns an optional C preprocessor guard.
    fn guard(&self) -> Option<&str> {
        match self {
            Self::Path(_) => None,
            Self::Detailed(spec) => spec.guard.as_deref(),
        }
    }

    /// Returns an optional exact C declaration override.
    fn c_declaration(&self) -> Option<&str> {
        match self {
            Self::Path(_) => None,
            Self::Detailed(spec) => spec.c_declaration.as_deref(),
        }
    }

    /// Returns an optional exact C value override.
    fn c_value(&self) -> Option<&str> {
        match self {
            Self::Path(_) => None,
            Self::Detailed(spec) => spec.c_value.as_deref(),
        }
    }

    /// Returns whether documentation should be omitted.
    fn no_comment(&self) -> bool {
        match self {
            Self::Path(_) => false,
            Self::Detailed(spec) => spec.no_comment,
        }
    }

    /// Returns whether this binding emits C output.
    fn emit(&self) -> bool {
        match self {
            Self::Path(_) => true,
            Self::Detailed(spec) => spec.emit,
        }
    }

    /// Returns the named output group for this export.
    fn group(&self) -> &str {
        match self {
            Self::Path(_) => "",
            Self::Detailed(spec) => &spec.group,
        }
    }

    /// Returns an optional section title override.
    fn section_title(&self) -> Option<&str> {
        match self {
            Self::Path(_) => None,
            Self::Detailed(spec) => spec.section_title.as_deref(),
        }
    }

    /// Returns additional exact-declaration C names mapped to Rust paths.
    fn c_bindings(&self) -> Option<&BTreeMap<String, String>> {
        match self {
            Self::Path(_) => None,
            Self::Detailed(spec) => Some(&spec.c_bindings),
        }
    }
}

/// C-specific metadata for a Rust constant export.
#[derive(Debug, Deserialize)]
struct RustConstantSpec {
    /// Fully qualified Rust path.
    path: String,
    /// C macro name. Defaults to the Rust identifier.
    #[serde(default)]
    c_name: Option<String>,
    /// C documentation override. Defaults to the first Rust documentation paragraph.
    #[serde(default)]
    comment: Option<String>,
    /// Optional `#ifndef` redefinition guard.
    #[serde(default)]
    guard: Option<String>,
    /// Exact C replacement value. Rust remains the selected source symbol.
    #[serde(default)]
    c_value: Option<String>,
    /// Exact complete C declaration, including `#define`.
    #[serde(default)]
    c_declaration: Option<String>,
    /// Suppress the Rust documentation summary.
    #[serde(default)]
    no_comment: bool,
    /// Whether this source binding emits C output.
    #[serde(default = "default_emit")]
    emit: bool,
    /// Named output group used by `rust_constants:<group>` content entries.
    #[serde(default)]
    group: String,
    /// Section title for this output group.
    #[serde(default)]
    section_title: Option<String>,
    /// Additional object macro names in `c_declaration`, mapped to Rust constant paths.
    #[serde(default)]
    c_bindings: BTreeMap<String, String>,
    /// Why `c_value` cannot be mechanically compared with the Rust constant expression.
    #[serde(default)]
    unchecked_c_value_reason: Option<String>,
    /// Why additional unbound declarations in `c_declaration` are genuinely C-only.
    #[serde(default)]
    c_only_reason: Option<String>,
}

/// C declaration style for an exported Rust aggregate.
#[derive(Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum RustTypeStyle {
    /// An anonymous aggregate introduced by a typedef.
    #[default]
    Typedef,
    /// A named `struct` or `union` tag without a typedef.
    Tag,
    /// A named tag plus a typedef alias.
    TypedefTag,
}

/// C-specific metadata for a Rust type export.
#[derive(Debug, Deserialize)]
struct RustTypeSpec {
    /// Fully qualified Rust path.
    path: String,
    /// C type or typedef name. Defaults to the Rust identifier.
    #[serde(default)]
    c_name: Option<String>,
    /// C declaration style.
    #[serde(default)]
    style: RustTypeStyle,
    /// Tag name for `typedef_tag` declarations. Defaults to `c_name`.
    #[serde(default)]
    tag_name: Option<String>,
    /// Type documentation override.
    #[serde(default)]
    comment: Option<String>,
    /// Rust-field-to-C-field renames.
    #[serde(default)]
    field_renames: BTreeMap<String, String>,
    /// Per-field documentation overrides.
    #[serde(default)]
    field_comments: BTreeMap<String, String>,
    /// Whether to spell a packed attribute in C. Defaults to the Rust representation.
    #[serde(default)]
    packed: Option<bool>,
    /// Whether to emit a tagged forward declaration before this type definition.
    #[serde(default)]
    forward_declare: bool,
    /// Exact C spelling for the underlying type of a Rust alias.
    #[serde(default)]
    c_type: Option<String>,
    /// Exact C declaration for this selected Rust type.
    #[serde(default)]
    c_declaration: Option<String>,
    /// Exact C field declarations, keyed by Rust field name.
    #[serde(default)]
    field_c_declarations: BTreeMap<String, String>,
    /// Declarator suffix appended to an alias name, such as `[1]`.
    #[serde(default)]
    declarator_suffix: String,
    /// Optional `#ifndef` guard around this C type declaration.
    #[serde(default)]
    guard: Option<String>,
    /// Suppress the Rust documentation summary.
    #[serde(default)]
    no_comment: bool,
    /// Whether this source binding emits C output.
    #[serde(default = "default_emit")]
    emit: bool,
    /// Named output group used by `rust_types:<group>` content entries.
    #[serde(default)]
    group: String,
    /// Section title for this output group.
    #[serde(default)]
    section_title: Option<String>,
    /// Additional C type names in `c_declaration`, mapped to Rust type paths.
    #[serde(default)]
    c_bindings: BTreeMap<String, String>,
    /// Why additional unbound declarations in `c_declaration` are genuinely C-only.
    #[serde(default)]
    c_only_reason: Option<String>,
}

impl RustTypeSpec {
    /// Returns the C type or typedef name.
    fn c_name(&self) -> &str {
        self.c_name
            .as_deref()
            .unwrap_or_else(|| self.path.rsplit("::").next().unwrap_or(&self.path))
    }

    /// Returns the C tag name.
    fn tag_name(&self) -> &str {
        self.tag_name.as_deref().unwrap_or_else(|| self.c_name())
    }
}

/// A block of raw C text.
#[derive(Debug, Deserialize)]
struct RawText {
    /// Verbatim C text.
    text: String,
    /// Why unbound object constants or types in this block must remain C-only.
    #[serde(default)]
    c_only_reason: Option<String>,
}

/// A titled group of function declarations.
#[derive(Debug, Deserialize)]
struct Section {
    /// Section bar title.
    title: String,
    /// Functions declared in this section, in order.
    functions: Vec<String>,
}

/// A titled block of raw C text.
#[derive(Debug, Deserialize)]
struct RawSection {
    /// Section bar title. An empty title suppresses the bar.
    #[serde(default)]
    title: String,
    /// Verbatim C text.
    text: String,
    /// Why unbound object constants or types in this block must remain C-only.
    #[serde(default)]
    c_only_reason: Option<String>,
}

//==================================================================================================
// Parsed Function Signatures
//==================================================================================================

/// A parsed C function signature.
#[derive(Debug)]
struct FuncSig {
    /// Function name.
    name: String,
    /// Parameters as `(c_type, name)` pairs.
    params: Vec<(String, String)>,
    /// C return type.
    return_type: String,
    /// Whether the function is variadic.
    is_variadic: bool,
}

/// A public Rust constant discovered in a source file.
#[derive(Clone)]
struct RustConstant {
    /// Documentation collected from `#[doc = ...]` attributes.
    docs: String,
    /// Declared Rust type, used to preserve expression semantics in C.
    ty: Box<syn::Type>,
    /// Constant expression.
    expr: syn::Expr,
    /// Module containing the constant, used to resolve relative aliases.
    module_path: Vec<String>,
    /// Original fully qualified path, retained through public re-exports.
    canonical_path: String,
    /// Named imports visible where the constant is declared.
    imports: BTreeMap<String, String>,
    /// Glob-imported module paths visible where the constant is declared.
    glob_imports: Vec<String>,
    /// Whether this path is reachable through public modules or a public re-export.
    public: bool,
}

/// One field in a C-compatible Rust aggregate.
#[derive(Clone)]
struct RustField {
    /// Rust field name.
    name: String,
    /// Documentation collected from `#[doc = ...]` attributes.
    docs: String,
    /// Rust field type.
    ty: syn::Type,
}

/// Kind and contents of a public Rust type.
#[derive(Clone)]
enum RustTypeKind {
    /// A public type alias.
    Alias(Box<syn::Type>),
    /// A `repr(C)` structure.
    Struct(Vec<RustField>),
    /// A `repr(C)` union.
    Union(Vec<RustField>),
}

/// A public C-compatible Rust type discovered in a source file.
#[derive(Clone)]
struct RustType {
    /// Documentation collected from `#[doc = ...]` attributes.
    docs: String,
    /// Type declaration.
    kind: RustTypeKind,
    /// Module containing the type, used to resolve nested paths.
    module_path: Vec<String>,
    /// Original fully qualified path, retained through public re-exports.
    canonical_path: String,
    /// Named imports visible where the type is declared.
    imports: BTreeMap<String, String>,
    /// Glob-imported module paths visible where the type is declared.
    glob_imports: Vec<String>,
    /// Whether the Rust aggregate has `repr(packed)`.
    packed: bool,
    /// Explicit Rust alignment from `repr(align(N))`.
    alignment: Option<usize>,
    /// Whether this path is reachable through public modules or a public re-export.
    public: bool,
}

/// A public Rust `use` declaration resolved after all source modules are indexed.
struct RustReexport {
    /// Fully qualified source symbol or module path.
    source: String,
    /// Public module receiving the re-export.
    destination_module: Vec<String>,
    /// Destination name for a named re-export.
    alias: Option<String>,
    /// Whether this is a glob re-export.
    glob: bool,
    /// Whether the destination module is publicly reachable from the crate root.
    public: bool,
}

/// Public Rust symbols indexed by fully qualified module path.
#[derive(Default)]
struct RustSymbols {
    /// Public constants.
    constants: BTreeMap<String, RustConstant>,
    /// Public aliases and C-compatible aggregate types.
    types: BTreeMap<String, RustType>,
    /// Public re-exports awaiting resolution.
    reexports: Vec<RustReexport>,
    /// Module paths that are private within their parent modules.
    private_modules: BTreeSet<String>,
}

//==================================================================================================
// Rust-to-C Type Mapping
//==================================================================================================

/// Maps a Rust type identifier to its C equivalent, if known.
fn map_ident_to_c(ident: &str) -> Option<&'static str> {
    Some(match ident {
        // Core FFI types (sysapi).
        "c_char" => "char",
        "c_schar" => "signed char",
        "c_uchar" => "unsigned char",
        "c_short" => "short",
        "c_ushort" => "unsigned short",
        "c_int" => "int",
        "c_uint" => "unsigned int",
        "c_long" => "long",
        "c_ulong" => "unsigned long",
        "c_longlong" => "long long",
        "c_ulonglong" => "unsigned long long",
        "c_void" => "void",
        "c_size_t" => "size_t",
        "c_ssize_t" => "ssize_t",
        // Rust primitives that appear in extern "C" signatures.
        "i8" => "int8_t",
        "i16" => "int16_t",
        "i32" => "int32_t",
        "i64" => "int64_t",
        "u8" => "uint8_t",
        "u16" => "uint16_t",
        "u32" => "uint32_t",
        "u64" => "uint64_t",
        "f32" => "float",
        "f64" => "double",
        "bool" => "bool",
        "usize" => "size_t",
        "isize" => "ssize_t",
        // Domain-specific aliases (sysapi/sys_types.rs).
        "time_t" => "time_t",
        "clock_t" => "clock_t",
        "clockid_t" => "clockid_t",
        "off_t" => "off_t",
        "pid_t" => "pid_t",
        "mode_t" => "mode_t",
        "intmax_t" => "intmax_t",
        "uintmax_t" => "uintmax_t",
        "sigset_t" => "sigset_t",
        "wchar_t" => "wchar_t",
        "wint_t" => "wint_t",
        "wctype_t" => "wctype_t",
        "wctrans_t" => "wctrans_t",
        "locale_t" => "locale_t",
        "nl_item" => "nl_item",
        // Struct types.
        "tm" => "struct tm",
        "FILE" => "FILE",
        "lconv" => "struct lconv",
        "div_t" => "div_t",
        "ldiv_t" => "ldiv_t",
        "lldiv_t" => "lldiv_t",
        "imaxdiv_t" => "imaxdiv_t",
        "jmp_buf" => "jmp_buf",
        "sigaction" => "struct sigaction",
        "timespec" => "struct timespec",
        // Function pointer aliases.
        "SignalHandler" => "sighandler_t",
        // VaList -> va_list.
        "VaList" => "va_list",
        // POSIX scalar type aliases (sysapi/sys_types.rs et al.). The C typedefs
        // live in the generated headers' type blocks; these map the Rust alias
        // name to the identical C typedef name.
        "size_t" => "size_t",
        "ssize_t" => "ssize_t",
        "uid_t" => "uid_t",
        "gid_t" => "gid_t",
        "dev_t" => "dev_t",
        "ino_t" => "ino_t",
        "nlink_t" => "nlink_t",
        "blkcnt_t" => "blkcnt_t",
        "blksize_t" => "blksize_t",
        "suseconds_t" => "suseconds_t",
        "useconds_t" => "useconds_t",
        "socklen_t" => "socklen_t",
        "sa_family_t" => "sa_family_t",
        "in_port_t" => "in_port_t",
        "in_addr_t" => "in_addr_t",
        "nfds_t" => "nfds_t",
        "id_t" => "id_t",
        "key_t" => "key_t",
        // POSIX thread type aliases (sysapi/sys_types.rs, pthread.rs).
        "pthread_t" => "pthread_t",
        "pthread_attr_t" => "pthread_attr_t",
        "pthread_mutex_t" => "pthread_mutex_t",
        "pthread_mutexattr_t" => "pthread_mutexattr_t",
        "pthread_cond_t" => "pthread_cond_t",
        "pthread_condattr_t" => "pthread_condattr_t",
        "pthread_rwlock_t" => "pthread_rwlock_t",
        "pthread_rwlockattr_t" => "pthread_rwlockattr_t",
        "pthread_barrier_t" => "pthread_barrier_t",
        "pthread_barrierattr_t" => "pthread_barrierattr_t",
        "pthread_spinlock_t" => "pthread_spinlock_t",
        "pthread_key_t" => "pthread_key_t",
        "pthread_once_t" => "pthread_once_t",
        // POSIX unnamed semaphore (sysapi/sys_types.rs).
        "sem_t" => "sem_t",
        // POSIX struct types (the C side spells these `struct X`; the Rust side
        // refers to them by the bare struct name, possibly module-qualified —
        // `map_type_path` already reduces a path to its last segment).
        "stat" => "struct stat",
        "utsname" => "struct utsname",
        "sockaddr" => "struct sockaddr",
        "sockaddr_storage" => "struct sockaddr_storage",
        "sockaddr_in" => "struct sockaddr_in",
        "sockaddr_un" => "struct sockaddr_un",
        "in_addr" => "struct in_addr",
        "iovec" => "struct iovec",
        "msghdr" => "struct msghdr",
        "timeval" => "struct timeval",
        "tms" => "struct tms",
        "sched_param" => "struct sched_param",
        "rlimit" => "struct rlimit",
        "rusage" => "struct rusage",
        "dirent" => "struct dirent",
        "DIR" => "DIR",
        // The Nanvix directory-stream handle is the POSIX `DIR`.
        "DirectoryStream" => "DIR",
        // The dynamic-linker symbol-info struct is spelled `Dl_info_t` in C.
        "DlInfo" => "Dl_info_t",
        _ => return None,
    })
}

/// Converts a `syn` type to its C equivalent.
fn map_type(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Ptr(ptr) => {
            let is_const: bool = matches!(&ptr.mutability, syn::PointerMutability::Const(_));
            let inner: String = map_type(&ptr.elem);
            if inner == "void" {
                return if is_const {
                    "const void *".to_string()
                } else {
                    "void *".to_string()
                };
            }
            // Pointer to pointer.
            if inner.ends_with('*') {
                return if is_const {
                    format!("const {inner}*")
                } else {
                    format!("{inner}*")
                };
            }
            if is_const {
                format!("const {inner} *")
            } else {
                format!("{inner} *")
            }
        },
        syn::Type::Never(_) => "void".to_string(),
        syn::Type::FnPtr(fn_ptr) => map_fn_ptr(fn_ptr),
        syn::Type::Path(path) => map_type_path(path),
        // Strip a redundant outer parenthesis or group, then map the inner type.
        syn::Type::Paren(inner) => map_type(&inner.elem),
        syn::Type::Group(inner) => map_type(&inner.elem),
        _ => "/* UNMAPPED */".to_string(),
    }
}

/// Maps a path type, handling `VaList`, `Option<fn>`, and named aliases.
fn map_type_path(path: &syn::TypePath) -> String {
    let Some(seg) = path.path.segments.last() else {
        return "/* UNMAPPED */".to_string();
    };
    let ident: String = seg.ident.to_string();

    // VaList<'_, '_> and VaListImpl<'_> -> va_list.
    if ident == "VaList" || ident == "VaListImpl" {
        return "va_list".to_string();
    }

    // Option<unsafe extern "C" fn(...) -> T> (function pointer).
    if ident == "Option" {
        if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
            if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                return map_type(inner);
            }
        }
    }

    match map_ident_to_c(&ident) {
        Some(c) => c.to_string(),
        None => format!("/* UNMAPPED: {ident} */"),
    }
}

/// Maps a function pointer type to a C function pointer placeholder.
fn map_fn_ptr(fn_ptr: &syn::TypeFnPtr) -> String {
    let ret: String = match &fn_ptr.output {
        syn::ReturnType::Default => "void".to_string(),
        syn::ReturnType::Type(_, ty) => map_type(ty.as_ref()),
    };
    let params: Vec<String> = fn_ptr.inputs.iter().map(|arg| map_type(&arg.ty)).collect();
    let params_str: String = if params.is_empty() {
        "void".to_string()
    } else {
        params.join(", ")
    };
    format!("{ret} (*)({params_str})")
}

//==================================================================================================
// Rust Constant Translation
//==================================================================================================

/// Returns documentation text from Rust `#[doc = ...]` attributes.
fn rust_docs(attrs: &[syn::Attribute]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        if let syn::Meta::NameValue(meta) = &attr.meta {
            if let syn::Expr::Lit(expr) = &meta.value {
                if let syn::Lit::Str(value) = &expr.lit {
                    lines.push(value.value().trim().to_string());
                }
            }
        }
    }
    lines.join("\n").trim().to_string()
}

/// Returns the first prose paragraph from Rust documentation.
fn rust_doc_summary(docs: &str) -> Option<String> {
    let mut summary: Vec<&str> = Vec::new();
    let mut started: bool = false;
    for line in docs.lines().map(str::trim) {
        if line.starts_with('#') {
            continue;
        }
        if line.is_empty() {
            if started {
                break;
            }
            continue;
        }
        started = true;
        summary.push(line);
    }
    (!summary.is_empty()).then(|| summary.join(" "))
}

/// Converts a Rust integer literal to C syntax.
fn translate_integer_literal(literal: &syn::LitInt) -> Result<String> {
    let raw: String = literal.to_string();
    let suffix: &str = literal.suffix();
    let digits: &str = raw
        .strip_suffix(suffix)
        .context("failed to remove Rust integer suffix")?;
    let digits: String = digits.replace('_', "");

    if let Some(octal) = digits.strip_prefix("0o") {
        return Ok(format!("0{octal}"));
    }
    if let Some(binary) = digits.strip_prefix("0b") {
        let value: u128 = u128::from_str_radix(binary, 2)
            .with_context(|| format!("invalid Rust binary literal `{raw}`"))?;
        return Ok(value.to_string());
    }
    Ok(digits)
}

/// Returns the C type encoded by a Rust integer-literal suffix, when present.
fn integer_literal_c_type(literal: &syn::LitInt) -> Result<Option<String>> {
    let suffix: &str = literal.suffix();
    if suffix.is_empty() {
        return Ok(None);
    }
    map_ident_to_c(suffix)
        .map(str::to_string)
        .map(Some)
        .with_context(|| format!("unsupported Rust integer suffix `{suffix}`"))
}

/// Escapes one byte for use in a C character or string literal.
fn escape_c_byte(byte: u8, quote: u8) -> String {
    match byte {
        b'\0' => "\\000".to_string(),
        b'\n' => "\\n".to_string(),
        b'\r' => "\\r".to_string(),
        b'\t' => "\\t".to_string(),
        b'\\' => "\\\\".to_string(),
        value if value == quote => format!("\\{}", quote as char),
        0x20..=0x7e => (byte as char).to_string(),
        _ => format!("\\{:03o}", byte),
    }
}

/// Converts an ASCII Rust character to a C character literal.
fn translate_char_literal(value: char) -> Result<String> {
    if !value.is_ascii() {
        bail!("non-ASCII Rust character constants require an explicit C override");
    }
    Ok(format!("'{}'", escape_c_byte(value as u8, b'\'')))
}

/// Converts an ASCII Rust string to a C string literal.
fn translate_string_literal(value: &str) -> Result<String> {
    if !value.is_ascii() {
        bail!("non-ASCII Rust string constants require an explicit C override");
    }
    let escaped: String = value
        .bytes()
        .map(|byte| escape_c_byte(byte, b'\"'))
        .collect();
    Ok(format!("\"{escaped}\""))
}

/// Resolves a Rust path relative to the module containing a constant.
fn resolve_constant_path(path: &syn::Path, module_path: &[String]) -> String {
    let segments: Vec<String> = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    if segments.is_empty() {
        return String::new();
    }

    let mut resolved: Vec<String> = module_path.to_vec();
    let mut index: usize = 0;
    if segments[0] == "crate" {
        resolved.clear();
        index = 1;
    } else if segments[0] == "self" {
        index = 1;
    } else {
        while segments
            .get(index)
            .is_some_and(|segment| segment == "super")
        {
            resolved.pop();
            index += 1;
        }
    }
    resolved.extend(segments[index..].iter().cloned());
    resolved.join("::")
}

/// Evaluates one `cfg` predicate for the fixed i686 Nanvix guest target.
fn cfg_matches_guest(meta: &syn::Meta) -> Result<bool> {
    match meta {
        syn::Meta::Path(path) => {
            let predicate: String = path
                .get_ident()
                .map(ToString::to_string)
                .context("unsupported compound cfg path in Rust ABI item")?;
            match predicate.as_str() {
                "unix" => Ok(true),
                "windows" | "test" | "debug_assertions" => Ok(false),
                _ => bail!("unsupported cfg predicate `{predicate}` in Rust ABI item"),
            }
        },
        syn::Meta::NameValue(meta) => {
            let key: String = meta
                .path
                .get_ident()
                .map(ToString::to_string)
                .context("unsupported compound cfg key in Rust ABI item")?;
            let syn::Expr::Lit(expr) = &meta.value else {
                bail!("cfg key `{key}` has a non-literal value");
            };
            let syn::Lit::Str(value) = &expr.lit else {
                bail!("cfg key `{key}` has a non-string value");
            };
            let value: String = value.value();
            match key.as_str() {
                "target_arch" => Ok(value == "x86"),
                "target_pointer_width" => Ok(value == "32"),
                "target_endian" => Ok(value == "little"),
                "target_os" => Ok(value == "nanvix"),
                "target_family" => Ok(value == "unix"),
                _ => bail!("unsupported cfg key `{key}` in Rust ABI item"),
            }
        },
        syn::Meta::List(list) => {
            let operation: String = list
                .path
                .get_ident()
                .map(ToString::to_string)
                .context("unsupported compound cfg operation in Rust ABI item")?;
            let nested: syn::punctuated::Punctuated<syn::Meta, syn::Token![,]> = list
                .parse_args_with(
                    syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                )?;
            match operation.as_str() {
                "all" => {
                    for predicate in &nested {
                        if !cfg_matches_guest(predicate)? {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                },
                "any" => {
                    for predicate in &nested {
                        if cfg_matches_guest(predicate)? {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                },
                "not" if nested.len() == 1 => Ok(!cfg_matches_guest(&nested[0])?),
                "not" => bail!("cfg not() expects exactly one predicate"),
                _ => bail!("unsupported cfg operation `{operation}` in Rust ABI item"),
            }
        },
    }
}

/// Returns whether an item is enabled for the fixed i686 Nanvix guest target.
fn item_matches_guest(attrs: &[syn::Attribute]) -> Result<bool> {
    for attr in attrs {
        if !attr.path().is_ident("cfg") {
            continue;
        }
        let syn::Meta::List(list) = &attr.meta else {
            bail!("malformed cfg attribute in Rust ABI item");
        };
        let predicate: syn::Meta = list.parse_args()?;
        if !cfg_matches_guest(&predicate)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Returns `(repr_c, packed, alignment)` metadata from Rust representation attributes.
fn rust_repr(attrs: &[syn::Attribute]) -> Result<(bool, bool, Option<usize>)> {
    let mut repr_c: bool = false;
    let mut packed: bool = false;
    let mut alignment: Option<usize> = None;
    for attr in attrs {
        if !attr.path().is_ident("repr") {
            continue;
        }
        let syn::Meta::List(list) = &attr.meta else {
            bail!("malformed repr attribute in Rust ABI item");
        };
        let nested: syn::punctuated::Punctuated<syn::Meta, syn::Token![,]> = list.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        )?;
        for representation in &nested {
            match representation {
                syn::Meta::Path(path) if path.is_ident("C") => repr_c = true,
                syn::Meta::Path(path) if path.is_ident("packed") => packed = true,
                syn::Meta::List(list) if list.path.is_ident("packed") => {
                    bail!(
                        "repr(packed(N)) is unsupported for generated C types; use bare \
                         repr(packed) or a raw C type override"
                    );
                },
                syn::Meta::List(list) if list.path.is_ident("align") => {
                    let value: syn::LitInt = list.parse_args()?;
                    alignment = Some(value.base10_parse()?);
                },
                unsupported => {
                    let name: String = unsupported
                        .path()
                        .segments
                        .last()
                        .map(|segment| segment.ident.to_string())
                        .unwrap_or_else(|| "<unknown>".to_string());
                    bail!(
                        "unsupported Rust representation `{name}` for generated C types; use \
                         repr(C), optional bare repr(packed), or a raw C type override"
                    );
                },
            }
        }
    }
    Ok((repr_c, packed, alignment))
}

/// Returns the C operator corresponding to a supported Rust binary operator.
fn translate_binary_operator(operator: &syn::BinOp) -> Option<&'static str> {
    Some(match operator {
        syn::BinOp::Add(_) => "+",
        syn::BinOp::Sub(_) => "-",
        syn::BinOp::Mul(_) => "*",
        syn::BinOp::Div(_) => "/",
        syn::BinOp::Rem(_) => "%",
        syn::BinOp::And(_) => "&&",
        syn::BinOp::Or(_) => "||",
        syn::BinOp::BitXor(_) => "^",
        syn::BinOp::BitAnd(_) => "&",
        syn::BinOp::BitOr(_) => "|",
        syn::BinOp::Shl(_) => "<<",
        syn::BinOp::Shr(_) => ">>",
        syn::BinOp::Eq(_) => "==",
        syn::BinOp::Lt(_) => "<",
        syn::BinOp::Le(_) => "<=",
        syn::BinOp::Ne(_) => "!=",
        syn::BinOp::Ge(_) => ">=",
        syn::BinOp::Gt(_) => ">",
        _ => return None,
    })
}

/// Returns the C type that controls integer unary operations, when known.
fn constant_integer_c_type(expr: &syn::Expr, expected_type: Option<&syn::Type>) -> Result<String> {
    if let syn::Expr::Lit(expr) = expr {
        if let syn::Lit::Int(literal) = &expr.lit {
            let suffix: &str = literal.suffix();
            if !suffix.is_empty() {
                return map_ident_to_c(suffix)
                    .map(str::to_string)
                    .with_context(|| format!("unsupported Rust integer suffix `{suffix}`"));
            }
        }
    }

    if let Some(expected_type) = expected_type {
        let c_type: String = map_type(expected_type);
        if !c_type.contains("UNMAPPED") && c_type != "bool" {
            return Ok(c_type);
        }
    }
    bail!(
        "cannot determine the width of a Rust bitwise-not expression; add an integer suffix or \
         use an explicit C override"
    )
}

/// Returns whether a unary-not expression has Rust boolean semantics.
fn is_boolean_not_operand(expr: &syn::Expr, expected_type: Option<&syn::Type>) -> bool {
    if matches!(
        expr,
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Bool(_),
            ..
        })
    ) {
        return true;
    }
    expected_type
        .is_some_and(|ty| matches!(ty, syn::Type::Path(path) if path.path.is_ident("bool")))
}

/// Translates the supported Rust constant-expression subset to C.
fn translate_constant_expr(
    expr: &syn::Expr,
    module_path: &[String],
    exported_names: &BTreeMap<String, String>,
) -> Result<String> {
    translate_constant_expr_typed(expr, module_path, exported_names, None)
}

/// Translates a Rust constant expression with an optional declared result type.
fn translate_constant_expr_typed(
    expr: &syn::Expr,
    module_path: &[String],
    exported_names: &BTreeMap<String, String>,
    expected_type: Option<&syn::Type>,
) -> Result<String> {
    match expr {
        syn::Expr::Lit(expr) => match &expr.lit {
            syn::Lit::Int(value) => {
                let translated: String = translate_integer_literal(value)?;
                if let Some(c_type) = integer_literal_c_type(value)? {
                    Ok(format!("(({c_type}){translated})"))
                } else {
                    Ok(translated)
                }
            },
            syn::Lit::Char(value) => translate_char_literal(value.value()),
            syn::Lit::Str(value) => translate_string_literal(&value.value()),
            syn::Lit::Byte(value) => translate_char_literal(value.value() as char),
            syn::Lit::ByteStr(value) => {
                let bytes: Vec<u8> = value.value();
                let text: &str = str::from_utf8(&bytes)
                    .context("non-UTF-8 byte strings require an explicit C override")?;
                translate_string_literal(text)
            },
            syn::Lit::Bool(value) => Ok(if value.value { "1" } else { "0" }.to_string()),
            _ => bail!("unsupported Rust literal; use an explicit C override"),
        },
        syn::Expr::Unary(expr) => {
            let inner: String = translate_constant_expr_typed(
                &expr.expr,
                module_path,
                exported_names,
                expected_type,
            )?;
            match expr.op {
                syn::UnOp::Neg(_) => Ok(format!("(-{inner})")),
                syn::UnOp::Not(_) if is_boolean_not_operand(&expr.expr, expected_type) => {
                    Ok(format!("(!{inner})"))
                },
                syn::UnOp::Not(_) => {
                    let c_type: String = constant_integer_c_type(&expr.expr, expected_type)?;
                    Ok(format!("(({c_type})(~(({c_type}){inner})))"))
                },
                _ => bail!("unsupported Rust unary operator; use an explicit C override"),
            }
        },
        syn::Expr::Binary(expr) => {
            let operator: &str = translate_binary_operator(&expr.op)
                .context("unsupported Rust binary operator; use an explicit C override")?;
            let preserve_integer_width: bool = matches!(
                expr.op,
                syn::BinOp::Add(_)
                    | syn::BinOp::Sub(_)
                    | syn::BinOp::Mul(_)
                    | syn::BinOp::Div(_)
                    | syn::BinOp::Rem(_)
                    | syn::BinOp::BitXor(_)
                    | syn::BinOp::BitAnd(_)
                    | syn::BinOp::BitOr(_)
                    | syn::BinOp::Shl(_)
                    | syn::BinOp::Shr(_)
            );
            let operand_type: Option<&syn::Type> =
                preserve_integer_width.then_some(expected_type).flatten();
            let left: String = translate_constant_expr_typed(
                &expr.left,
                module_path,
                exported_names,
                operand_type,
            )?;
            let right: String = translate_constant_expr_typed(
                &expr.right,
                module_path,
                exported_names,
                operand_type,
            )?;
            if let Some(operand_type) = operand_type {
                let c_type: String = map_type(operand_type);
                if c_type.contains("UNMAPPED") {
                    bail!(
                        "binary constant expression uses an unsupported result type; use an \
                         explicit C override"
                    );
                }
                Ok(format!("(({c_type})((({c_type}){left}) {operator} (({c_type}){right})))"))
            } else {
                Ok(format!("({left} {operator} {right})"))
            }
        },
        syn::Expr::Paren(expr) => {
            translate_constant_expr_typed(&expr.expr, module_path, exported_names, expected_type)
        },
        syn::Expr::Group(expr) => {
            translate_constant_expr_typed(&expr.expr, module_path, exported_names, expected_type)
        },
        syn::Expr::Path(expr) if expr.qself.is_none() => {
            let resolved: String = resolve_constant_path(&expr.path, module_path);
            exported_names.get(&resolved).cloned().with_context(|| {
                format!(
                    "constant alias `{resolved}` is not exported by this header; export it or use \
                     an explicit C override"
                )
            })
        },
        syn::Expr::Cast(expr) => {
            let c_type: String = map_type(&expr.ty);
            if c_type.contains("UNMAPPED") {
                bail!("constant cast uses an unsupported type; use an explicit C override");
            }
            let inner: String =
                translate_constant_expr_typed(&expr.expr, module_path, exported_names, None)?;
            Ok(format!("(({c_type}){inner})"))
        },
        _ => bail!("unsupported Rust constant expression; use an explicit C override"),
    }
}

//==================================================================================================
// Rust Source Parsing
//==================================================================================================

/// Extracts the C name of a parameter pattern, stripping leading underscores.
fn pat_name(pat: &syn::Pat) -> String {
    let raw: String = match pat {
        syn::Pat::Ident(ident) => ident.ident.to_string(),
        _ => String::new(),
    };
    let trimmed: &str = raw.trim_start_matches('_');
    if trimmed.is_empty() {
        "arg".to_string()
    } else {
        sanitize_c_ident(trimmed)
    }
}

/// C++ keywords that are not also C keywords. Using any of these as a C function
/// parameter name compiles as C but is a hard parse error in C++, which breaks
/// every C++ translation unit (libunwind, libc++, user code) that includes the
/// generated headers. Parameter names are not ABI-significant, so the generator
/// rewrites a colliding name by appending an underscore.
const CPP_KEYWORDS: &[&str] = &[
    "alignas",
    "alignof",
    "and",
    "and_eq",
    "asm",
    "bitand",
    "bitor",
    "bool",
    "catch",
    "char8_t",
    "char16_t",
    "char32_t",
    "class",
    "co_await",
    "co_return",
    "co_yield",
    "compl",
    "concept",
    "const_cast",
    "consteval",
    "constexpr",
    "constinit",
    "decltype",
    "delete",
    "dynamic_cast",
    "explicit",
    "export",
    "false",
    "friend",
    "mutable",
    "namespace",
    "new",
    "noexcept",
    "not",
    "not_eq",
    "nullptr",
    "operator",
    "or",
    "or_eq",
    "private",
    "protected",
    "public",
    "reinterpret_cast",
    "requires",
    "static_assert",
    "static_cast",
    "template",
    "this",
    "thread_local",
    "throw",
    "true",
    "try",
    "typeid",
    "typename",
    "using",
    "virtual",
    "wchar_t",
    "xor",
    "xor_eq",
];

/// Rewrites a C identifier that collides with a C++ keyword so the generated
/// header parses as both C and C++. A trailing underscore is appended because
/// parameter names carry no ABI significance.
fn sanitize_c_ident(name: &str) -> String {
    if CPP_KEYWORDS.contains(&name) {
        format!("{name}_")
    } else {
        name.to_string()
    }
}

/// Returns `true` if `byte` may appear within a C identifier.
fn is_c_ident_byte(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}

/// Returns `true` if any rendered line uses the C99 `restrict` keyword as a
/// standalone identifier. The keyword is valid (and desirable) C but a parse
/// error in C++, so a header that emits it needs a portable compatibility shim.
/// Tokens such as `__restrict` do not match, while a comment that merely
/// mentions the word would over-match harmlessly (the rewrite is keyword-aware).
fn uses_restrict_keyword(lines: &[String]) -> bool {
    lines.iter().any(|line| {
        line.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .any(|token| token == "restrict")
    })
}

/// Rewrites every standalone `restrict` keyword in `line` to the private
/// `__nanvix_restrict` qualifier macro (see [`RESTRICT_SHIM`]). Adjacent
/// identifier characters guard against touching `__restrict` or any larger
/// identifier; headers are ASCII, so byte-wise boundaries are sufficient.
fn rewrite_restrict_tokens(line: &str) -> String {
    const KEYWORD: &str = "restrict";
    const REPLACEMENT: &str = "__nanvix_restrict";

    let bytes: &[u8] = line.as_bytes();
    let mut out: String = String::with_capacity(line.len());
    let mut index: usize = 0;
    while index < bytes.len() {
        let at_word_start: bool =
            !is_c_ident_byte(bytes[index]) || index == 0 || !is_c_ident_byte(bytes[index - 1]);
        let after: usize = index + KEYWORD.len();
        if at_word_start
            && line[index..].starts_with(KEYWORD)
            && (after == bytes.len() || !is_c_ident_byte(bytes[after]))
        {
            out.push_str(REPLACEMENT);
            index = after;
        } else {
            out.push(bytes[index] as char);
            index += 1;
        }
    }
    out
}

/// Parses a single Rust source file, returning its `pub extern "C"` signatures.
fn parse_rust_file(path: &Path) -> Result<Vec<FuncSig>> {
    let content: String =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let file: syn::File =
        syn::parse_file(&content).with_context(|| format!("failed to parse {}", path.display()))?;

    let mut results: Vec<FuncSig> = Vec::new();
    collect_extern_c_fns(&file.items, &mut results);
    Ok(results)
}

/// Source location and default child-module directory for one Rust module.
struct RustModuleContext {
    /// Source file containing the module's items.
    source_path: PathBuf,
    /// Directory where ordinary out-of-line child modules are resolved.
    child_dir: PathBuf,
}

/// Returns the default child-module directory for a Rust source file.
fn rust_child_module_dir(path: &Path) -> Result<PathBuf> {
    let parent: &Path = path
        .parent()
        .with_context(|| format!("Rust source {} has no parent directory", path.display()))?;
    let stem: &str = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .with_context(|| format!("invalid Rust source path {}", path.display()))?;
    if stem == "lib" || stem == "main" || stem == "mod" {
        Ok(parent.to_path_buf())
    } else {
        Ok(parent.join(stem))
    }
}

/// Returns an optional `#[path = "..."]` override from a module declaration.
fn rust_module_path_override(module: &syn::ItemMod) -> Result<Option<PathBuf>> {
    let mut result: Option<PathBuf> = None;
    for attr in &module.attrs {
        if !attr.path().is_ident("path") {
            continue;
        }
        let syn::Meta::NameValue(meta) = &attr.meta else {
            bail!("module `{}` has a malformed path attribute", module.ident);
        };
        let syn::Expr::Lit(expr) = &meta.value else {
            bail!("module `{}` has a non-literal path attribute", module.ident);
        };
        let syn::Lit::Str(value) = &expr.lit else {
            bail!("module `{}` has a non-string path attribute", module.ident);
        };
        if result.replace(PathBuf::from(value.value())).is_some() {
            bail!("module `{}` has multiple path attributes", module.ident);
        }
    }
    Ok(result)
}

/// Resolves one enabled public out-of-line module to its source and child-module directory.
fn resolve_rust_module_source(
    module: &syn::ItemMod,
    context: &RustModuleContext,
) -> Result<RustModuleContext> {
    if let Some(path) = rust_module_path_override(module)? {
        let path: PathBuf = context.child_dir.join(path);
        if !path.is_file() {
            bail!(
                "path override for module `{}` does not name a file ({})",
                module.ident,
                path.display()
            );
        }
        let child_dir: PathBuf = path
            .parent()
            .with_context(|| format!("Rust source {} has no parent directory", path.display()))?
            .to_path_buf();
        return Ok(RustModuleContext {
            source_path: path,
            child_dir,
        });
    }

    let direct: PathBuf = context.child_dir.join(format!("{}.rs", module.ident));
    let nested: PathBuf = context
        .child_dir
        .join(module.ident.to_string())
        .join("mod.rs");
    match (direct.is_file(), nested.is_file()) {
        (true, false) => Ok(RustModuleContext {
            source_path: direct,
            child_dir: context.child_dir.join(module.ident.to_string()),
        }),
        (false, true) => Ok(RustModuleContext {
            source_path: nested,
            child_dir: context.child_dir.join(module.ident.to_string()),
        }),
        (true, true) => bail!(
            "module `{}` is ambiguous; both {} and {} exist",
            module.ident,
            direct.display(),
            nested.display()
        ),
        (false, false) => bail!(
            "source for public module `{}` was not found (tried {} and {})",
            module.ident,
            direct.display(),
            nested.display()
        ),
    }
}

/// Converts named or tuple Rust fields to indexed generator metadata.
fn collect_rust_fields(fields: &syn::Fields) -> Result<Vec<RustField>> {
    let mut results: Vec<RustField> = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        if !item_matches_guest(&field.attrs)? {
            continue;
        }
        results.push(RustField {
            name: field
                .ident
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| format!("_{index}")),
            docs: rust_docs(&field.attrs),
            ty: field.ty.clone(),
        });
    }
    Ok(results)
}

/// Collects public constants and C-compatible types while preserving module paths.
#[cfg(test)]
fn collect_rust_symbols(
    items: &[syn::Item],
    module_path: &[String],
    symbols: &mut RustSymbols,
) -> Result<()> {
    collect_rust_symbols_in_module(items, module_path, symbols, None, true)?;
    apply_rust_reexports(symbols)
}

/// One flattened binding from a Rust `use` tree.
struct RustUseBinding {
    /// Source path as written in the use tree.
    source: Vec<String>,
    /// Local name for a named import.
    alias: Option<String>,
    /// Whether this is a glob import.
    glob: bool,
}

/// Flattens a Rust `use` tree into named and glob bindings.
fn flatten_rust_use_tree(
    tree: &syn::UseTree,
    prefix: &[String],
    bindings: &mut Vec<RustUseBinding>,
) {
    match tree {
        syn::UseTree::Path(path) => {
            let mut child_prefix: Vec<String> = prefix.to_vec();
            child_prefix.push(path.ident.to_string());
            flatten_rust_use_tree(&path.tree, &child_prefix, bindings);
        },
        syn::UseTree::Name(name) if name.ident == "self" => {
            if let Some(alias) = prefix.last() {
                bindings.push(RustUseBinding {
                    source: prefix.to_vec(),
                    alias: Some(alias.clone()),
                    glob: false,
                });
            }
        },
        syn::UseTree::Name(name) => {
            let mut source: Vec<String> = prefix.to_vec();
            source.push(name.ident.to_string());
            bindings.push(RustUseBinding {
                source,
                alias: Some(name.ident.to_string()),
                glob: false,
            });
        },
        syn::UseTree::Rename(rename) => {
            let mut source: Vec<String> = prefix.to_vec();
            if rename.ident != "self" {
                source.push(rename.ident.to_string());
            }
            bindings.push(RustUseBinding {
                source,
                alias: Some(rename.rename.to_string()),
                glob: false,
            });
        },
        syn::UseTree::Glob(_) => bindings.push(RustUseBinding {
            source: prefix.to_vec(),
            alias: None,
            glob: true,
        }),
        syn::UseTree::Group(group) => {
            for item in &group.items {
                flatten_rust_use_tree(item, prefix, bindings);
            }
        },
    }
}

/// Resolves a Rust `use` source relative to its declaring module.
fn resolve_rust_use_source(source: &[String], module_path: &[String], absolute: bool) -> String {
    if source.is_empty() {
        return String::new();
    }
    let mut resolved: Vec<String> = if absolute {
        Vec::new()
    } else {
        module_path.to_vec()
    };
    let mut index: usize = 0;
    if source[0] == "crate" {
        resolved.clear();
        index = 1;
    } else if source[0] == "self" {
        index = 1;
    } else {
        while source.get(index).is_some_and(|segment| segment == "super") {
            resolved.pop();
            index += 1;
        }
    }
    resolved.extend(source[index..].iter().cloned());
    resolved.join("::")
}

/// Collects imports visible in one module and records its effective public re-exports.
fn collect_rust_imports(
    items: &[syn::Item],
    module_path: &[String],
    public_module: bool,
    symbols: &mut RustSymbols,
) -> Result<(BTreeMap<String, String>, Vec<String>)> {
    let mut imports: BTreeMap<String, String> = BTreeMap::new();
    let mut glob_imports: Vec<String> = Vec::new();
    for item in items {
        let syn::Item::Use(item_use) = item else {
            continue;
        };
        if !item_matches_guest(&item_use.attrs)? {
            continue;
        }
        let mut bindings: Vec<RustUseBinding> = Vec::new();
        flatten_rust_use_tree(&item_use.tree, &[], &mut bindings);
        for binding in bindings {
            let source: String = resolve_rust_use_source(
                &binding.source,
                module_path,
                item_use.leading_colon.is_some(),
            );
            if binding.glob {
                glob_imports.push(source.clone());
            } else if let Some(alias) = &binding.alias {
                imports.insert(alias.clone(), source.clone());
            }
            if matches!(item_use.vis, syn::Visibility::Public(_)) {
                symbols.reexports.push(RustReexport {
                    source,
                    destination_module: module_path.to_vec(),
                    alias: binding.alias,
                    glob: binding.glob,
                    public: public_module,
                });
            }
        }
    }
    glob_imports.sort();
    glob_imports.dedup();
    Ok((imports, glob_imports))
}

/// Collects symbols from one module and recursively loads enabled public child modules.
fn collect_rust_symbols_in_module(
    items: &[syn::Item],
    module_path: &[String],
    symbols: &mut RustSymbols,
    context: Option<&RustModuleContext>,
    public_module: bool,
) -> Result<()> {
    let (imports, glob_imports): (BTreeMap<String, String>, Vec<String>) =
        collect_rust_imports(items, module_path, public_module, symbols)?;
    for item in items {
        match item {
            syn::Item::Mod(module) => {
                if !item_matches_guest(&module.attrs)? {
                    continue;
                }
                let public_child: bool =
                    public_module && matches!(module.vis, syn::Visibility::Public(_));
                let mut child_path: Vec<String> = module_path.to_vec();
                child_path.push(module.ident.to_string());
                if !matches!(module.vis, syn::Visibility::Public(_)) {
                    symbols.private_modules.insert(child_path.join("::"));
                }
                if let Some((_, inner)) = &module.content {
                    let path_override: Option<PathBuf> = rust_module_path_override(module)?;
                    let child_context: Option<RustModuleContext> = match context {
                        Some(context) => Some(RustModuleContext {
                            source_path: context.source_path.clone(),
                            child_dir: context.child_dir.join(
                                path_override
                                    .unwrap_or_else(|| PathBuf::from(module.ident.to_string())),
                            ),
                        }),
                        None if path_override.is_none() => None,
                        None => bail!(
                            "cannot resolve path override for inline module `{}` without a source \
                             context",
                            module.ident
                        ),
                    };
                    collect_rust_symbols_in_module(
                        inner,
                        &child_path,
                        symbols,
                        child_context.as_ref(),
                        public_child,
                    )?;
                } else {
                    let context: &RustModuleContext = context.with_context(|| {
                        format!(
                            "cannot resolve out-of-line module `{}` without a source context",
                            module.ident
                        )
                    })?;
                    let child_context: RustModuleContext =
                        resolve_rust_module_source(module, context)?;
                    let content: String = fs::read_to_string(&child_context.source_path)
                        .with_context(|| {
                            format!("failed to read {}", child_context.source_path.display())
                        })?;
                    let file: syn::File = syn::parse_file(&content).with_context(|| {
                        format!("failed to parse {}", child_context.source_path.display())
                    })?;
                    if !item_matches_guest(&file.attrs)? {
                        continue;
                    }
                    collect_rust_symbols_in_module(
                        &file.items,
                        &child_path,
                        symbols,
                        Some(&child_context),
                        public_child,
                    )?;
                }
            },
            syn::Item::Const(constant)
                if matches!(constant.vis, syn::Visibility::Public(_))
                    && item_matches_guest(&constant.attrs)? =>
            {
                let mut symbol_path: Vec<String> = module_path.to_vec();
                symbol_path.push(constant.ident.to_string());
                let symbol_path: String = symbol_path.join("::");
                let symbol: RustConstant = RustConstant {
                    docs: rust_docs(&constant.attrs),
                    ty: constant.ty.clone(),
                    expr: (*constant.expr).clone(),
                    module_path: module_path.to_vec(),
                    canonical_path: symbol_path.clone(),
                    imports: imports.clone(),
                    glob_imports: glob_imports.clone(),
                    public: public_module,
                };
                if symbols
                    .constants
                    .insert(symbol_path.clone(), symbol)
                    .is_some()
                {
                    bail!("duplicate public Rust constant `{symbol_path}`");
                }
            },
            syn::Item::Type(alias)
                if matches!(alias.vis, syn::Visibility::Public(_))
                    && item_matches_guest(&alias.attrs)? =>
            {
                let mut symbol_path: Vec<String> = module_path.to_vec();
                symbol_path.push(alias.ident.to_string());
                let symbol_path: String = symbol_path.join("::");
                let symbol: RustType = RustType {
                    docs: rust_docs(&alias.attrs),
                    kind: RustTypeKind::Alias(alias.ty.clone()),
                    module_path: module_path.to_vec(),
                    canonical_path: symbol_path.clone(),
                    imports: imports.clone(),
                    glob_imports: glob_imports.clone(),
                    packed: false,
                    alignment: None,
                    public: public_module,
                };
                if symbols.types.insert(symbol_path.clone(), symbol).is_some() {
                    bail!("duplicate public Rust type `{symbol_path}`");
                }
            },
            syn::Item::Struct(structure)
                if matches!(structure.vis, syn::Visibility::Public(_))
                    && item_matches_guest(&structure.attrs)? =>
            {
                let (repr_c, packed, alignment): (bool, bool, Option<usize>) =
                    rust_repr(&structure.attrs)?;
                if !repr_c {
                    continue;
                }
                let mut symbol_path: Vec<String> = module_path.to_vec();
                symbol_path.push(structure.ident.to_string());
                let symbol_path: String = symbol_path.join("::");
                let symbol: RustType = RustType {
                    docs: rust_docs(&structure.attrs),
                    kind: RustTypeKind::Struct(collect_rust_fields(&structure.fields)?),
                    module_path: module_path.to_vec(),
                    canonical_path: symbol_path.clone(),
                    imports: imports.clone(),
                    glob_imports: glob_imports.clone(),
                    packed,
                    alignment,
                    public: public_module,
                };
                if symbols.types.insert(symbol_path.clone(), symbol).is_some() {
                    bail!("duplicate public Rust type `{symbol_path}`");
                }
            },
            syn::Item::Union(union)
                if matches!(union.vis, syn::Visibility::Public(_))
                    && item_matches_guest(&union.attrs)? =>
            {
                let (repr_c, packed, alignment): (bool, bool, Option<usize>) =
                    rust_repr(&union.attrs)?;
                if !repr_c {
                    continue;
                }
                let mut symbol_path: Vec<String> = module_path.to_vec();
                symbol_path.push(union.ident.to_string());
                let symbol_path: String = symbol_path.join("::");
                let fields: syn::Fields = syn::Fields::Named(union.fields.clone());
                let symbol: RustType = RustType {
                    docs: rust_docs(&union.attrs),
                    kind: RustTypeKind::Union(collect_rust_fields(&fields)?),
                    module_path: module_path.to_vec(),
                    canonical_path: symbol_path.clone(),
                    imports: imports.clone(),
                    glob_imports: glob_imports.clone(),
                    packed,
                    alignment,
                    public: public_module,
                };
                if symbols.types.insert(symbol_path.clone(), symbol).is_some() {
                    bail!("duplicate public Rust type `{symbol_path}`");
                }
            },
            _ => {},
        }
    }
    Ok(())
}

/// Joins a module path and one relative symbol path.
fn join_rust_path(module_path: &[String], relative: &str) -> String {
    if module_path.is_empty() {
        relative.to_string()
    } else if relative.is_empty() {
        module_path.join("::")
    } else {
        format!("{}::{relative}", module_path.join("::"))
    }
}

/// Returns the direct child name below `module` for a fully qualified symbol path.
fn direct_rust_child<'a>(path: &'a str, module: &str) -> Option<&'a str> {
    let suffix: &str = if module.is_empty() {
        path
    } else {
        path.strip_prefix(&format!("{module}::"))?
    };
    (!suffix.is_empty() && !suffix.contains("::")).then_some(suffix)
}

/// Returns whether every module between `source` and a descendant symbol is public.
fn rust_path_is_public_below(path: &str, source: &str, private_modules: &BTreeSet<String>) -> bool {
    let source_prefix: String = format!("{source}::");
    let Some(suffix) = path.strip_prefix(&source_prefix) else {
        return false;
    };
    let segments: Vec<&str> = suffix.split("::").collect();
    let mut prefix: String = source.to_string();
    for segment in segments.iter().take(segments.len().saturating_sub(1)) {
        prefix.push_str("::");
        prefix.push_str(segment);
        if private_modules.contains(&prefix) {
            return false;
        }
    }
    true
}

/// Applies Rust re-exports until all resolvable aliases have been materialized.
fn apply_rust_reexports(symbols: &mut RustSymbols) -> Result<()> {
    let mut explicit_constants: BTreeSet<String> = symbols.constants.keys().cloned().collect();
    let mut explicit_types: BTreeSet<String> = symbols.types.keys().cloned().collect();
    let mut ambiguous_constants: BTreeSet<String> = BTreeSet::new();
    let mut ambiguous_types: BTreeSet<String> = BTreeSet::new();

    for _ in 0..=symbols.reexports.len() {
        let mut changed: bool = false;
        // Named re-exports take precedence over glob re-exports.
        for reexport in symbols.reexports.iter().filter(|reexport| !reexport.glob) {
            let alias: &str = reexport
                .alias
                .as_deref()
                .context("named Rust re-export is missing an alias")?;
            let destination: String = join_rust_path(&reexport.destination_module, alias);
            if let Some(source) = symbols.constants.get(&reexport.source).cloned() {
                let replace: bool = symbols
                    .constants
                    .get(&destination)
                    .is_none_or(|existing| existing.canonical_path != source.canonical_path);
                if replace {
                    let mut symbol: RustConstant = source;
                    symbol.public = reexport.public;
                    symbols.constants.insert(destination.clone(), symbol);
                    changed = true;
                }
                explicit_constants.insert(destination.clone());
                ambiguous_constants.remove(&destination);
            }
            if let Some(source) = symbols.types.get(&reexport.source).cloned() {
                let replace: bool = symbols
                    .types
                    .get(&destination)
                    .is_none_or(|existing| existing.canonical_path != source.canonical_path);
                if replace {
                    let mut symbol: RustType = source;
                    symbol.public = reexport.public;
                    symbols.types.insert(destination.clone(), symbol);
                    changed = true;
                }
                explicit_types.insert(destination.clone());
                ambiguous_types.remove(&destination);
            }

            let source_prefix: String = format!("{}::", reexport.source);
            let constant_paths: Vec<String> = symbols
                .constants
                .keys()
                .filter(|path| {
                    path.starts_with(&source_prefix)
                        && rust_path_is_public_below(
                            path,
                            &reexport.source,
                            &symbols.private_modules,
                        )
                })
                .cloned()
                .collect();
            let type_paths: Vec<String> = symbols
                .types
                .keys()
                .filter(|path| {
                    path.starts_with(&source_prefix)
                        && rust_path_is_public_below(
                            path,
                            &reexport.source,
                            &symbols.private_modules,
                        )
                })
                .cloned()
                .collect();
            for source_path in constant_paths {
                let suffix: &str = source_path
                    .strip_prefix(&source_prefix)
                    .expect("source path was filtered by prefix");
                let destination: String =
                    join_rust_path(&reexport.destination_module, &format!("{alias}::{suffix}"));
                let source: RustConstant = symbols.constants[&source_path].clone();
                let replace: bool = symbols
                    .constants
                    .get(&destination)
                    .is_none_or(|existing| existing.canonical_path != source.canonical_path);
                if replace {
                    let mut symbol: RustConstant = source;
                    symbol.public = reexport.public;
                    symbols.constants.insert(destination.clone(), symbol);
                    changed = true;
                }
                explicit_constants.insert(destination.clone());
                ambiguous_constants.remove(&destination);
            }
            for source_path in type_paths {
                let suffix: &str = source_path
                    .strip_prefix(&source_prefix)
                    .expect("source path was filtered by prefix");
                let destination: String =
                    join_rust_path(&reexport.destination_module, &format!("{alias}::{suffix}"));
                let source: RustType = symbols.types[&source_path].clone();
                let replace: bool = symbols
                    .types
                    .get(&destination)
                    .is_none_or(|existing| existing.canonical_path != source.canonical_path);
                if replace {
                    let mut symbol: RustType = source;
                    symbol.public = reexport.public;
                    symbols.types.insert(destination.clone(), symbol);
                    changed = true;
                }
                explicit_types.insert(destination.clone());
                ambiguous_types.remove(&destination);
            }
        }

        for reexport in symbols.reexports.iter().filter(|reexport| reexport.glob) {
            let source_prefix: String = format!("{}::", reexport.source);
            let constant_paths: Vec<String> = symbols
                .constants
                .keys()
                .filter(|path| {
                    path.starts_with(&source_prefix)
                        && rust_path_is_public_below(
                            path,
                            &reexport.source,
                            &symbols.private_modules,
                        )
                })
                .cloned()
                .collect();
            let type_paths: Vec<String> = symbols
                .types
                .keys()
                .filter(|path| {
                    path.starts_with(&source_prefix)
                        && rust_path_is_public_below(
                            path,
                            &reexport.source,
                            &symbols.private_modules,
                        )
                })
                .cloned()
                .collect();
            for source_path in constant_paths {
                let suffix: &str = source_path
                    .strip_prefix(&source_prefix)
                    .expect("glob source was filtered by prefix");
                let destination: String = join_rust_path(&reexport.destination_module, suffix);
                if explicit_constants.contains(&destination)
                    || ambiguous_constants.contains(&destination)
                {
                    continue;
                }
                let source: RustConstant = symbols.constants[&source_path].clone();
                if let Some(existing) = symbols.constants.get(&destination) {
                    if existing.canonical_path != source.canonical_path {
                        symbols.constants.remove(&destination);
                        ambiguous_constants.insert(destination);
                        changed = true;
                    }
                } else {
                    let mut symbol: RustConstant = source;
                    symbol.public = reexport.public;
                    symbols.constants.insert(destination, symbol);
                    changed = true;
                }
            }
            for source_path in type_paths {
                let suffix: &str = source_path
                    .strip_prefix(&source_prefix)
                    .expect("glob source was filtered by prefix");
                let destination: String = join_rust_path(&reexport.destination_module, suffix);
                if explicit_types.contains(&destination) || ambiguous_types.contains(&destination) {
                    continue;
                }
                let source: RustType = symbols.types[&source_path].clone();
                if let Some(existing) = symbols.types.get(&destination) {
                    if existing.canonical_path != source.canonical_path {
                        symbols.types.remove(&destination);
                        ambiguous_types.insert(destination);
                        changed = true;
                    }
                } else {
                    let mut symbol: RustType = source;
                    symbol.public = reexport.public;
                    symbols.types.insert(destination, symbol);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    Ok(())
}

/// Adds lexical import aliases to a fully qualified selected-symbol map.
fn scoped_rust_names<T: Clone>(
    names: &BTreeMap<String, T>,
    module_path: &[String],
    imports: &BTreeMap<String, String>,
    glob_imports: &[String],
) -> BTreeMap<String, T> {
    let mut scoped: BTreeMap<String, T> = names.clone();
    for _ in 0..=imports.len() {
        let mut changed: bool = false;
        let available: Vec<(String, T)> = scoped
            .iter()
            .map(|(path, value)| (path.clone(), value.clone()))
            .collect();
        for (alias, source) in imports {
            let source_prefix: String = format!("{source}::");
            for (path, value) in &available {
                let relative: Option<String> = if path == source {
                    Some(alias.clone())
                } else {
                    path.strip_prefix(&source_prefix)
                        .map(|suffix| format!("{alias}::{suffix}"))
                };
                if let Some(relative) = relative {
                    let destination: String = join_rust_path(module_path, &relative);
                    if let std::collections::btree_map::Entry::Vacant(entry) =
                        scoped.entry(destination)
                    {
                        entry.insert(value.clone());
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    for source in glob_imports {
        for (path, value) in names {
            if let Some(child) = direct_rust_child(path, source) {
                scoped
                    .entry(join_rust_path(module_path, child))
                    .or_insert_with(|| value.clone());
            }
        }
    }
    scoped
}

/// Scans a crate's Rust sources for explicitly exportable public symbols.
fn scan_rust_symbols(crate_dir: &Path) -> Result<RustSymbols> {
    let src_dir: PathBuf = crate_dir.join("src");
    let lib_path: PathBuf = src_dir.join("lib.rs");
    let main_path: PathBuf = src_dir.join("main.rs");
    let root_path: PathBuf = match (lib_path.is_file(), main_path.is_file()) {
        (true, false) => lib_path,
        (false, true) => main_path,
        (true, true) => bail!(
            "cannot select a Rust symbol root for {}; both lib.rs and main.rs exist",
            crate_dir.display()
        ),
        (false, false) => bail!(
            "cannot select a Rust symbol root for {}; neither lib.rs nor main.rs exists",
            crate_dir.display()
        ),
    };
    let content: String = fs::read_to_string(&root_path)
        .with_context(|| format!("failed to read {}", root_path.display()))?;
    let file: syn::File = syn::parse_file(&content)
        .with_context(|| format!("failed to parse {}", root_path.display()))?;
    let mut symbols: RustSymbols = RustSymbols::default();
    if !item_matches_guest(&file.attrs)? {
        return Ok(symbols);
    }
    let context: RustModuleContext = RustModuleContext {
        child_dir: rust_child_module_dir(&root_path)?,
        source_path: root_path.clone(),
    };
    collect_rust_symbols_in_module(&file.items, &[], &mut symbols, Some(&context), true)
        .with_context(|| format!("failed to index {}", root_path.display()))?;
    apply_rust_reexports(&mut symbols)?;
    Ok(symbols)
}

/// Prefixes every path in a Rust symbol index with a crate namespace.
fn prefix_rust_symbols(mut symbols: RustSymbols, prefix: &str) -> RustSymbols {
    let qualify = |path: &str| -> String {
        if path.is_empty() {
            prefix.to_string()
        } else {
            format!("{prefix}::{path}")
        }
    };
    let qualify_modules = |modules: &mut Vec<String>| {
        modules.insert(0, prefix.to_string());
    };

    symbols.constants = symbols
        .constants
        .into_iter()
        .map(|(path, mut symbol)| {
            qualify_modules(&mut symbol.module_path);
            symbol.canonical_path = qualify(&symbol.canonical_path);
            for source in symbol.imports.values_mut() {
                *source = qualify(source);
            }
            for source in &mut symbol.glob_imports {
                *source = qualify(source);
            }
            (qualify(&path), symbol)
        })
        .collect();
    symbols.types = symbols
        .types
        .into_iter()
        .map(|(path, mut symbol)| {
            qualify_modules(&mut symbol.module_path);
            symbol.canonical_path = qualify(&symbol.canonical_path);
            for source in symbol.imports.values_mut() {
                *source = qualify(source);
            }
            for source in &mut symbol.glob_imports {
                *source = qualify(source);
            }
            (qualify(&path), symbol)
        })
        .collect();
    symbols.reexports.clear();
    symbols.private_modules = symbols
        .private_modules
        .into_iter()
        .map(|path| qualify(&path))
        .collect();
    symbols
}

/// Merges one Rust symbol index into another, rejecting namespace collisions.
fn merge_rust_symbols(target: &mut RustSymbols, source: RustSymbols) -> Result<()> {
    for (path, symbol) in source.constants {
        if target.constants.insert(path.clone(), symbol).is_some() {
            bail!("duplicate Rust constant path `{path}` across symbol crates");
        }
    }
    for (path, symbol) in source.types {
        if target.types.insert(path.clone(), symbol).is_some() {
            bail!("duplicate Rust type path `{path}` across symbol crates");
        }
    }
    target.private_modules.extend(source.private_modules);
    Ok(())
}

/// Scans the owner crate plus supplemental crate-qualified symbol roots.
fn scan_header_symbols(
    owner_dir: &Path,
    libs_dir: &Path,
    symbol_crates: &[String],
) -> Result<RustSymbols> {
    let mut symbols: RustSymbols = scan_rust_symbols(owner_dir)?;
    for crate_name in symbol_crates {
        let supplemental: RustSymbols =
            prefix_rust_symbols(scan_rust_symbols(&libs_dir.join(crate_name))?, crate_name);
        merge_rust_symbols(&mut symbols, supplemental)?;
    }
    Ok(symbols)
}

/// Collects `pub extern "C"` signatures from a list of items, descending into
/// inline modules (e.g. the `pub mod bindings { ... }` blocks the syscall crate
/// uses to group its C entry points).
fn collect_extern_c_fns(items: &[syn::Item], results: &mut Vec<FuncSig>) {
    for item in items {
        match item {
            syn::Item::Mod(module) => {
                if let Some((_, inner)) = &module.content {
                    collect_extern_c_fns(inner, results);
                }
            },
            syn::Item::Fn(func) => {
                if let Some(sig) = parse_extern_c_fn(func) {
                    results.push(sig);
                }
            },
            _ => {},
        }
    }
}

/// Returns the parsed signature of a function item iff it is `pub extern "C"`.
fn parse_extern_c_fn(func: &syn::ItemFn) -> Option<FuncSig> {
    if !matches!(func.vis, syn::Visibility::Public(_)) {
        return None;
    }
    // Require an explicit `extern "C"` ABI.
    let abi = func.sig.abi.as_ref()?;
    if !matches!(&abi.name, Some(name) if name.value() == "C") {
        return None;
    }

    let mut params: Vec<(String, String)> = Vec::new();
    for input in &func.sig.inputs {
        if let syn::FnArg::Typed(arg) = input {
            params.push((map_type(&arg.ty), pat_name(&arg.pat)));
        }
    }
    let return_type: String = match &func.sig.output {
        syn::ReturnType::Default => "void".to_string(),
        syn::ReturnType::Type(_, ty) => map_type(ty),
    };

    Some(FuncSig {
        name: func.sig.ident.to_string(),
        params,
        return_type,
        is_variadic: func.sig.variadic.is_some(),
    })
}

/// Scans a crate's `src/*.rs` files, returning a name-to-signature map.
fn scan_crate(crate_dir: &Path) -> Result<BTreeMap<String, FuncSig>> {
    let src_dir: PathBuf = crate_dir.join("src");
    let mut funcs: BTreeMap<String, FuncSig> = BTreeMap::new();
    if !src_dir.is_dir() {
        return Ok(funcs);
    }

    let mut rs_files: Vec<PathBuf> = fs::read_dir(&src_dir)
        .with_context(|| format!("failed to read {}", src_dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    rs_files.sort();

    for rs_file in &rs_files {
        for sig in parse_rust_file(rs_file)? {
            funcs.insert(sig.name.clone(), sig);
        }
    }
    Ok(funcs)
}

/// Recursively scans a directory's `*.rs` files, merging signatures into `funcs`.
fn scan_dir_recursive(dir: &Path, funcs: &mut BTreeMap<String, FuncSig>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("failed to read {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    entries.sort();
    for path in &entries {
        if path.is_dir() {
            scan_dir_recursive(path, funcs)?;
        } else if path.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            for sig in parse_rust_file(path)? {
                funcs.insert(sig.name.clone(), sig);
            }
        }
    }
    Ok(())
}

/// Scans each named crate's `src/` tree recursively, returning a merged map.
///
/// Crate names are directory names under `libs_dir` (e.g. `syscall`, `posix`).
fn scan_named_crates(libs_dir: &Path, crates: &[String]) -> Result<BTreeMap<String, FuncSig>> {
    let mut funcs: BTreeMap<String, FuncSig> = BTreeMap::new();
    for crate_name in crates {
        let src_dir: PathBuf = libs_dir.join(crate_name).join("src");
        scan_dir_recursive(&src_dir, &mut funcs)?;
    }
    Ok(funcs)
}

//==================================================================================================
// C Declaration Formatting
//==================================================================================================

/// Formats a parsed signature as a C function declaration.
fn format_c_declaration(sig: &FuncSig) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (ptype, pname) in &sig.params {
        // Function pointer parameters: "ret (*)(args)" -> "ret (*name)(args)".
        if let Some(pos) = ptype.find("(*)(") {
            let prefix: &str = &ptype[..pos];
            let suffix: &str = &ptype[pos + 3..];
            parts.push(format!("{prefix}(*{pname}){suffix}"));
        } else if ptype.ends_with('*') {
            parts.push(format!("{ptype}{pname}"));
        } else {
            parts.push(format!("{ptype} {pname}"));
        }
    }
    if sig.is_variadic {
        parts.push("...".to_string());
    }
    let params_str: String = if parts.is_empty() {
        "void".to_string()
    } else {
        parts.join(", ")
    };

    // Keep the pointer star attached to the return type.
    if sig.return_type.ends_with(" *") {
        format!("extern {}{}({params_str});", sig.return_type, sig.name)
    } else {
        format!("extern {} {}({params_str});", sig.return_type, sig.name)
    }
}

//==================================================================================================
// Header Rendering
//==================================================================================================

/// Copyright block emitted at the top of every header.
const COPYRIGHT: &str =
    "/*\n * Copyright(c) The Maintainers of Nanvix.\n * Licensed under the MIT License.\n */";

/// Renders a section bar with the given title.
fn section_bar(title: &str) -> String {
    let rule: String = "=".repeat(98);
    format!("/*{rule}\n * {title}\n *{rule}*/")
}

/// Emits a `Constants` section of `#define` entries.
fn emit_macros_with_title(lines: &mut Vec<String>, macros: &[Macro], title: &str) {
    if macros.is_empty() {
        return;
    }
    if !title.is_empty() {
        lines.push(section_bar(title));
    }
    lines.push(String::new());
    for macro_def in macros {
        if let Some(guard) = &macro_def.guard {
            lines.push(format!("#ifndef {guard}"));
        }
        match (&macro_def.declaration, &macro_def.comment) {
            (Some(declaration), _) => lines.push(declaration.clone()),
            (None, Some(comment)) => lines.push(format!(
                "#define {} {} /**< {} */",
                macro_def.name, macro_def.value, comment
            )),
            (None, None) => lines.push(format!("#define {} {}", macro_def.name, macro_def.value)),
        }
        if macro_def.guard.is_some() {
            lines.push("#endif".to_string());
            lines.push(String::new());
        }
    }
    if lines.last().map(String::as_str) != Some("") {
        lines.push(String::new());
    }
}

/// Emits the default `Constants` section.
fn emit_macros(lines: &mut Vec<String>, macros: &[Macro]) {
    emit_macros_with_title(lines, macros, "Constants");
}

/// Validates the shared preprocessor/type names introduced by generated declarations.
fn validate_c_export_names(spec: &HeaderSpec) -> Result<()> {
    let mut owners: BTreeMap<String, String> = BTreeMap::new();
    let mut register = |name: &str, owner: String| -> Result<()> {
        if let Some(existing) = owners.get(name) {
            if existing != &owner {
                bail!(
                    "generated C name `{name}` is used by both {existing} and {owner}; use an \
                     explicit C rename"
                );
            }
        } else {
            owners.insert(name.to_string(), owner);
        }
        Ok(())
    };

    for macro_def in &spec.macros {
        register(&macro_def.name, format!("macro `{}`", macro_def.name))?;
    }
    for export in &spec.rust_constants {
        register(export.c_name(), format!("Rust constant `{}`", export.path()))?;
    }
    for export in &spec.rust_types {
        let owner: String = format!("Rust type `{}`", export.path);
        register(export.c_name(), owner.clone())?;
        if export.style == RustTypeStyle::TypedefTag {
            register(export.tag_name(), owner)?;
        }
    }
    Ok(())
}

/// Returns object-like macro names declared by a C text block.
fn c_object_macros(text: &str) -> BTreeSet<String> {
    let mut names: BTreeSet<String> = BTreeSet::new();
    for line in text.lines() {
        let line: &str = line.trim_start();
        let Some(rest) = line.strip_prefix("#define") else {
            continue;
        };
        let rest: &str = rest.trim_start();
        let name: &str = rest
            .split(|character: char| character.is_ascii_whitespace() || character == '(')
            .next()
            .unwrap_or("");
        if name.is_empty() || rest[name.len()..].starts_with('(') {
            continue;
        }
        names.insert(name.to_string());
    }
    names
}

/// Returns object-like macro replacement text, joining line continuations.
fn c_object_macro_values(text: &str) -> BTreeMap<String, String> {
    let mut values: BTreeMap<String, String> = BTreeMap::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut index: usize = 0;
    while index < lines.len() {
        let line: &str = lines[index].trim_start();
        let Some(rest) = line.strip_prefix("#define") else {
            index += 1;
            continue;
        };
        let rest: &str = rest.trim_start();
        let name: &str = rest
            .split(|character: char| character.is_ascii_whitespace() || character == '(')
            .next()
            .unwrap_or("");
        if name.is_empty() || rest[name.len()..].starts_with('(') {
            index += 1;
            continue;
        }
        let mut value: String = rest[name.len()..].trim().to_string();
        while value.ends_with('\\') && index + 1 < lines.len() {
            value.pop();
            index += 1;
            value.push(' ');
            value.push_str(lines[index].trim());
        }
        if let Some(comment) = value.find("/**<") {
            value.truncate(comment);
        }
        values.insert(name.to_string(), value.trim().to_string());
        index += 1;
    }
    values
}

/// Normalizes semantically comparable C/Rust constant expressions.
fn normalize_constant_value(value: &str) -> String {
    let mut normalized: String = value
        .chars()
        .filter(|character| !character.is_ascii_whitespace() && *character != '_')
        .collect::<String>()
        .replace("((uint8_t)", "(")
        .replace("((uint16_t)", "(")
        .replace("((uint32_t)", "(")
        .replace("((uint64_t)", "(")
        .replace("((int8_t)", "(")
        .replace("((int16_t)", "(")
        .replace("((int32_t)", "(")
        .replace("((int64_t)", "(");
    loop {
        if !normalized.starts_with('(') || !normalized.ends_with(')') {
            break;
        }
        let mut depth: usize = 0;
        let mut encloses_all: bool = true;
        for (index, character) in normalized.char_indices() {
            match character {
                '(' => depth += 1,
                ')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 && index + character.len_utf8() != normalized.len() {
                        encloses_all = false;
                        break;
                    }
                },
                _ => {},
            }
        }
        if !encloses_all || depth != 0 {
            break;
        }
        normalized = normalized[1..normalized.len() - 1].to_string();
    }
    normalized
}

/// Returns whether text inside parentheses names a simple C scalar type.
fn is_simple_c_scalar_type(value: &str) -> bool {
    let value: &str = value.trim();
    if value.is_empty() || value.contains('*') {
        return false;
    }
    let keywords: &[&str] = &[
        "const", "volatile", "signed", "unsigned", "char", "short", "int", "long", "bool",
    ];
    value.split_ascii_whitespace().all(|token| {
        keywords.contains(&token)
            || token.ends_with("_t")
            || token.starts_with("c_")
            || matches!(token, "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64")
    })
}

/// Removes simple C scalar casts while retaining grouping parentheses.
fn strip_c_scalar_casts(value: &str) -> String {
    let bytes: &[u8] = value.as_bytes();
    let mut result: String = String::with_capacity(value.len());
    let mut index: usize = 0;
    while index < bytes.len() {
        if bytes[index] == b'(' {
            if let Some(relative_end) = value[index + 1..].find(')') {
                let end: usize = index + 1 + relative_end;
                let candidate: &str = &value[index + 1..end];
                if !candidate.contains('(') && is_simple_c_scalar_type(candidate) {
                    index = end + 1;
                    continue;
                }
            }
        }
        result.push(bytes[index] as char);
        index += 1;
    }
    result
}

/// Converts C integer literals to unsuffixed decimal literals accepted by `syn`.
fn normalize_c_integer_literals(value: &str) -> String {
    let bytes: &[u8] = value.as_bytes();
    let mut result: String = String::with_capacity(value.len());
    let mut index: usize = 0;
    while index < bytes.len() {
        if matches!(bytes[index], b'\'' | b'"') {
            let quote: u8 = bytes[index];
            result.push(quote as char);
            index += 1;
            while index < bytes.len() {
                result.push(bytes[index] as char);
                if bytes[index] == b'\\' && index + 1 < bytes.len() {
                    index += 1;
                    result.push(bytes[index] as char);
                } else if bytes[index] == quote {
                    index += 1;
                    break;
                }
                index += 1;
            }
            continue;
        }
        let starts_number: bool = bytes[index].is_ascii_digit()
            && (index == 0
                || (!bytes[index - 1].is_ascii_alphanumeric() && bytes[index - 1] != b'_'));
        if !starts_number {
            result.push(bytes[index] as char);
            index += 1;
            continue;
        }
        let start: usize = index;
        while index < bytes.len() && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
        {
            index += 1;
        }
        let token: String = value[start..index].replace('_', "");
        let (digits, radix): (&str, u32) = if let Some(digits) = token.strip_prefix("0x") {
            let end: usize = digits
                .find(|character: char| !character.is_ascii_hexdigit())
                .unwrap_or(digits.len());
            (&digits[..end], 16)
        } else if let Some(digits) = token.strip_prefix("0X") {
            let end: usize = digits
                .find(|character: char| !character.is_ascii_hexdigit())
                .unwrap_or(digits.len());
            (&digits[..end], 16)
        } else {
            let end: usize = token
                .find(|character: char| !character.is_ascii_digit())
                .unwrap_or(token.len());
            let digits: &str = &token[..end];
            (
                digits,
                if digits.len() > 1 && digits.starts_with('0') {
                    8
                } else {
                    10
                },
            )
        };
        match u128::from_str_radix(digits, radix) {
            Ok(number) => result.push_str(&number.to_string()),
            Err(_) => result.push_str(&value[start..index]),
        }
    }
    result
}

/// Produces a parenthesis-insensitive canonical form for a parsed expression.
fn canonicalize_expr(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Lit(expr) => match &expr.lit {
            syn::Lit::Int(value) => value
                .base10_parse::<u128>()
                .ok()
                .map(|value| value.to_string()),
            syn::Lit::Bool(value) => Some(if value.value { "1" } else { "0" }.to_string()),
            syn::Lit::Char(value) => Some((value.value() as u32).to_string()),
            syn::Lit::Str(value) => Some(format!("{:?}", value.value())),
            _ => None,
        },
        syn::Expr::Path(path) if path.qself.is_none() => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string()),
        syn::Expr::Unary(expr) => {
            let operator: &str = match expr.op {
                syn::UnOp::Neg(_) => "-",
                syn::UnOp::Not(_) => "!",
                _ => return None,
            };
            Some(format!("{operator}({})", canonicalize_expr(&expr.expr)?))
        },
        syn::Expr::Binary(expr) => Some(format!(
            "({}{}{})",
            canonicalize_expr(&expr.left)?,
            translate_binary_operator(&expr.op)?,
            canonicalize_expr(&expr.right)?
        )),
        syn::Expr::Paren(expr) => canonicalize_expr(&expr.expr),
        syn::Expr::Group(expr) => canonicalize_expr(&expr.expr),
        syn::Expr::Cast(expr) => canonicalize_expr(&expr.expr),
        _ => None,
    }
}

/// Canonicalizes a supported C constant expression.
fn canonicalize_c_constant(value: &str) -> Option<String> {
    let value: String = strip_c_scalar_casts(value).replace('~', "!");
    let value: String = normalize_c_integer_literals(&value);
    let expr: syn::Expr = syn::parse_str(&value).ok()?;
    canonicalize_expr(&expr)
}

/// Returns whether two supported constant expressions have equivalent structure and literals.
fn constant_values_equivalent(left: &str, right: &str) -> bool {
    if normalize_constant_value(left) == normalize_constant_value(right) {
        return true;
    }
    matches!(
        (canonicalize_c_constant(left), canonicalize_c_constant(right)),
        (Some(left), Some(right)) if left == right
    )
}

/// Returns named C typedefs, structs, and unions declared by a C text block.
fn c_type_names(text: &str) -> BTreeSet<String> {
    let mut names: BTreeSet<String> = BTreeSet::new();
    for line in text.lines() {
        let line: &str = line.trim();
        for keyword in ["struct ", "union "] {
            if let Some(rest) = line.strip_prefix(keyword) {
                let is_definition: bool = line.contains('{');
                let is_forward_declaration: bool = line.ends_with(';')
                    && !line.contains('*')
                    && rest.split_ascii_whitespace().count() == 1;
                if !is_definition && !is_forward_declaration {
                    continue;
                }
                let name: &str = rest
                    .split(|character: char| {
                        character.is_ascii_whitespace() || matches!(character, '{' | ';')
                    })
                    .next()
                    .unwrap_or("");
                if !name.is_empty() {
                    names.insert(name.to_string());
                }
            }
        }
        if line.starts_with("typedef ") && line.ends_with(';') {
            if let Some(pointer_start) = line.find("(*") {
                let rest: &str = &line[pointer_start + 2..];
                let name: &str = rest
                    .split(|character: char| {
                        character.is_ascii_whitespace() || matches!(character, ')' | '(')
                    })
                    .next()
                    .unwrap_or("");
                if !name.is_empty() {
                    names.insert(name.to_string());
                    continue;
                }
            }
            let declarator: &str = line.trim_end_matches(';').trim_end();
            let name: &str = declarator
                .rsplit(|character: char| {
                    character.is_ascii_whitespace() || matches!(character, '*' | ']' | ')')
                })
                .find(|token| !token.is_empty() && *token != "const")
                .unwrap_or("");
            if !name.is_empty() && name != "struct" && name != "union" {
                names.insert(name.trim_end_matches('[').to_string());
            }
        }
    }
    names
}

/// Validates that raw object declarations are Rust-bound or explicitly C-only.
fn validate_raw_declarations(spec: &HeaderSpec) -> Result<()> {
    let mut bound_macros: BTreeSet<String> = spec
        .rust_constants
        .iter()
        .map(|export| export.c_name().to_string())
        .collect();
    let mut bound_types: BTreeSet<String> = spec
        .rust_types
        .iter()
        .flat_map(|export| [export.c_name().to_string(), export.tag_name().to_string()])
        .collect();
    for export in &spec.rust_constants {
        if let Some(bindings) = export.c_bindings() {
            bound_macros.extend(bindings.keys().cloned());
        }
    }
    for export in &spec.rust_types {
        bound_types.extend(export.c_bindings.keys().cloned());
    }

    for macro_def in &spec.macros {
        if !bound_macros.contains(&macro_def.name)
            && macro_def.c_only_reason.as_deref().is_none_or(str::is_empty)
        {
            bail!(
                "{}: object macro `{}` is not Rust-bound and has no c_only_reason",
                spec.file,
                macro_def.name
            );
        }
    }

    let audit_block = |label: &str, text: &str, reason: Option<&str>| -> Result<()> {
        let unbound_macros: Vec<String> = c_object_macros(text)
            .difference(&bound_macros)
            .cloned()
            .collect();
        let unbound_types: Vec<String> = c_type_names(text)
            .difference(&bound_types)
            .cloned()
            .collect();
        if (!unbound_macros.is_empty() || !unbound_types.is_empty())
            && reason.is_none_or(str::is_empty)
        {
            bail!(
                "{}: {label} has unbound C declarations (macros={unbound_macros:?}, \
                 types={unbound_types:?}); add Rust exports or c_only_reason",
                spec.file
            );
        }
        Ok(())
    };
    for (index, type_def) in spec.types.iter().enumerate() {
        audit_block(&format!("types:{index}"), &type_def.text, type_def.c_only_reason.as_deref())?;
    }
    for (index, section) in spec.raw_sections.iter().enumerate() {
        audit_block(&format!("raw:{index}"), &section.text, section.c_only_reason.as_deref())?;
    }
    for export in &spec.rust_constants {
        if let Some(declaration) = export.c_declaration() {
            let reason: Option<&str> = match export {
                RustConstantExport::Path(_) => None,
                RustConstantExport::Detailed(export) => export.c_only_reason.as_deref(),
            };
            audit_block(
                &format!("Rust constant `{}` c_declaration", export.path()),
                declaration,
                reason,
            )?;
        }
    }
    for export in &spec.rust_types {
        if let Some(declaration) = &export.c_declaration {
            audit_block(
                &format!("Rust type `{}` c_declaration", export.path),
                declaration,
                export.c_only_reason.as_deref(),
            )?;
        }
    }
    if let Some(trailer) = &spec.trailer {
        audit_block("trailer", &trailer.text, trailer.c_only_reason.as_deref())?;
    }
    Ok(())
}

/// Validates supplemental exact-declaration bindings against indexed public Rust symbols.
fn validate_exact_bindings(spec: &HeaderSpec, symbols: &RustSymbols) -> Result<()> {
    for export in &spec.rust_constants {
        if let Some(bindings) = export.c_bindings() {
            for (c_name, path) in bindings {
                let symbol: &RustConstant = symbols.constants.get(path).with_context(|| {
                    format!(
                        "{}: C macro `{c_name}` binds missing Rust constant `{path}`",
                        spec.file
                    )
                })?;
                if !symbol.public {
                    bail!(
                        "{}: C macro `{c_name}` binds non-public Rust constant `{path}`",
                        spec.file
                    );
                }
            }
        }
    }
    for export in &spec.rust_types {
        for (c_name, path) in &export.c_bindings {
            let symbol: &RustType = symbols.types.get(path).with_context(|| {
                format!("{}: C type `{c_name}` binds missing Rust type `{path}`", spec.file)
            })?;
            if !symbol.public {
                bail!("{}: C type `{c_name}` binds non-public Rust type `{path}`", spec.file);
            }
        }
    }
    Ok(())
}

/// Maps every public path of each selected Rust constant to its generated C name.
fn exported_constant_names(
    exports: &[RustConstantExport],
    symbols: &RustSymbols,
) -> Result<BTreeMap<String, String>> {
    let mut exported_names: BTreeMap<String, String> = BTreeMap::new();
    for export in exports {
        let symbol: &RustConstant = symbols.constants.get(export.path()).with_context(|| {
            format!("Rust constant `{}` was not found in scanned sources", export.path())
        })?;
        if !symbol.public {
            bail!("Rust constant `{}` is not publicly reachable", export.path());
        }
        let mut paths: Vec<&str> = vec![export.path(), symbol.canonical_path.as_str()];
        paths.extend(symbols.constants.iter().filter_map(|(path, candidate)| {
            (candidate.public && candidate.canonical_path == symbol.canonical_path)
                .then_some(path.as_str())
        }));
        for path in paths {
            if let Some(existing) =
                exported_names.insert(path.to_string(), export.c_name().to_string())
            {
                if existing != export.c_name() {
                    bail!(
                        "Rust constant `{path}` is exported with conflicting C names `{existing}` \
                         and `{}`",
                        export.c_name()
                    );
                }
            }
        }
    }
    Ok(exported_names)
}

/// Validates source-bound raw macro values against their Rust constants.
fn validate_raw_constant_values(spec: &HeaderSpec, symbols: &RustSymbols) -> Result<()> {
    let exported_names: BTreeMap<String, String> =
        exported_constant_names(&spec.rust_constants, symbols)?;
    let mut raw_values: BTreeMap<String, String> = BTreeMap::new();
    for section in &spec.raw_sections {
        raw_values.extend(c_object_macro_values(&section.text));
    }
    for type_def in &spec.types {
        raw_values.extend(c_object_macro_values(&type_def.text));
    }
    if let Some(trailer) = &spec.trailer {
        raw_values.extend(c_object_macro_values(&trailer.text));
    }

    for export in &spec.rust_constants {
        let Some(raw_value) = raw_values.get(export.c_name()) else {
            continue;
        };
        let symbol: &RustConstant = &symbols.constants[export.path()];
        let scoped_names: BTreeMap<String, String> = scoped_rust_names(
            &exported_names,
            &symbol.module_path,
            &symbol.imports,
            &symbol.glob_imports,
        );
        let translated: String = translate_constant_expr_typed(
            &symbol.expr,
            &symbol.module_path,
            &scoped_names,
            Some(&symbol.ty),
        )?;
        let (expected, unchecked_reason): (&str, Option<&str>) = match export {
            RustConstantExport::Path(_) => (&translated, None),
            RustConstantExport::Detailed(export) => (
                export.c_value.as_deref().unwrap_or(&translated),
                export.unchecked_c_value_reason.as_deref(),
            ),
        };
        if unchecked_reason.is_some() {
            continue;
        }
        if !constant_values_equivalent(raw_value, expected) {
            bail!(
                "{}: raw macro `{}` value `{raw_value}` differs from Rust-bound value \
                 `{expected}`; set c_value for exact spelling or unchecked_c_value_reason for a \
                 non-comparable C representation",
                spec.file,
                export.c_name()
            );
        }
    }
    Ok(())
}

/// Resolves selected Rust constants and converts them to C macro definitions.
#[cfg(test)]
fn resolve_rust_constants(
    exports: &[RustConstantExport],
    symbols: &RustSymbols,
) -> Result<Vec<Macro>> {
    resolve_rust_constants_with_docs(exports, symbols, true)
}

/// Resolves selected Rust constants with a header-level documentation policy.
fn resolve_rust_constants_with_docs(
    exports: &[RustConstantExport],
    symbols: &RustSymbols,
    rust_docs_enabled: bool,
) -> Result<Vec<Macro>> {
    let exported_names: BTreeMap<String, String> = exported_constant_names(exports, symbols)?;
    let mut macros: Vec<Macro> = Vec::new();
    for export in exports {
        let symbol: &RustConstant = symbols.constants.get(export.path()).with_context(|| {
            format!("Rust constant `{}` was not found in scanned sources", export.path())
        })?;
        if !symbol.public {
            bail!("Rust constant `{}` is not publicly reachable", export.path());
        }
        let scoped_names: BTreeMap<String, String> = scoped_rust_names(
            &exported_names,
            &symbol.module_path,
            &symbol.imports,
            &symbol.glob_imports,
        );
        let value: String = translate_constant_expr_typed(
            &symbol.expr,
            &symbol.module_path,
            &scoped_names,
            Some(&symbol.ty),
        )
        .with_context(|| format!("failed to translate Rust constant `{}`", export.path()))?;
        if export.emit() {
            let comment: Option<String> = if export.no_comment() {
                None
            } else {
                export.comment().map(str::to_string).or_else(|| {
                    rust_docs_enabled
                        .then(|| rust_doc_summary(&symbol.docs))
                        .flatten()
                })
            };
            macros.push(Macro {
                name: export.c_name().to_string(),
                value: export.c_value().unwrap_or(&value).to_string(),
                comment,
                guard: export.guard().map(str::to_string),
                declaration: export.c_declaration().map(str::to_string),
                c_only_reason: None,
            });
        }
    }
    Ok(macros)
}

/// Resolves one named group of selected Rust constants.
fn resolve_rust_constant_group(
    exports: &[RustConstantExport],
    symbols: &RustSymbols,
    group: &str,
    rust_docs_enabled: bool,
) -> Result<(Vec<Macro>, String)> {
    let exported_names: BTreeMap<String, String> = exported_constant_names(exports, symbols)?;
    let mut macros: Vec<Macro> = Vec::new();
    let mut title: Option<String> = None;
    for export in exports.iter().filter(|export| export.group() == group) {
        let symbol: &RustConstant = symbols.constants.get(export.path()).with_context(|| {
            format!("Rust constant `{}` was not found in scanned sources", export.path())
        })?;
        if !symbol.public {
            bail!("Rust constant `{}` is not publicly reachable", export.path());
        }
        let scoped_names: BTreeMap<String, String> = scoped_rust_names(
            &exported_names,
            &symbol.module_path,
            &symbol.imports,
            &symbol.glob_imports,
        );
        let value: String = translate_constant_expr_typed(
            &symbol.expr,
            &symbol.module_path,
            &scoped_names,
            Some(&symbol.ty),
        )
        .with_context(|| format!("failed to translate Rust constant `{}`", export.path()))?;
        if let Some(section_title) = export.section_title() {
            match &title {
                Some(existing) if existing != section_title => bail!(
                    "Rust constant group `{group}` has conflicting section titles `{existing}` \
                     and `{section_title}`"
                ),
                None => title = Some(section_title.to_string()),
                _ => {},
            }
        }
        if export.emit() {
            let comment: Option<String> = if export.no_comment() {
                None
            } else {
                export.comment().map(str::to_string).or_else(|| {
                    rust_docs_enabled
                        .then(|| rust_doc_summary(&symbol.docs))
                        .flatten()
                })
            };
            macros.push(Macro {
                name: export.c_name().to_string(),
                value: export.c_value().unwrap_or(&value).to_string(),
                comment,
                guard: export.guard().map(str::to_string),
                declaration: export.c_declaration().map(str::to_string),
                c_only_reason: None,
            });
        }
    }
    Ok((macros, title.unwrap_or_else(|| "Constants".to_string())))
}

/// Maps a Rust path type to a C base type, including types exported by this header.
fn map_exported_path(
    path: &syn::TypePath,
    module_path: &[String],
    type_names: &BTreeMap<String, String>,
) -> Result<String> {
    let segment: &syn::PathSegment = path.path.segments.last().context("empty Rust type path")?;
    let ident: String = segment.ident.to_string();
    let resolved: String = resolve_constant_path(&path.path, module_path);
    if let Some(c_name) = type_names.get(&resolved) {
        return Ok(c_name.clone());
    }
    if let Some(imported_ident) = resolved.rsplit("::").next() {
        if let Some(c_name) = map_ident_to_c(imported_ident) {
            return Ok(c_name.to_string());
        }
    }
    map_ident_to_c(&ident)
        .map(str::to_string)
        .with_context(|| format!("unsupported Rust ABI type `{resolved}`"))
}

/// Formats a C declarator for a Rust type and identifier.
fn format_c_declarator(
    ty: &syn::Type,
    name: &str,
    module_path: &[String],
    type_names: &BTreeMap<String, String>,
    constant_names: &BTreeMap<String, String>,
    array_aliases: &BTreeMap<String, bool>,
) -> Result<String> {
    format_c_declarator_qualified(
        ty,
        name,
        false,
        module_path,
        type_names,
        constant_names,
        array_aliases,
    )
}

/// Returns whether a Rust type is a direct array or a selected alias to one.
fn is_direct_array_type(
    ty: &syn::Type,
    module_path: &[String],
    array_aliases: &BTreeMap<String, bool>,
) -> bool {
    match ty {
        syn::Type::Array(_) => true,
        syn::Type::Path(path) => {
            let resolved: String = resolve_constant_path(&path.path, module_path);
            array_aliases.contains_key(&resolved)
        },
        syn::Type::Paren(inner) => is_direct_array_type(&inner.elem, module_path, array_aliases),
        syn::Type::Group(inner) => is_direct_array_type(&inner.elem, module_path, array_aliases),
        _ => false,
    }
}

/// Recursively formats a C declarator, applying `object_const` to the current type level.
fn format_c_declarator_qualified(
    ty: &syn::Type,
    name: &str,
    object_const: bool,
    module_path: &[String],
    type_names: &BTreeMap<String, String>,
    constant_names: &BTreeMap<String, String>,
    array_aliases: &BTreeMap<String, bool>,
) -> Result<String> {
    match ty {
        syn::Type::Path(path) => {
            let segment: &syn::PathSegment =
                path.path.segments.last().context("empty Rust type path")?;
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if args.args.len() == 1 {
                        if let Some(syn::GenericArgument::Type(inner @ syn::Type::FnPtr(_))) =
                            args.args.first()
                        {
                            return format_c_declarator_qualified(
                                inner,
                                name,
                                object_const,
                                module_path,
                                type_names,
                                constant_names,
                                array_aliases,
                            );
                        }
                    }
                    if let Some(syn::GenericArgument::Type(_)) = args.args.first() {
                        bail!(
                            "only Option<extern \"C\" fn> has a supported C ABI; use a raw C type \
                             override for other Option types"
                        );
                    }
                }
                bail!("Option in a Rust ABI type must contain exactly one type");
            }
            let c_type: String = map_exported_path(path, module_path, type_names)?;
            let qualifier: &str = if object_const { "const " } else { "" };
            if name.is_empty() {
                Ok(format!("{qualifier}{c_type}"))
            } else {
                Ok(format!("{qualifier}{c_type} {name}"))
            }
        },
        syn::Type::Ptr(pointer) => {
            let pointer_name: String = if object_const {
                format!("* const {name}")
            } else {
                format!("*{name}")
            };
            let pointee_const: bool =
                matches!(&pointer.mutability, syn::PointerMutability::Const(_));
            format_c_declarator_qualified(
                &pointer.elem,
                &pointer_name,
                pointee_const,
                module_path,
                type_names,
                constant_names,
                array_aliases,
            )
        },
        syn::Type::Array(array) => {
            let length: String = translate_constant_expr(&array.len, module_path, constant_names)
                .context("failed to translate Rust array length")?;
            let array_name: String = if name.starts_with('*') {
                format!("({name})[{length}]")
            } else {
                format!("{name}[{length}]")
            };
            format_c_declarator_qualified(
                &array.elem,
                &array_name,
                object_const,
                module_path,
                type_names,
                constant_names,
                array_aliases,
            )
        },
        syn::Type::FnPtr(function) => {
            if !matches!(&function.abi, Some(abi) if matches!(&abi.name, Some(name) if name.value() == "C"))
            {
                bail!("Rust function pointers exported to C must use the C ABI");
            }
            let mut parameters: Vec<String> = Vec::new();
            for parameter in &function.inputs {
                if is_direct_array_type(&parameter.ty, module_path, array_aliases) {
                    bail!(
                        "direct Rust array parameters have no equivalent C function ABI; use a \
                         pointer-to-array type or a raw C type override"
                    );
                }
                parameters.push(format_c_declarator_qualified(
                    &parameter.ty,
                    "",
                    false,
                    module_path,
                    type_names,
                    constant_names,
                    array_aliases,
                )?);
            }
            if function.variadic.is_some() {
                parameters.push("...".to_string());
            }
            let parameters: String = if parameters.is_empty() {
                "void".to_string()
            } else {
                parameters.join(", ")
            };
            let pointer_name: String = if object_const {
                format!("* const {name}")
            } else {
                format!("*{name}")
            };
            let function_name: String = format!("({pointer_name})({parameters})");
            match &function.output {
                syn::ReturnType::Default => Ok(format!("void {function_name}")),
                syn::ReturnType::Type(_, ty) => {
                    if is_direct_array_type(ty, module_path, array_aliases) {
                        bail!(
                            "direct Rust array return types have no equivalent C function ABI; \
                             use a pointer-to-array type or a raw C type override"
                        );
                    }
                    format_c_declarator_qualified(
                        ty,
                        &function_name,
                        false,
                        module_path,
                        type_names,
                        constant_names,
                        array_aliases,
                    )
                },
            }
        },
        syn::Type::Paren(inner) => format_c_declarator_qualified(
            &inner.elem,
            name,
            object_const,
            module_path,
            type_names,
            constant_names,
            array_aliases,
        ),
        syn::Type::Group(inner) => format_c_declarator_qualified(
            &inner.elem,
            name,
            object_const,
            module_path,
            type_names,
            constant_names,
            array_aliases,
        ),
        _ => bail!("unsupported Rust ABI type; use a raw C type override"),
    }
}

/// Adds selected-type dependencies referenced by a Rust type.
fn collect_type_dependencies(
    ty: &syn::Type,
    module_path: &[String],
    exported_indices: &BTreeMap<String, usize>,
    specs: &[RustTypeSpec],
    behind_pointer: bool,
    dependencies: &mut Vec<usize>,
) {
    match ty {
        syn::Type::Path(path) => {
            if path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "Option")
            {
                if let Some(syn::PathSegment {
                    arguments: syn::PathArguments::AngleBracketed(arguments),
                    ..
                }) = path.path.segments.last()
                {
                    if let Some(syn::GenericArgument::Type(inner)) = arguments.args.first() {
                        collect_type_dependencies(
                            inner,
                            module_path,
                            exported_indices,
                            specs,
                            true,
                            dependencies,
                        );
                    }
                }
                return;
            }
            let resolved: String = resolve_constant_path(&path.path, module_path);
            if let Some(index) = exported_indices.get(&resolved) {
                if !(behind_pointer && specs[*index].forward_declare)
                    && !dependencies.contains(index)
                {
                    dependencies.push(*index);
                }
            }
        },
        syn::Type::Ptr(pointer) => collect_type_dependencies(
            &pointer.elem,
            module_path,
            exported_indices,
            specs,
            true,
            dependencies,
        ),
        syn::Type::Array(array) => collect_type_dependencies(
            &array.elem,
            module_path,
            exported_indices,
            specs,
            false,
            dependencies,
        ),
        syn::Type::FnPtr(function) => {
            if let syn::ReturnType::Type(_, ty) = &function.output {
                collect_type_dependencies(
                    ty,
                    module_path,
                    exported_indices,
                    specs,
                    true,
                    dependencies,
                );
            }
            for parameter in &function.inputs {
                collect_type_dependencies(
                    &parameter.ty,
                    module_path,
                    exported_indices,
                    specs,
                    true,
                    dependencies,
                );
            }
        },
        syn::Type::Paren(inner) => collect_type_dependencies(
            &inner.elem,
            module_path,
            exported_indices,
            specs,
            behind_pointer,
            dependencies,
        ),
        syn::Type::Group(inner) => collect_type_dependencies(
            &inner.elem,
            module_path,
            exported_indices,
            specs,
            behind_pointer,
            dependencies,
        ),
        _ => {},
    }
}

/// Visits one exported type while producing a stable topological order.
fn visit_rust_type(
    index: usize,
    dependencies: &[Vec<usize>],
    states: &mut [u8],
    order: &mut Vec<usize>,
    specs: &[RustTypeSpec],
) -> Result<()> {
    match states[index] {
        2 => return Ok(()),
        1 => bail!(
            "cyclic Rust type dependency at `{}`; use tagged forward declarations",
            specs[index].path
        ),
        _ => {},
    }
    states[index] = 1;
    for dependency in &dependencies[index] {
        visit_rust_type(*dependency, dependencies, states, order, specs)?;
    }
    states[index] = 2;
    order.push(index);
    Ok(())
}

/// Returns selected Rust types in stable dependency order.
fn exported_type_indices(
    specs: &[RustTypeSpec],
    symbols: &RustSymbols,
) -> Result<BTreeMap<String, usize>> {
    let mut exported_indices: BTreeMap<String, usize> = BTreeMap::new();
    for (index, spec) in specs.iter().enumerate() {
        let symbol: &RustType = symbols.types.get(&spec.path).with_context(|| {
            format!("Rust type `{}` was not found in scanned sources", spec.path)
        })?;
        if !symbol.public {
            bail!("Rust type `{}` is not publicly reachable", spec.path);
        }
        let mut paths: Vec<&String> = vec![&spec.path, &symbol.canonical_path];
        paths.extend(symbols.types.iter().filter_map(|(path, candidate)| {
            (candidate.public && candidate.canonical_path == symbol.canonical_path).then_some(path)
        }));
        for path in paths {
            if let Some(existing) = exported_indices.insert(path.clone(), index) {
                if existing != index {
                    bail!("Rust type `{path}` is exported more than once by the same header");
                }
            }
        }
    }
    Ok(exported_indices)
}

/// Resolves whether selected aliases eventually name direct array types.
struct ArrayAliasResolver<'a> {
    specs: &'a [RustTypeSpec],
    symbols: &'a RustSymbols,
    exported_indices: &'a BTreeMap<String, usize>,
    states: Vec<u8>,
    results: Vec<bool>,
}

impl ArrayAliasResolver<'_> {
    /// Determines whether one selected alias resolves to an array type.
    fn resolve(&mut self, index: usize) -> Result<bool> {
        match self.states[index] {
            2 => return Ok(self.results[index]),
            1 => bail!("cyclic Rust alias dependency at `{}`", self.specs[index].path),
            _ => {},
        }
        self.states[index] = 1;
        let symbol: RustType = self.symbols.types[&self.specs[index].path].clone();
        let scoped_indices: BTreeMap<String, usize> = scoped_rust_names(
            self.exported_indices,
            &symbol.module_path,
            &symbol.imports,
            &symbol.glob_imports,
        );
        self.results[index] = match &symbol.kind {
            RustTypeKind::Alias(ty) => {
                self.type_is_array(ty, &scoped_indices, &symbol.module_path)?
            },
            RustTypeKind::Struct(_) | RustTypeKind::Union(_) => false,
        };
        self.states[index] = 2;
        Ok(self.results[index])
    }

    /// Determines whether a type is an array or an alias to one.
    fn type_is_array(
        &mut self,
        ty: &syn::Type,
        scoped_indices: &BTreeMap<String, usize>,
        module_path: &[String],
    ) -> Result<bool> {
        match ty {
            syn::Type::Array(_) => Ok(true),
            syn::Type::Paren(inner) => self.type_is_array(&inner.elem, scoped_indices, module_path),
            syn::Type::Group(inner) => self.type_is_array(&inner.elem, scoped_indices, module_path),
            syn::Type::Path(path) => {
                let resolved: String = resolve_constant_path(&path.path, module_path);
                if let Some(next) = scoped_indices.get(&resolved) {
                    self.resolve(*next)
                } else {
                    Ok(false)
                }
            },
            _ => Ok(false),
        }
    }
}

/// Returns every selected Rust path whose alias resolves directly to an array.
fn selected_array_aliases(
    specs: &[RustTypeSpec],
    symbols: &RustSymbols,
) -> Result<BTreeMap<String, bool>> {
    let exported_indices: BTreeMap<String, usize> = exported_type_indices(specs, symbols)?;
    let mut resolver: ArrayAliasResolver<'_> = ArrayAliasResolver {
        specs,
        symbols,
        exported_indices: &exported_indices,
        states: vec![0; specs.len()],
        results: vec![false; specs.len()],
    };
    for index in 0..specs.len() {
        resolver.resolve(index)?;
    }
    let results: Vec<bool> = resolver.results.clone();
    drop(resolver);
    Ok(exported_indices
        .into_iter()
        .filter_map(|(path, index)| results[index].then_some((path, true)))
        .collect())
}

/// Returns selected Rust types in stable dependency order.
fn order_rust_types(specs: &[RustTypeSpec], symbols: &RustSymbols) -> Result<Vec<usize>> {
    let exported_indices: BTreeMap<String, usize> = exported_type_indices(specs, symbols)?;

    let mut dependencies: Vec<Vec<usize>> = vec![Vec::new(); specs.len()];
    for (index, spec) in specs.iter().enumerate() {
        let symbol: &RustType = symbols.types.get(&spec.path).with_context(|| {
            format!("Rust type `{}` was not found in scanned sources", spec.path)
        })?;
        let scoped_indices: BTreeMap<String, usize> = scoped_rust_names(
            &exported_indices,
            &symbol.module_path,
            &symbol.imports,
            &symbol.glob_imports,
        );
        match &symbol.kind {
            RustTypeKind::Alias(ty) => collect_type_dependencies(
                ty,
                &symbol.module_path,
                &scoped_indices,
                specs,
                false,
                &mut dependencies[index],
            ),
            RustTypeKind::Struct(fields) | RustTypeKind::Union(fields) => {
                for field in fields {
                    collect_type_dependencies(
                        &field.ty,
                        &symbol.module_path,
                        &scoped_indices,
                        specs,
                        false,
                        &mut dependencies[index],
                    );
                }
            },
        }
    }

    let mut states: Vec<u8> = vec![0; specs.len()];
    let mut order: Vec<usize> = Vec::new();
    for index in 0..specs.len() {
        visit_rust_type(index, &dependencies, &mut states, &mut order, specs)?;
    }
    Ok(order)
}

/// Returns the C reference spelling for one selected Rust type.
fn rust_type_reference(spec: &RustTypeSpec, symbol: &RustType) -> String {
    match spec.style {
        RustTypeStyle::Tag => match &symbol.kind {
            RustTypeKind::Union(_) => format!("union {}", spec.c_name()),
            _ => format!("struct {}", spec.c_name()),
        },
        RustTypeStyle::Typedef | RustTypeStyle::TypedefTag => spec.c_name().to_string(),
    }
}

/// Renders one selected Rust type definition.
fn render_rust_type(
    spec: &RustTypeSpec,
    symbol: &RustType,
    type_names: &BTreeMap<String, String>,
    constant_names: &BTreeMap<String, String>,
    array_aliases: &BTreeMap<String, bool>,
    rust_docs_enabled: bool,
) -> Result<String> {
    let mut lines: Vec<String> = Vec::new();
    if let Some(guard) = &spec.guard {
        lines.push(format!("#ifndef {guard}"));
    }
    if let Some(declaration) = &spec.c_declaration {
        lines.push(declaration.trim().to_string());
        if spec.guard.is_some() {
            lines.push("#endif".to_string());
        }
        return Ok(lines.join("\n"));
    }
    let type_names: BTreeMap<String, String> =
        scoped_rust_names(type_names, &symbol.module_path, &symbol.imports, &symbol.glob_imports);
    let constant_names: BTreeMap<String, String> = scoped_rust_names(
        constant_names,
        &symbol.module_path,
        &symbol.imports,
        &symbol.glob_imports,
    );
    let array_aliases: BTreeMap<String, bool> = scoped_rust_names(
        array_aliases,
        &symbol.module_path,
        &symbol.imports,
        &symbol.glob_imports,
    );
    let comment: Option<String> = if spec.no_comment {
        None
    } else {
        spec.comment.clone().or_else(|| {
            rust_docs_enabled
                .then(|| rust_doc_summary(&symbol.docs))
                .flatten()
        })
    };
    if let Some(comment) = comment {
        lines.push(format!("/** @brief {comment} */"));
    }

    if let RustTypeKind::Alias(ty) = &symbol.kind {
        if spec.style != RustTypeStyle::Typedef {
            bail!("Rust type alias `{}` only supports typedef style", spec.path);
        }
        let declaration: String = if let Some(c_type) = &spec.c_type {
            format!("{c_type} {}{}", spec.c_name(), spec.declarator_suffix)
        } else {
            format_c_declarator(
                ty,
                &format!("{}{}", spec.c_name(), spec.declarator_suffix),
                &symbol.module_path,
                &type_names,
                &constant_names,
                &array_aliases,
            )?
        };
        lines.push(format!("typedef {declaration};"));
        if spec.guard.is_some() {
            lines.push("#endif".to_string());
        }
        return Ok(lines.join("\n"));
    }

    let (keyword, fields): (&str, &[RustField]) = match &symbol.kind {
        RustTypeKind::Struct(fields) => ("struct", fields),
        RustTypeKind::Union(fields) => ("union", fields),
        RustTypeKind::Alias(_) => unreachable!(),
    };
    let packed: bool = spec.packed.unwrap_or(symbol.packed);
    let mut attributes: Vec<String> = Vec::new();
    if packed {
        attributes.push("packed".to_string());
    }
    if let Some(alignment) = symbol.alignment {
        attributes.push(format!("aligned({alignment})"));
    }
    let aggregate_attr: String = if attributes.is_empty() {
        String::new()
    } else {
        format!(" __attribute__(({}))", attributes.join(", "))
    };
    let opening: String = match spec.style {
        RustTypeStyle::Typedef => format!("typedef {keyword}{aggregate_attr} {{"),
        RustTypeStyle::Tag => format!("{keyword}{aggregate_attr} {} {{", spec.c_name()),
        RustTypeStyle::TypedefTag if spec.forward_declare => {
            format!("{keyword}{aggregate_attr} {} {{", spec.tag_name())
        },
        RustTypeStyle::TypedefTag => {
            format!("typedef {keyword}{aggregate_attr} {} {{", spec.tag_name())
        },
    };
    lines.push(opening);

    let mut declarations: Vec<(String, Option<String>)> = Vec::new();
    for field in fields {
        let c_name: &str = spec
            .field_renames
            .get(&field.name)
            .map(String::as_str)
            .unwrap_or(&field.name);
        let declaration: String = match spec.field_c_declarations.get(&field.name) {
            Some(declaration) => declaration.clone(),
            None => format_c_declarator(
                &field.ty,
                c_name,
                &symbol.module_path,
                &type_names,
                &constant_names,
                &array_aliases,
            )?,
        };
        let comment: Option<String> = spec.field_comments.get(&field.name).cloned().or_else(|| {
            rust_docs_enabled
                .then(|| rust_doc_summary(&field.docs))
                .flatten()
        });
        declarations.push((format!("{declaration};"), comment));
    }
    let declaration_width: usize = declarations
        .iter()
        .map(|(declaration, _)| declaration.len())
        .max()
        .unwrap_or(0);
    let comment_width: usize = declarations
        .iter()
        .filter_map(|(_, comment)| comment.as_ref().map(String::len))
        .max()
        .unwrap_or(0);
    for (declaration, comment) in declarations {
        if let Some(comment) = comment {
            lines.push(format!(
                "    {declaration:<declaration_width$} /**< {comment:<comment_width$} */"
            ));
        } else {
            lines.push(format!("    {declaration}"));
        }
    }

    let closing: String = match spec.style {
        RustTypeStyle::Typedef => {
            format!("}} {};", spec.c_name())
        },
        RustTypeStyle::TypedefTag if spec.forward_declare => "};".to_string(),
        RustTypeStyle::TypedefTag => format!("}} {};", spec.c_name()),
        RustTypeStyle::Tag => "};".to_string(),
    };
    lines.push(closing);
    if spec.guard.is_some() {
        lines.push("#endif".to_string());
    }
    Ok(lines.join("\n"))
}

/// Emits selected Rust types with forward declarations and dependency ordering.
#[cfg(test)]
fn emit_rust_types(
    lines: &mut Vec<String>,
    specs: &[RustTypeSpec],
    symbols: &RustSymbols,
    constant_exports: &[RustConstantExport],
) -> Result<()> {
    emit_rust_type_group(lines, specs, symbols, constant_exports, "", "Types", true)
}

/// Emits one named group of selected Rust types with dependency ordering.
fn emit_rust_type_group(
    lines: &mut Vec<String>,
    specs: &[RustTypeSpec],
    symbols: &RustSymbols,
    constant_exports: &[RustConstantExport],
    group: &str,
    default_title: &str,
    rust_docs_enabled: bool,
) -> Result<()> {
    if specs.is_empty() {
        return Ok(());
    }
    let order: Vec<usize> = order_rust_types(specs, symbols)?;
    let array_aliases: BTreeMap<String, bool> = selected_array_aliases(specs, symbols)?;
    let mut type_names: BTreeMap<String, String> = BTreeMap::new();
    for spec in specs {
        let symbol: &RustType = symbols.types.get(&spec.path).with_context(|| {
            format!("Rust type `{}` was not found in scanned sources", spec.path)
        })?;
        let c_name: String = rust_type_reference(spec, symbol);
        let mut paths: Vec<&String> = vec![&spec.path, &symbol.canonical_path];
        paths.extend(symbols.types.iter().filter_map(|(path, candidate)| {
            (candidate.public && candidate.canonical_path == symbol.canonical_path).then_some(path)
        }));
        for path in paths {
            if let Some(existing) = type_names.insert(path.clone(), c_name.clone()) {
                if existing != c_name {
                    bail!(
                        "Rust type `{path}` is exported with conflicting C names `{existing}` and \
                         `{c_name}`"
                    );
                }
            }
        }
    }
    let constant_names: BTreeMap<String, String> =
        exported_constant_names(constant_exports, symbols)?;

    let grouped_indices: BTreeSet<usize> = specs
        .iter()
        .enumerate()
        .filter_map(|(index, spec)| (spec.group == group && spec.emit).then_some(index))
        .collect();
    if grouped_indices.is_empty() {
        return Ok(());
    }
    let mut title: Option<String> = None;
    for spec in specs.iter().filter(|spec| spec.group == group) {
        if let Some(section_title) = &spec.section_title {
            match &title {
                Some(existing) if existing != section_title => bail!(
                    "Rust type group `{group}` has conflicting section titles `{existing}` and \
                     `{section_title}`"
                ),
                None => title = Some(section_title.clone()),
                _ => {},
            }
        }
    }
    let title: String = title.unwrap_or_else(|| default_title.to_string());
    if !title.is_empty() {
        lines.push(section_bar(&title));
    }
    lines.push(String::new());
    let mut emitted_forward: bool = false;
    for spec in specs.iter().filter(|spec| spec.forward_declare) {
        let symbol: &RustType = symbols
            .types
            .get(&spec.path)
            .expect("selected Rust type was checked while building type names");
        let keyword: &str = match &symbol.kind {
            RustTypeKind::Struct(_) => "struct",
            RustTypeKind::Union(_) => "union",
            RustTypeKind::Alias(_) => {
                bail!("Rust type alias `{}` cannot be forward declared", spec.path)
            },
        };
        match spec.style {
            RustTypeStyle::Typedef => {
                bail!("anonymous typedef `{}` cannot be forward declared", spec.path)
            },
            RustTypeStyle::Tag => lines.push(format!("{keyword} {};", spec.c_name())),
            RustTypeStyle::TypedefTag => {
                lines.push(format!("typedef {keyword} {} {};", spec.tag_name(), spec.c_name()))
            },
        }
        emitted_forward = true;
    }
    if emitted_forward {
        lines.push(String::new());
    }

    for index in order
        .into_iter()
        .filter(|index| grouped_indices.contains(index))
    {
        let spec: &RustTypeSpec = &specs[index];
        let symbol: &RustType = symbols
            .types
            .get(&spec.path)
            .expect("selected Rust type was checked while building type names");
        lines.push(render_rust_type(
            spec,
            symbol,
            &type_names,
            &constant_names,
            &array_aliases,
            rust_docs_enabled,
        )?);
        lines.push(String::new());
    }
    Ok(())
}

/// Emits a titled section of function declarations.
fn emit_section(
    lines: &mut Vec<String>,
    section: &Section,
    funcs: &BTreeMap<String, FuncSig>,
    overrides: &HashMap<String, String>,
) {
    if !section.title.is_empty() {
        lines.push(section_bar(&section.title));
        lines.push(String::new());
    }
    for fn_name in &section.functions {
        if let Some(override_decl) = overrides.get(fn_name) {
            lines.push(override_decl.clone());
        } else if let Some(sig) = funcs.get(fn_name) {
            lines.push(format_c_declaration(sig));
        } else {
            lines.push(format!("/* TODO: {fn_name} — not found in Rust source */"));
        }
    }
    lines.push(String::new());
}

/// Emits a titled block of raw C text.
fn emit_raw_section(lines: &mut Vec<String>, raw: &RawSection) {
    if !raw.title.is_empty() {
        lines.push(section_bar(&raw.title));
        lines.push(String::new());
    }
    lines.push(raw.text.trim().to_string());
    lines.push(String::new());
}

/// Builds the default content ordering when a spec omits `content_order`.
fn default_content_order(spec: &HeaderSpec) -> Vec<String> {
    let mut order: Vec<String> = Vec::new();
    if !spec.macros.is_empty() {
        order.push("macros".to_string());
    }
    if !spec.rust_constants.is_empty() {
        order.push("rust_constants".to_string());
    }
    if !spec.rust_types.is_empty() {
        order.push("rust_types".to_string());
    }
    for index in 0..spec.types.len() {
        order.push(format!("types:{index}"));
    }
    let count: usize = spec.sections.len().max(spec.raw_sections.len());
    for index in 0..count {
        if index < spec.sections.len() {
            order.push(format!("section:{index}"));
        }
        if index < spec.raw_sections.len() {
            order.push(format!("raw:{index}"));
        }
    }
    order
}

/// Emits a `Types` block for an explicit `types:N` ordering entry.
fn emit_indexed_type(lines: &mut Vec<String>, spec: &HeaderSpec, index: usize) {
    if index == 0 {
        lines.push(section_bar("Types"));
        lines.push(String::new());
    }
    if index < spec.types.len() {
        lines.push(spec.types[index].text.trim().to_string());
        lines.push(String::new());
    }
}

/// Renders a complete C header from a spec and parsed functions.
fn generate_header(
    spec: &HeaderSpec,
    funcs: &BTreeMap<String, FuncSig>,
    symbols: &RustSymbols,
) -> Result<String> {
    let mut lines: Vec<String> = Vec::new();
    validate_c_export_names(spec)?;
    validate_raw_declarations(spec)?;
    validate_exact_bindings(spec, symbols)?;
    validate_raw_constant_values(spec, symbols)?;
    let rust_constants: Vec<Macro> =
        resolve_rust_constants_with_docs(&spec.rust_constants, symbols, spec.rust_constant_docs)?;

    lines.push(COPYRIGHT.to_string());
    lines.push(String::new());

    lines.push(format!("#ifndef {}", spec.guard));
    lines.push(format!("#define {}", spec.guard));
    lines.push(String::new());

    // @file block.
    lines.push("/**".to_string());
    lines.push(format!(" * @file {}", spec.file));
    lines.push(format!(" * @brief {}", spec.brief));
    if !spec.description.trim().is_empty() {
        lines.push(" *".to_string());
        for desc_line in spec.description.trim().split('\n') {
            let desc_line: &str = desc_line.trim();
            if desc_line.is_empty() {
                lines.push(" *".to_string());
            } else {
                lines.push(format!(" * {desc_line}"));
            }
        }
    }
    lines.push(" */".to_string());
    lines.push(String::new());

    // Includes.
    for include in &spec.includes {
        lines.push(format!("#include <{include}>"));
    }
    if !spec.includes.is_empty() {
        lines.push(String::new());
    }

    // Anchor for a possible C++ `restrict` shim: it must sit after the includes
    // and before the `extern "C"` block. Whether the shim is actually needed is
    // only known once the body is rendered, so it is spliced in at the end.
    let restrict_shim_anchor: usize = lines.len();

    // extern "C" open.
    if spec.extern_c {
        lines.push("#ifdef __cplusplus".to_string());
        lines.push("extern \"C\" {".to_string());
        lines.push("#endif".to_string());
        lines.push(String::new());
    }

    // Content blocks.
    let content_order: Vec<String> = spec
        .content_order
        .clone()
        .unwrap_or_else(|| default_content_order(spec));
    for entry in &content_order {
        if entry == "macros" {
            emit_macros(&mut lines, &spec.macros);
        } else if entry == "rust_constants" {
            emit_macros(&mut lines, &rust_constants);
        } else if entry == "rust_types" {
            emit_rust_type_group(
                &mut lines,
                &spec.rust_types,
                symbols,
                &spec.rust_constants,
                "",
                "Types",
                spec.rust_type_docs,
            )?;
        } else if let Some(group) = entry.strip_prefix("rust_constants:") {
            let (macros, title): (Vec<Macro>, String) = resolve_rust_constant_group(
                &spec.rust_constants,
                symbols,
                group,
                spec.rust_constant_docs,
            )?;
            emit_macros_with_title(&mut lines, &macros, &title);
        } else if let Some(group) = entry.strip_prefix("rust_types:") {
            emit_rust_type_group(
                &mut lines,
                &spec.rust_types,
                symbols,
                &spec.rust_constants,
                group,
                "Types",
                spec.rust_type_docs,
            )?;
        } else if entry == "types" {
            if !spec.types.is_empty() {
                lines.push(section_bar("Types"));
                lines.push(String::new());
                for type_def in &spec.types {
                    lines.push(type_def.text.trim().to_string());
                    lines.push(String::new());
                }
            }
        } else if let Some(index) = entry.strip_prefix("types:") {
            if let Ok(index) = index.parse::<usize>() {
                emit_indexed_type(&mut lines, spec, index);
            }
        } else if let Some(index) = entry.strip_prefix("section:") {
            if let Ok(index) = index.parse::<usize>() {
                if index < spec.sections.len() {
                    emit_section(&mut lines, &spec.sections[index], funcs, &spec.overrides);
                }
            }
        } else if let Some(index) = entry.strip_prefix("raw:") {
            if let Ok(index) = index.parse::<usize>() {
                if index < spec.raw_sections.len() {
                    emit_raw_section(&mut lines, &spec.raw_sections[index]);
                }
            }
        }
    }

    // extern "C" close.
    if spec.extern_c {
        lines.push("#ifdef __cplusplus".to_string());
        lines.push("}".to_string());
        lines.push("#endif".to_string());
        lines.push(String::new());
    }

    // Trailer (outside the include guard, e.g. the assert macro).
    if let Some(trailer) = &spec.trailer {
        lines.push(trailer.text.trim().to_string());
        lines.push(String::new());
    }

    lines.push(format!("#endif /* {} */", spec.guard));
    lines.push(String::new());

    // C99's `restrict` is a parse error in C++. If the rendered body uses the
    // keyword, rewrite it to the private `__nanvix_restrict` qualifier macro and
    // emit that macro's definition after the includes (before the `extern "C"`
    // block). The macro expands to `restrict` in C and to nothing in C++.
    //
    // A private macro is used (rather than `#define restrict ...` directly) so
    // the public `restrict` name is never redefined: that avoids leaking an
    // empty `restrict` macro into the rest of a C++ translation unit and avoids
    // the `!defined(restrict)` hole where a caller that pre-defines `restrict`
    // would defeat the shim. The C++ expansion is empty rather than `__restrict`
    // because `__restrict` is itself rejected inside array-parameter brackets,
    // e.g. `regmatch_t pmatch[restrict]` in <regex.h>.
    if uses_restrict_keyword(&lines[restrict_shim_anchor..]) {
        for line in lines.iter_mut().skip(restrict_shim_anchor) {
            if line.contains("restrict") {
                *line = rewrite_restrict_tokens(line);
            }
        }
        let shim: [String; 8] = [
            "/* `restrict` is C99-only; expand it to the keyword in C and to nothing in C++,"
                .to_string(),
            "   where it is a parse error (even as `__restrict`) inside array parameters. */"
                .to_string(),
            "#ifndef __nanvix_restrict".to_string(),
            "#ifdef __cplusplus".to_string(),
            "#define __nanvix_restrict".to_string(),
            "#else".to_string(),
            "#define __nanvix_restrict restrict".to_string(),
            "#endif".to_string(),
        ];
        // Close the outer `#ifndef` and separate the shim from the body.
        let mut block: Vec<String> = shim.to_vec();
        block.push("#endif".to_string());
        block.push(String::new());
        lines.splice(restrict_shim_anchor..restrict_shim_anchor, block);
    }

    Ok(lines.join("\n"))
}

//==================================================================================================
// Discovery and Entry Point
//==================================================================================================

/// Discovers `libc_*` crates under `libs_dir` that carry a `header.toml`.
fn discover_specs(libs_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut crates: Vec<PathBuf> = fs::read_dir(libs_dir)
        .with_context(|| format!("failed to read {}", libs_dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("libc_"))
                && path.join("header.toml").is_file()
        })
        .collect();
    crates.sort();
    Ok(crates)
}

/// Discovers POSIX header specs under `src/libs/sysapi/headers/*.toml`.
///
/// These specs emit the POSIX C headers from Rust definitions in the
/// `sysapi`/`syscall`/`posix` crates. Each spec names the crates to scan for
/// function signatures and explicitly selects Rust constants and types; C-only
/// constructs remain represented as structured or raw TOML entries.
fn discover_posix_specs(libs_dir: &Path) -> Result<Vec<PathBuf>> {
    let dir: PathBuf = libs_dir.join("sysapi").join("headers");
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut specs: Vec<PathBuf> = fs::read_dir(&dir)
        .with_context(|| format!("failed to read {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    specs.sort();
    Ok(specs)
}

/// Loads, parses, and renders the header for a single `libc_*` crate.
///
/// Scans the crate's own `src/` plus any `scan_crates` declared in the spec
/// (so a `libc_*` header can also pull in prototypes whose implementations live
/// in another crate, e.g. the `clock_*` syscalls behind `<time.h>`).
fn render_crate(crate_dir: &Path, libs_dir: &Path) -> Result<(String, String)> {
    let spec_path: PathBuf = crate_dir.join("header.toml");
    let spec_text: String = fs::read_to_string(&spec_path)
        .with_context(|| format!("failed to read {}", spec_path.display()))?;
    let spec: HeaderSpec = toml::from_str(&spec_text)
        .with_context(|| format!("failed to parse {}", spec_path.display()))?;
    // Start from any extra scanned crates, then overlay the crate's own
    // definitions so a locally-defined name wins over an imported one.
    let mut funcs: BTreeMap<String, FuncSig> = scan_named_crates(libs_dir, &spec.scan_crates)?;
    funcs.extend(scan_crate(crate_dir)?);
    let symbols: RustSymbols = if spec.rust_constants.is_empty() && spec.rust_types.is_empty() {
        RustSymbols::default()
    } else {
        scan_header_symbols(crate_dir, libs_dir, &spec.symbol_crates)?
    };
    let rendered: String = generate_header(&spec, &funcs, &symbols)?;
    Ok((spec.file, rendered))
}

/// Loads, parses, and renders a POSIX header spec, scanning its `scan_crates`.
fn render_posix_spec(spec_path: &Path, libs_dir: &Path) -> Result<(String, String)> {
    let spec_text: String = fs::read_to_string(spec_path)
        .with_context(|| format!("failed to read {}", spec_path.display()))?;
    let spec: HeaderSpec = toml::from_str(&spec_text)
        .with_context(|| format!("failed to parse {}", spec_path.display()))?;
    let funcs: BTreeMap<String, FuncSig> = scan_named_crates(libs_dir, &spec.scan_crates)?;
    let symbols: RustSymbols = if spec.rust_constants.is_empty() && spec.rust_types.is_empty() {
        RustSymbols::default()
    } else {
        scan_header_symbols(&libs_dir.join("sysapi"), libs_dir, &spec.symbol_crates)?
    };
    let rendered: String = generate_header(&spec, &funcs, &symbols)?;
    Ok((spec.file, rendered))
}

/// Scans rendered header text for unresolved generator placeholders.
///
/// A non-empty result means a spec referenced a Rust type the generator cannot
/// map (`UNMAPPED`) or a function missing from the scanned sources (`TODO`).
/// Both emit broken or incomplete C, so generation must never ship them.
fn find_placeholders(rendered: &str) -> Vec<String> {
    rendered
        .lines()
        .filter(|line| line.contains("UNMAPPED") || line.contains("not found in Rust source"))
        .map(|line| line.trim().to_string())
        .collect()
}

fn main() -> Result<()> {
    let cli: Cli = Cli::parse();

    // Resolve the repository root from this crate's compile-time location:
    // <root>/src/utils/gen-headers -> <root>.
    let root: &Path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .context("failed to locate repository root")?;
    let libs_dir: PathBuf = root.join("src").join("libs");
    let include_dir: PathBuf = root.join("include");

    // Render every header: the per-crate `libc_*` specs, then the POSIX specs.
    let mut headers: Vec<(String, String)> = Vec::new();
    for crate_dir in &discover_specs(&libs_dir)? {
        headers.push(render_crate(crate_dir, &libs_dir)?);
    }
    for spec_path in &discover_posix_specs(&libs_dir)? {
        headers.push(render_posix_spec(spec_path, &libs_dir)?);
    }

    // Fail fast on any unresolved placeholder before touching the tree: a spec
    // referenced a type the generator cannot map or a function missing from the
    // scanned sources, either of which yields broken or incomplete C.
    let mut placeholders: Vec<String> = Vec::new();
    for (file, rendered) in &headers {
        for line in find_placeholders(rendered) {
            placeholders.push(format!("{file}: {line}"));
        }
    }
    if !placeholders.is_empty() {
        for placeholder in &placeholders {
            eprintln!("unresolved placeholder: {placeholder}");
        }
        bail!(
            "header generation produced {} unresolved placeholder(s); add the missing type \
             mapping or function override",
            placeholders.len()
        );
    }

    let mut stale: Vec<String> = Vec::new();
    let mut generated: usize = 0;

    for (file, rendered) in &headers {
        if let Some(only) = &cli.header {
            if file != only {
                continue;
            }
        }

        let out_path: PathBuf = include_dir.join(file);
        if cli.check {
            let existing: String = fs::read_to_string(&out_path).unwrap_or_default();
            if &existing != rendered {
                stale.push(file.clone());
            }
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            // Only rewrite when the contents actually change so that the
            // header's mtime is preserved on no-op builds. This keeps the
            // generator safe to drive on every build without churning
            // timestamps and forcing dependent C sources to recompile.
            let existing: String = fs::read_to_string(&out_path).unwrap_or_default();
            if existing != *rendered {
                fs::write(&out_path, rendered)
                    .with_context(|| format!("failed to write {}", out_path.display()))?;
                generated += 1;
            }
        }
    }

    if cli.check {
        if !stale.is_empty() {
            for file in &stale {
                eprintln!("out of date: {file}");
            }
            bail!("headers are out of date; run `cargo run -p gen-headers` to regenerate");
        }
        eprintln!("all headers up to date");
    } else {
        eprintln!("generated {generated} header(s)");
    }

    Ok(())
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::syn::parse_quote;

    #[test]
    fn map_type_maps_raw_pointer_mutability() {
        let const_void: syn::Type = parse_quote!(*const c_void);
        let mut_void: syn::Type = parse_quote!(*mut c_void);
        let const_char: syn::Type = parse_quote!(*const c_char);
        let mut_char: syn::Type = parse_quote!(*mut c_char);

        assert_eq!(map_type(&const_void), "const void *");
        assert_eq!(map_type(&mut_void), "void *");
        assert_eq!(map_type(&const_char), "const char *");
        assert_eq!(map_type(&mut_char), "char *");
    }

    #[test]
    fn map_type_maps_function_pointer() {
        let fn_ptr: syn::Type =
            parse_quote!(unsafe extern "C" fn(*const c_void, c_size_t) -> c_int);

        assert_eq!(map_type(&fn_ptr), "int (*)(const void *, size_t)");
    }

    /// Parses and translates one Rust constant expression for a unit test.
    fn translate(expression: &str) -> Result<String> {
        let expression: syn::Expr = syn::parse_str(expression)?;
        translate_constant_expr(&expression, &[], &BTreeMap::new())
    }

    /// Parses and translates one Rust constant expression with a declared type.
    fn translate_typed(expression: &str, ty: &str) -> Result<String> {
        let expression: syn::Expr = syn::parse_str(expression)?;
        let ty: syn::Type = syn::parse_str(ty)?;
        translate_constant_expr_typed(&expression, &[], &BTreeMap::new(), Some(&ty))
    }

    #[test]
    fn translates_integer_literals() {
        assert_eq!(
            translate("42_u32").expect("decimal literal should translate"),
            "((uint32_t)42)"
        );
        assert_eq!(
            translate("0o755u16").expect("octal literal should translate"),
            "((uint16_t)0755)"
        );
        assert_eq!(translate("0b1010").expect("binary literal should translate"), "10");
        assert_eq!(
            translate("0xffff_ffffu32").expect("hex literal should translate"),
            "((uint32_t)0xffffffff)"
        );
    }

    #[test]
    fn translates_character_and_string_literals() {
        assert_eq!(translate("'\\0'").expect("NUL character should translate"), "'\\000'");
        assert_eq!(
            translate("\"\\01\"").expect("NUL followed by an octal digit should translate"),
            "\"\\0001\""
        );
    }

    #[test]
    fn translates_operators_and_casts() {
        assert_eq!(
            translate("((1u32 << 3) | 2) as c_int").expect("operator expression should translate"),
            "((int)((((uint32_t)1) << 3) | 2))"
        );
        assert_eq!(
            translate("!0u32").expect("bitwise not should translate"),
            "((uint32_t)(~((uint32_t)((uint32_t)0))))"
        );
        assert_eq!(
            translate_typed("!0", "u8").expect("declared width should control bitwise not"),
            "((uint8_t)(~((uint8_t)0)))"
        );
        assert_eq!(
            translate("!0xffff_ffffu64").expect("wide bitwise not should translate"),
            "((uint64_t)(~((uint64_t)((uint64_t)0xffffffff))))"
        );
        assert_eq!(
            translate_typed("1 << 32", "u64").expect("wide shift should translate"),
            "((uint64_t)(((uint64_t)1) << ((uint64_t)32)))"
        );
        assert_eq!(translate("!true").expect("boolean not should translate"), "(!1)");
    }

    #[test]
    fn translates_exported_aliases() {
        let expression: syn::Expr =
            syn::parse_str("super::BASE").expect("alias expression should parse");
        let exported: BTreeMap<String, String> =
            BTreeMap::from([("outer::BASE".to_string(), "C_BASE".to_string())]);
        assert_eq!(
            translate_constant_expr(
                &expression,
                &["outer".to_string(), "inner".to_string()],
                &exported
            )
            .expect("exported alias should translate"),
            "C_BASE"
        );
    }

    #[test]
    fn rejects_unsupported_expressions() {
        let error: anyhow::Error =
            translate("size_of::<u32>()").expect_err("const function calls must be rejected");
        assert!(
            error.to_string().contains("explicit C override"),
            "unexpected diagnostic: {error}"
        );
    }

    #[test]
    fn keeps_module_names_in_symbol_index() {
        let source: syn::File = syn::parse_file(
            "pub mod left { pub const VALUE: u32 = 1; }\npub mod right { pub const VALUE: u32 = \
             2; }",
        )
        .expect("source should parse");
        let mut symbols: RustSymbols = RustSymbols::default();
        collect_rust_symbols(&source.items, &[], &mut symbols)
            .expect("constants should be indexed");
        assert!(symbols.constants.contains_key("left::VALUE"));
        assert!(symbols.constants.contains_key("right::VALUE"));
    }

    #[test]
    fn resolves_imported_and_reexported_symbols() {
        let source: syn::File = syn::parse_file(
            "pub mod source {\npub const COUNT: u32 = 4;\n#[repr(C)] pub struct Item { pub value: \
             c_int }\n}\npub mod consumer {\nuse crate::source::{COUNT, Item};\npub const DOUBLE: \
             u32 = COUNT + COUNT;\n#[repr(C)] pub struct Holder { pub items: [Item; COUNT] \
             }\n}\nmod private {\npub type Hidden = u32;\n#[repr(C)] pub struct Node { pub next: \
             *mut Node }\n}\npub use private::*;",
        )
        .expect("source should parse");
        let mut symbols: RustSymbols = RustSymbols::default();
        collect_rust_symbols(&source.items, &[], &mut symbols)
            .expect("imports and re-exports should resolve");

        assert!(!symbols.types["private::Hidden"].public);
        assert!(symbols.types["Hidden"].public);
        assert!(symbols.types["Node"].public);

        let constants: Vec<RustConstantExport> = vec![
            RustConstantExport::Path("source::COUNT".to_string()),
            RustConstantExport::Path("consumer::DOUBLE".to_string()),
        ];
        let macros: Vec<Macro> =
            resolve_rust_constants(&constants, &symbols).expect("constant imports should resolve");
        assert_eq!(macros[1].name, "DOUBLE");
        assert!(macros[1].value.contains("COUNT"));

        let specs: Vec<RustTypeSpec> = vec![
            toml::from_str("path = \"consumer::Holder\"").expect("Holder spec should parse"),
            toml::from_str("path = \"source::Item\"").expect("Item spec should parse"),
            toml::from_str("path = \"Hidden\"").expect("Hidden spec should parse"),
            toml::from_str(
                "path = \"Node\"\nstyle = \"typedef_tag\"\ntag_name = \"node\"\nforward_declare = \
                 true",
            )
            .expect("Node spec should parse"),
        ];
        assert_eq!(
            order_rust_types(&specs, &symbols).expect("imported type should order"),
            vec![1, 0, 2, 3]
        );
        let mut lines: Vec<String> = Vec::new();
        emit_rust_types(&mut lines, &specs, &symbols, &constants)
            .expect("imported and re-exported types should render");
        let rendered: String = lines.join("\n");
        assert!(rendered.contains("Item items[COUNT];"));
        assert!(rendered.contains("typedef uint32_t Hidden;"));
        assert!(rendered.contains("Node *next;"));

        let private_spec: RustTypeSpec =
            toml::from_str("path = \"private::Hidden\"").expect("private spec should parse");
        let error: anyhow::Error = order_rust_types(&[private_spec], &symbols)
            .expect_err("private source path must not be exportable");
        assert!(
            error.to_string().contains("not publicly reachable"),
            "unexpected diagnostic: {error}"
        );
    }

    #[test]
    fn resolves_reexport_chains_without_visibility_leaks() {
        let source: syn::File = syn::parse_file(
            "pub mod source {\npub type T = u32;\npub type V = u32;\npub mod child { pub type U = \
             u32; }\nmod hidden { pub type Secret = u32; }\n}\npub mod other { pub type V = u64; \
             }\nmod bridge { pub use crate::source::T; }\npub use bridge::T as Bridged;\npub use \
             source as alias;\npub use source::*;\npub use other::*;\npub use core::fmt::Debug;",
        )
        .expect("source should parse");
        let mut symbols: RustSymbols = RustSymbols::default();
        collect_rust_symbols(&source.items, &[], &mut symbols)
            .expect("supported re-exports should resolve");

        assert!(symbols.types["Bridged"].public);
        assert!(symbols.types["alias::T"].public);
        assert!(symbols.types["alias::child::U"].public);
        assert!(symbols.types["child::U"].public);
        assert!(!symbols.types.contains_key("alias::hidden::Secret"));
        assert!(!symbols.types.contains_key("V"));
        assert!(!symbols.types.contains_key("Debug"));
    }

    #[test]
    fn gives_local_and_named_imports_precedence_over_globs() {
        let names: BTreeMap<String, u32> = BTreeMap::from([
            ("source::T".to_string(), 1),
            ("other::T".to_string(), 2),
            ("consumer::T".to_string(), 3),
        ]);
        let imports: BTreeMap<String, String> = BTreeMap::from([
            ("s".to_string(), "source".to_string()),
            ("Alias".to_string(), "consumer::s::T".to_string()),
        ]);
        let scoped: BTreeMap<String, u32> =
            scoped_rust_names(&names, &["consumer".to_string()], &imports, &["other".to_string()]);
        assert_eq!(scoped["consumer::T"], 3);
        assert_eq!(scoped["consumer::Alias"], 1);
    }

    #[test]
    fn rejects_array_aliases_in_function_abi_positions() {
        let source: syn::File = syn::parse_file(
            "pub type Four = [c_int; 4];\npub type Callback = unsafe extern \"C\" fn(Four);",
        )
        .expect("source should parse");
        let mut symbols: RustSymbols = RustSymbols::default();
        collect_rust_symbols(&source.items, &[], &mut symbols).expect("types should be indexed");
        let specs: Vec<RustTypeSpec> = vec![
            toml::from_str("path = \"Four\"").expect("Four spec should parse"),
            toml::from_str("path = \"Callback\"").expect("Callback spec should parse"),
        ];
        let mut lines: Vec<String> = Vec::new();
        let error: anyhow::Error = emit_rust_types(&mut lines, &specs, &symbols, &[])
            .expect_err("array alias parameter must be rejected");
        assert!(
            error.to_string().contains("direct Rust array parameters"),
            "unexpected diagnostic: {error}"
        );
    }

    #[test]
    fn rejects_generated_c_name_collisions() {
        let spec: HeaderSpec = toml::from_str(
            "file = \"collision.h\"\nbrief = \"Collision.\"\ndescription = \"\"\nguard = \
             \"_COLLISION_H\"\n[[rust_constants]]\npath = \"constants::VALUE\"\nc_name = \
             \"same\"\n[[rust_types]]\npath = \"types::Value\"\nc_name = \"same\"",
        )
        .expect("header spec should parse");
        let error: anyhow::Error =
            validate_c_export_names(&spec).expect_err("C name collision must be rejected");
        assert!(
            error.to_string().contains("generated C name `same`"),
            "unexpected diagnostic: {error}"
        );
    }

    #[test]
    fn audits_unbound_raw_declarations() {
        let unclassified: HeaderSpec = toml::from_str(
            "file = \"audit.h\"\nbrief = \"Audit.\"\ndescription = \"\"\nguard = \
             \"_AUDIT_H\"\n[[raw_sections]]\ntext = \"#define VALUE 1\"",
        )
        .expect("header spec should parse");
        let error: anyhow::Error = validate_raw_declarations(&unclassified)
            .expect_err("unbound raw object macro must fail the audit");
        assert!(
            error.to_string().contains("VALUE") && error.to_string().contains("c_only_reason"),
            "unexpected diagnostic: {error}"
        );

        let classified: HeaderSpec = toml::from_str(
            "file = \"audit.h\"\nbrief = \"Audit.\"\ndescription = \"\"\nguard = \
             \"_AUDIT_H\"\n[[raw_sections]]\ntext = \"#define VALUE 1\"\nc_only_reason = \
             \"Compiler-provided value.\"",
        )
        .expect("header spec should parse");
        validate_raw_declarations(&classified).expect("classified C-only macro should pass");
    }

    #[test]
    fn audits_exact_declaration_extras() {
        let spec: HeaderSpec = toml::from_str(
            "file = \"audit.h\"\nbrief = \"Audit.\"\ndescription = \"\"\nguard = \
             \"_AUDIT_H\"\n[[rust_constants]]\npath = \"VALUE\"\nc_declaration = \"#define VALUE \
             1\\n#define EXTRA 2\"",
        )
        .expect("header spec should parse");
        let error: anyhow::Error = validate_raw_declarations(&spec)
            .expect_err("unbound extra exact declaration must fail");
        assert!(error.to_string().contains("EXTRA"), "unexpected diagnostic: {error}");
    }

    #[test]
    fn audits_raw_constant_value_drift() {
        let source: syn::File =
            syn::parse_file("pub const VALUE: u32 = 1;").expect("source should parse");
        let mut symbols: RustSymbols = RustSymbols::default();
        collect_rust_symbols(&source.items, &[], &mut symbols).expect("constant should be indexed");
        let spec: HeaderSpec = toml::from_str(
            "file = \"audit.h\"\nbrief = \"Audit.\"\ndescription = \"\"\nguard = \
             \"_AUDIT_H\"\n[[rust_constants]]\npath = \"VALUE\"\nemit = \
             false\n[[raw_sections]]\ntext = \"#define VALUE 2\"",
        )
        .expect("header spec should parse");
        let error: anyhow::Error = validate_raw_constant_values(&spec, &symbols)
            .expect_err("raw/Rust value drift must fail");
        assert!(error.to_string().contains("VALUE"), "unexpected diagnostic: {error}");
    }

    #[test]
    fn rejects_grouped_private_constant_exports() {
        let source: syn::File = syn::parse_file("mod private { pub const VALUE: u32 = 1; }")
            .expect("source should parse");
        let mut symbols: RustSymbols = RustSymbols::default();
        collect_rust_symbols(&source.items, &[], &mut symbols).expect("constant should be indexed");
        let exports: Vec<RustConstantExport> =
            vec![
                toml::from_str("path = \"private::VALUE\"\ngroup = \"private\"")
                    .expect("constant export should parse"),
            ];
        let error: anyhow::Error = resolve_rust_constant_group(&exports, &symbols, "private", true)
            .expect_err("private grouped constant must be rejected");
        assert!(error.to_string().contains("not publicly reachable"));
    }

    #[test]
    fn follows_enabled_public_out_of_line_modules() {
        let crate_dir: PathBuf =
            std::env::temp_dir().join(format!("nanvix-gen-headers-modules-{}", std::process::id()));
        let _ = fs::remove_dir_all(&crate_dir);
        fs::create_dir_all(crate_dir.join("src/enabled"))
            .expect("temporary crate directories should be created");
        fs::create_dir_all(crate_dir.join("src/inline-files"))
            .expect("inline path directory should be created");
        fs::write(
            crate_dir.join("src/lib.rs"),
            "#[cfg(target_os = \"nanvix\")] pub mod enabled;\n#[cfg(windows)] pub mod \
             disabled;\nmod private;\npub use private::EXPOSED;\n#[path = \"renamed.rs\"] pub mod \
             via_path;\n#[path = \"inline-files\"] pub mod inline { pub mod child; }",
        )
        .expect("temporary crate root should be written");
        fs::write(
            crate_dir.join("src/enabled.rs"),
            "pub const VISIBLE: u32 = 1;\npub mod nested;\nmod hidden;",
        )
        .expect("enabled module should be written");
        fs::write(crate_dir.join("src/enabled/nested.rs"), "pub const NESTED: u32 = 2;")
            .expect("nested module should be written");
        fs::write(crate_dir.join("src/enabled/hidden.rs"), "pub const HIDDEN: u32 = 9;")
            .expect("private child module should be written");
        fs::write(crate_dir.join("src/private.rs"), "pub const EXPOSED: u32 = 8;")
            .expect("private re-export source should be written");
        fs::write(crate_dir.join("src/renamed.rs"), "pub const PATHED: u32 = 3;\npub mod child;")
            .expect("path-overridden module should be written");
        fs::write(crate_dir.join("src/child.rs"), "pub const RENAMED_CHILD: u32 = 4;")
            .expect("path-overridden child module should be written");
        fs::write(crate_dir.join("src/inline-files/child.rs"), "pub const INLINE_CHILD: u32 = 5;")
            .expect("inline path child module should be written");

        let symbols: RustSymbols =
            scan_rust_symbols(&crate_dir).expect("public module graph should be scanned");
        assert!(symbols.constants.contains_key("enabled::VISIBLE"));
        assert!(symbols.constants.contains_key("enabled::nested::NESTED"));
        assert!(symbols.constants.contains_key("via_path::PATHED"));
        assert!(symbols
            .constants
            .contains_key("via_path::child::RENAMED_CHILD"));
        assert!(symbols
            .constants
            .contains_key("inline::child::INLINE_CHILD"));
        assert!(!symbols
            .constants
            .keys()
            .any(|path| path.starts_with("disabled::")));
        assert!(!symbols.constants["private::EXPOSED"].public);
        assert!(!symbols.constants["enabled::hidden::HIDDEN"].public);
        assert!(symbols.constants["EXPOSED"].public);

        fs::remove_dir_all(&crate_dir).expect("temporary crate should be removed");
    }

    #[test]
    fn selects_i686_type_definition() {
        let source: syn::File = syn::parse_file(
            "#[cfg(target_pointer_width = \"32\")] pub type Word = \
             u32;\n#[cfg(target_pointer_width = \"64\")] pub type Word = u64;",
        )
        .expect("source should parse");
        let mut symbols: RustSymbols = RustSymbols::default();
        collect_rust_symbols(&source.items, &[], &mut symbols)
            .expect("target-specific type should be indexed");
        let symbol: &RustType = symbols.types.get("Word").expect("Word should be selected");
        let RustTypeKind::Alias(alias) = &symbol.kind else {
            panic!("Word should be an alias");
        };
        let syn::Type::Path(path) = alias.as_ref() else {
            panic!("Word should be a path alias");
        };
        assert!(path.path.is_ident("u32"));
    }

    #[test]
    fn matches_nanvix_guest_cfg_values() {
        for predicate in [
            "unix",
            "target_arch = \"x86\"",
            "target_pointer_width = \"32\"",
            "target_endian = \"little\"",
            "target_os = \"nanvix\"",
            "target_family = \"unix\"",
        ] {
            let predicate: syn::Meta = syn::parse_str(predicate).expect("cfg should parse");
            assert!(cfg_matches_guest(&predicate).expect("cfg should be supported"));
        }
        let windows: syn::Meta = syn::parse_str("windows").expect("cfg should parse");
        assert!(!cfg_matches_guest(&windows).expect("cfg should be supported"));
    }

    #[test]
    fn rejects_unsupported_repr_modifiers() {
        let source: syn::File =
            syn::parse_file("#[repr(C, packed(2))] pub struct Value { pub value: c_int }")
                .expect("source should parse");
        let mut symbols: RustSymbols = RustSymbols::default();
        let error: anyhow::Error = collect_rust_symbols(&source.items, &[], &mut symbols)
            .expect_err("parameterized packing must be rejected");
        assert!(
            error.to_string().contains("repr(packed(N)) is unsupported"),
            "unexpected diagnostic: {error}"
        );
    }

    #[test]
    fn indexes_explicit_repr_alignment() {
        let source: syn::File =
            syn::parse_file("#[repr(C, align(8))] pub struct Value { pub value: c_int }")
                .expect("source should parse");
        let mut symbols: RustSymbols = RustSymbols::default();
        collect_rust_symbols(&source.items, &[], &mut symbols)
            .expect("explicit alignment should be indexed");
        assert_eq!(symbols.types["Value"].alignment, Some(8));
    }

    #[test]
    fn formats_pointers_arrays_and_function_pointers() {
        let type_names: BTreeMap<String, String> = BTreeMap::new();
        let constant_names: BTreeMap<String, String> = BTreeMap::new();
        let array_aliases: BTreeMap<String, bool> = BTreeMap::new();
        let pointer: syn::Type = syn::parse_str("*mut *mut c_int").expect("type should parse");
        assert_eq!(
            format_c_declarator(
                &pointer,
                "values",
                &[],
                &type_names,
                &constant_names,
                &array_aliases,
            )
            .expect("pointer should render"),
            "int **values"
        );

        let pointer_to_const_pointer: syn::Type =
            syn::parse_str("*const *mut c_int").expect("type should parse");
        assert_eq!(
            format_c_declarator(
                &pointer_to_const_pointer,
                "values",
                &[],
                &type_names,
                &constant_names,
                &array_aliases,
            )
            .expect("pointer to const pointer should render"),
            "int * const *values"
        );

        let pointer_to_pointer_to_const: syn::Type =
            syn::parse_str("*mut *const c_int").expect("type should parse");
        assert_eq!(
            format_c_declarator(
                &pointer_to_pointer_to_const,
                "values",
                &[],
                &type_names,
                &constant_names,
                &array_aliases,
            )
            .expect("pointer to pointer to const should render"),
            "const int **values"
        );

        let array: syn::Type = syn::parse_str("[c_int; 4]").expect("type should parse");
        assert_eq!(
            format_c_declarator(
                &array,
                "values",
                &[],
                &type_names,
                &constant_names,
                &array_aliases,
            )
            .expect("array should render"),
            "int values[4]"
        );

        let pointer_to_array: syn::Type =
            syn::parse_str("*const [c_int; 4]").expect("type should parse");
        assert_eq!(
            format_c_declarator(
                &pointer_to_array,
                "values",
                &[],
                &type_names,
                &constant_names,
                &array_aliases,
            )
            .expect("pointer to array should render"),
            "const int (*values)[4]"
        );

        let function: syn::Type =
            syn::parse_str("unsafe extern \"C\" fn(c_int) -> c_int").expect("type should parse");
        assert_eq!(
            format_c_declarator(
                &function,
                "callback",
                &[],
                &type_names,
                &constant_names,
                &array_aliases,
            )
            .expect("function pointer should render"),
            "int (*callback)(int)"
        );

        let function_returning_pointer_to_array: syn::Type =
            syn::parse_str("unsafe extern \"C\" fn() -> *mut [c_int; 4]")
                .expect("type should parse");
        assert_eq!(
            format_c_declarator(
                &function_returning_pointer_to_array,
                "callback",
                &[],
                &type_names,
                &constant_names,
                &array_aliases,
            )
            .expect("composed function pointer should render"),
            "int (*(*callback)(void))[4]"
        );

        for (ty, diagnostic) in [
            ("unsafe extern \"C\" fn([c_int; 4])", "direct Rust array parameters"),
            ("unsafe extern \"C\" fn() -> [c_int; 4]", "direct Rust array return types"),
        ] {
            let function: syn::Type = syn::parse_str(ty).expect("type should parse");
            let error: anyhow::Error = format_c_declarator(
                &function,
                "callback",
                &[],
                &type_names,
                &constant_names,
                &array_aliases,
            )
            .expect_err("direct function array must be rejected");
            assert!(error.to_string().contains(diagnostic), "unexpected diagnostic: {error}");
        }

        let optional_function: syn::Type =
            syn::parse_str("Option<unsafe extern \"C\" fn(c_int) -> c_int>")
                .expect("type should parse");
        assert_eq!(
            format_c_declarator(
                &optional_function,
                "callback",
                &[],
                &type_names,
                &constant_names,
                &array_aliases,
            )
            .expect("nullable function pointer should render"),
            "int (*callback)(int)"
        );

        let optional_integer: syn::Type =
            syn::parse_str("Option<c_int>").expect("type should parse");
        let error: anyhow::Error = format_c_declarator(
            &optional_integer,
            "value",
            &[],
            &type_names,
            &constant_names,
            &array_aliases,
        )
        .expect_err("Option<c_int> must be rejected");
        assert!(
            error.to_string().contains("only Option<extern \"C\" fn>"),
            "unexpected diagnostic: {error}"
        );
    }

    #[test]
    fn renders_field_renames_and_packed_layouts() {
        let source: syn::File = syn::parse_file(
            "/// Value.\n#[repr(C, packed)]\npub struct Value {\n/// Kind.\n    pub type_: \
             c_int,\n}",
        )
        .expect("source should parse");
        let mut symbols: RustSymbols = RustSymbols::default();
        collect_rust_symbols(&source.items, &[], &mut symbols).expect("type should be indexed");
        let spec: RustTypeSpec =
            toml::from_str("path = \"Value\"\nfield_renames = { type_ = \"type\" }")
                .expect("type spec should parse");
        let symbol: &RustType = symbols.types.get("Value").expect("Value should be indexed");
        assert_eq!(
            render_rust_type(
                &spec,
                symbol,
                &BTreeMap::new(),
                &BTreeMap::new(),
                &BTreeMap::new(),
                true,
            )
            .expect("type should render"),
            concat!(
                "/** @brief Value. */\n",
                "typedef struct __attribute__((packed)) {\n",
                "    int type; /**< Kind. */\n",
                "} Value;"
            )
        );
    }

    #[test]
    fn orders_dependencies_and_supports_forward_declarations() {
        let source: syn::File = syn::parse_file(
            "#[repr(C)] pub struct Outer { pub inner: Inner }\n#[repr(C)] pub struct Inner { pub \
             value: c_int }\n#[repr(C)] pub struct Node { pub next: *mut Node }",
        )
        .expect("source should parse");
        let mut symbols: RustSymbols = RustSymbols::default();
        collect_rust_symbols(&source.items, &[], &mut symbols).expect("types should be indexed");
        let specs: Vec<RustTypeSpec> = vec![
            toml::from_str("path = \"Outer\"").expect("Outer spec should parse"),
            toml::from_str("path = \"Inner\"").expect("Inner spec should parse"),
            toml::from_str(
                "path = \"Node\"\nstyle = \"typedef_tag\"\nc_name = \"node_t\"\ntag_name = \
                 \"node\"\nforward_declare = true",
            )
            .expect("Node spec should parse"),
        ];
        let order: Vec<usize> =
            order_rust_types(&specs, &symbols).expect("type dependencies should be ordered");
        assert_eq!(order, vec![1, 0, 2]);

        let mut lines: Vec<String> = Vec::new();
        emit_rust_types(&mut lines, &specs, &symbols, &[])
            .expect("types should render with a forward declaration");
        let rendered: String = lines.join("\n");
        assert!(rendered.contains("typedef struct node node_t;"));
        assert!(rendered.contains("struct node {\n    node_t *next;\n};"));
    }

    #[test]
    fn orders_pointer_to_array_element_before_holder() {
        let source: syn::File = syn::parse_file(
            "#[repr(C)] pub struct Holder { pub items: *mut [Item; 4] }\n#[repr(C)] pub struct \
             Item { pub value: c_int }",
        )
        .expect("source should parse");
        let mut symbols: RustSymbols = RustSymbols::default();
        collect_rust_symbols(&source.items, &[], &mut symbols).expect("types should be indexed");
        let specs: Vec<RustTypeSpec> = vec![
            toml::from_str("path = \"Holder\"").expect("Holder spec should parse"),
            toml::from_str(
                "path = \"Item\"\nstyle = \"typedef_tag\"\nc_name = \"item_t\"\ntag_name = \
                 \"item\"\nforward_declare = true",
            )
            .expect("Item spec should parse"),
        ];
        assert_eq!(
            order_rust_types(&specs, &symbols).expect("types should be ordered"),
            vec![1, 0]
        );

        let mut lines: Vec<String> = Vec::new();
        emit_rust_types(&mut lines, &specs, &symbols, &[]).expect("types should render");
        let rendered: String = lines.join("\n");
        let item_definition: usize = rendered.find("struct item {").expect("Item should render");
        let holder_definition: usize = rendered
            .find("item_t (*items)[4];")
            .expect("Holder array pointer should render");
        assert!(item_definition < holder_definition);
    }
}
