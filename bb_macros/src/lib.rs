use proc_macro::TokenStream;

mod config;
mod default;
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

#[proc_macro_derive(Transfer, attributes(id, must_exist))]
pub fn transfer(input: TokenStream) -> TokenStream { transfer::transfer(input) }

#[proc_macro_derive(Config)]
pub fn config(input: TokenStream) -> TokenStream { config::config(input) }

#[proc_macro_derive(Default, attributes(default))]
pub fn default(input: TokenStream) -> TokenStream { default::default(input) }
