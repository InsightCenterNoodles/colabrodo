use std::str::FromStr;

use proc_macro::TokenStream;

use quote::quote;
use syn::{self, parse::Parse, parse2, DeriveInput};

#[proc_macro_derive(UpdatableStateItem)]
pub fn emit_optional_patch_function(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let name_string = quote!(#name).to_string();

    let host_name = name_string.as_str().strip_suffix("Updatable").unwrap();

    let mut ret_impl = format!(
        "
        impl UpdatableStateItem for {name} {{
            type HostState = ComponentReference<{host_name}>;
            fn patch(self, h: &mut Self::HostState){{
                let write_tuple = (
                    {host_name}::update_message_id(),
                    Bouncer {{
                        id: h.id(),
                        content: &self,
                    }}
                );

                let mut recorder = Recorder::default();

                ciborium::ser::into_writer(&write_tuple, &mut recorder.data).unwrap();

                h.send_to_broadcast(recorder);

                let mut host_state = h.0.borrow_mut();
        "
    );

    fn handle_part(fld: &syn::Field, updater_impl: &mut String) {
        let name_string = &fld.ident;
        //let type_string = &fld.ty;

        let name_string = quote!(#name_string).to_string();
        //let type_string = quote!(#type_string).to_string();

        //let unwrapped_type_string = extract_type_from_option(&fld.ty);
        //let unwrapped_type_string = quote!(#unwrapped_type_string).to_string();

        match name_string.as_str() {
            "name" => return,
            "id" => return,
            "notifier" => return,
            _ => (),
        }

        updater_impl.push_str(&format!(
            "if self.{name_string}.is_some() {{
                host_state.mutable.{name_string} = self.{name_string};
            }}\n"
        ));
    }

    if let syn::Data::Struct(struct_info) = input.data {
        if let syn::Fields::Named(named) = struct_info.fields {
            for fld in named.named {
                handle_part(&fld, &mut ret_impl);
            }
        }
    }

    // finish off each part
    ret_impl.push_str("\n}\n}\n");

    //print!("{}", ret_impl);

    return TokenStream::from_str(ret_impl.as_str()).unwrap();
}

struct PatchParams {
    tys: Vec<syn::Ident>,
}

impl Parse for PatchParams {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        syn::parenthesized!(content in input);

        let mut ret = Vec::<syn::Ident>::new();
        while let Ok(p) = content.parse() {
            ret.push(p);

            let comma = content.parse::<syn::Token![,]>();
            if comma.is_err() {
                break;
            }
        }
        Ok(PatchParams { tys: ret })
    }
}

#[proc_macro_derive(DeltaPatch, attributes(patch_generic))]
pub fn emit_delta(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    let generics = input.attrs.iter().find(|a| {
        a.path.segments.len() == 1
            && a.path.segments[0].ident == "patch_generic"
    });

    let generics = match generics {
        None => String::new(),
        Some(this_attr) => {
            let parts: PatchParams = parse2(this_attr.tokens.clone()).unwrap();

            let v: Vec<String> = parts
                .tys
                .iter()
                .map(|x: &syn::Ident| x.to_string())
                .collect();

            format!("<{}>", v.join(","))
        }
    };

    let struct_name = input.ident;

    let struct_name_string = quote!(#struct_name).to_string();

    let mut ret_impl = format!(
        "
        impl{generics} DeltaPatch for {struct_name_string}{generics} {{
            fn patch(&mut self, other: Self){{
        "
    );

    fn handle_part(fld: &syn::Field, updater_impl: &mut String) {
        let name_string = &fld.ident;
        let name_string = quote!(#name_string).to_string();

        updater_impl.push_str(&format!(
            "if other.{name_string}.is_some() {{
                self.{name_string} = other.{name_string};
            }}\n"
        ));
    }

    if let syn::Data::Struct(struct_info) = input.data {
        if let syn::Fields::Named(named) = struct_info.fields {
            for fld in named.named {
                handle_part(&fld, &mut ret_impl);
            }
        }
    }

    // finish off each part
    ret_impl.push_str("\n}\n}\n");

    //print!("{}", ret_impl);

    return TokenStream::from_str(ret_impl.as_str()).unwrap();
}
