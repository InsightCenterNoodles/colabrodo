use std::{collections::HashMap, str::FromStr};

use proc_macro::TokenStream;

use quote::quote;
use syn::{self, parse::Parse, parse2, DeriveInput, Type};

/// Builds machinery to update a component
#[proc_macro_derive(UpdatableStateItem)]
pub fn emit_optional_patch_function(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let name_string = quote!(#name).to_string();

    let host_name = name_string.as_str().strip_suffix("Updatable").unwrap();

    let id_name = host_name
        .strip_prefix("Server")
        .unwrap()
        .strip_suffix("State")
        .unwrap();

    let mut ret_impl = format!(
        "
        impl UpdatableStateItem for {name} {{
            type HostState = ComponentReference<{id_name}ID, {host_name}>;
            fn patch(self, h: &Self::HostState){{

                if log::log_enabled!(log::Level::Debug) {{
                    log::debug!(\"Patching component with {{:?}}\", self);
                }}

                let recorder = Recorder::record(
                    {host_name}::update_message_id() as u32, 
                    &Bouncer {{
                        id: h.id(),
                        content: &self,
                    }}
                );

                h.send_to_broadcast(recorder);

                h.0.mutate(|host_state| {{
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
    ret_impl.push_str("});\n}\n}\n");

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

fn is_std_option(ty: &syn::Type) -> bool {
    match ty {
        Type::Group(syn::TypeGroup { elem, .. })
        | Type::Paren(syn::TypeParen { elem, .. })
        | Type::Path(syn::TypePath {
            qself: Some(syn::QSelf { ty: elem, .. }),
            ..
        }) => is_std_option(elem),

        Type::Path(syn::TypePath { qself: None, path }) => {
            (path.leading_colon.is_none()
                && path.segments.len() == 1
                && path.segments[0].ident == "Option")
                || (path.segments.len() == 3
                    && (path.segments[0].ident == "std"
                        || path.segments[0].ident == "core")
                    && path.segments[1].ident == "option"
                    && path.segments[2].ident == "Option")
        }

        _ => false,
    }
}

#[proc_macro_derive(CBORTransform, attributes(vserde))]
pub fn value_serde(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    let struct_name = input.ident;

    let struct_name_string = quote!(#struct_name).to_string();

    let mut ret_impl = format!(
        "
        impl CBORTransform for {struct_name_string} {{
            fn try_from_cbor(value: Value)  -> Result<Self, FromValueError> {{
                if let Value::Map(m) = value {{
                    let mut map = convert_to_value_map(m);

                    return Ok(Self {{
        "
    );

    let mut ret_impl_de = String::from(
        "
        fn to_cbor(&self) -> Value {
            let mut ret = Vec::<(Value, Value)>::new();
        ",
    );

    fn handle_part(
        fld: &syn::Field,
        updater_impl: &mut String,
        de_impl: &mut String,
    ) {
        let name_string = &fld.ident;
        let name_string = quote!(#name_string).to_string();
        let mut decode_name = name_string.clone();

        let generics = fld.attrs.iter().find(|a| {
            a.path.segments.len() == 1 && a.path.segments[0].ident == "vserde"
        });

        if let Some(this_attr) = generics {
            let parts: CBORParams = parse2(this_attr.tokens.clone()).unwrap();

            //println!("DEBUG {:?}", parts.tys);

            if let Some(syn::Lit::Str(rn)) = parts.tys.get("rename") {
                decode_name = rn.token().to_string();
                decode_name =
                    decode_name[1..(decode_name.len() - 1)].to_string();
            }
        }

        if is_std_option(&fld.ty) {
            updater_impl.push_str(&format!(
                "{name_string} : from_cbor_option(map.remove(\"{decode_name}\"))?,\n"));
            de_impl.push_str(&format!(
                "if let Some(v) = &self.{name_string} {{ 
                    ret.push( 
                        (Value::Text(\"{decode_name}\".into()), 
                        to_cbor(v)) 
                    );
                }}
                "
            ));
        } else {
            updater_impl.push_str(&format!(
                "{name_string} : from_cbor(get_map(&mut map, \"{decode_name}\")?)?,\n"
            ));
            de_impl.push_str(&format!(
                "ret.push( (Value::Text(\"{decode_name}\".into()), to_cbor(&self.{name_string})) );\n"
            ));
        }
    }

    if let syn::Data::Struct(struct_info) = input.data {
        if let syn::Fields::Named(named) = struct_info.fields {
            for fld in named.named {
                handle_part(&fld, &mut ret_impl, &mut ret_impl_de);
            }
        }
    }

    // finish off each part
    ret_impl.push_str(
        "
});
}

Err(FromValueError::WrongType {
    expected: \"Map\".into(),
    found: get_value_type(&value),
})
}
",
    );

    ret_impl_de.push_str(" Value::Map(ret) }");
    ret_impl.push_str(ret_impl_de.as_str());
    ret_impl.push_str("}\n");

    //print!("{}", ret_impl);

    return TokenStream::from_str(ret_impl.as_str()).unwrap();
}

struct CBORParams {
    tys: HashMap<String, syn::Lit>,
}

impl Parse for CBORParams {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        syn::parenthesized!(content in input);

        //println!("DEBUG CONTENT {}", content.to_string());

        let mut ret = HashMap::<String, syn::Lit>::new();
        while let Ok(p) = content.parse() {
            let key: syn::Ident = p;
            let key = key.to_string();
            let equals = content.parse::<syn::Token![=]>();
            if equals.is_err() {
                continue;
            }
            let value: syn::Lit = content.parse().unwrap();
            ret.insert(key, value);
        }
        Ok(CBORParams { tys: ret })
    }
}
