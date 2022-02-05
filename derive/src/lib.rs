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
    Attributes,
    Children,
    ConvertWith(TokenStream),
    Default,
    Name(TokenStream),
    Optional,
    Skip,
}

impl BinElAttribute {
    fn filter_map_iter<'a, I: IntoIterator<Item = &'a Attribute> + 'a>(
        iter: I,
    ) -> impl Iterator<Item = Self> + 'a {
        iter.into_iter().filter_map(|attr: &Attribute| {
            match attr.path.get_ident()?.to_string().as_str() {
                "bin_el_skip" => Some(BinElAttribute::Skip),
                "children" => Some(BinElAttribute::Children),
                "convert_with" => Some(BinElAttribute::ConvertWith(attr.tokens.clone())),
                "default" => Some(BinElAttribute::Default),
                "name" => Some(BinElAttribute::Name(attr.tokens.clone())),
                "optional" => Some(BinElAttribute::Optional),
                "attributes" => Some(BinElAttribute::Attributes),
                s => panic!("unrecognized attribute \"{}\"", s),
            }
        })
    }
}

#[proc_macro_derive(
    TryFromBinEl,
    attributes(attributes, bin_el_skip, children, convert_with, default, name, optional)
)]
pub fn try_from_bin_el(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let mut name = None;
    let mut convert_with = quote! {DefaultConverter};
    for attr in BinElAttribute::filter_map_iter(&input.attrs) {
        match attr {
            BinElAttribute::Name(n) => name = Some(n),
            BinElAttribute::ConvertWith(c) => convert_with = c,
            _ => todo!(),
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
                let mut default = false;
                let mut children = false;
                let mut optional = false;
                let mut attributes = false;
                let mut convert_with = convert_with.clone();
                for attr in BinElAttribute::filter_map_iter(&field.attrs) {
                    match attr {
                        BinElAttribute::Skip => skip = true,
                        BinElAttribute::Name(n) => name = n,
                        BinElAttribute::ConvertWith(c) => convert_with = c,
                        BinElAttribute::Default => default = true,
                        BinElAttribute::Optional => optional = true,
                        BinElAttribute::Children => children = true,
                        BinElAttribute::Attributes => attributes = true,
                    }
                }
                fields.push(ident);

                field_values.push(if skip {
                    quote! {
                        Default::default()
                    }
                } else if children {
                    quote! {
                        Vec::try_from_bin_el(elem)?
                    }
                } else if default {
                    quote! {
                        <#convert_with>::from_bin_el_optional(elem, #name)?.unwrap_or_default()
                    }
                } else if attributes {
                    quote! {
                        elem
                            .attributes
                            .iter()
                            .filter(|kv| !fields_list.contains(&kv.0.as_str()))
                            .map(|(k, v)| (k.to_owned(), v.clone()))
                            .collect()
                    }
                } else if optional {
                    quote! {
                        <#convert_with>::from_bin_el_optional(elem, #name)?
                    }
                } else if name.is_empty() {
                    quote! {
                        (&elem.name).into()
                    }
                } else {
                    quote! {
                        <#convert_with>::from_bin_el(elem, #name)?
                    }
                });
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
                let fields_list = [#(stringify!(#fields)),*];
                #(let #fields = #field_values;)*

                Ok(Self {
                    #(#fields,)*
                })
            }
        }
    };

    proc_macro::TokenStream::from(impl_.into_token_stream())
}
