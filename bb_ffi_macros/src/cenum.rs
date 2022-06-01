use super::gen_docs;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
  parse_macro_input, punctuated::Punctuated, spanned::Spanned, token::Brace, Field, Fields,
  FieldsNamed, Ident, ItemEnum, ItemStruct, ItemUnion, Path, PathArguments, PathSegment, Token,
  Type, TypePath, Visibility,
};

macro_rules! punct {
  [ $($field:expr),* ] => {{
    let mut punct = Punctuated::new();
    punct.extend(vec![$($field),*]);
    punct
  }}
}
macro_rules! fields_named {
  { $($name:ident: $ty:expr,)* } => {
    FieldsNamed { brace_token: Brace { span: Span::call_site() }, named: punct![$(
      Field {
        attrs: vec![],
        vis: Visibility::Inherited,
        ident: Some(Ident::new(stringify!($name), Span::call_site())),
        colon_token: Some(Token![:](Span::call_site())),
        ty: $ty,
      }
    ),*] }
  }
}
macro_rules! path {
  ( :: $($ident:ident)::* ) => {
    Path {
      leading_colon: Some(Token![::](Span::call_site())),
      segments: punct![$($ident),*],
    }
  };
  ( $($ident:ident)::* ) => {
    Path {
      leading_colon: None,
      segments: punct![$(
        PathSegment {
          ident: Ident::new(stringify!($ident), Span::call_site()),
          arguments: PathArguments::None,
        }
      ),*],
    }
  };
}

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
  let into_funcs = input.variants.iter().enumerate().map(|(variant, v)| {
    let name = Ident::new(&to_lower(&v.ident.to_string()), v.ident.span());
    let into_name = Ident::new(&format!("into_{name}"), v.ident.span());
    let ty = &v.fields;
    quote!(
      #[allow(unused_parens)]
      pub fn #into_name(self) -> Option<#ty> {
        if self.variant == #variant {
          unsafe {
            Some(self.data.#name)
          }
        } else {
          None
        }
      }
    )
  });

  let gen_struct = ItemStruct {
    attrs:        vec![],
    vis:          input.vis,
    struct_token: Token![struct](input.enum_token.span()),
    ident:        input.ident.clone(),
    generics:     input.generics,
    fields:       Fields::Named(fields_named! {
      variant: Type::Path(TypePath { qself: None, path: path!(usize) }),
      data: Type::Path(TypePath { qself: None, path: data_name.clone().into() }),
    }),
    semi_token:   None,
  };
  /*
    pub struct #name {
      variant: usize,
      data: #data_name,
    }
  );
  */
  let gen_union = quote!(
    union #data_name {
      #(#fields),*
    }
  );

  let struct_docs = gen_docs(&gen_struct);
  let union_docs = gen_docs(&gen_union);

  let out = quote! {
    #(#input_attrs)*
    #[doc = "Original enum:"]
    #[doc = #original_docs]
    #[doc = "Converted to struct:"]
    #[doc = #struct_docs]
    #[doc = "Along with the union:"]
    #[doc = #union_docs]
    #[repr(C)]
    #[derive(Clone)]
    #gen_struct
    #[allow(unused_parens)]
    #[derive(Clone, Copy)]
    #gen_union

    impl #name {
      #(#as_funcs)*
      #(#into_funcs)*
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
