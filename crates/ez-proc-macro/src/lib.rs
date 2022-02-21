mod common;
mod doop;
mod errors;
mod main;

#[proc_macro_attribute]
pub fn throws(
    attribute_tokens: proc_macro::TokenStream,
    function_tokens: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    crate::errors::throws(attribute_tokens.into(), function_tokens.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_attribute]
pub fn try_throws(
    attribute_tokens: proc_macro::TokenStream,
    function_tokens: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    crate::errors::try_throws(attribute_tokens.into(), function_tokens.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_attribute]
pub fn main(
    attribute_tokens: proc_macro::TokenStream,
    function_tokens: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    crate::main::main(attribute_tokens.into(), function_tokens.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro]
pub fn doop(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    crate::doop::doop(tokens.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
