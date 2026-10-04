use syn::{ItemFn, parse_quote};

pub fn with_tracing(mut item: ItemFn) -> ItemFn {
    let setup = parse_quote!(
        let _otlp_flush_guard = {
            ::sed_telemetry::macro_support::with_tracing();
        };
    );

    item.block.stmts.insert(0, setup);
    item
}
