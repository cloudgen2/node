#![feature(drain_filter)]
extern crate proc_macro;
use proc_macro::TokenStream;

mod pallet_module_impl;
mod pallet_storage;
mod syntax;
mod typelevel;

#[proc_macro_attribute]
pub fn rio_pallet_storage(metadata: TokenStream, input: TokenStream) -> TokenStream {
  pallet_storage::transform(metadata, input)
}

#[proc_macro_attribute]
pub fn rio_pallet_module_impl(metadata: TokenStream, input: TokenStream) -> TokenStream {
  pallet_module_impl::transform(metadata, input)
}

#[proc_macro_attribute]
pub fn rio_syntax(metadata: TokenStream, input: TokenStream) -> TokenStream {
  syntax::transform(metadata, input)
}

#[proc_macro_attribute]
pub fn rio_typelevel(metadata: TokenStream, input: TokenStream) -> TokenStream {
  typelevel::transform(metadata, input)
}
