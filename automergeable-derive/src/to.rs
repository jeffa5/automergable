use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    punctuated::Punctuated, token::Comma, Attribute, Data, DataEnum, DataStruct, DeriveInput,
    Fields, Lit, Meta, NestedMeta, Variant,
};

use crate::utils;

pub fn to_automerge(input: &DeriveInput) -> TokenStream {
    match &input.data {
        Data::Struct(DataStruct { fields, .. }) => to_automerge_struct(input, fields),
        Data::Enum(DataEnum { variants, .. }) => to_automerge_enum(input, variants),
        Data::Union(_) => panic!("this derive macro only works on structs with named fields"),
    }
}

fn to_automerge_struct(input: &DeriveInput, fields: &Fields) -> TokenStream {
    let crate_path = utils::crate_path(input);
    let t_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let fields_to_automerge = fields_to_automerge(fields, true, &crate_path);
    quote! {
        #[automatically_derived]
        impl #impl_generics #crate_path::ToAutomerge for #t_name #ty_generics #where_clause {
            fn to_automerge(&self) -> automerge::Value {
                #fields_to_automerge
            }
        }
    }
}

fn to_automerge_enum(input: &DeriveInput, variants: &Punctuated<Variant, Comma>) -> TokenStream {
    let crate_path = utils::crate_path(input);
    let t_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let variants = variants.iter().map(|v| {
        let v_name = &v.ident;
        let fields = match &v.fields {
            Fields::Named(n) => {
                let names = n.named.iter().map(|n| {
                    let name = &n.ident;
                    quote! { #name, }
                });
                Some(quote! {{
                    #(#names)*
                }})
            }
            Fields::Unnamed(u) => {
                let items = u.unnamed.iter().enumerate().map(|(i, _)| {
                    let a = Ident::new(&format!("f{}", i), Span::call_site());
                    quote! { #a, }
                });
                Some(quote! {( #(#items)* )})
            }
            Fields::Unit => None,
        };
        let v_name_string = v_name.to_string();
        if let Some(fields) = fields {
            let fields_to_automerge = fields_to_automerge(&v.fields, false, &crate_path);
            quote! {
                Self::#v_name#fields => {
                    let mut outer = ::std::collections::HashMap::new();
                    let fields = {#fields_to_automerge};
                    outer.insert(#v_name_string.into(), fields);
                    automerge::Value::Map(outer)
                }
            }
        } else {
            quote! {
                Self::#v_name#fields => {
                    automerge::Value::Primitive(automerge::Primitive::Str(#v_name_string.into()))
                }
            }
        }
    });
    quote! {
        #[automatically_derived]
        impl #impl_generics #crate_path::ToAutomerge for #t_name #ty_generics #where_clause {
            fn to_automerge(&self) -> automerge::Value {
                match self {
                    #(#variants)*
                }
            }
        }
    }
}

fn get_representation_type(
    attrs: &[Attribute],
    field_name: &TokenStream,
    crate_path: &TokenStream,
) -> TokenStream {
    let mut ty = None;
    for a in attrs {
        match a.parse_meta().unwrap() {
            Meta::NameValue(_) | Meta::Path(_) => {}
            Meta::List(meta) => {
                if Some("automergeable".to_owned())
                    == meta.path.get_ident().map(ToString::to_string)
                {
                    for m in meta.nested {
                        match m {
                            NestedMeta::Meta(meta) => match meta {
                                Meta::Path(_) | Meta::List(_) => {}
                                Meta::NameValue(n) => {
                                    if let Lit::Str(lit) = &n.lit {
                                        ty = Some(lit.value())
                                    }
                                }
                            },
                            NestedMeta::Lit(_) => {}
                        }
                    }
                }
            }
        }
    }
    match ty.map(|s| s.to_lowercase()).as_deref() {
        Some("text") => {
            quote! {{
                use #crate_path::unicode_segmentation::UnicodeSegmentation;
                automerge::Value::Text(#field_name.graphemes(true).map(|s| s.into()).collect::<::std::vec::Vec<_>>())
            }}
        }
        Some("counter") => {
            quote! { automerge::Value::Primitive(automerge::Primitive::Counter(#field_name)) }
        }
        Some("timestamp") => {
            quote! { automerge::Value::Primitive(automerge::Primitive::Timestamp(#field_name)) }
        }
        _ => quote! { #field_name.to_automerge() },
    }
}

fn fields_to_automerge(fields: &Fields, is_struct: bool, crate_path: &TokenStream) -> TokenStream {
    match fields {
        Fields::Named(n) => {
            let fields = n.named.iter().map(|f| {
                let field_name = f.ident.as_ref().unwrap();
                let field_name_string = format_ident!("{}", field_name).to_string();

                let field_name = if is_struct {
                    quote! {self.#field_name}
                } else {
                    quote! {#field_name}
                };
                let repr = get_representation_type(&f.attrs, &field_name, crate_path);
                quote! {
                    fields.insert(#field_name_string.into(), #repr);
                }
            });
            quote! {
                let mut fields = ::std::collections::HashMap::new();
                #(#fields)*
                automerge::Value::Map(fields)
            }
        }
        Fields::Unnamed(u) => {
            if u.unnamed.len() == 1 {
                let field = u.unnamed.first().unwrap();
                let field_name = if is_struct {
                    let field_name = syn::Index::from(0);
                    quote! {self.#field_name}
                } else {
                    let f = Ident::new(&format!("f{}", 0), Span::call_site());
                    quote! {#f}
                };
                let repr = get_representation_type(&field.attrs, &field_name, crate_path);
                quote! {
                    #repr
                }
            } else {
                let fields = u.unnamed.iter().enumerate().map(|(i, f)| {
                    let field_name = if is_struct {
                        let field_name = syn::Index::from(i);
                        quote! {self.#field_name}
                    } else {
                        let f = Ident::new(&format!("f{}", i), Span::call_site());
                        quote! {#f}
                    };
                    let repr = get_representation_type(&f.attrs, &field_name, crate_path);
                    quote! {
                        fields.push(#repr);
                    }
                });
                quote! {
                    let mut fields = Vec::new();
                    #(#fields)*
                    automerge::Value::List(fields)
                }
            }
        }
        Fields::Unit => {
            quote! {
                automerge::Value::Primitive(automerge::Primitive::Null)
            }
        }
    }
}
