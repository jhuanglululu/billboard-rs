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
/// Emits the `_billboard_main` wasm export (runtime init + the user's fn as
/// task 0) and the `_billboard_abi` version-handshake export, so the plugin
/// can refuse modules built against a different ABI before running them.
#[proc_macro_attribute]
pub fn main(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = parse_macro_input!(item as ItemFn);
    let name = &func.sig.ident;
    if !func.sig.inputs.is_empty() || func.sig.asyncness.is_some() {
        return syn::Error::new_spanned(
            &func.sig,
            "#[billboard::main] requires a plain `fn name()` with no arguments",
        )
        .to_compile_error()
        .into();
    }
    quote! {
        #func

        #[unsafe(no_mangle)]
        pub extern "C" fn _billboard_main() {
            ::billboard::__rt::init();
            #name();
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn _billboard_abi() -> i32 {
            ::billboard::ABI_VERSION
        }
    }
    .into()
}

/// Generates the SDK's vector math types: structs, constructors, constants,
/// physics-typed operators, scalar scaling, and explicit `From` conversions.
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
                #[derive(Clone, Copy, Debug, Default, PartialEq)]
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
