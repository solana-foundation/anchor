pub mod accounts;
pub mod error;
#[cfg(feature = "idl")]
pub mod file;
pub mod program;

pub fn tts_to_string<T: quote::ToTokens>(item: T) -> String {
    let mut tts = proc_macro2::TokenStream::new();
    item.to_tokens(&mut tts);
    tts.to_string()
}
