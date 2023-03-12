use proc_macro::TokenStream;

#[derive(Debug)]
struct Seq {
    name: syn::Ident,
    range: std::ops::Range<u64>,
    body: proc_macro2::TokenStream,
    repeated_section: bool,
}

impl syn::parse::Parse for Seq {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: syn::Ident = input.parse()?;
        let _: syn::Token![in] = input.parse()?;
        let start: u64 = input.parse::<syn::LitInt>()?.base10_parse()?;
        let _: syn::Token![..] = input.parse()?;
        let end: u64 = input.parse::<syn::LitInt>()?.base10_parse()?;
        let content;
        syn::braced!(content in input);
        let body: proc_macro2::TokenStream = content.parse()?;
        dbg!(&body);

        Ok(Seq {
            name,
            range: std::ops::Range { start, end },
            body: proc_macro2::TokenStream::new(),
            repeated_section: true,
        })
    }
}

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let input: Seq = syn::parse_macro_input!(input as Seq);
    dbg!(input);

    quote::quote! {}.into()
}
