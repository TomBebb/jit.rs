extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

use quote::Tokens;
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

fn jit_type_of(ty: &Ty) -> Tokens {
    match *ty {
        Ty::Path(None, ref path @ Path { global: _ , segments: _}) => {
            quote!(&<#path as jit::Compile>::get_type())
        },
        _ => panic!("type {:?} has no LibJIT equivalent", ty)
    }
}

fn impl_compile(ast: &syn::DeriveInput) -> quote::Tokens {
    let name = &ast.ident;
    match ast.body {
        Body::Struct(ref variant) => {
            let mut type_args = Tokens::new();
            let mut names = Tokens::new();
            type_args.append("&[");
            names.append("&[");
            for field in variant.fields() {
                type_args.append(jit_type_of(&field.ty));
                type_args.append(", ");
                if let Some(ref id) = field.ident {
                    names.append(format!("{:?}", id.as_ref()));
                    names.append(", ");
                }
            }
            type_args.append("]");
            names.append("]");
            quote!{
                impl<'a> Compile<'a> for #name {
                    fn compile(self, func:&'a jit::UncompiledFunction) -> &'a jit::Val {
                        jit::Val::new(func, &Self::get_type())
                    }
                    fn get_type() -> CowType<'a> {
                        let mut ty = jit::Type::new_struct(#type_args);
                        ty.set_names(#names);
                        ty.into()
                    }
                }
            }
        },
        _ => panic!("#[derive(Compile)] is only defined for structs, not for enums!")

    }
}
