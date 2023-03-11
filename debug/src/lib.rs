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

    // ident of type that included in PhantomData
    let mut phantom_types = vec![];
    let mut ass_types = vec![];
    let mut gen_idents = vec![];
    input.generics.params.iter().for_each(|e| {
        if let syn::GenericParam::Type(syn::TypeParam { ident, .. }) = e {
            gen_idents.push(ident.clone());
        }
    });

    for f in fields.iter() {
        if let Some(syn::Type::Path(syn::TypePath {
            path: syn::Path { segments, .. },
            ..
        })) = get_inner(Some("PhantomData"), &f.ty)
        {
            phantom_types.push(&segments[0].ident);
        }
        if let Some(tt) = get_assosiate_type(&f.ty, &gen_idents) {
            ass_types.push(tt);
        };
        let meta = get_meta(&f.attrs);
        // dbg!(&meta);
        let lit = syn::LitStr::new(
            &f.ident.as_ref().unwrap().to_string(),
            proc_macro2::Span::call_site(),
        );
        let field_ident = &f.ident;
        field_methods.push(if let Some(m) = meta {
            let lit_meta = get_lit_from_meta(&m);
            match lit_meta {
                Ok(lit_meta) => {
                    quote! {
                        field(#lit, &format_args!(#lit_meta, &self.#field_ident))
                    }
                }
                Err(_) => todo!("Handle case wrong attr name!"),
            }
        } else {
            quote! {
                field(#lit, &self.#field_ident)
            }
        })
    }

    let mut generics = input.generics.clone();
    let mut where_statement: Vec<_> = ass_types
        .iter()
        .map(|e| {
            quote! {
                #e : ::std::fmt::Debug,
            }
        })
        .collect();
    // dbg!(&ass_types);
    if let Some(meta) = get_meta(&input.attrs) {
        let lit = get_lit_from_meta(&meta).unwrap();
        let val = lit.value();
        let w: syn::WherePredicate =  syn::parse_str(&val).unwrap();
        where_statement.clear();
        where_statement.push(quote! {#w});
    } else {
        generics = add_trait_bounds(generics, &phantom_types, &ass_types);
    }

    let (impl_generics, ty_generics, _) = generics.split_for_impl();

    // dbg!(&where_statement);

    quote! {
        impl #impl_generics ::std::fmt::Debug for #ident #ty_generics
        where #(#where_statement)*
        {
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

fn get_meta(attrs: &[syn::Attribute]) -> Option<syn::Meta> {
    attrs.get(0)?.parse_meta().ok()
}

fn get_lit_from_meta(m: &syn::Meta) -> Result<&syn::LitStr, Box<dyn std::error::Error>> {
    let err = Err("Wrong sytax");
    match m {
        syn::Meta::NameValue(syn::MetaNameValue { path, lit, .. }) => {
            if path.segments[0].ident != "debug" {
                return err?;
            }
            if let syn::Lit::Str(lt) = lit {
                return Ok(lt);
            }
        }
        syn::Meta::List(syn::MetaList {
            path: syn::Path { segments, .. },
            nested,
            ..
        }) => {
            if segments[0].ident == "debug" {
                if let syn::NestedMeta::Meta(syn::Meta::NameValue(syn::MetaNameValue {
                    lit: syn::Lit::Str(str),
                    ..
                })) = &nested[0]
                {
                    return Ok(str);
                }
            }
        }
        syn::Meta::Path(_) => todo!(),
    }
    err?
}

// Add a bound `T: Debug` to every type parameter T.
// exclude bound in PhantomData
fn add_trait_bounds(
    mut generics: syn::Generics,
    phantom_types: &[&syn::Ident],
    ass_types: &[&syn::Path],
) -> syn::Generics {
    let ass_idents: Vec<_> = ass_types.iter().map(|e| &e.segments[0].ident).collect();

    for param in &mut generics.params {
        if let syn::GenericParam::Type(tp) = param {
            if phantom_types.contains(&&tp.ident) || ass_idents.contains(&&tp.ident) {
                continue;
            }
        }
        if let syn::GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(syn::parse_quote!(std::fmt::Debug));
        }
    }
    generics
}

fn get_inner<'a>(wrapper: Option<&str>, ty: &'a syn::Type) -> Option<&'a syn::Type> {
    if let syn::Type::Path(syn::TypePath {
        path: syn::Path { segments, .. },
        ..
    }) = ty
    {
        let v: Vec<_> = segments.iter().map(|e| e.ident.to_string()).collect();
        if wrapper.is_none() || v.last()? == wrapper.unwrap() {
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

// check if first segment of ty in included in Vec of trait
fn get_assosiate_type<'a>(
    ty: &'a syn::Type,
    gen_idents: &Vec<syn::Ident>,
) -> Option<&'a syn::Path> {
    if let Some(inner) = get_inner(None, ty) {
        return get_assosiate_type(inner, gen_idents);
    };
    if let syn::Type::Path(syn::TypePath { path, .. }) = ty {
        if path.segments.len() > 1 && gen_idents.contains(&path.segments[0].ident) {
            return Some(path);
        }
    }
    None
}
