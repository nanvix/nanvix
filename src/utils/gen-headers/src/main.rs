// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
//! Generates C headers from Nanvix `libc_*` Rust crate sources.
//!
//! Each `libc_*` crate that carries a `header.toml` specification owns exactly one C header under
//! `include/`. This tool parses the crate's `extern "C"` function signatures with `syn`, combines
//! them with the specification (macros, includes, section layout), and renders the header. The
//! output is byte-for-byte identical to the committed `include/*.h` files.
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
    /// Raw C type definitions.
    #[serde(default)]
    types: Vec<RawText>,
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
}

/// A block of raw C text.
#[derive(Debug, Deserialize)]
struct RawText {
    /// Verbatim C text.
    text: String,
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
fn emit_macros(lines: &mut Vec<String>, macros: &[Macro]) {
    if macros.is_empty() {
        return;
    }
    lines.push(section_bar("Constants"));
    lines.push(String::new());
    for macro_def in macros {
        if let Some(guard) = &macro_def.guard {
            lines.push(format!("#ifndef {guard}"));
        }
        match &macro_def.comment {
            Some(comment) => lines.push(format!(
                "#define {} {} /**< {} */",
                macro_def.name, macro_def.value, comment
            )),
            None => lines.push(format!("#define {} {}", macro_def.name, macro_def.value)),
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
fn generate_header(spec: &HeaderSpec, funcs: &BTreeMap<String, FuncSig>) -> String {
    let mut lines: Vec<String> = Vec::new();

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

    lines.join("\n")
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
/// These specs emit the POSIX C headers from the existing Rust definitions in
/// the `sysapi`/`syscall`/`posix` crates (the C ABI's single source of truth).
/// Each one names the crates to scan for its function signatures via the spec's
/// `scan_crates` field; its types and constants are carried as raw text.
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
    let rendered: String = generate_header(&spec, &funcs);
    Ok((spec.file, rendered))
}

/// Loads, parses, and renders a POSIX header spec, scanning its `scan_crates`.
fn render_posix_spec(spec_path: &Path, libs_dir: &Path) -> Result<(String, String)> {
    let spec_text: String = fs::read_to_string(spec_path)
        .with_context(|| format!("failed to read {}", spec_path.display()))?;
    let spec: HeaderSpec = toml::from_str(&spec_text)
        .with_context(|| format!("failed to parse {}", spec_path.display()))?;
    let funcs: BTreeMap<String, FuncSig> = scan_named_crates(libs_dir, &spec.scan_crates)?;
    let rendered: String = generate_header(&spec, &funcs);
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
