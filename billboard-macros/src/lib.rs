//! Proc macros for the Billboard SDK. Animation authors only ever see
//! `#[billboard::main]`; `vectors!` is internal to the SDK's math module.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, ItemFn, Token, braced, parenthesized, parse_macro_input};

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
    let args = match syn::parse::<MainArgs>(attr) {
        Ok(args) => args,
        Err(e) => return e.to_compile_error().into(),
    };
    let func = parse_macro_input!(item as ItemFn);
    let name = &func.sig.ident;
    if !func.sig.inputs.is_empty() || func.sig.asyncness.is_some() {
        return syn::Error::new_spanned(
            &func.sig,
            "#[billboard::main] requires `fn name() -> ExitCode` with no arguments",
        )
        .to_compile_error()
        .into();
    }
    // Seeding is part of init, so it runs before the user's first line and
    // before any task is spawned — the routing flag is then immutable, and a
    // fork just copies an already-correct value.
    let seed = args.random_seed.map(|seed| {
        // Emitted through proc_macro2's own literal, which renders i64::MIN as
        // `-9223372036854775808i64` — a form rustc accepts, unlike a negation
        // applied to a literal that has already overflowed.
        let seed = proc_macro2::Literal::i64_suffixed(seed);
        quote! { ::billboard::__rt::seed_random(#seed); }
    });
    quote! {
        #func

        #[unsafe(no_mangle)]
        pub extern "C" fn _billboard_main() -> i32 {
            ::billboard::__rt::init();
            #seed
            // Bind the result to `ExitCode` so a wrong return type fails here
            // with a clear "expected ExitCode" mismatch rather than deep in
            // the conversion.
            let __code: ::billboard::ExitCode = #name();
            __code.as_i32()
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn _billboard_abi() -> i32 {
            ::billboard::ABI_VERSION
        }
    }
    .into()
}

/// The arguments `#[billboard::main]` accepts. One so far, `random_seed = N`;
/// parsed as a name/value list so adding another stays backwards-compatible.
struct MainArgs {
    /// The seed to hand to `seed_random`.
    random_seed: Option<i64>,
}

impl Parse for MainArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut args = MainArgs { random_seed: None };
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "random_seed" => {
                    if args.random_seed.is_some() {
                        return Err(syn::Error::new(key.span(), "duplicate `random_seed`"));
                    }
                    args.random_seed = Some(parse_seed(input)?);
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!(
                            "unknown `#[billboard::main]` argument `{other}`; expected `random_seed`"
                        ),
                    ));
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(args)
    }
}

/// Parse `random_seed`'s value: an integer literal, optionally negated.
///
/// The magnitude is read as `i128` and the sign applied *before* the range
/// check, because `i64::MIN`'s magnitude (9223372036854775808) does not fit in
/// an `i64` on its own — parsing it as one first rejected a perfectly good seed.
fn parse_seed(input: ParseStream) -> syn::Result<i64> {
    let negative = if input.peek(Token![-]) {
        input.parse::<Token![-]>()?;
        true
    } else {
        false
    };
    let lit: syn::LitInt = input.parse()?;
    let magnitude = lit
        .base10_parse::<i128>()
        .map_err(|_| syn::Error::new_spanned(&lit, "`random_seed` must be an integer literal"))?;
    let value = if negative { -magnitude } else { magnitude };
    i64::try_from(value).map_err(|_| {
        syn::Error::new_spanned(
            &lit,
            format!(
                "`random_seed` must fit in i64 ({} to {}), got {value}",
                i64::MIN,
                i64::MAX
            ),
        )
    })
}

/// Generates the SDK's vector math types: structs, constructors, constants,
/// physics-typed operators, scalar scaling, and explicit `From` conversions.
///
/// # Internal to the SDK: the invoking crate must depend on `bytemuck`
///
/// The generated types derive `bytemuck::Pod`/`Zeroable`, so a vector can cross
/// a channel — and those derives expand to absolute `::bytemuck::…` paths. Any
/// crate invoking this macro therefore needs `bytemuck` in its own dependencies,
/// which is why this stays internal to the SDK's math module (the `billboard`
/// crate has it) rather than being part of the animation-facing API.
///
/// An animation wanting its own `Pod` struct should use `billboard::payload!`
/// instead: it routes the same derives through the SDK's re-export of bytemuck,
/// so it needs no dependency of its own.
///
/// ```ignore
/// vectors! {
///     pub struct Vector3d(f64);
///     pub struct Position(f64);
///     ops { Position + Offset = Position; }
///     scale { Offset * f64; }
///     convert { Position, Offset, Vector3d; }
/// }
/// ```
#[proc_macro]
pub fn vectors(input: TokenStream) -> TokenStream {
    let defs = parse_macro_input!(input as VectorDefs);
    defs.expand()
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

struct VectorDef {
    doc: Vec<syn::Attribute>,
    name: Ident,
    elem: Ident,
}

struct OpRule {
    lhs: Ident,
    op: char,
    rhs: Ident,
    out: Ident,
}

struct ScaleRule {
    vec: Ident,
    scalar: Ident,
}

struct VectorDefs {
    types: Vec<VectorDef>,
    ops: Vec<OpRule>,
    scales: Vec<ScaleRule>,
    converts: Vec<Vec<Ident>>,
}

impl Parse for VectorDefs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut defs = VectorDefs {
            types: vec![],
            ops: vec![],
            scales: vec![],
            converts: vec![],
        };
        while !input.is_empty() {
            if input.peek(Token![pub]) || input.peek(Token![struct]) || input.peek(Token![#]) {
                let doc = input.call(syn::Attribute::parse_outer)?;
                if input.peek(Token![pub]) {
                    input.parse::<Token![pub]>()?;
                }
                input.parse::<Token![struct]>()?;
                let name: Ident = input.parse()?;
                let elem_content;
                parenthesized!(elem_content in input);
                let elem: Ident = elem_content.parse()?;
                input.parse::<Token![;]>()?;
                defs.types.push(VectorDef { doc, name, elem });
                continue;
            }
            let section: Ident = input.parse()?;
            let content;
            braced!(content in input);
            match section.to_string().as_str() {
                "ops" => {
                    while !content.is_empty() {
                        let lhs: Ident = content.parse()?;
                        let op = if content.peek(Token![+]) {
                            content.parse::<Token![+]>()?;
                            '+'
                        } else {
                            content.parse::<Token![-]>()?;
                            '-'
                        };
                        let rhs: Ident = content.parse()?;
                        content.parse::<Token![=]>()?;
                        let out: Ident = content.parse()?;
                        content.parse::<Token![;]>()?;
                        defs.ops.push(OpRule { lhs, op, rhs, out });
                    }
                }
                "scale" => {
                    while !content.is_empty() {
                        let vec: Ident = content.parse()?;
                        content.parse::<Token![*]>()?;
                        let scalar: Ident = content.parse()?;
                        content.parse::<Token![;]>()?;
                        defs.scales.push(ScaleRule { vec, scalar });
                    }
                }
                "convert" => {
                    while !content.is_empty() {
                        let group: Punctuated<Ident, Token![,]> =
                            Punctuated::parse_separated_nonempty(&content)?;
                        if !content.is_empty() {
                            content.parse::<Token![;]>()?;
                        }
                        defs.converts.push(group.into_iter().collect());
                    }
                }
                other => {
                    return Err(syn::Error::new(
                        section.span(),
                        format!("unknown section `{other}`; expected ops, scale, or convert"),
                    ));
                }
            }
        }
        Ok(defs)
    }
}

impl VectorDefs {
    fn elem_of(&self, name: &Ident) -> syn::Result<&Ident> {
        self.types
            .iter()
            .find(|t| t.name == *name)
            .map(|t| &t.elem)
            .ok_or_else(|| {
                syn::Error::new(
                    name.span(),
                    format!("`{name}` is not a declared vector type"),
                )
            })
    }

    fn expand(&self) -> syn::Result<proc_macro2::TokenStream> {
        let mut out = proc_macro2::TokenStream::new();

        for VectorDef { doc, name, elem } in &self.types {
            let zero = lit_zero(elem);
            let one = lit_one(elem);
            out.extend(quote! {
                #(#doc)*
                // repr(C) + Pod: three same-typed scalars, so no padding and
                // every bit pattern is valid — which is what lets a vector be
                // a channel payload, on its own or inside a user struct.
                #[repr(C)]
                #[derive(
                    Clone, Copy, Debug, Default, PartialEq,
                    ::bytemuck::Pod, ::bytemuck::Zeroable,
                )]
                pub struct #name {
                    pub x: #elem,
                    pub y: #elem,
                    pub z: #elem,
                }

                impl #name {
                    pub const ZERO: Self = Self::splat(#zero);
                    /// Unit vector along +X.
                    pub const X: Self = Self::new(#one, #zero, #zero);
                    /// Unit vector along +Y.
                    pub const Y: Self = Self::new(#zero, #one, #zero);
                    /// Unit vector along +Z.
                    pub const Z: Self = Self::new(#zero, #zero, #one);

                    pub const fn new(x: #elem, y: #elem, z: #elem) -> Self {
                        Self { x, y, z }
                    }

                    pub const fn splat(v: #elem) -> Self {
                        Self::new(v, v, v)
                    }
                }

                impl ::core::ops::Neg for #name {
                    type Output = Self;
                    fn neg(self) -> Self {
                        Self::new(-self.x, -self.y, -self.z)
                    }
                }

                // So a shared `&#name` can be applied to many entities by
                // reference (entity setters take `impl AsRef<T>`).
                impl ::core::convert::AsRef<#name> for #name {
                    fn as_ref(&self) -> &#name {
                        self
                    }
                }

                impl From<(#elem, #elem, #elem)> for #name {
                    fn from(v: (#elem, #elem, #elem)) -> Self {
                        Self::new(v.0, v.1, v.2)
                    }
                }

                impl From<[#elem; 3]> for #name {
                    fn from(v: [#elem; 3]) -> Self {
                        Self::new(v[0], v[1], v[2])
                    }
                }

                impl From<#name> for (#elem, #elem, #elem) {
                    fn from(v: #name) -> Self {
                        (v.x, v.y, v.z)
                    }
                }

                impl From<#name> for [#elem; 3] {
                    fn from(v: #name) -> Self {
                        [v.x, v.y, v.z]
                    }
                }
            });
        }

        for OpRule {
            lhs,
            op,
            rhs,
            out: res,
        } in &self.ops
        {
            self.elem_of(lhs)?;
            let (trait_name, method) = if *op == '+' {
                ("Add", "add")
            } else {
                ("Sub", "sub")
            };
            let trait_ident = format_ident!("{trait_name}");
            let method_ident = format_ident!("{method}");
            let op_token = if *op == '+' { quote!(+) } else { quote!(-) };
            out.extend(quote! {
                impl ::core::ops::#trait_ident<#rhs> for #lhs {
                    type Output = #res;
                    fn #method_ident(self, rhs: #rhs) -> #res {
                        #res::new(self.x #op_token rhs.x, self.y #op_token rhs.y, self.z #op_token rhs.z)
                    }
                }
            });
            if res == lhs {
                let assign_trait = format_ident!("{trait_name}Assign");
                let assign_method = format_ident!("{method}_assign");
                out.extend(quote! {
                    impl ::core::ops::#assign_trait<#rhs> for #lhs {
                        fn #assign_method(&mut self, rhs: #rhs) {
                            *self = ::core::ops::#trait_ident::#method_ident(*self, rhs);
                        }
                    }
                });
            }
        }

        for ScaleRule { vec, scalar } in &self.scales {
            out.extend(quote! {
                impl ::core::ops::Mul<#scalar> for #vec {
                    type Output = Self;
                    fn mul(self, s: #scalar) -> Self {
                        Self::new(self.x * s, self.y * s, self.z * s)
                    }
                }

                impl ::core::ops::Mul<#vec> for #scalar {
                    type Output = #vec;
                    fn mul(self, v: #vec) -> #vec {
                        v * self
                    }
                }

                impl ::core::ops::Div<#scalar> for #vec {
                    type Output = Self;
                    fn div(self, s: #scalar) -> Self {
                        Self::new(self.x / s, self.y / s, self.z / s)
                    }
                }

                impl ::core::ops::MulAssign<#scalar> for #vec {
                    fn mul_assign(&mut self, s: #scalar) {
                        *self = *self * s;
                    }
                }

                impl ::core::ops::DivAssign<#scalar> for #vec {
                    fn div_assign(&mut self, s: #scalar) {
                        *self = *self / s;
                    }
                }
            });
        }

        for group in &self.converts {
            for a in group {
                let elem_a = self.elem_of(a)?;
                for b in group {
                    if a == b {
                        continue;
                    }
                    let elem_b = self.elem_of(b)?;
                    // Same element type: exact. Cross-element: explicit `as`
                    // cast (float→int truncates toward zero, saturating).
                    let (cx, cy, cz) = if elem_a == elem_b {
                        (quote!(v.x), quote!(v.y), quote!(v.z))
                    } else {
                        (
                            quote!(v.x as #elem_b),
                            quote!(v.y as #elem_b),
                            quote!(v.z as #elem_b),
                        )
                    };
                    out.extend(quote! {
                        impl From<#a> for #b {
                            fn from(v: #a) -> Self {
                                Self::new(#cx, #cy, #cz)
                            }
                        }
                    });
                }
            }
        }

        Ok(out)
    }
}

fn lit_zero(elem: &Ident) -> proc_macro2::TokenStream {
    if elem == "f64" || elem == "f32" {
        let lit = syn::LitFloat::new(&format!("0.0{elem}"), Span::call_site());
        quote!(#lit)
    } else {
        let lit = syn::LitInt::new(&format!("0{elem}"), Span::call_site());
        quote!(#lit)
    }
}

fn lit_one(elem: &Ident) -> proc_macro2::TokenStream {
    if elem == "f64" || elem == "f32" {
        let lit = syn::LitFloat::new(&format!("1.0{elem}"), Span::call_site());
        quote!(#lit)
    } else {
        let lit = syn::LitInt::new(&format!("1{elem}"), Span::call_site());
        quote!(#lit)
    }
}

#[cfg(test)]
mod tests {
    use super::MainArgs;

    fn seed_of(args: &str) -> syn::Result<Option<i64>> {
        syn::parse_str::<MainArgs>(args).map(|a| a.random_seed)
    }

    #[test]
    fn no_arguments_means_no_seeding() {
        assert_eq!(seed_of("").expect("empty args parse"), None);
    }

    #[test]
    fn positive_and_negative_seeds() {
        assert_eq!(seed_of("random_seed = 20260726").unwrap(), Some(20_260_726));
        assert_eq!(seed_of("random_seed = -7").unwrap(), Some(-7));
        assert_eq!(seed_of("random_seed = 0").unwrap(), Some(0));
        // Underscores and suffixes are the literal's business, not ours.
        assert_eq!(
            seed_of("random_seed = 20_260_726").unwrap(),
            Some(20_260_726)
        );
        assert_eq!(seed_of("random_seed = 5i64").unwrap(), Some(5));
        // Hex literals parse too.
        assert_eq!(seed_of("random_seed = 0xFF").unwrap(), Some(255));
    }

    /// Regression: `i64::MIN`'s magnitude (9223372036854775808) does not fit in
    /// an `i64`, so parsing the magnitude as `i64` before applying the sign
    /// rejected a seed that is perfectly representable.
    #[test]
    fn the_most_negative_seed_is_accepted() {
        assert_eq!(
            seed_of("random_seed = -9223372036854775808").unwrap(),
            Some(i64::MIN)
        );
        // And its positive neighbour, the other boundary.
        assert_eq!(
            seed_of("random_seed = 9223372036854775807").unwrap(),
            Some(i64::MAX)
        );
    }

    #[test]
    fn seeds_outside_i64_are_rejected_with_the_range_in_the_message() {
        let too_big = seed_of("random_seed = 9223372036854775808").expect_err("must reject");
        assert!(
            too_big.to_string().contains("must fit in i64"),
            "unhelpful message: {too_big}"
        );
        let too_small = seed_of("random_seed = -9223372036854775809").expect_err("must reject");
        assert!(too_small.to_string().contains("must fit in i64"));
    }

    #[test]
    fn unknown_arguments_and_duplicates_are_rejected() {
        let unknown = seed_of("random_tomato = 1").expect_err("must reject");
        assert!(unknown.to_string().contains("random_seed"), "{unknown}");
        let dup = seed_of("random_seed = 1, random_seed = 2").expect_err("must reject");
        assert!(dup.to_string().contains("duplicate"), "{dup}");
    }
}
