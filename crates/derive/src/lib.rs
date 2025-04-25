use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Addressable)]
pub fn derive_addressable(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let ident = ast.ident;

    let gen = quote! {
        // See the docs for Addressable
        unsafe impl Addressable for #ident {
            // Each bevy_harmonize mod gets its own __imports crate

            const COMPONENT_ID: usize = __imports::__resolve_component_id::<#ident>();

            // 1. Before resolving the manifest, __imports::__resolve_address returns a dangling placeholder pointer
            // 2. Allocate unique, non-overlapping ranges for each struct
            // 3. Then __resolve_address is adjusted to return pointers for each struct
            // 4. Finally, as a post-compilation step, we find instructions that read/write to these addresses and correct the address space and rebase the pointer from zero
            const PTR: *mut Self = __imports::__resolve_address::<#ident>();
        }
    };

    gen.into()
}
