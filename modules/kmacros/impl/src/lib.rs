// use proc_macro::TokenStream;
// use quote::{quote_spanned, quote};
// use syn::parse_macro_input;

// #[proc_macro_attribute]
// pub fn byteos_driver(args: TokenStream, input: TokenStream) -> TokenStream {
//     let input:Element = parse_macro_input!(input);
//     let mut attrs = input.attrs;
//     let vis = input.vis;
//     let ident = input.ident;
//     let ty = input.ty;
//     let expr = input.expr;
//     let orig_item = input.orig_item;

//     // let sort_key = pos.into_iter().map(|pos| format!("{:04}", pos));

//     let path = "kmacros::linkme";
//     let new = quote_spanned!(input.start_span=> __new);
//     let uninit = quote_spanned!(input.end_span=> #new());

//     let expanded = quote! {
//         #path ! {
//             #(
//                 #![linkme_macro = #path]
//                 // #![linkme_sort_key = #sort_key]
//             )*
//             #(#attrs)*
//             #vis static #ident : #ty = {
//                 unsafe fn __typecheck(_: #linkme_path::__private::Void) {
//                     let #new = #linkme_path::__private::value::<#ty>;
//                     #linkme_path::DistributedSlice::private_typecheck(#path, #uninit)
//                 }

//                 #expr
//             };
//         }

//         #orig_item
//     };
//     TokenStream::from(expanded)
// }
