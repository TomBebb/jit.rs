extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

use quote::Tokens;
use std::borrow::Cow;
use syn::{Body, Ty, Path};

#[proc_macro_derive(Compile)]
pub fn hello_world(input: TokenStream) -> TokenStream {
    // Construct a string representation of the type definition
    let s = input.to_string();
    
    // Parse the string representation
    let ast = syn::parse_derive_input(&s).unwrap();

    // Build the impl
    let gen = impl_compile(&ast);
    
    // Return the generated impl
    gen.parse().unwrap()
}

fn jit_type_of(ty: &Ty) -> Cow<str> {
    match *ty {
        Ty::Path(None, ref path @ Path { global: true , segments: _}) => {
            format!("<{:?} as jit::Compile>::get_type()", path).into()
        },
        _ => panic!("type {:?} has no LibJIT equivalent", ty)
    }
}

fn impl_compile(ast: &syn::DeriveInput) -> quote::Tokens {
    let name = &ast.ident;
    match ast.body {
        Body::Struct(ref variant) => {
            let mut type_args = Tokens::new();
            type_args.append("&[");
            for field in variant.fields() {
                type_args.append(jit_type_of(&field.ty));
            }
            type_args.append("]");
            let _struct_ty = quote!(jit::Type::new_struct($type_args));
            quote!{
                impl<'a> Compile<'a> for #name {
                    fn compile(self, func:&'a jit::UncompiledFunction) -> &'a jit::Val {
                        jit::Val::new(func, &Self::get_type())
                    }
                    fn get_type() -> CowType<'a> {
                        $_struct_ty
                    }
                }
            }
        },
        _ => panic!("#[derive(Compile)] is only defined for structs, not for enums!")

    }
}
