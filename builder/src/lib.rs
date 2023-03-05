use proc_macro::TokenStream;
use quote::quote;
use syn::{DataStruct, DeriveInput, Field, Fields, FieldsNamed, Token};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let builder_name = quote::format_ident!("{}Builder", input.ident);
    let fields = get_all_fields(input.data.clone());

    let mut builder_values: Vec<proc_macro2::TokenStream> = vec![];
    let mut ty_fields: Vec<proc_macro2::TokenStream> = vec![];
    let mut setters: Vec<proc_macro2::TokenStream> = vec![];
    let mut build_values: Vec<proc_macro2::TokenStream> = vec![];

    fields.iter().for_each(|e| {
        let ident = &e.ident;
        let ty = &e.ty;
        let inner_option = get_inner(ty, "Option");
        let inner_vec = get_inner(ty, "Vec");
        let attr = &get_attr(e);
        builder_values.push(
            // if have attr, it will be vec
            if attr.is_some() {
                quote! {
                    #ident: vec![],
                }
            } else {
                quote! {
                    #ident: None,
                }
            },
        );
        ty_fields.push(if inner_option.is_some() || attr.is_some() {
            quote! {
                #ident: #ty,
            }
        } else {
            quote! {
                #ident: Option<#ty>,
            }
        });
        let mut ty_option = ty;
        if let Some(ref inner) = inner_option {
            ty_option = inner;
        }
        setters.push(if let Some(lit) = attr {
            let mut lit = lit.to_string();
            lit.pop();
            lit.remove(0);
            let lit = quote::format_ident!("{lit}");
            quote! {
                fn #lit(&mut self, val: #inner_vec) -> &mut Self {
                    self.#ident.push(val);
                    self
                }
            }
        } else {
            quote! {
                fn #ident(&mut self, #ident: #ty_option) -> &mut Self {
                    self.#ident = Some(#ident);
                    self
                }
            }
        });
        build_values.push(if inner_option.is_some() || attr.is_some() {
            quote! {
                #ident: self.#ident.clone(),
            }
        } else {
            let lit = syn::LitStr::new(
                &format!("missing attribute {}", ident.clone().unwrap()),
                proc_macro2::Span::call_site(),
            );
            quote! {
                #ident: self.#ident.clone().ok_or(#lit)?,
            }
        })
    });

    let build_method = quote! {
        pub fn build(&mut self) -> Result<#name, Box<dyn std::error::Error>> {
            Ok(#name {
                #(#build_values)*
            })
        }
    };

    quote! {
        pub struct #builder_name {
            #(#ty_fields)*
        }

        impl #builder_name {
            #(#setters)*
            // #(#setters_with_attrs)*
            #build_method
        }

        impl Command {
            pub fn builder() -> #builder_name {
                CommandBuilder {
                    #(#builder_values)*
                }
            }
        }
    }
    .into()
}

// extract struct data
fn get_all_fields(data: syn::Data) -> syn::punctuated::Punctuated<Field, Token![,]> {
    if let syn::Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { named, .. }),
        ..
    }) = data
    {
        return named;
    };
    unimplemented!()
}

fn get_inner(ty: &syn::Type, name: &str) -> Option<syn::Type> {
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = ty
    {
        let v: Vec<_> = segments.iter().map(|e| e.ident.to_string()).collect();
        if v.last()? == name {
            if let syn::PathSegment {
                arguments:
                    syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                        args, ..
                    }),
                ..
            } = segments.last()?
            {
                if let syn::GenericArgument::Type(tp) = args.first()? {
                    return Some(tp.clone());
                }
            }
        }
    }
    None
}

// get first attribute if exist
fn get_attr(e: &syn::Field) -> Option<proc_macro2::TokenTree> {
    let syn::Attribute { tokens, .. } = &e.attrs.get(0)?;
    if let proc_macro2::TokenTree::Group(group) = tokens.clone().into_iter().next()? {
        let attrs: Vec<_> = group.stream().into_iter().collect();
        if attrs[0].to_string() != "each" {
            panic!("You are dead wrong");
        }
        return Some(attrs[2].clone());
    }
    None
}
