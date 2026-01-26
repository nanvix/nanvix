// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![doc = "Attribute macros that inject syslog instrumentation into function bodies."]

//==================================================================================================
// Imports
//==================================================================================================

use ::proc_macro::TokenStream;
use ::proc_macro2::{
    Ident,
    TokenStream as TokenStream2,
};
use ::quote::quote;
use ::std::{
    string::String,
    vec::Vec,
};
use ::syn::{
    spanned::Spanned,
    FnArg,
    ItemFn,
    LitStr,
    Pat,
    Result,
};

///
/// # Description
///
/// Annotates a function so that it emits a `syscall_trace!` log message before
/// the function body executes.
///
/// # Parameters
///
/// - `attr`: The attribute tokens supplied by the caller (unused, but validated).
/// - `item`: The target function annotated with `trace_syscall`.
///
/// # Returns
///
/// An instrumented version of the input function with tracing injected.
///
/// # Errors
///
/// Propagates parsing errors or validation failures through the generated token
/// stream.
///
#[proc_macro_attribute]
pub fn trace_syscall(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand(attr, item, TraceKind::Syscall)
}

///
/// # Description
///
/// Annotates a function so that it emits a `libcall_trace!` log message before
/// the function body executes.
///
/// # Parameters
///
/// - `attr`: The attribute tokens supplied by the caller (unused, but validated).
/// - `item`: The target function annotated with `trace_libcall`.
///
/// # Returns
///
/// An instrumented version of the input function with tracing injected.
///
/// # Errors
///
/// Propagates parsing errors or validation failures through the generated token
/// stream.
///
#[proc_macro_attribute]
pub fn trace_libcall(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand(attr, item, TraceKind::Libcall)
}

//==================================================================================================
// Constants
//==================================================================================================

const CHATTY_LIBCALL_PREFIXES: [&str; 3] = ["pthread_", "sem_", "__errno_location"];
const CHATTY_SYSCALL_PREFIXES: [&str; 6] = [
    "lock_mutex",
    "unlock_mutex",
    "signal_cond",
    "wait_cond",
    "sched_yield",
    "write",
];

//==================================================================================================
// Enumerations
//==================================================================================================

///
/// # Description
///
/// Distinguishes whether a trace macro invocation targets syscalls or libcalls.
///
#[derive(Copy, Clone)]
enum TraceKind {
    Syscall,
    Libcall,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Dispatches expansion to the internal helper while converting diagnostic
/// errors into compilable token streams.
///
/// # Parameters
///
/// - `attr`: Attribute tokens supplied to the macro.
/// - `item`: The annotated item provided by the user.
/// - `kind`: Indicates whether the expansion targets syscalls or libcalls.
///
/// # Returns
///
/// Instrumented tokens when expansion succeeds or compile-error tokens when it
/// fails.
///
/// # Errors
///
/// Never directly returns errors; diagnostics are emitted through token streams.
///
fn expand(attr: TokenStream, item: TokenStream, kind: TraceKind) -> TokenStream {
    match expand_inner(attr, item, kind) {
        Ok(tokens) => tokens,
        Err(error) => error.to_compile_error().into(),
    }
}

///
/// # Description
///
/// Parses the annotated function, injects the trace statement, and re-emits the
/// modified function body.
///
/// # Parameters
///
/// - `attr`: Attribute tokens supplied to the macro.
/// - `item`: The annotated function in token form.
/// - `kind`: Indicates which syslog macro to call.
///
/// # Returns
///
/// A token stream representing the instrumented function.
///
/// # Errors
///
/// Returns a parsing error if the input item is not a function or if trace
/// generation fails.
///
fn expand_inner(attr: TokenStream, item: TokenStream, kind: TraceKind) -> Result<TokenStream> {
    let mut function: ItemFn = syn::parse::<ItemFn>(item.clone())?;
    let attr_tokens: TokenStream2 = TokenStream2::from(attr);

    let statement: syn::Stmt = if should_skip_instrumentation(&function, kind) {
        if !attr_tokens.is_empty() {
            return Err(syn::Error::new(
                function.sig.ident.span(),
                "trace_* macros no longer accept custom format strings",
            ));
        }
        build_suppressed_statement(&function)?
    } else {
        build_statement(attr_tokens, &function, kind)?
    };
    function.block.stmts.insert(0, statement);
    Ok(TokenStream::from(quote!(#function)))
}

///
/// # Description
///
/// Builds the tracing statement inserted at the beginning of the annotated
/// function.
///
/// # Parameters
///
/// - `attr_tokens`: Raw attribute tokens that must be empty.
/// - `function`: The parsed function definition.
/// - `kind`: Determines whether to call `syscall_trace!` or `libcall_trace!`.
///
/// # Returns
///
/// A single statement that invokes the appropriate syslog macro.
///
/// # Errors
///
/// Returns an error if custom attribute tokens are provided or if message
/// construction fails.
///
fn build_statement(
    attr_tokens: TokenStream2,
    function: &ItemFn,
    kind: TraceKind,
) -> Result<syn::Stmt> {
    if !attr_tokens.is_empty() {
        return Err(syn::Error::new(
            function.sig.ident.span(),
            "trace_* macros no longer accept custom format strings",
        ));
    }

    let target_macro: TokenStream2 = match kind {
        TraceKind::Syscall => quote!(::syslog::syscall_trace!),
        TraceKind::Libcall => quote!(::syslog::libcall_trace!),
    };

    let (message, args): (LitStr, Vec<TokenStream2>) = build_auto_message(function)?;
    let statement: TokenStream2 = quote! {
        #target_macro(#message #(, #args)*);
    };

    syn::parse2(statement)
}

///
/// # Description
///
/// Emits no-op statements for functions whose instrumentation has been
/// suppressed so that compiler warnings remain silenced.
///
/// # Parameters
///
/// - `function`: Parsed representation of the annotated function.
///
/// # Returns
///
/// A statement that references each parameter to avoid unused-variable
/// warnings.
///
fn build_suppressed_statement(function: &ItemFn) -> Result<syn::Stmt> {
    let (_, args) = build_auto_message(function)?;

    if args.is_empty() {
        return syn::parse2(quote!({}));
    }

    let statement: TokenStream2 = quote!({ #(let _ = &#args;)* });
    syn::parse2(statement)
}

///
/// # Description
///
/// Generates an automatic format string and argument list based on the function
/// signature so traces include every parameter.
///
/// # Parameters
///
/// - `function`: The parsed function definition.
///
/// # Returns
///
/// A tuple containing a literal format string and the ordered argument tokens.
///
/// # Errors
///
/// Returns an error if any parameter pattern is unsupported (e.g., destructured
/// bindings).
///
fn build_auto_message(function: &ItemFn) -> Result<(LitStr, Vec<TokenStream2>)> {
    let ident: &Ident = &function.sig.ident;
    let mut parts: Vec<String> = Vec::new();
    let mut args: Vec<TokenStream2> = Vec::new();

    for input in &function.sig.inputs {
        match input {
            FnArg::Receiver(receiver) => {
                let is_reference: bool = receiver.reference.is_some();
                let is_mutable: bool = receiver.mutability.is_some();

                let label: String = match (is_reference, is_mutable) {
                    (false, _) => "self={:?}".to_string(),
                    (true, false) => "&self={:?}".to_string(),
                    (true, true) => "&mut self={:?}".to_string(),
                };

                let capture: TokenStream2 = match (is_reference, is_mutable) {
                    (false, _) => quote!(&self),
                    (true, false) => quote!(self),
                    (true, true) => quote!(&*self),
                };

                parts.push(label);
                args.push(capture);
            },
            FnArg::Typed(pat_type) => {
                if let Pat::Ident(pat_ident) = pat_type.pat.as_ref() {
                    if pat_ident.subpat.is_some() {
                        return Err(syn::Error::new(
                            pat_type.pat.span(),
                            "trace_* macros require simple identifier parameters",
                        ));
                    }

                    let name: String = pat_ident.ident.to_string();
                    parts.push(format!("{name}={{:?}}"));
                    let parameter_ident: &Ident = &pat_ident.ident;
                    args.push(quote!(&#parameter_ident));
                } else {
                    return Err(syn::Error::new(
                        pat_type.pat.span(),
                        "trace_* macros require simple identifier parameters",
                    ));
                }
            },
        }
    }
    let message: String = if parts.is_empty() {
        format!("{}()", ident)
    } else {
        format!("{}({})", ident, parts.join(", "))
    };

    Ok((LitStr::new(&message, ident.span()), args))
}

///
/// # Description
///
/// Determines whether instrumentation should be skipped for the provided
/// function based on its namespace and the trace kind.
///
/// # Parameters
///
/// - `function`: Parsed representation of the annotated function.
/// - `kind`: Indicates whether the macro targets syscalls or libcalls.
///
/// # Returns
///
/// Returns `true` when the macro should leave the function body untouched.
///
fn should_skip_instrumentation(function: &ItemFn, kind: TraceKind) -> bool {
    let ident_name: String = function.sig.ident.to_string();

    match kind {
        TraceKind::Syscall => CHATTY_SYSCALL_PREFIXES
            .iter()
            .any(|name| ident_name.starts_with(name)),
        TraceKind::Libcall => CHATTY_LIBCALL_PREFIXES
            .iter()
            .any(|prefix| ident_name.starts_with(prefix)),
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::syn::parse_quote;

    #[test]
    // Ensures owned arguments are borrowed before tracing so the function body retains ownership.
    fn build_auto_message_borrows_owned_inputs() {
        let function: ItemFn = parse_quote! {
            fn demo(buffer: ::std::vec::Vec<u8>, count: usize) {}
        };

        let (message, args) =
            build_auto_message(&function).expect("message generation must succeed");
        assert_eq!(message.value(), "demo(buffer={:?}, count={:?})");

        let rendered: Vec<String> = args
            .iter()
            .map(::std::string::ToString::to_string)
            .collect();
        assert_eq!(rendered, ["& buffer", "& count"]);
    }

    #[test]
    // Verifies that `&mut self` plus additional parameters remain borrowable after instrumentation.
    fn build_auto_message_borrows_mut_self_and_args() {
        let mut function: ItemFn = parse_quote! {
            fn demo(delta: usize) {}
        };

        let receiver: ::syn::Receiver = parse_quote!(&mut self);
        function.sig.inputs.insert(0, FnArg::Receiver(receiver));

        let (message, args) =
            build_auto_message(&function).expect("message generation must succeed");
        assert_eq!(message.value(), "demo(&mut self={:?}, delta={:?})");

        let rendered: Vec<String> = args
            .iter()
            .map(::std::string::ToString::to_string)
            .collect();
        assert_eq!(rendered, ["& * self", "& delta"]);
    }
}
