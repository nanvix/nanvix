// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
    let fn_ptr: syn::Type = parse_quote!(unsafe extern "C" fn(*const c_void, c_size_t) -> c_int);

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
    assert_eq!(translate("42_u32").expect("decimal literal should translate"), "((uint32_t)42)");
    assert_eq!(translate("0o755u16").expect("octal literal should translate"), "((uint16_t)0755)");
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
    assert!(error.to_string().contains("explicit C override"), "unexpected diagnostic: {error}");
}

#[test]
fn keeps_module_names_in_symbol_index() {
    let source: syn::File = syn::parse_file(
        "pub mod left { pub const VALUE: u32 = 1; }\npub mod right { pub const VALUE: u32 = 2; }",
    )
    .expect("source should parse");
    let mut symbols: RustSymbols = RustSymbols::default();
    collect_rust_symbols(&source.items, &[], &mut symbols).expect("constants should be indexed");
    assert!(symbols.constants.contains_key("left::VALUE"));
    assert!(symbols.constants.contains_key("right::VALUE"));
}

#[test]
fn resolves_imported_and_reexported_symbols() {
    let source: syn::File = syn::parse_file(
        "pub mod source {\npub const COUNT: u32 = 4;\n#[repr(C)] pub struct Item { pub value: \
         c_int }\n}\npub mod consumer {\nuse crate::source::{COUNT, Item};\npub const DOUBLE: u32 \
         = COUNT + COUNT;\n#[repr(C)] pub struct Holder { pub items: [Item; COUNT] }\n}\nmod \
         private {\npub type Hidden = u32;\n#[repr(C)] pub struct Node { pub next: *mut Node \
         }\n}\npub use private::*;",
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
            "path = \"Node\"\nstyle = \"typedef_tag\"\ntag_name = \"node\"\nforward_declare = true",
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
    assert!(error.to_string().contains("not publicly reachable"), "unexpected diagnostic: {error}");
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
    let error: anyhow::Error =
        validate_raw_declarations(&spec).expect_err("unbound extra exact declaration must fail");
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
         \"_AUDIT_H\"\n[[rust_constants]]\npath = \"VALUE\"\nemit = false\n[[raw_sections]]\ntext \
         = \"#define VALUE 2\"",
    )
    .expect("header spec should parse");
    let error: anyhow::Error =
        validate_raw_constant_values(&spec, &symbols).expect_err("raw/Rust value drift must fail");
    assert!(error.to_string().contains("VALUE"), "unexpected diagnostic: {error}");
}

#[test]
fn rejects_grouped_private_constant_exports() {
    let source: syn::File =
        syn::parse_file("mod private { pub const VALUE: u32 = 1; }").expect("source should parse");
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
        "#[cfg(target_os = \"nanvix\")] pub mod enabled;\n#[cfg(windows)] pub mod disabled;\nmod \
         private;\npub use private::EXPOSED;\n#[path = \"renamed.rs\"] pub mod via_path;\n#[path \
         = \"inline-files\"] pub mod inline { pub mod child; }",
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
        "#[cfg(target_pointer_width = \"32\")] pub type Word = u32;\n#[cfg(target_pointer_width = \
         \"64\")] pub type Word = u64;",
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
        format_c_declarator(&pointer, "values", &[], &type_names, &constant_names, &array_aliases,)
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
        format_c_declarator(&array, "values", &[], &type_names, &constant_names, &array_aliases,)
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
        syn::parse_str("unsafe extern \"C\" fn() -> *mut [c_int; 4]").expect("type should parse");
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

    let optional_integer: syn::Type = syn::parse_str("Option<c_int>").expect("type should parse");
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
        "/// Value.\n#[repr(C, packed)]\npub struct Value {\n/// Kind.\n    pub type_: c_int,\n}",
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
        "#[repr(C)] pub struct Holder { pub items: *mut [Item; 4] }\n#[repr(C)] pub struct Item { \
         pub value: c_int }",
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
    assert_eq!(order_rust_types(&specs, &symbols).expect("types should be ordered"), vec![1, 0]);

    let mut lines: Vec<String> = Vec::new();
    emit_rust_types(&mut lines, &specs, &symbols, &[]).expect("types should render");
    let rendered: String = lines.join("\n");
    let item_definition: usize = rendered.find("struct item {").expect("Item should render");
    let holder_definition: usize = rendered
        .find("item_t (*items)[4];")
        .expect("Holder array pointer should render");
    assert!(item_definition < holder_definition);
}
