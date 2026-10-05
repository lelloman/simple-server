use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, Lit, Meta, Token, parse_macro_input, punctuated::Punctuated};

fn entry(options: TokenStream, item: TokenStream, test: bool) -> TokenStream {
    let options = parse_macro_input!(options with Punctuated::<Meta, Token![,]>::parse_terminated);
    let mut function = parse_macro_input!(item as ItemFn);
    if function.sig.asyncness.take().is_none() {
        return syn::Error::new_spanned(&function.sig, "engine entry point must be async")
            .into_compile_error()
            .into();
    }
    if !function.sig.inputs.is_empty() {
        return syn::Error::new_spanned(
            &function.sig.inputs,
            "engine entry point takes no arguments",
        )
        .into_compile_error()
        .into();
    }
    let mut current_thread = test;
    let mut paused = false;
    let mut workers = None;
    for option in options {
        let Meta::NameValue(ref value) = option else {
            return syn::Error::new_spanned(option, "expected name = value")
                .into_compile_error()
                .into();
        };
        let syn::Expr::Lit(ref literal) = value.value else {
            return syn::Error::new_spanned(value, "expected a literal")
                .into_compile_error()
                .into();
        };
        match (&value.path, &literal.lit) {
            (path, Lit::Str(s)) if path.is_ident("flavor") && s.value() == "current_thread" => {
                current_thread = true
            }
            (path, Lit::Str(s)) if path.is_ident("flavor") && s.value() == "multi_thread" => {
                current_thread = false
            }
            (path, Lit::Bool(b)) if path.is_ident("start_paused") => paused = b.value,
            (path, Lit::Int(n)) if path.is_ident("worker_threads") => {
                match n.base10_parse::<usize>() {
                    Ok(n) if n > 0 => workers = Some(n),
                    _ => {
                        return syn::Error::new_spanned(n, "worker_threads must be positive")
                            .into_compile_error()
                            .into();
                    }
                }
            }
            _ => {
                return syn::Error::new_spanned(option, "unsupported engine entry-point option")
                    .into_compile_error()
                    .into();
            }
        }
    }
    if paused && !current_thread || current_thread && workers.is_some() {
        return syn::Error::new_spanned(
            &function.sig,
            "paused time requires current_thread; worker_threads requires multi_thread",
        )
        .into_compile_error()
        .into();
    }
    let builder = if current_thread {
        quote!(new_current_thread)
    } else {
        quote!(new_multi_thread)
    };
    let configure_workers = workers.map(|n| quote!(builder.worker_threads(#n);));
    let body = function.block;
    function.block = syn::parse_quote!({
        let mut builder = ::simple_server::runtime::Builder::#builder();
        builder.start_paused(#paused);
        #configure_workers
        let runtime = builder.build().expect("cannot create simple-server engine runtime");
        runtime.block_on(async #body)
    });
    let attr = test.then(|| quote!(#[::core::prelude::v1::test]));
    quote!(#attr #function).into()
}

#[proc_macro_attribute]
pub fn main(options: TokenStream, item: TokenStream) -> TokenStream {
    entry(options, item, false)
}

#[proc_macro_attribute]
pub fn test(options: TokenStream, item: TokenStream) -> TokenStream {
    entry(options, item, true)
}

#[proc_macro_derive(FromRow, attributes(simple_server))]
pub fn from_row(item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as syn::DeriveInput);
    let name = &item.ident;
    let syn::Data::Struct(data) = &item.data else {
        return syn::Error::new_spanned(&item, "FromRow requires a struct")
            .into_compile_error()
            .into();
    };
    let syn::Fields::Named(fields) = &data.fields else {
        return syn::Error::new_spanned(&item, "FromRow requires named fields")
            .into_compile_error()
            .into();
    };
    let mut assignments = Vec::new();
    for field in &fields.named {
        let field_name = field.ident.as_ref().unwrap();
        let mut column = field_name.to_string();
        for attr in &field.attrs {
            if attr.path().is_ident("simple_server")
                && let Err(error) = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename") {
                        column = meta.value()?.parse::<syn::LitStr>()?.value();
                        Ok(())
                    } else {
                        Err(meta.error("expected rename = \"column\""))
                    }
                })
            {
                return error.into_compile_error().into();
            }
        }
        assignments.push(quote!(#field_name: row.try_get(#column)?));
    }
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();
    quote!(impl #impl_generics ::simple_server::database::client::FromRow for #name #ty_generics #where_clause {
        fn from_row(row:&::simple_server::database::client::Row)->Result<Self,::simple_server::database::client::Error> {Ok(Self {#(#assignments),*})}
    }).into()
}
