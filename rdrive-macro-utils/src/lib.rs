extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate core;
extern crate proc_macro2;
extern crate syn;

use proc_macro::TokenStream;
use syn::parse_str;

pub fn module_driver_with_linker(input: TokenStream, use_prefix: &str) -> TokenStream {
    let input = proc_macro2::TokenStream::from(input);
    let mut name = None;

    {
        let mut it = input.clone().into_iter();
        while let Some(t) = it.next() {
            if let proc_macro2::TokenTree::Ident(i) = t {
                if i == "name" {
                    it.next();
                    if let Some(proc_macro2::TokenTree::Literal(l)) = it.next() {
                        let l = l.to_string();
                        let l = l.trim_matches('"');
                        name = Some(l.to_string());
                        break;
                    }
                }
            }
        }
    }

    let st_name = name.unwrap_or_default().replace("-", "_").replace(" ", "_");

    let static_name = format_ident!("DRIVER_{}", st_name.to_uppercase());

    // 解析路径
    let path_str = format!("{}::DriverRegister", use_prefix.trim_end_matches("::"));
    let type_register: syn::Path = parse_str(&path_str).expect("Failed to parse path");

    quote! {
        #[unsafe(link_section = ".driver.register")]
        #[unsafe(no_mangle)]
        #[used(linker)]
        pub static #static_name: #type_register = #type_register{
            #input
        };
    }
    .into()
}
