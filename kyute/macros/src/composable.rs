use crate::CRATE;
use proc_macro2::{Span, Ident};
use quote::{quote, ToTokens};
use syn::{spanned::Spanned, FnArg, Path};
use syn::parse::{Parser, ParseStream};
use syn::punctuated::Punctuated;

struct ComposableArgs {
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
        Ok(ComposableArgs {
            uncached
        })
    }
}

pub fn generate_composable(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // works only on trait declarations
    let fn_item: syn::ItemFn = syn::parse_macro_input!(item as syn::ItemFn);
    let attr_args : ComposableArgs = syn::parse_macro_input!(attr as ComposableArgs);

    let vis = &fn_item.vis;
    let attrs = &fn_item.attrs;
    let sig = &fn_item.sig;
    let orig_block = &fn_item.block;
    let return_type = &fn_item.sig.output;

    let altered_fn = if attr_args.uncached {
        quote! {
            #[track_caller]
            #(#attrs)* #vis #sig {
                ::#CRATE::Context::scoped(0, move || #return_type {
                    #orig_block
                })
            }
        }
    } else {
        // convert fn args to tuple
        let args: Vec<_> = sig
            .inputs
            .iter()
            .map(|arg| match arg {
                FnArg::Receiver(r) => {
                    // FIXME, tbh, methods could be cached composables, we just need `self` to be any+clone
                    syn::Error::new(r.span(), "methods cannot be cached `composable` functions: consider using `composable(uncached)`")
                        .to_compile_error()
                }
                FnArg::Typed(arg) => arg.pat.to_token_stream(),
            })
            .collect();

        quote! {
            #[track_caller]
            #(#attrs)* #vis #sig {
                ::#CRATE::Context::scoped(0, move || {
                    ::#CRATE::Context::cache((#(#args,)*), move || #return_type {
                        #orig_block
                    })
                })
            }
        }
    };

    eprintln!("{}", altered_fn);
    altered_fn.into()
}
