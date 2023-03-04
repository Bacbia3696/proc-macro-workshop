use proc_macro::TokenStream;
use quote::quote;
use syn::{DataStruct, DeriveInput, Field, Fields, FieldsNamed, Token};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let builder_name = quote::format_ident!("{}Builder", input.ident);
    let fields = get_all_fields(input.data.clone());

    let mut attr_none: Vec<proc_macro2::TokenStream> = vec![];
    let mut attr_opts: Vec<proc_macro2::TokenStream> = vec![];
    let mut setters: Vec<proc_macro2::TokenStream> = vec![];
    let mut attr_opt_errs: Vec<proc_macro2::TokenStream> = vec![];

    fields.iter().for_each(|e| {
        let ident = &e.ident;
        let ty = &e.ty;
        let inner_option = get_inner(ty, "Option");
        // attr_none
        attr_none.push(quote! {
            #ident: None,
        });
        // attr_opts
        attr_opts.push(if inner_option.is_none() {
            quote! {
                #ident: Option<#ty>,
            }
        } else {
            quote! {
                #ident: #ty,
            }
        });
        // setters
        let mut ty_option = ty;
        if let Some(ref inner) = inner_option {
            ty_option = inner;
        }
        setters.push(quote! {
            fn #ident(&mut self, #ident: #ty_option) -> &mut Self {
                self.#ident = Some(#ident);
                self
            }
        });
        // attr_opt_errs
        attr_opt_errs.push(if inner_option.is_some() {
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
                #(#attr_opt_errs)*
            })
        }
    };

    // let setters_with_attrs = fields.iter().map(|e| {
    //     let ident = &e.ident;
    //     let ty = &e.ty;
    //
    //     // NOTE: extract the value of attribute builder
    //     if let Some(lits) = get_attrs(e) {
    //         // check
    //         if let proc_macro2::TokenTree::Literal(ref lit) = lits[0] {
    //             if lit.to_string() != "each" {
    //                 panic!("you are wrong")
    //             }
    //         }
    //
    //         if let proc_macro2::TokenTree::Literal(ref lit) = lits[2] {
    //             let mut lit = lit.to_string();
    //             lit.pop();
    //             lit.remove(0);
    //             let lit = quote::format_ident!("{lit}");
    //             let inner = get_inner(ty, "Vec");
    //             return quote! {
    //                 fn #lit(&mut self, val: #inner) -> &mut Self {
    //                     self.#ident.push(val);
    //                     self
    //                 }
    //             };
    //         };
    //     }
    //     quote!()
    // });

    quote! {
        pub struct #builder_name {
            #(#attr_opts)*
        }

        impl #builder_name {
            #(#setters)*
            // #(#setters_with_attrs)*
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

fn get_attrs(e: &syn::Field) -> Option<Vec<proc_macro2::TokenTree>> {
    let syn::Attribute { tokens, .. } = &e.attrs.get(0)?;
    if let proc_macro2::TokenTree::Group(group) = tokens.clone().into_iter().next()? {
        return Some(group.stream().into_iter().collect());
    }
    None
}
