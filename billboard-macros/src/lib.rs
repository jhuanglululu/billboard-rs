//! Billboard's one proc macro: `#[billboard::main]`.
//!
//! It is a **shim**. The entry-point machinery — argument parsing, the
//! signature check, the generated exports — lives in `wasmachine-macros`, which
//! knows nothing about Billboard; this crate instantiates it with Billboard's
//! export names and crate path. (A `proc-macro` crate cannot export plain
//! functions, so "instantiate" means attaching the generic attribute, which the
//! `billboard` crate re-exports as `__sdk_main` so the path resolves inside the
//! animation's crate.)

use proc_macro::TokenStream;
use quote::quote;

/// Marks the animation entry point.
///
/// Requires `fn main() -> ExitCode`. Emits the `_billboard_main` wasm export
/// (runtime init + the user's fn as task 0, whose returned [`ExitCode`]
/// crosses the ABI as an `i32`) and the `_billboard_abi` version-handshake
/// export, so the plugin can refuse modules built against a different ABI
/// before running them.
///
/// # `random_seed`
///
/// ```ignore
/// #[billboard::main(random_seed = 20260726)]
/// fn main() -> ExitCode { … }
/// ```
///
/// Reseeds the host's deterministic random stream with that literal and makes
/// `default_random()` draw from it, so the animation plays out identically
/// every run. Without it, `default_random()` is non-deterministic. The reseed
/// happens during init, *before* `main`, so the very first draw is already
/// seeded.
///
/// [`ExitCode`]: ../billboard/enum.ExitCode.html
#[proc_macro_attribute]
pub fn main(attr: TokenStream, item: TokenStream) -> TokenStream {
    // The author's arguments are forwarded verbatim, spans and all, so parse
    // errors point at what they wrote rather than at anything generated here.
    let args = proc_macro2::TokenStream::from(attr);
    let item = proc_macro2::TokenStream::from(item);
    quote! {
        #[::billboard::__sdk_main(
            config(
                sdk = ::billboard,
                attribute = "#[billboard::main]",
                main_export = _billboard_main,
                abi_export = _billboard_abi,
            ),
            #args
        )]
        #item
    }
    .into()
}
