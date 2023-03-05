use proc_macro::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, DataStruct, DeriveInput, Field, Fields, FieldsNamed, Token};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let builder_name = quote::format_ident!("{}Builder", input.ident);
    let fields = get_all_fields(&input.data);

    let mut builder_values: Vec<proc_macro2::TokenStream> = vec![];
    let mut ty_fields: Vec<proc_macro2::TokenStream> = vec![];
    let mut setters: Vec<proc_macro2::TokenStream> = vec![];
    let mut build_values: Vec<proc_macro2::TokenStream> = vec![];

    for e in fields.iter() {
        let ident = &e.ident;
        let ty = &e.ty;
        let inner_option = get_inner("Option", ty);
        let inner_vec = get_inner("Vec", ty);
        let meta = get_meta(e);

        builder_values.push(
            // if have attr, it will be vec
            if meta.is_some() {
                quote! {
                    #ident: vec![],
                }
            } else {
                quote! {
                    #ident: ::std::option::Option::None,
                }
            },
        );
        ty_fields.push(if inner_option.is_some() || meta.is_some() {
            quote! {
                #ident: #ty,
            }
        } else {
            quote! {
                #ident: ::std::option::Option<#ty>,
            }
        });
        let mut ty_option = ty;
        if let Some(inner) = inner_option {
            ty_option = inner;
        }
        setters.push(if let Some(ref m) = meta {
            match get_lit_from_meta(m) {
                Ok(lit) => {
                    let lit_ident = quote::format_ident!("{}", lit.value());
                    quote! {
                        fn #lit_ident(&mut self, val: #inner_vec) -> &mut Self {
                            self.#ident.push(val);
                            self
                        }
                    }
                }
                Err(err) => {
                    return syn::Error::new(meta.span(), err.to_string())
                        .into_compile_error()
                        .into();
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
        build_values.push(if inner_option.is_some() || meta.is_some() {
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
    }

    let build_method = quote! {
        pub fn build(&mut self) -> ::std::result::Result<#name, ::std::boxed::Box<dyn ::std::error::Error>> {
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
fn get_all_fields(data: &syn::Data) -> &syn::punctuated::Punctuated<Field, Token![,]> {
    if let syn::Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { named, .. }),
        ..
    }) = data
    {
        return named;
    };
    unimplemented!()
}

fn get_inner<'a>(wrapper: &str, ty: &'a syn::Type) -> Option<&'a syn::Type> {
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = ty
    {
        let v: Vec<_> = segments.iter().map(|e| e.ident.to_string()).collect();
        if v.last()? == wrapper {
            if let syn::PathSegment {
                arguments:
                    syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                        args, ..
                    }),
                ..
            } = segments.last()?
            {
                if let syn::GenericArgument::Type(tp) = args.first()? {
                    return Some(tp);
                }
            }
        }
    }
    None
}

fn get_meta(e: &syn::Field) -> Option<syn::Meta> {
    e.attrs.get(0)?.parse_meta().ok()
}

fn get_lit_from_meta(m: &syn::Meta) -> Result<&syn::LitStr, Box<dyn std::error::Error>> {
    let err = Err("expected `builder(each = \"...\")`");
    if let syn::Meta::List(syn::MetaList { path, nested, .. }) = m {
        if path.segments.first().unwrap().ident != "builder" {
            return err?;
        }
        if let syn::NestedMeta::Meta(syn::Meta::NameValue(syn::MetaNameValue {
            path, lit, ..
        })) = &nested[0]
        {
            if path.segments.first().unwrap().ident != "each" {
                return err?;
            }
            if let syn::Lit::Str(ls) = lit {
                return Ok(ls);
            } else {
                return err?;
            }
        }
    }
    err?
}
