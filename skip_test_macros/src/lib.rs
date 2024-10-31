use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{self};

struct AttributeList {
    outer: Vec<syn::Attribute>,
    inner: Vec<syn::Attribute>,
}

fn is_outer_attribute(attr: &syn::Attribute) -> bool {
    attr.path().is_ident("test")
}

fn separate_attributes(attrs: Vec<syn::Attribute>) -> AttributeList {
    let mut attribute_list = AttributeList { outer: vec![], inner: vec![] };
    for attr in attrs {
        if is_outer_attribute(&attr) {
            attribute_list.outer.push(attr);
        } else {
            attribute_list.inner.push(attr);
        }
    }
    attribute_list
}

fn make_inner_fn(attrs: Vec<syn::Attribute>, sig: syn::Signature, block: syn::Block) -> syn::ItemFn {
    let vis = syn::Visibility::Inherited;
    syn::ItemFn { attrs: attrs, vis: vis, sig: sig, block: block.into() }
}

fn make_arg_ref(arg: &syn::FnArg) -> TokenStream2 {
    match arg {
        syn::FnArg::Receiver(_) => quote! { self },
        syn::FnArg::Typed(arg) => arg.pat.clone().into_token_stream(),
    }
}

fn make_comma_separated(args: Vec<TokenStream2>) -> TokenStream2 {
    let punctuated = syn::punctuated::Punctuated::<TokenStream2, syn::Token![,]>::from_iter(args.into_iter());
    punctuated.into_token_stream()
}

fn make_outer_fn(
    attrs: Vec<syn::Attribute>,
    vis: syn::Visibility,
    sig: syn::Signature,
    inner_func: TokenStream2,
) -> syn::ItemFn {
    let forwarded_args: Vec<_> = sig.inputs.iter().map(|input| -> TokenStream2 { make_arg_ref(input) }).collect();
    let forwarded_args_concat = make_comma_separated(forwarded_args);
    let func_name = &sig.ident;
    let return_ty = match &sig.output {
        syn::ReturnType::Default => quote! {()},
        syn::ReturnType::Type(_, ty) => ty.to_token_stream(),
    };
    let block_tokens = quote! {
        {
            #inner_func
            let unwind_result = ::std::panic::catch_unwind(move || -> #return_ty {
                #func_name(#forwarded_args_concat)
            });
            match unwind_result {
                ::core::result::Result::Ok(retval) => retval,
                ::core::result::Result::Err(cause) => {
                    if cause.type_id() == ::std::any::TypeId::of::<::skip_test::Skip>() {
                        return ::skip_test::Skipped.into()
                    }
                    else {
                        ::std::panic::resume_unwind(cause);
                    }
                }
            }
        }
    };
    let block = match syn::parse2::<syn::Block>(block_tokens) {
        Ok(block) => block,
        Err(err) => panic!("{}", err),
    };
    syn::ItemFn { attrs: attrs, vis: vis, sig: sig, block: block.into() }
}

fn wrap_test_func(item: TokenStream2) -> TokenStream2 {
    let func = match syn::parse2::<syn::ItemFn>(item) {
        Ok(func) => func,
        Err(err) => return err.into_compile_error(),
    };
    let AttributeList { outer: outer_attrs, inner: inner_attrs } = separate_attributes(func.attrs);
    let inner_func = make_inner_fn(inner_attrs, func.sig.clone(), *func.block);
    make_outer_fn(outer_attrs, func.vis, func.sig, inner_func.into_token_stream()).into_token_stream()
}

#[proc_macro_attribute]
pub fn may_skip(_attr: TokenStream, item: TokenStream) -> TokenStream {
    wrap_test_func(item.into()).into()
}
