use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::{self, spanned::Spanned};

struct Struct {
    pub name: syn::Ident,
    pub fields: Vec<Field>,
    pub array_traits: Vec<ArrayTrait>,
}

enum Field {
    Ident(syn::Ident),
    Index(syn::Index),
}

struct ArrayTrait {
    path: syn::Path,
    func_name: syn::Ident,
}

impl Struct {
    pub fn parse(ast: &syn::DeriveInput) -> Result<Self, syn::Error> {
        use core::sync::atomic::{AtomicU32, Ordering::Relaxed};

        let syn::Data::Struct(data) = &ast.data else {
            return Err(syn::Error::new(ast.span(), "expected a struct"));
        };

        let name = ast.ident.clone();
        let array_traits_attr = &ast.attrs.iter().find(|attr| attr.path().is_ident("as_array_traits"));
        let array_traits = match array_traits_attr {
            Some(array_traits_attr) => parse_array_traits(array_traits_attr)?,
            None => Vec::new(),
        };

        let index: AtomicU32 = 0.into(); // I just want fetch_add.
        let fields: Vec<_> = data
            .fields
            .iter()
            .map(|field| {
                if let Some(ident) = &field.ident {
                    Field::Ident(ident.clone())
                } else {
                    Field::Index(syn::Index { index: index.fetch_add(1, Relaxed), span: ast.span() })
                }
            })
            .collect();
        Ok(Self { name, fields, array_traits })
    }
}

impl ToTokens for Field {
    fn into_token_stream(self) -> TokenStream2
    where
        Self: Sized,
    {
        match self {
            Self::Ident(v) => v.into_token_stream(),
            Self::Index(v) => v.into_token_stream(),
        }
    }

    fn to_token_stream(&self) -> TokenStream2 {
        match self {
            Self::Ident(v) => v.to_token_stream(),
            Self::Index(v) => v.to_token_stream(),
        }
    }

    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Self::Ident(v) => v.to_tokens(tokens),
            Self::Index(v) => v.to_tokens(tokens),
        }
    }
}

fn parse_array_traits(attr: &syn::Attribute) -> Result<Vec<ArrayTrait>, syn::Error> {
    let mut array_traits = Vec::new();
    attr.parse_nested_meta(|meta| {
        let mut args = Vec::<syn::Ident>::new();
        if !meta.input.is_empty() {
            meta.parse_nested_meta(|arg| {
                if let Some(ident) = arg.path.get_ident() {
                    args.push(ident.clone());
                    Ok(())
                } else {
                    Err(meta.error("expected an identifier"))
                }
            })?;
        }
        let func_name = args.pop().unwrap_or(syn::Ident::new("as_array", meta.path.span().clone()));
        array_traits.push(ArrayTrait { path: meta.path, func_name });
        Ok(())
    })?;
    Ok(array_traits)
}

fn gen_as_array(name: &syn::Ident, fields: &[Field], array_trait: &ArrayTrait) -> TokenStream2 {
    let as_array = &array_trait.func_name;
    let as_array_mut = format_ident!("{}_mut", as_array);
    let trait_path = &array_trait.path;
    let count = fields.len();
    quote! {
        impl #name {
            pub fn #as_array(&self) -> [&dyn #trait_path; #count] {
                [
                    #(&self.#fields as &dyn #trait_path,)*
                ]
            }

            pub fn #as_array_mut(&mut self) -> [&mut dyn #trait_path; #count] {
                [
                    #(&mut self.#fields as &mut dyn #trait_path,)*
                ]
            }
        }
    }
}

#[proc_macro_derive(AsArray, attributes(as_array_traits))]
pub fn derive_enumeration_type(tokens: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(tokens as syn::DeriveInput);
    let desc = match Struct::parse(&input) {
        Ok(declaration) => declaration,
        Err(err) => return err.to_compile_error().into(),
    };
    let mut impls = quote! {};
    for array_trait in &desc.array_traits {
        impls.append_all(gen_as_array(&desc.name, &desc.fields, array_trait));
    }
    impls.into()
}
