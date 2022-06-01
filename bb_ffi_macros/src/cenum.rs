use super::gen_docs;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemEnum};

#[allow(clippy::collapsible_match)]
pub fn cenum(_args: TokenStream, input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as ItemEnum);

  let original_docs = gen_docs(&input);

  let input_attrs = &input.attrs;

  let gen_struct = quote!(
    pub struct Foo {}
  );
  let gen_union = quote!(union FooData {a: u32});

  quote! {
    #(#input_attrs)*
    #[doc = "Original enum:"]
    #[doc = #original_docs]
    #[repr(C)]
    #[derive(Clone)]
    #gen_struct
    #gen_union
  }
  .into()
}
