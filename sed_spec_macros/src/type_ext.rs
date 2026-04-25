use syn::{GenericArgument, PathArguments, PathSegment, Type, TypePath};

pub trait TypeExt {
    fn option_inner(&self) -> Option<&Self>;
    fn is_option(&self) -> bool {
        self.option_inner().is_some()
    }
    fn remove_option(&self) -> &Self {
        self.option_inner().unwrap_or(self)
    }
}

impl TypeExt for Type {
    fn option_inner(&self) -> Option<&Self> {
        match self {
            syn::Type::Path(TypePath { qself: _, path }) => {
                let mut rev_segments = path.segments.iter().rev();
                match rev_segments.next() {
                    Some(segment) if segment.ident == "Option" => (),
                    _ => return None,
                };
                match rev_segments.next() {
                    Some(segment) if segment.ident == "option" => (),
                    None if path.leading_colon.is_none() => (),
                    _ => return None,
                };
                match rev_segments.next() {
                    Some(segment) if segment.ident == "std" || segment.ident == "core" => (),
                    None if path.leading_colon.is_none() => (),
                    _ => return None,
                };
                if let Some(PathSegment { arguments: PathArguments::AngleBracketed(args), .. }) = path.segments.last() {
                    if let (Some(GenericArgument::Type(ty)), 1) = (args.args.first(), args.args.len()) {
                        return Some(ty);
                    }
                };
                None
            }
            _ => None,
        }
    }
}
