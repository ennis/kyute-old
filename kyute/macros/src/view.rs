use crate::CRATE;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{quote, TokenStreamExt};
use syn::{
    braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Token,
    Token,
};

struct ViewKeyword;

impl Parse for ViewKeyword {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        if ident == "view" {
            Ok(ViewKeyword)
        } else {
            Err(syn::Error::new(Span::call_site(), "expected `view`"))
        }
    }
}

#[derive(Debug)]
struct StateField {
    name: syn::Ident,
    ty: syn::Type,
    init: syn::Expr,
}

impl StateField {
    fn gen_field(&self) -> TokenStream {
        let name = &self.name;
        let ty = &self.ty;
        quote! { #name: #ty }
    }
}

impl Parse for StateField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _let: Token![let] = input.parse()?;
        let _mut: Token![mut] = input.parse()?;
        let name: syn::Ident = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        let ty: syn::Type = input.parse()?;
        let _eq: Token![=] = input.parse()?;
        let init: syn::Expr = input.parse()?;
        let _semi: Token![;] = input.parse()?;
        let state_field = StateField { name, ty, init };
        eprintln!("StateField {:?}", state_field);
        Ok(state_field)
    }
}

#[derive(Debug)]
struct PropertyBinding {
    name: syn::Ident,
    init: syn::Expr,
}

impl Parse for PropertyBinding {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        eprintln!("PropertyBinding");
        let name: syn::Ident = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        let init: syn::Expr = input.parse()?;
        let _semi: Token![;] = input.parse()?;
        Ok(PropertyBinding { name, init })
    }
}

#[derive(Debug)]
struct WidgetExpr {
    ty: syn::Type,
    data: Option<syn::Expr>,
    properties: Vec<PropertyBinding>,
    child_widgets: Vec<WidgetExpr>,
}

impl Parse for WidgetExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        eprintln!("enter WidgetExpr");

        let ty: syn::Type = input.parse()?;
        eprintln!("WidgetExpr ty={:?}", ty);

        let mut properties = Vec::new();
        let mut child_widgets = Vec::new();
        let mut data: Option<syn::Expr> = None;

        eprintln!("Body");
        if input.peek(syn::token::Brace) {
            eprintln!("Body enter");
            // child widgets
            let body;
            let _brace = braced!(body in input);

            while !body.is_empty() {
                if body.peek2(Token![:]) {
                    // parse property binding
                    properties.push(body.parse()?);
                } else {
                    // parse child widget decl
                    child_widgets.push(body.parse()?);
                }
            }
        }

        eprintln!("WidgetExpr end");

        Ok(WidgetExpr {
            ty,
            data,
            properties,
            child_widgets,
        })
    }
}

#[derive(Debug)]
struct PropertyDecl {
    name: syn::Ident,
    mutable: bool,
    ty: syn::Type,
    default_value: Option<syn::Expr>,
}

impl PropertyDecl {
    fn gen_method(&self) -> TokenStream {
        let name = &self.name;
        let ty = &self.ty;

        let getter = syn::Ident::new(&format!("get_{}", name), Span::call_site());

        let mut tokens = quote! {
            fn #getter(&self) -> #ty;
        };

        if self.mutable {
            let setter = syn::Ident::new(&format!("set_{}", name), Span::call_site());
            tokens.extend(quote! {
                fn #setter(&mut self, value: #ty);
            })
        }

        tokens
    }
}

impl Parse for PropertyDecl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mutable = if input.peek(Token![mut]) {
            let _mut: Token![mut] = input.parse()?;
            true
        } else {
            false
        };

        let name: syn::Ident = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        let ty: syn::Type = input.parse()?;

        let default_value = if input.peek(Token![=]) {
            let _eq: Token![=] = input.parse()?;
            let expr: syn::Expr = input.parse()?;
            Some(expr)
        } else {
            None
        };

        let prop = PropertyDecl {
            name,
            mutable,
            ty,
            default_value,
        };
        Ok(dbg!(prop))
    }
}

#[derive(Debug)]
struct ViewDecl {
    name: syn::Ident,
    props: Punctuated<PropertyDecl, Token![,]>,
    span: Span,
    state_fields: Vec<StateField>,
    root_widget: WidgetExpr,
}

impl Parse for ViewDecl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        eprintln!("ViewDecl");

        // `view`
        let view_kw: ViewKeyword = input.parse()?;

        // name
        let name: syn::Ident = input.parse()?;

        // (<properties>)
        let props: Punctuated<PropertyDecl, Token![,]> = if input.peek(syn::token::Paren) {
            let props_buffer;
            let _parens = parenthesized!(props_buffer in input);
            props_buffer.parse_terminated(PropertyDecl::parse)?
        } else {
            Default::default()
        };

        // { ..body.. }
        let body;
        let _braces = braced!(body in input);

        // state fields (let mut xxx = ...);
        let mut state_fields = vec![];
        while body.peek(Token![let]) {
            state_fields.push(body.parse()?);
        }

        eprintln!("past state_fields");

        let root_widget = body.parse()?;

        Ok(ViewDecl {
            name,
            span: input.span(),
            props,
            state_fields,
            root_widget,
        })
    }
}

impl ViewDecl {
    fn generate(&self) -> TokenStream {
        // generated unique identifier
        let span = self.span;
        let base_ident = format!(
            "View_{}_{}_{}_{}",
            span.start().line,
            span.start().column,
            span.end().line,
            span.end().column
        );

        let state = syn::Ident::new(&format!("{}_State", self.name), Span::call_site());
        let data = syn::Ident::new(&format!("{}_Data", self.name), Span::call_site());
        let prop_trait = syn::Ident::new(&format!("{}_Properties", self.name), Span::call_site());
        let view = &self.name;
        let state_fields: Vec<_> = self.state_fields.iter().map(|f| f.gen_field()).collect();
        let prop_methods: Vec<_> = self.props.iter().map(|p| p.gen_method()).collect();

        let wrap_inner_widget_call = |method_call: TokenStream| {
            let data = &data;
            quote! {
                let mut r = None;
                #CRATE::take_mut::take(data, |outer_data| {
                    let mut inner_data = #data {
                        outer_data,
                        state: self.state.take().unwrap()
                    };
                    r = Some(#method_call);
                    self.state.replace(inner_data.state);
                    inner_data.outer_data
                });
                r.unwrap()
            }
        };

        let event_inner_call = wrap_inner_widget_call(quote!{ self.inner.event(ctx, event, &mut inner_data) });
        let lifecycle_inner_call = wrap_inner_widget_call(quote!{ self.inner.lifecycle(ctx, event, &mut inner_data) });
        let layout_inner_call = wrap_inner_widget_call(quote!{ self.inner.layout(ctx, constraints, &mut inner_data, env) });
        let inner_widget_ty = &self.root_widget.ty;

        quote! {
            // props
            trait #prop_trait {
                #(#prop_methods)*
            }

            // data
            struct #data <T: #prop_trait> {
                outer_data: T,
                state: #state,
            }

            struct #state {
                #(#state_fields,)*
            }

            struct #view <T: #prop_trait>  {
                state: Option<#state>,
                inner: #inner_widget_ty<#data<T>>
            }

            impl #view {

            }

            impl<T: #prop_trait + #CRATE::Model> #CRATE::Widget<T> for #view<T> {
                fn debug_name(&self) -> &str {
                    stringify!(#view)
                }

                fn event(&mut self, ctx: &mut #CRATE::EventCtx, event: &#CRATE::Event, data: &mut T) -> Option<<T as #CRATE::Model>::Change> {
                    #event_inner_call
                }

                fn update(&mut self, ctx: &mut #CRATE::UpdateCtx, data: &mut T, change: &<T as #CRATE::Model>::Change) {
                    todo!()
                }

                fn lifecycle(&mut self, ctx: &mut #CRATE::EventCtx, event: &#CRATE::LifecycleEvent, data: &mut T) {
                    #lifecycle_inner_call
                }

                fn layout(&mut self, ctx: &mut #CRATE::LayoutCtx, constraints: #CRATE::BoxConstraints, data: &mut T, env: &#CRATE::Environment) -> #CRATE::Measurements {
                     #layout_inner_call
                }

                fn paint(&self, ctx: &mut #CRATE::PaintCtx, bounds: #CRATE::Rect, env: &#CRATE::Environment) {
                     todo!()
                }
            }
        }
    }
}

pub(crate) fn generate_view(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    eprintln!("generate_view");
    let view_decl = syn::parse_macro_input!(input as ViewDecl);
    eprintln!("{:#?}", view_decl);
    let result = view_decl.generate();
    result.into()
}
