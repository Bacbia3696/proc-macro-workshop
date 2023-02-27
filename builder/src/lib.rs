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
        quote! {
            #ident: Option<#ty>,
        }
    });
    let setters = fields.iter().map(|e| {
        let ident = &e.ident;
        let ty = &e.ty;
        quote! {
            fn #ident(&mut self, #ident: #ty) -> &mut Self {
                self.#ident = Some(#ident);
                self
            }
        }
    });
    let attr_opt_errs = fields.iter().map(|e| {
        let ident = &e.ident;
        quote! {
            #ident: self.#ident.clone().ok_or("missing attribute XXXX")?,
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
