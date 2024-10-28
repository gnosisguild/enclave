use alloy::primitives::Address;
use clap::builder::TypedValueParser;


#[derive(Clone)]
pub struct EthAddressParser;

impl TypedValueParser for EthAddressParser {
    type Value = Address;

    fn parse_ref(
        &self,
        cmd: &clap::Command,
        arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        let address_str = value.to_str().ok_or_else(|| {
            clap::Error::raw(
                clap::error::ErrorKind::InvalidUtf8,
                "Ethereum address must be valid UTF-8",
            )
        })?;

        // Remove '0x' prefix if present
        let cleaned_address = address_str.strip_prefix("0x").unwrap_or(address_str);

        // Validate address format
        if !cleaned_address.len() == 40 {
            return Err(clap::Error::raw(
                clap::error::ErrorKind::InvalidValue,
                "Ethereum address must be 40 hexadecimal characters",
            ));
        }

        // Parse the address using alloy
        Address::from_str(address_str).map_err(|_| {
            clap::Error::raw(
                clap::error::ErrorKind::InvalidValue,
                "Invalid Ethereum address format",
            )
        })
    }
}
