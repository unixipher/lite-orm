use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, LitStr, parse_macro_input};

#[proc_macro_derive(Model, attributes(lite_orm))]
pub fn derive_model(input: TokenStream) -> TokenStream {
    match expand_model(parse_macro_input!(input as DeriveInput)) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn expand_model(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let (table, schema) = model_options(&input)?;
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    &input,
                    "Model requires a struct with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "Model can only be derived for structs",
            ));
        }
    };

    let mut names = Vec::new();
    let mut types = Vec::new();
    let mut column_names = Vec::new();
    let mut insert_names = Vec::new();
    let mut insert_fields = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().expect("named fields");
        let column_name = column_name(field)?;
        let skip_insert = has_flag(field, "auto_increment")? || has_flag(field, "skip_insert")?;
        names.push(field_name);
        types.push(&field.ty);
        column_names.push(column_name.clone());
        if !skip_insert {
            insert_names.push(column_name);
            insert_fields.push(field_name);
        }
    }

    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let indexes = 0..names.len();

    Ok(quote! {
        impl #impl_generics ::lite_orm::Model for #name #type_generics #where_clause {
            const TABLE: &'static str = #table;
            const SCHEMA: &'static str = #schema;
            const COLUMNS: &'static [&'static str] = &[#(#column_names),*];

            fn from_row(row: &::lite_orm::__private::tokio_postgres::Row)
                -> ::std::result::Result<Self, ::lite_orm::__private::tokio_postgres::Error>
            {
                Ok(Self { #(#names: row.try_get(#indexes)?),* })
            }

            fn insert_columns() -> &'static [&'static str] {
                &[#(#insert_names),*]
            }

            fn insert_values(&self) -> ::std::vec::Vec<::lite_orm::Value> {
                vec![#(::lite_orm::ToValue::to_value(&self.#insert_fields)),*]
            }
        }

        #[allow(non_upper_case_globals)]
        impl #impl_generics #name #type_generics #where_clause {
            #(pub const #names: ::lite_orm::Column<Self, #types> =
                ::lite_orm::Column::new(#column_names);)*
        }
    })
}

fn model_options(input: &DeriveInput) -> syn::Result<(String, String)> {
    let mut table = input.ident.to_string().to_snake_case();
    let mut schema = String::from("public");
    for attribute in &input.attrs {
        if attribute.path().is_ident("lite_orm") {
            attribute.parse_nested_meta(|meta| {
                if meta.path.is_ident("table") {
                    table = meta.value()?.parse::<LitStr>()?.value();
                    Ok(())
                } else if meta.path.is_ident("schema") {
                    schema = meta.value()?.parse::<LitStr>()?.value();
                    Ok(())
                } else {
                    Err(meta.error("supported model attributes: table = \"...\", schema = \"...\""))
                }
            })?;
        }
    }
    Ok((table, schema))
}

fn column_name(field: &syn::Field) -> syn::Result<String> {
    let mut result = field.ident.as_ref().expect("named field").to_string();
    for attribute in &field.attrs {
        if attribute.path().is_ident("lite_orm") {
            attribute.parse_nested_meta(|meta| {
                if meta.path.is_ident("column") {
                    result = meta.value()?.parse::<LitStr>()?.value();
                    Ok(())
                } else if meta.path.is_ident("primary_key")
                    || meta.path.is_ident("auto_increment")
                    || meta.path.is_ident("skip_insert")
                {
                    Ok(())
                } else {
                    Err(meta.error("unknown lite_orm field attribute"))
                }
            })?;
        }
    }
    Ok(result)
}

fn has_flag(field: &syn::Field, flag: &str) -> syn::Result<bool> {
    let mut found = false;
    for attribute in &field.attrs {
        if attribute.path().is_ident("lite_orm") {
            attribute.parse_nested_meta(|meta| {
                if meta.path.is_ident(flag) {
                    found = true;
                } else if meta.path.is_ident("column") {
                    let _ = meta.value()?.parse::<LitStr>()?;
                }
                Ok(())
            })?;
        }
    }
    Ok(found)
}
