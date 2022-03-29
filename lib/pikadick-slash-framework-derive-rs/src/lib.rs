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

#[proc_macro_derive(FromOptions, attributes(pikadick_slash_framework))]
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

// Make the FromOptions impl
fn gen_from_options_impl(data: &Data) -> Result<TokenStream> {
    match data {
        Data::Struct(data) => gen_from_options_struct_impl(data),
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

fn gen_from_options_struct_impl(data: &syn::DataStruct) -> Result<TokenStream> {
    match &data.fields {
        Fields::Named(fields) => gen_from_options_named_fields_impl(fields),
        Fields::Unnamed(fields) => Err(Error::new(
            fields.span(),
            "unnamed fields are not supported",
        )),
        Fields::Unit => Err(Error::new(
            data.fields.span(),
            "unit structs are not supported",
        )),
    }
}

fn gen_from_options_named_fields_impl(fields: &syn::FieldsNamed) -> Result<TokenStream> {
    let fields = fields
        .named
        .iter()
        .map(|field| {
            let ident = field
                .ident
                .as_ref()
                .expect("named struct fields should have names for all fields");

            let mut maybe_rename = None;

            for attr in field
                .attrs
                .iter()
                .filter(|field| matches!(field.style, syn::AttrStyle::Outer))
            {
                let meta = attr.parse_meta()?;

                match meta {
                    syn::Meta::List(list) => {
                        if list
                            .path
                            .get_ident()
                            .map_or(false, |ident| ident == "pikadick_slash_framework")
                        {
                            for nested in list.nested.iter() {
                                match nested {
                                    syn::NestedMeta::Meta(meta) => match meta {
                                        syn::Meta::Path(path) => {
                                            let message =
                                                format!("unexpected meta path `{:?}`", path);
                                            return Err(Error::new(meta.span(), message));
                                        }
                                        syn::Meta::List(list) => {
                                            let message =
                                                format!("unexpected meta list `{:?}`", list);
                                            return Err(Error::new(meta.span(), message));
                                        }
                                        syn::Meta::NameValue(name_value) => {
                                            let ident =
                                                name_value.path.get_ident().ok_or_else(|| {
                                                    Error::new(
                                                        name_value.path.span(),
                                                        "expected ident",
                                                    )
                                                })?;

                                            if ident == "rename" {
                                                match &name_value.lit {
                                                    syn::Lit::Str(lit) => {
                                                        if maybe_rename.is_some() {
                                                            return Err(Error::new(
                                                                name_value.lit.span(),
                                                                "duplicate rename attribute",
                                                            ));
                                                        }

                                                        // TODO: Consider validating name

                                                        maybe_rename =
                                                            Some((lit.value(), lit.span()));
                                                    }
                                                    _ => {
                                                        return Err(Error::new(
                                                            name_value.lit.span(),
                                                            "unexpected literal type",
                                                        ));
                                                    }
                                                }
                                            } else {
                                                return Err(Error::new(
                                                    ident.span(),
                                                    format!("unexpected ident {:?}", ident),
                                                ));
                                            }
                                        }
                                    },
                                    syn::NestedMeta::Lit(lit) => {
                                        let message =
                                            format!("unexpected nested meta literal `{:?}`", lit);
                                        return Err(Error::new(nested.span(), message));
                                    }
                                }
                            }
                        }
                    }
                    syn::Meta::NameValue(_name_value) => {
                        // doc comments show up here.
                        // TODO: Consider doing something with them
                    }
                    _ => {}
                }
            }

            Ok(Field {
                ident,
                span: field.span(),
                ty: &field.ty,

                rename: maybe_rename,
            })
        })
        .collect::<Result<Vec<Field>>>()?;

    let optional_field_recurse = fields.iter().map(|field| {
        let name = &field.ident;
        quote_spanned! {field.span=>
            let mut #name = ::std::option::Option::None;
        }
    });

    let match_recurse = fields.iter().map(|field| {
        let name = &field.ident;
        let name_lit = LitStr::new(&name.to_string(), name.span());
        let ty = &field.ty;
        quote_spanned! {field.span=>
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

    let unwrap_field_recurse = fields.iter().map(|field| {
        let name = &field.ident;
        let name_lit = field.get_name_literal();
        let ty = &field.ty;
        quote_spanned! {field.span=>
            let #name = #name
                .or_else(<#ty as ::pikadick_slash_framework::FromOptionValue>::get_missing_default)
                .ok_or(::pikadick_slash_framework::ConvertError::MissingRequiredField {
                    name: #name_lit,
                    expected: <#ty as ::pikadick_slash_framework::FromOptionValue>::get_expected_data_type()
                })?;
        }
    });

    let recurse = fields.iter().map(|field| {
        let name = &field.ident;
        quote_spanned! {field.span=>
            #name,
        }
    });

    Ok(quote! {
        #(#optional_field_recurse)*

        for option in interaction.data.options.iter() {
            match option.name.as_str() {
                #(#match_recurse)*
                _ => {}
            }
        }

        #(#unwrap_field_recurse)*

        Ok(Self { #(#recurse)* })
    })
}

struct Field<'a> {
    ident: &'a proc_macro2::Ident,
    span: proc_macro2::Span,
    ty: &'a syn::Type,

    /// The renamed name of this field
    rename: Option<(String, proc_macro2::Span)>,
}

impl Field<'_> {
    /// Get the string literal name of this field.
    ///
    /// This will take into account field renames
    fn get_name_literal(&self) -> LitStr {
        match &self.rename {
            Some((name, span)) => LitStr::new(name, *span),
            None => LitStr::new(&self.ident.to_string(), self.ident.span()),
        }
    }
}
