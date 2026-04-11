use sed_tper_macros::TokenizeMethodArgs;

#[derive(Debug, TokenizeMethodArgs, DetokenizeMethod)]
pub struct Properties {
    
}

#[derive(Debug, TokenizeMethod, DetokenizeMethod)]
pub struct StartSession {}

#[derive(Debug, TokenizeMethod, DetokenizeMethod)]
pub struct SyncSession {}

#[derive(Debug, TokenizeMethod, DetokenizeMethod)]
pub struct CloseSession {}
