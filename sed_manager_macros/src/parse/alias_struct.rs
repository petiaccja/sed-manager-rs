use syn::{self, spanned::Spanned};

pub struct AliasStruct {
    pub name: syn::Ident,
    pub ty: syn::Type,
}

impl AliasStruct {
    pub fn parse(ast: &syn::DeriveInput) -> Result<Self, syn::Error> {
        let syn::Data::Struct(struct_ast) = &ast.data else {
            return Err(syn::Error::new(ast.span(), "expected a struct"));
        };
        let name = ast.ident.clone();
        if struct_ast.fields.len() != 1 {
            return Err(syn::Error::new(ast.span(), "alias struct must have data"));
        };
        let Some(field) = struct_ast.fields.iter().next() else {
            return Err(syn::Error::new(ast.span(), "alias struct must have data"));
        };
        if field.ident.is_some() {
            return Err(syn::Error::new(field.span(), "alias struct data must be a single type"));
        }
        let ty = field.ty.clone();
        Ok(Self { name, ty })
    }
}
