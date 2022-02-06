use proc_macro2::TokenStream;
use quote::{
    quote,
    quote_spanned,
};
use syn::{
    parse_macro_input,
    spanned::Spanned,
    Data,
    DeriveInput,
    Error,
    Fields,
    LitStr,
    Result,
};

#[proc_macro_derive(FromOptions)]
pub fn derive_from_options(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let from_options_impl =
        gen_from_options_impl(&input.data).unwrap_or_else(Error::into_compile_error);

    let expanded = quote! {
        impl ::pikadick_slash_framework::FromOptions for #name {
            fn from_options(interaction: &::serenity::model::prelude::application_command::ApplicationCommandInteraction) -> Result<Self, ::pikadick_slash_framework::ConvertError> {
                #from_options_impl
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

fn gen_from_options_impl(data: &Data) -> Result<TokenStream> {
    match data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                let optional_field_recurse = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    quote_spanned! {f.span()=>
                        let mut #name = ::std::option::Option::None;
                    }
                });

                let recurse = fields.named.iter().map(|f| {
                    let name = &f
                        .ident
                        .as_ref()
                        .expect("named struct fields should have names for all fields");
                    let name_lit = LitStr::new(&name.to_string(), name.span());
                    let ty = &f.ty;
                    quote_spanned! {f.span()=>
                        #name_lit => {
                            #name = Some(
                                <#ty as ::pikadick_slash_framework::FromOptionValue>::from_option_value(
                                    #name_lit,
                                    option.resolved.as_ref()
                                )?
                            );
                        }
                    }
                });

                let unwrap_field_recurse = fields.named.iter().map(|f| {
                    let name = &f.ident.as_ref().expect("named struct fields should have names for all fields");
                    let name_lit = LitStr::new(&name.to_string(), name.span());
                    let ty = &f.ty;
                    quote_spanned! {f.span()=>
                        let #name = #name
                            .or_else(<#ty as ::pikadick_slash_framework::FromOptionValue>::get_missing_default)
                            .ok_or(ConvertError::MissingRequiredField {
                                name: #name_lit,
                                expected: <#ty as ::pikadick_slash_framework::FromOptionValue>::get_expected_data_type()
                            })?;
                    }
                });

                Ok(quote! {
                    #(#optional_field_recurse)*

                    for option in interaction.data.options.iter() {
                        match option.name.as_str() {
                            #(#recurse)*
                            _ => {}
                        }
                    }

                    #(#unwrap_field_recurse)*

                    Ok(Self { nsfw })
                })
            }
            Fields::Unnamed(fields) => Err(Error::new(
                fields.span(),
                "unnamed fields are not supported",
            )),
            Fields::Unit => Err(Error::new(
                data.fields.span(),
                "unit structs are not supported",
            )),
        },
        Data::Enum(data) => Err(Error::new(
            data.enum_token.span(),
            "enums are not supported",
        )),
        Data::Union(data) => Err(Error::new(
            data.union_token.span(),
            "unions are not supported",
        )),
    }
}
