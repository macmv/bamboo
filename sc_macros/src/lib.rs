use proc_macro::TokenStream;

mod lookup_table;
mod protocol_version;
mod transfer;

#[proc_macro_error::proc_macro_error]
#[proc_macro]
pub fn lookup_table(input: TokenStream) -> TokenStream { lookup_table::lookup_table(input) }

#[proc_macro_attribute]
pub fn protocol_version(_args: TokenStream, input: TokenStream) -> TokenStream {
  protocol_version::protocol_version(input)
}

#[proc_macro_attribute]
pub fn transfer(_args: TokenStream, input: TokenStream) -> TokenStream {
  transfer::transfer(input.into()).into()
}
