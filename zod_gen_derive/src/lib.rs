extern crate proc_macro;

use heck::{
    ToKebabCase, ToLowerCamelCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase,
    ToUpperCamelCase,
};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Fields, LitStr};

fn find_serde_rename(attrs: &[Attribute]) -> Option<String> {
    let mut result = None;
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value: LitStr = meta.value()?.parse()?;
                result = Some(value.value());
            }
            Ok(())
        });
        if result.is_some() {
            break;
        }
    }
    result
}

fn find_serde_rename_all(attrs: &[Attribute]) -> Option<String> {
    let mut result = None;
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename_all") {
                let value: LitStr = meta.value()?.parse()?;
                result = Some(value.value());
            }
            Ok(())
        });
        if result.is_some() {
            break;
        }
    }
    result
}

fn apply_rename_rule(name: &str, rule: &str) -> Option<String> {
    let renamed = match rule {
        "camelCase" => name.to_lower_camel_case(),
        "PascalCase" => name.to_upper_camel_case(),
        "snake_case" => name.to_snake_case(),
        "SCREAMING_SNAKE_CASE" => name.to_shouty_snake_case(),
        "kebab-case" => name.to_kebab_case(),
        "SCREAMING-KEBAB-CASE" => name.to_shouty_kebab_case(),
        "lowercase" => name.to_ascii_lowercase(),
        "UPPERCASE" => name.to_ascii_uppercase(),
        _ => return None,
    };
    Some(renamed)
}

/// Derive macro for ZodSchema
#[proc_macro_derive(ZodSchema)]
pub fn derive_zod_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let container_rename_all = find_serde_rename_all(&input.attrs);

    let expanded = match input.data {
        Data::Struct(data_struct) => match data_struct.fields {
            Fields::Named(fields_named) => {
                let fields = fields_named.named.iter().map(|f| {
                    let ident = f.ident.as_ref().unwrap();
                    let ident_string = ident.to_string();
                    let field_ident = find_serde_rename(&f.attrs)
                        .or_else(|| {
                            container_rename_all
                                .as_deref()
                                .and_then(|rule| apply_rename_rule(&ident_string, rule))
                        })
                        .unwrap_or(ident_string);
                    let field_name = LitStr::new(&field_ident, ident.span());
                    let ty = &f.ty;
                    quote! { (#field_name, <#ty as zod_gen::ZodSchema>::zod_schema().as_str()) }
                });
                quote! {
                    impl zod_gen::ZodSchema for #name {
                        fn zod_schema() -> String {
                            zod_gen::zod_object(&[#(#fields),*])
                        }
                    }
                }
            }
            _ => panic!("ZodSchema derive only supports structs with named fields"),
        },
        Data::Enum(data_enum) => {
            let variants = data_enum.variants.iter().map(|v| {
                let ident_string = v.ident.to_string();
                let renamed_value = find_serde_rename(&v.attrs)
                    .or_else(|| {
                        container_rename_all
                            .as_deref()
                            .and_then(|rule| apply_rename_rule(&ident_string, rule))
                    })
                    .unwrap_or(ident_string);
                let var_name = LitStr::new(&renamed_value, v.ident.span());
                quote! { #var_name }
            });
            quote! {
                impl zod_gen::ZodSchema for #name {
                    fn zod_schema() -> String {
                        zod_gen::zod_enum(&[#(#variants),*])
                    }
                }
            }
        }
        _ => panic!("ZodSchema derive only supports structs and enums"),
    };

    TokenStream::from(expanded)
}
