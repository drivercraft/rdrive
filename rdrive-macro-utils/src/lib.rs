extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate core;
extern crate proc_macro2;
extern crate syn;

use proc_macro::TokenStream;
use syn::parse_str;

pub fn module_driver_with_linker(
    input: TokenStream,
    use_prefix: &str,
    link_section: Option<&str>,
) -> TokenStream {
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
    let mod_name = format_ident!("__{}", st_name.to_lowercase());

    // 解析路径
    let path_str = format!("{}::DriverRegister", use_prefix.trim_end_matches("::"));
    let type_register: syn::Path = parse_str(&path_str).expect("Failed to parse path");

    let path_driver_kind = format!("{}::DriverKind", use_prefix.trim_end_matches("::"));
    let type_driver_kind: syn::Path = parse_str(&path_driver_kind).expect("Failed to parse path");

    let path_probe_kind = format!("{}::register::ProbeKind", use_prefix.trim_end_matches("::"));
    let type_probe_kind: syn::Path = parse_str(&path_probe_kind).expect("Failed to parse path");


    let section = link_section.unwrap_or(".driver.register");

    quote! {

        mod #mod_name{
            use super::*;
            use #type_driver_kind;
            use #type_probe_kind;

            #[unsafe(link_section = #section)]
            #[unsafe(no_mangle)]
            #[used(linker)]
            pub static #static_name: #type_register = #type_register{
                #input
            };
        }
    }
    .into()
}
