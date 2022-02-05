use proc_macro2::{Ident, TokenStream};
use quote::quote;
use quote::ToTokens;
use syn::{parse_macro_input, Attribute, Fields, ItemStruct};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

enum BinElAttribute {
    ConvertWith(TokenStream),
    Skip,
    Name(TokenStream),
}

impl BinElAttribute {
    fn filter_map_iter<'a, I: IntoIterator<Item = &'a Attribute> + 'a>(
        iter: I,
    ) -> impl Iterator<Item = Self> + 'a {
        iter.into_iter().filter_map(|attr: &Attribute| {
            match attr.path.get_ident()?.to_string().as_str() {
                "bin_el_skip" => Some(BinElAttribute::Skip),
                "name" => Some(BinElAttribute::Name(attr.tokens.clone())),
                "convert_with" => Some(BinElAttribute::ConvertWith(attr.tokens.clone())),
                s => panic!("unrecognized attribute \"{}\"", s),
            }
        })
    }
}

#[proc_macro_derive(
    TryFromBinEl,
    attributes(bin_el_skip, convert_with, name)
)]
pub fn try_from_bin_el(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let mut name = None;
    let mut convert_with = quote! {DefaultConverter};
    for attr in BinElAttribute::filter_map_iter(&input.attrs) {
        match attr {
            BinElAttribute::Name(n) => name = Some(n),
            BinElAttribute::ConvertWith(c) => convert_with = c,
            BinElAttribute::Skip => todo!("Write fail message for skip"),
        }
    }
    let mut fields = Vec::new();
    let mut field_values = Vec::new();

    match &input.fields {
        Fields::Named(named) => {
            for field in &named.named {
                let ident: &Ident = field.ident.as_ref().unwrap();
                let name = ident.to_string();
                let mut name = quote! {#name};
                let mut skip = false;
                let mut convert_with = convert_with.clone();
                for attr in BinElAttribute::filter_map_iter(&field.attrs) {
                    match attr {
                        BinElAttribute::Skip => skip = true,
                        BinElAttribute::Name(n) => name = n,
                        BinElAttribute::ConvertWith(c) => convert_with = c,
                    }
                }
                fields.push(ident);

                if skip {
                    field_values.push(quote! {
                        Default::default()
                    });
                } else {
                    if let Some(converter) = Some(convert_with) {
                        field_values.push(quote! {
                            <#converter>::from_bin_el(elem, #name)?
                        })
                    } else {
                        field_values.push(quote! {
                            TryFromBinEl::try_from_bin_el(get_nested_child(elem, #name).ok_or_else(|| CelesteMapError::missing_child(&elem.name, #name))?)?
                        });
                    }
                }
            }
        }
        _ => todo!(),
    }

    let assertion = name.map(|name| {
        quote! {
            if (elem.name != #name) {
                return Err(CelesteMapError {
                    kind: CelesteMapErrorType::ParseError,
                    description: format!("Expected {} element, found {}", #name, elem.name),
                });
            }
        }
    });

    let ident: syn::Ident = input.ident.clone();

    <syn::Ident as quote::ToTokens>::to_token_stream(&ident);

    let ident = proc_macro2::TokenStream::from(TokenStream::from(ident.into_token_stream()));

    let impl_ = quote! {
        impl crate::from_binel::TryFromBinEl for #ident {
            fn try_from_bin_el(elem: &BinEl) -> Result<Self, CelesteMapError> {
                #assertion
                #(let #fields = #field_values;)*

                Ok(Self {
                    #(#fields,)*
                })
            }
        }
    };

    proc_macro::TokenStream::from(impl_.into_token_stream())
}
