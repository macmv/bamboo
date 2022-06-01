use proc_macro::TokenStream;

mod ctype;

#[proc_macro_attribute]
pub fn ctype(args: TokenStream, input: TokenStream) -> TokenStream { ctype::ctype(args, input) }
