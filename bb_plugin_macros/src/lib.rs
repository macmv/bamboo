use proc_macro::TokenStream;

mod behavior;
mod define_ty;
mod window;

#[proc_macro_error::proc_macro_error]
#[proc_macro_attribute]
pub fn define_ty(args: TokenStream, input: TokenStream) -> TokenStream {
  define_ty::define_ty(args, input)
}

#[proc_macro]
pub fn behavior(input: TokenStream) -> TokenStream { behavior::behavior(input) }

#[proc_macro_derive(Window, attributes(filter, output, name, ignore))]
pub fn window(input: TokenStream) -> TokenStream { window::window(input) }
