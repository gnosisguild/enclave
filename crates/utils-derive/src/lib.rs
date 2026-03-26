// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Error, Fields};

/// Derives `Serialize` and `Deserialize` for types that implement `BytesSerde`.
///
/// - Human-readable formats (JSON, TOML): hex string with `0x` prefix
/// - Binary formats (bincode, postcard): raw bytes
///
/// The type must also implement `e3_utils::serde_bytes::AsBytesSerde`.
///
/// # Example
///
/// ```ignore
/// use e3_utils::BytesSerde;
/// use e3_utils::AsBytesSerde;
///
/// #[derive(BytesSerde)]
/// pub struct EventId(pub [u8; 32]);
///
/// impl AsBytesSerde for EventId {
///     fn as_bytes(&self) -> &[u8] { &self.0 }
///     fn try_from_bytes(bytes: Vec<u8>) -> Result<Self, String> {
///         Ok(EventId(bytes.try_into().map_err(|_| "requires 32 bytes".to_string())?))
///     }
/// }
/// ```
#[proc_macro_derive(BytesSerde)]
pub fn derive_bytes_serde(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Sanity check: must be a struct
    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {}
            _ => {
                return Error::new_spanned(
                    &input,
                    "BytesSerde can only be derived for newtype structs (e.g., `struct Foo(pub [u8; 32])`)",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return Error::new_spanned(&input, "BytesSerde can only be derived for structs")
                .to_compile_error()
                .into();
        }
    }

    let expanded = quote! {
        impl #impl_generics ::serde::Serialize for #name #ty_generics #where_clause {
            fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                use ::e3_utils::serde_bytes::AsBytesSerde;
                if serializer.is_human_readable() {
                    serializer.serialize_str(&::std::format!("0x{}", ::hex::encode(self.as_bytes())))
                } else {
                    serializer.serialize_bytes(self.as_bytes())
                }
            }
        }

        impl<'de> ::serde::Deserialize<'de> for #name #ty_generics #where_clause {
            fn deserialize<D>(deserializer: D) -> ::core::result::Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                use ::e3_utils::serde_bytes::AsBytesSerde;
                let bytes = if deserializer.is_human_readable() {
                    let s = <::std::string::String as ::serde::Deserialize>::deserialize(
                        deserializer,
                    )?;
                    let stripped = s.strip_prefix("0x").unwrap_or(&s);
                    ::hex::decode(stripped).map_err(::serde::de::Error::custom)?
                } else {
                    <::std::vec::Vec<u8> as ::serde::Deserialize>::deserialize(deserializer)?
                };
                Self::try_from_bytes(bytes).map_err(::serde::de::Error::custom)
            }
        }
    };

    expanded.into()
}
