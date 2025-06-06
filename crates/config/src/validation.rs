use std::str::FromStr;

use url::Url;

#[derive(Clone, Debug)]
pub struct ValidUrl(Url);

impl FromStr for ValidUrl {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ValidUrl(Url::parse(s)?))
    }
}

impl From<ValidUrl> for String {
    fn from(value: ValidUrl) -> Self {
        value.0.to_string()
    }
}
