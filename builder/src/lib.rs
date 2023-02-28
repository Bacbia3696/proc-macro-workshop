use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{DataStruct, DeriveInput, Field, Fields, FieldsNamed, Token};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let builder_name = Ident::new(&format!("{}Builder", input.ident), Span::call_site());
    let fields = get_all_fields(input.data.clone());

    let attr_none = fields.iter().map(|e| {
        let ident = &e.ident;
        quote! {
            #ident: None,
        }
    });
    let attr_opts = fields.iter().map(|e| {
        let ident = &e.ident;
        let ty = &e.ty;
        if get_inner_option(ty).is_none() {
            quote! {
                #ident: Option<#ty>,
            }
        } else {
            quote! {
                #ident: #ty,
            }
        }
    });
    let setters = fields.iter().map(|e| {
        let ident = &e.ident;
        let ty = &e.ty;
        let inner = get_inner_option(ty);
        if let Some(inner) = inner {
            quote! {
                fn #ident(&mut self, #ident: #inner) -> &mut Self {
                    self.#ident = Some(#ident);
                    self
                }
            }
        } else {
            quote! {
                fn #ident(&mut self, #ident: #ty) -> &mut Self {
                    self.#ident = Some(#ident);
                    self
                }
            }
        }
    });
    let attr_opt_errs = fields.iter().map(|e| {
        let ident = &e.ident;
        let ty = &e.ty;
        let inner = get_inner_option(ty);
        if inner.is_some() {
            quote! {
                #ident: self.#ident.clone(),
            }
        } else {
            quote! {
                #ident: self.#ident.clone().ok_or("missing attribute XXXX")?,
            }
        }
    });
    let build_method = quote! {
    pub fn build(&mut self) -> Result<#name, Box<dyn std::error::Error>> {
        Ok(#name {
            #(#attr_opt_errs)*
        })
    }
    };

    quote! {
        pub struct #builder_name {
            #(#attr_opts)*
        }

        impl #builder_name {
            #(#setters)*
            #build_method
        }

        impl Command {
            pub fn builder() -> #builder_name {
                CommandBuilder {
                    #(#attr_none)*
                }
            }
        }
    }
    .into()
}

// extract struct data
fn get_all_fields(data: syn::Data) -> syn::punctuated::Punctuated<Field, Token![,]> {
    let syn::Data::Struct(DataStruct{fields: Fields::Named(FieldsNamed{named, ..}), ..}) = data else {
        unimplemented!()
    };
    named
}

fn get_inner_option(ty: &syn::Type) -> Option<syn::Type> {
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = ty
    {
        let v: Vec<_> = segments.iter().map(|e| e.ident.to_string()).collect();
        if v.last().unwrap() == "Option" {
            // get first element of
            if let syn::PathSegment {
                arguments:
                    syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                        args, ..
                    }),
                ..
            } = segments.iter().last().unwrap()
            {
                if let syn::GenericArgument::Type(tp) = args.first().unwrap() {
                    return Some(tp.clone());
                }
            }
        }
    }
    None
}
