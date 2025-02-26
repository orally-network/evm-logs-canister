use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

extern crate proc_macro;

#[proc_macro_attribute]
pub fn cycles_count(_args: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let name = &input.sig.ident;
    let vis = &input.vis;
    let attr = &input.attrs;
    let inputs = &input.sig.inputs;
    let generics = &input.sig.generics;
    let fn_token = &input.sig.fn_token;
    let output = &input.sig.output;
    let block = &input.block;
    let asyncness = input.sig.asyncness;

    let expanded = quote! {
        #(#attr)*
        #vis #asyncness #fn_token #name #generics(#inputs) #output {
            let cycles_before = ic_cdk::api::canister_balance();
            let res = #block;
            let cycles_after = ic_cdk::api::canister_balance();
            log!("Cycles used for method {}: {:?}",stringify!(#name), cycles_before - cycles_after);
            res
        }
    };

    TokenStream::from(expanded)
}
