//! `#[derive(Model)]` macro
use crate::{ident_from_str, CRATE};
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::{
    parse::{ParseStream, Parser},
    punctuated::Punctuated,
    spanned::Spanned,
    Data, DataStruct, FnArg, Meta, NestedMeta, Path,
};

/*struct ComposableArgs {
    uncached: bool,
}

impl syn::parse::Parse for ComposableArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let idents = Punctuated::<Ident, syn::Token![,]>::parse_terminated(input)?;

        let mut uncached = false;
        for ident in idents {
            if ident == "uncached" {
                uncached = true;
            } else {
                // TODO warn unrecognized attrib
            }
        }
        Ok(ComposableArgs { uncached })
    }
}*/

struct ModelFieldAttrs {
    skip: bool,
}

impl ModelFieldAttrs {
    pub fn parse(field: &syn::Field) -> Result<ModelFieldAttrs, syn::Error> {
        let mut skip = false;
        for attr in &field.attrs {
            if attr.path.is_ident("model") {
                match attr.parse_meta()? {
                    syn::Meta::List(meta_list) => {
                        for meta_item in meta_list.nested.iter() {
                            match meta_item {
                                NestedMeta::Meta(Meta::Path(path)) if path.is_ident("skip") => {
                                    if skip {
                                        return Err(syn::Error::new(
                                            meta_item.span(),
                                            "duplicate attribute",
                                        ));
                                    }
                                    skip = true;
                                }
                                _ => {
                                    return Err(syn::Error::new(
                                        meta_item.span(),
                                        "unrecognized `model` attribute",
                                    ))
                                }
                            }
                        }
                    }
                    _ => {
                        return Err(syn::Error::new(
                            attr.span(),
                            "unrecognized `model` attribute",
                        ))
                    }
                }
            }
        }

        Ok(ModelFieldAttrs { skip })
    }
}

pub(crate) fn derive_model_impl(
    input: syn::DeriveInput,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    match &input.data {
        Data::Struct(s) => derive_model_struct(&input, s),
        _ => Err(syn::Error::new(
            input.span(),
            "Model implementations can only be derived on structs for now",
        )),
    }
}

fn derive_model_struct(
    input: &syn::DeriveInput,
    data_struct: &syn::DataStruct,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let (impl_generics, ty_generics, where_clause) = &input.generics.split_for_impl();

    let vis = &input.vis;
    let mut variants = vec![];
    for (i, field) in data_struct.fields.iter().enumerate() {
        let attrs = ModelFieldAttrs::parse(&field)?;
        if attrs.skip {
            continue;
        }
        let ident = field
            .ident
            .clone()
            .unwrap_or_else(|| ident_from_str(&format!("element_{}", i)));
        let ty = &field.ty;
        variants.push(quote! {
            #ident(<#ty as #CRATE::Model>::Change)
        })
    }

    let tyname = &input.ident;
    let change_enum_name = ident_from_str(&format!("__Change_{}", tyname));

    Ok(quote! {
        #vis enum #change_enum_name #ty_generics {
            #(#variants,)*
        }

        impl #impl_generics #CRATE::Model for #tyname #ty_generics #where_clause {
            type Change = #change_enum_name #ty_generics;
        }
    })
}
