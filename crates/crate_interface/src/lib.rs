#![doc = include_str!("../README.md")]
#![feature(iter_next_chunk)]

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{Error, FnArg, ImplItem, ImplItemFn, ItemImpl, ItemTrait, PathSegment, TraitItem, Type};

fn compiler_error(err: Error) -> TokenStream {
    err.to_compile_error().into()
}

/// Define an interface.
///
/// This attribute should be added above the definition of a trait. All traits
/// that use the attribute cannot have the same name.
///
/// It is not necessary to define it in the same crate as the implementation,
/// but it is required that these crates are linked together.
///
/// See the [crate-level documentation](crate) for more details.
#[proc_macro_attribute]
pub fn def_interface(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return compiler_error(Error::new(
            Span::call_site(),
            "expect an empty attribute: `#[crate_interface_def]`",
        ));
    }

    let mut ast = syn::parse_macro_input!(item as ItemTrait);
    let trait_name = ast.ident.clone();
    ast.ident = format_ident!("InnerTrait");

    let mut extern_fn_list = vec![];
    let mut mod_fn_list = vec![];
    for item in &ast.items {
        if let TraitItem::Fn(method) = item {
            let mut sig = method.sig.clone();
            let fn_name = sig.ident;
            let extern_fn_name = format_ident!("__{}_{}", trait_name, fn_name.clone());
            sig.ident = extern_fn_name.clone();
            sig.inputs = syn::punctuated::Punctuated::new();

            let mut args: Vec<Box<syn::Pat>> = vec![];
            for arg in &method.sig.inputs {
                if let FnArg::Typed(inner_args) = arg {
                    sig.inputs.push(arg.clone());
                    args.push(inner_args.pat.clone());
                }
            }

            let mut func = sig.clone();
            func.ident = fn_name.clone();
            mod_fn_list.push(quote! {
                #[inline(always)]
                pub #func {
                    unsafe {
                        #extern_fn_name(#(#args),*)
                    }
                }
            });

            extern_fn_list.push(quote!(#sig;));
        }
    }
    let mod_str = quote! {
        #[allow(non_snake_case)]
        pub mod #trait_name {
            use super::*;
            #ast
            extern "Rust" {
                #(#extern_fn_list)*
            }
            #(#mod_fn_list)*
        }
    };

    quote! {
        #mod_str
    }
    .into()
}

/// Implement the interface for a struct.
///
/// This attribute should be added above the implementation of a trait for a
/// struct, and the trait must be defined with
/// [`#[def_interface]`](macro@crate::def_interface).
///
/// It is not necessary to implement it in the same crate as the definition, but
/// it is required that these crates are linked together.
///
/// See the [crate-level documentation](crate) for more details.
#[proc_macro_attribute]
pub fn impl_interface(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return compiler_error(Error::new(
            Span::call_site(),
            "expect an empty attribute: `#[crate_interface_impl]`",
        ));
    }

    let mut ast = syn::parse_macro_input!(item as ItemImpl);
    let trait_name = if let Some((_, path, _)) = &ast.trait_ {
        &path.segments.last().unwrap().ident
    } else {
        return compiler_error(Error::new_spanned(ast, "expect a trait implementation"));
    };
    let impl_name = if let Type::Path(path) = &ast.self_ty.as_ref() {
        path.path.get_ident().unwrap()
    } else {
        return compiler_error(Error::new_spanned(ast, "expect a trait implementation"));
    };

    for item in &mut ast.items {
        if let ImplItem::Fn(method) = item {
            let (attrs, vis, sig, stmts) =
                (&method.attrs, &method.vis, &method.sig, &method.block.stmts);
            let fn_name = &sig.ident;
            let extern_fn_name = format_ident!("__{}_{}", trait_name, fn_name).to_string();

            let mut new_sig = sig.clone();
            new_sig.ident = format_ident!("{}", extern_fn_name);
            new_sig.inputs = syn::punctuated::Punctuated::new();

            let mut args = vec![];
            let mut has_self = false;
            for arg in &sig.inputs {
                match arg {
                    FnArg::Receiver(_) => has_self = true,
                    FnArg::Typed(ty) => {
                        args.push(ty.pat.clone());
                        new_sig.inputs.push(arg.clone());
                    }
                }
            }

            let call_impl = if has_self {
                quote! {
                    let IMPL: #impl_name = #impl_name;
                    IMPL.#fn_name( #(#args),* )
                }
            } else {
                quote! { #impl_name::#fn_name( #(#args),* ) }
            };

            let item = quote! {
                #(#attrs)*
                #vis
                #sig
                {
                    {
                        #[export_name = #extern_fn_name]
                        extern "Rust" #new_sig {
                            #call_impl
                        }
                    }
                    #(#stmts)*
                }
            }
            .into();
            *method = syn::parse_macro_input!(item as ImplItemFn);
        }
    }
    // ast.trait_.as_mut().unwrap().1.segments.last_mut().unwrap().ident = format_ident!("{}Trait",trait_name);
    ast.trait_.as_mut().unwrap().1.segments.push(PathSegment::from(format_ident!("InnerTrait")));
    quote! { 
        #ast 
    }.into()
}
