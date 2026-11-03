// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::syn::parse_quote;

#[test]
// Ensures owned arguments are borrowed before tracing so the function body retains ownership.
fn build_auto_message_borrows_owned_inputs() {
    let function: ItemFn = parse_quote! {
        fn demo(buffer: ::std::vec::Vec<u8>, count: usize) {}
    };

    let (message, args) = build_auto_message(&function).expect("message generation must succeed");
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

    let (message, args) = build_auto_message(&function).expect("message generation must succeed");
    assert_eq!(message.value(), "demo(&mut self={:?}, delta={:?})");

    let rendered: Vec<String> = args
        .iter()
        .map(::std::string::ToString::to_string)
        .collect();
    assert_eq!(rendered, ["& * self", "& delta"]);
}
