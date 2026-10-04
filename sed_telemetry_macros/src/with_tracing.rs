//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use syn::{Attribute, ItemFn, parse_quote};

pub fn with_tracing(mut item: ItemFn) -> Result<ItemFn, syn::Error> {
    let is_async = item.sig.asyncness.is_some();
    let potential_runtime_spawner = item.attrs.iter().find(|attr| is_potential_runtime_spawner(attr));
    if is_async && let Some(attr) = potential_runtime_spawner {
        return Err(syn::Error::new_spanned(
            attr,
            "`#[with_tracing]` must be placed below the async runtime's attribute, \
            otherwise spans that close when the runtime is dropped are not exported",
        ));
    }

    let setup = parse_quote!(
        let _otlp_flush_guard = ::sed_telemetry::macro_support::with_tracing();
    );

    item.block.stmts.insert(0, setup);
    Ok(item)
}

/// Checks if the attribute is potentially used to spawn an async runtime to run
/// an async function in. For example, `#[tokio::test]` or `#[test]` are flagged.
///
/// Note: this function flags **potential** spawners. `#[test]` is not valid on
/// async functions so it must be `tokio::test` or similar. All uses of
/// `#[apply]` are flagged as it might apply `smol_macros::test!` and similar.
fn is_potential_runtime_spawner(attr: &Attribute) -> bool {
    let path = attr.path().segments.iter().map(|segment| segment.ident.to_string()).collect::<Vec<_>>().join("::");
    [
        "test",
        "tokio::test",
        "tokio::main",
        "smol_potat::test",
        "smol_potat::main",
        "apply",
        "macro_rules_attribute::apply",
    ]
    .contains(&path.as_str())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn sync_fn() {
        let item: ItemFn = parse_quote!(
            #[test]
            fn test() {}
        );
        assert!(with_tracing(item).is_ok());
    }

    #[test]
    fn sync_fn_with_runtime_attr() {
        let item: ItemFn = parse_quote!(
            #[tokio::test]
            fn test() {}
        );
        assert!(with_tracing(item).is_ok());
    }

    #[test]
    fn async_fn_without_runtime() {
        let item: ItemFn = parse_quote!(
            async fn test() {}
        );
        assert!(with_tracing(item).is_ok());
    }

    #[rstest]
    #[case::tokio_test(parse_quote!(#[tokio::test]))]
    #[case::imported_test(parse_quote!(#[test]))]
    #[case::absolute_path_with_args(parse_quote!(#[::tokio::test(flavor = "multi_thread")]))]
    #[case::tokio_main(parse_quote!(#[tokio::main]))]
    #[case::smol_potat_test(parse_quote!(#[smol_potat::test]))]
    #[case::smol_potat_main(parse_quote!(#[smol_potat::main]))]
    #[case::apply(parse_quote!(#[apply(test!)]))]
    #[case::apply_absolute_path(parse_quote!(#[macro_rules_attribute::apply(smol_macros::main!)]))]
    fn async_fn_inside_runtime(#[case] attr: Attribute) {
        let mut item: ItemFn = parse_quote!(
            async fn test() {}
        );
        item.attrs.push(attr);
        assert!(with_tracing(item).is_err());
    }
}
