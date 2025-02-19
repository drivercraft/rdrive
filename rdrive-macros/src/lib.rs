use proc_macro::TokenStream;

#[proc_macro]
pub fn module_driver(input: TokenStream) -> TokenStream {
    rdrive_macro_utils::module_driver_with_linker(input, "rdrive")
}
