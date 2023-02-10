use std::str::FromStr;

use proc_macro::TokenStream;

use quote::quote;
use syn::{self, DeriveInput};

#[allow(unused_must_use)]
#[proc_macro_derive(ServerStateItem)]
pub fn server_state_item_derive(input: TokenStream) -> TokenStream {
    //let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let input = syn::parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let gen = quote! {
        impl ServerStateItem for #name {
            fn update_id(&mut self, nid : NooID, notifier: Rc<RefCell<Notifier>>) {
                self.id = nid;
                self.notifier = Some(notifier);
            }

            fn id(&self) -> NooID {
                return self.id;
            }
        }

        impl Drop for #name {
            fn drop(&mut self) {
                if (self.notifier.is_none()) {return};
                let mid = #name::delete_message_id();
                let write_tuple = (mid, CommonDeleteMessage{id: self.id()});

                let mut recorder = Recorder::default();

                ciborium::ser::into_writer(&write_tuple, &mut recorder.data).unwrap();

                send_to(&mut self.notifier, recorder);

                let id = self.id();

                write_free_id(&mut self.notifier, id);
            }
        }
    };

    proc_macro::TokenStream::from(gen)
}

//

#[proc_macro_derive(UpdatableStateItem)]
pub fn emit_optional_patch_function(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let name_string = quote!(#name).to_string();

    let host_name = name_string.as_str().strip_suffix("Updatable").unwrap();

    let mut ret_impl = format!(
        "

        #[derive(Serialize)]
        struct {name}Bouncer<'a> {{
            id: NooID,

            #[serde(flatten)]
            update_info : &'a {name}
        }}

        impl UpdatableStateItem for {name} {{
            type HostState = ComponentPtr<{host_name}>;
            fn patch(self, h: &mut Self::HostState){{
                let write_tuple = (
                    {host_name}::update_message_id(),
                    {name}Bouncer {{
                        id: h.id(),
                        update_info: &self,
                    }}
                );

                let mut recorder = Recorder::default();

                ciborium::ser::into_writer(&write_tuple, &mut recorder.data).unwrap();

                h.send_to_tx(recorder);

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
                host_state.state.extra.{name_string} = self.{name_string};
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
