//! Convenience macro to instrument a function as a HiTrace span
//!
//! The macro will automatically start a span when the function is entered, and close
//! the span when the function is left.
//!
//! ## Examples
//!
//! Cargo.toml:
//! ```toml
//! [dependencies]
//! hitrace = "0.1"
//! hitrace-macro = "0.1"
//! ```
//!
//! ```ignore
//! use hitrace_macro::trace_fn;
//! #[trace_fn]
//! fn do_something_and_measure() {
//!     println!("Doing something expensive....")
//! }
//! ```
//!

use proc_macro::TokenStream;

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, ItemMod};

#[proc_macro_attribute]
pub fn trace_fn(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = TokenStream2::from(input);
    trace_fn_(input).into()
}

#[proc_macro_attribute]
pub fn trace_all_fns(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_mod = parse_macro_input!(input as syn::ItemMod);
    trace_all_fns_in_mod(input_mod).into()
}

fn trace_fn_(input: TokenStream2) -> TokenStream2 {
    let mut item: syn::Item = syn::parse2(input).unwrap();
    let func = match &mut item {
        syn::Item::Fn(func) => func,
        _ => panic!("Expected a function"),
    };
    let fn_name = func.sig.ident.to_string();
    let fn_name_statement = quote!(
        const HITRACE_THIS_FN_NAME: &str = concat!(module_path!(), "::", #fn_name, "\0");
    );
    let call_hitrace = quote!(
        let guard = unsafe { hitrace::ScopedTrace::_start_trace_str_with_null(HITRACE_THIS_FN_NAME) };
    );
    let parsed_name_stmt: syn::Stmt = syn::parse2(fn_name_statement).unwrap();
    let call_hitrace_stmt: syn::Stmt = syn::parse2(call_hitrace).unwrap();

    func.block.stmts.insert(0, parsed_name_stmt);
    func.block.stmts.insert(1, call_hitrace_stmt);

    item.into_token_stream()
}

fn trace_all_fns_in_mod(input_mod: ItemMod) -> TokenStream2 {
    let elements = input_mod.content;
    todo!()
}
