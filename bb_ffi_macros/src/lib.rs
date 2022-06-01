use proc_macro::TokenStream;

mod cenum;
mod ctype;
mod docs;

use docs::gen_docs;

#[proc_macro_attribute]
pub fn ctype(args: TokenStream, input: TokenStream) -> TokenStream { ctype::ctype(args, input) }
#[proc_macro_attribute]
pub fn cenum(args: TokenStream, input: TokenStream) -> TokenStream { cenum::cenum(args, input) }
