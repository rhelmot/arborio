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
    Generate(TokenStream),
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
                "attributes" => Some(BinElAttribute::Attributes),
                "bin_el_skip" => Some(BinElAttribute::Skip),
                "children" => Some(BinElAttribute::Children),
                "convert_with" => Some(BinElAttribute::ConvertWith(attr.tokens.clone())),
                "default" => Some(BinElAttribute::Default),
                "generate" => Some(BinElAttribute::Generate(attr.tokens.clone())),
                "name" => Some(BinElAttribute::Name(attr.tokens.clone())),
                "optional" => Some(BinElAttribute::Optional),
                s => panic!("unrecognized attribute \"{}\"", s),
            }
        })
    }
}

#[proc_macro_derive(
    TryFromBinEl,
    attributes(attributes, bin_el_skip, children, convert_with, default, generate, name, optional)
)]
pub fn try_from_bin_el(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let mut struct_name = None;
    let mut convert_with = quote! {DefaultConverter};
    for attr in BinElAttribute::filter_map_iter(&input.attrs) {
        match attr {
            BinElAttribute::Name(n) => struct_name = Some(n),
            BinElAttribute::ConvertWith(c) => convert_with = c,
            _ => todo!(),
        }
    }
    let mut fields = Vec::new();
    let mut field_values = Vec::new();
    let mut into_values = Vec::new();
    let mut name_field = None;

    match &input.fields {
        Fields::Named(named) => {
            for field in &named.named {
                let ident: &Ident = field.ident.as_ref().unwrap();
                let name = ident.to_string();
                let type_ = &field.ty;
                let mut name = quote! {#name};
                let mut skip = false;
                let mut default = false;
                let mut children = false;
                let mut generate = TokenStream::new();
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
                        BinElAttribute::Generate(call) => generate = call,
                    }
                }
                fields.push(ident);
                
                if name.is_empty() {
                    name_field = Some(ident);
                }

                into_values.push(if skip {
                    None
                } else if !generate.is_empty() {
                    None
                } else if children {
                    Some(quote! {
                        let mut serialized_vec = <#convert_with>::serialize(&self.#ident);
                        for child in serialized_vec.drain() {
                            binel.insert(child);
                        }
                    })
                } else if default {
                    Some(quote! {
                        if self.#ident != <#type_>::default() {
                            let serialized_field = <#convert_with>::serialize(&self.#ident);
                            GetAttrOrChild::nested_apply_attr_or_child(&mut binel, #name, serialized_field);
                        }
                    })
                } else if attributes {
                    Some(quote!{
                        for (k, v) in self.#ident.clone().into_iter() {
                            binel.attributes.insert(k, v.into());
                        }
                    })
                } else if optional {
                    Some(quote! {
                        if let Some(ref field) = self.#ident {
                            let serialized_field = <#convert_with>::serialize(field);
                            GetAttrOrChild::nested_apply_attr_or_child(&mut binel, #name, serialized_field);
                        }
                    })
                } else if name.is_empty() {
                    None
                } else {
                    Some(quote! {
                        let serialized_field = <#convert_with>::serialize(&self.#ident);
                        GetAttrOrChild::nested_apply_attr_or_child(&mut binel, #name, serialized_field);
                    })
                });

                field_values.push(if skip {
                    quote! {
                        Default::default()
                    }
                } else if !generate.is_empty() {
                    generate
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
                            .map(|(k, v)| (k.to_owned(), v.clone().into()))
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

    let assertion = struct_name.as_ref().map(|name| {
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
    
    let name = if let Some(name_field) = name_field {
        quote!{&self.#name_field}
    } else if let Some(name) = struct_name {
        name
    } else {
        quote!{stringify!(#ident)}
    };

    let impl_ = quote! {
        impl crate::from_binel::TryFromBinEl for #ident {
            fn try_from_bin_el(elem: &BinEl) -> Result<Self, CelesteMapError> {
                #assertion
                let fields_list = [#(stringify!(#fields)),*];
                #(let #fields = #field_values;)*

                let struct_ = Self {
                    #(#fields,)*
                };

                // let reserialized = struct_.to_binel();
                // assert!(crate::from_binel::bin_el_fuzzy_equal(elem, &reserialized), "{:?} != {:?}", elem, &reserialized);

                Ok(struct_)
            }
            fn to_binel(&self) -> BinEl {
                let mut binel = BinEl::new(#name);

                #(#into_values)*

                binel
            }
        }
    };

    proc_macro::TokenStream::from(impl_.into_token_stream())
}
