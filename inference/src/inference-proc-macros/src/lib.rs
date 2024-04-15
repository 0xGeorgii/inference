#![no_std]
#![warn(clippy::all, clippy::pedantic)]

use proc_macro::TokenStream;

#[proc_macro]
pub fn inference(input: TokenStream) -> TokenStream {
    input
}

#[proc_macro_attribute]
pub fn inference_spec(attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn inference_fun(attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
