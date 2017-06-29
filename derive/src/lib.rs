// safesec/safesec-derive/src/lib.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

// ===========================================================================
// Externs
// ===========================================================================


// Stdlib externs
extern crate proc_macro;

// Third-party externs
extern crate num;

extern crate syn;

#[macro_use]
extern crate quote;

// Local externs


// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports
use proc_macro::TokenStream;

// Third-party imports
use num::ToPrimitive;

// Local imports


// ===========================================================================
//
// ===========================================================================


#[proc_macro_derive(CodeConvert)]
pub fn code_convert(input: TokenStream) -> TokenStream {
    // Construct string repr of type definition
    let s = input.to_string();

    // Parse string
    let ast = syn::parse_derive_input(&s).unwrap();

    // Build the impl
    let gen = impl_code_convert(&ast);

    // Return generated impl
    gen.parse().unwrap()
}


struct Literal<'a> {
    num: &'a syn::Lit
}


impl<'a> From<&'a syn::Lit> for Literal<'a> {
    fn from(num: &'a syn::Lit) -> Self {
        Self { num: num }
    }
}


impl<'a> ToPrimitive for Literal<'a> {
    fn to_i64(&self) -> Option<i64> {
        match self.num {
            &syn::Lit::Int(num, _) => Some(num as i64),
            _ => None
        }
    }

    fn to_u64(&self) -> Option<u64> {
        match self.num {
            &syn::Lit::Int(num, _) => Some(num),
            _ => None
        }
    }
}


fn impl_code_convert(ast: &syn::MacroInput) -> quote::Tokens {
    if let syn::Body::Enum(ref body) = ast.body {

        let name = &ast.ident;
        let mut num = 0;
        let cases: Vec<_> = body.iter().map(|case| {
            // Panic if the variant is a struct or tuple
            if let syn::VariantData::Unit = case.data {
                // Create variant identifier
                let variant = &case.ident;
                let ident = quote! { #name::#variant };

                // If literal number assigned to variant, assign to num
                if let Some(ref d) = case.discriminant {
                    if let &syn::ConstExpr::Lit(ref l) = d {
                        let lit = Literal::from(l);
                        num = match lit.to_u8() {
                            None =>  panic!("#[derive(CodeConvert)] only \
                                            supports mapping to u8"),
                            Some(v) => v
                        };
                    } else {
                        panic!("#[derive(CodeConvert)] only supports literals")
                    }
                }
                let ret = quote! { #num => Ok(#ident) };
                num += 1;
                ret
            } else {
                panic!("#[derive(CodeConvert)] currently does not support \
                       tuple or struct variants");
            }
        }).collect();

        quote! {
            impl CodeConvert<#name> for #name {
                fn from_number(num: u8) -> Result<#name> {
                    match num {
                        #(#cases),* ,
                        _ => Err(Error::new(GeneralError::InvalidValue, num.to_string()))
                    }
                }

                fn to_number(&self) -> u8 {
                    self.clone() as u8
                }
            }
        }
    } else {
        panic!("#[derive(CodeConvert)] is only defined for enums not structs");
    }
}


// ===========================================================================
// Tests
// ===========================================================================


// #[cfg(test)]
// mod tests {
//     #[test]
//     fn it_works() {
//     }
// }


// ===========================================================================
//
// ===========================================================================
