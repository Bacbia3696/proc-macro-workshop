use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    // dbg!(&input);
    let ident = &input.ident;
    let ident_lit = syn::LitStr::new(&ident.to_string(), proc_macro2::Span::call_site());
    let mut field_methods = vec![];

    let fields = get_fields(&input);
    // dbg!(&input.generics);
    // dbg!(&impl_generics, &ty_generics, &where_clause);

    // ident of type that included in PhantomData
    let mut phantom_types = vec![];

    for f in fields.iter() {
        if let syn::Type::Path(syn::TypePath {
            path: syn::Path { segments, .. },
            ..
        }) = &f.ty
        {
            let tp = segments.last().unwrap();
            if tp.ident == "PhantomData" {
                // dbg!(f);
                if let syn::PathArguments::AngleBracketed(arg) = &tp.arguments {
                    if let syn::GenericArgument::Type(syn::Type::Path(syn::TypePath {
                        path: syn::Path { segments, .. },
                        ..
                    })) = arg.args.first().unwrap()
                    {
                        phantom_types.push(&segments.first().unwrap().ident);
                    }
                }
            }
        }
        let meta = get_meta(f);
        // dbg!(&meta);
        let lit = syn::LitStr::new(
            &f.ident.as_ref().unwrap().to_string(),
            proc_macro2::Span::call_site(),
        );
        let ident = &f.ident;
        field_methods.push(if let Some(m) = meta {
            let lit_meta = get_lit_from_meta(&m);
            match lit_meta {
                Ok(lit_meta) => {
                    quote! {
                        field(#lit, &format_args!(#lit_meta, &self.#ident))
                    }
                }
                Err(_) => todo!(),
            }
        } else {
            quote! {
                field(#lit, &self.#ident)
            }
        })
    }

    let generics = add_trait_bounds(input.generics.clone(), phantom_types);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics ::std::fmt::Debug for #ident #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct(#ident_lit)
                    #(.#field_methods)*
                    .finish()
            }
        }
    }
    .into()
}

fn get_fields(e: &syn::DeriveInput) -> &syn::punctuated::Punctuated<syn::Field, syn::token::Comma> {
    if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
        ..
    }) = &e.data
    {
        named
    } else {
        unimplemented!()
    }
}

fn get_meta(f: &syn::Field) -> Option<syn::Meta> {
    f.attrs.get(0)?.parse_meta().ok()
}

fn get_lit_from_meta(m: &syn::Meta) -> Result<&syn::LitStr, Box<dyn std::error::Error>> {
    let err = Err("Wrong sytax");
    if let syn::Meta::NameValue(syn::MetaNameValue { path, lit, .. }) = m {
        if path.segments.first().unwrap().ident != "debug" {
            return err?;
        }
        if let syn::Lit::Str(lt) = lit {
            return Ok(lt);
        }
    }
    err?
}

// Add a bound `T: Debug` to every type parameter T.
fn add_trait_bounds(mut generics: syn::Generics, phantom_types: Vec<&syn::Ident>) -> syn::Generics {
    // dbg!(&generics);
    for param in &mut generics.params {
        if let syn::GenericParam::Type(tp) = param {
            if phantom_types.contains(&&tp.ident) {
                continue;
            }
        }
        if let syn::GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(syn::parse_quote!(std::fmt::Debug));
        }
    }
    generics
}
