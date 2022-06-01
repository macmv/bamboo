use super::gen_docs;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Ident, ItemEnum};

#[allow(clippy::collapsible_match)]
pub fn cenum(_args: TokenStream, input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as ItemEnum);

  let original_docs = gen_docs(&input);
  let input_attrs = &input.attrs;

  if input.variants.is_empty() {
    let name = &input.ident;
    return quote!(
      #(#input_attrs)*
      #[doc = "Original enum:"]
      #[doc = #original_docs]
      #[repr(C)]
      #[derive(Clone)]
      pub struct #name {}
    )
    .into();
  }

  let name = &input.ident;
  let data_name = Ident::new(&format!("{name}Data"), name.span());
  let fields = input.variants.iter().map(|v| {
    let name = Ident::new(&to_lower(&v.ident.to_string()), v.ident.span());
    let fields = &v.fields;
    quote!(#name: #fields)
  });
  let as_funcs = input.variants.iter().enumerate().map(|(variant, v)| {
    let name = Ident::new(&to_lower(&v.ident.to_string()), v.ident.span());
    let as_name = Ident::new(&format!("as_{name}"), v.ident.span());
    let ty = &v.fields;
    quote!(
      #[allow(unused_parens)]
      pub fn #as_name(&self) -> Option<&#ty> {
        if self.variant == #variant {
          unsafe {
            Some(&self.data.#name)
          }
        } else {
          None
        }
      }
    )
  });

  let gen_struct = quote!(
    pub struct #name {
      variant: usize,
      data: #data_name,
    }
  );
  let gen_union = quote!(
    union #data_name {
      #(#fields),*
    }
  );

  let out = quote! {
    #(#input_attrs)*
    #[doc = "Original enum:"]
    #[doc = #original_docs]
    #[repr(C)]
    #[derive(Clone)]
    #gen_struct
    #[allow(unused_parens)]
    #[derive(Clone, Copy)]
    #gen_union

    impl #name {
      #(#as_funcs)*
    }
  };
  out.into()
}

fn to_lower(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for c in s.chars() {
    if c.is_ascii_uppercase() {
      if !out.is_empty() {
        out.push('_');
      }
      out.push(c.to_ascii_lowercase());
    } else {
      out.push(c);
    }
  }
  out
}
