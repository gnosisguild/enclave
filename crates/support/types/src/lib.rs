use anyhow::Result;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ComputeResponse {
    pub ciphertext: Vec<u8>,
    pub proof: Vec<u8>,
}

#[derive(Debug, Deserialize)]
pub struct ComputeRequestPayload {
    pub e3_id: Option<u64>,
    #[serde(deserialize_with = "deserialize_hex_string")]
    pub params: Vec<u8>,
    #[serde(deserialize_with = "deserialize_hex_tuple")]
    pub ciphertext_inputs: Vec<(Vec<u8>, u64)>,
    pub callback_url: Option<String>,
}

pub fn deserialize_hex_string<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let hex_str = s.strip_prefix("0x").unwrap_or(&s);
    hex::decode(hex_str).map_err(serde::de::Error::custom)
}

pub fn deserialize_hex_tuple<'de, D>(deserializer: D) -> Result<Vec<(Vec<u8>, u64)>, D::Error>
where
    D: Deserializer<'de>,
{
    let tuples: Vec<(String, u64)> = Deserialize::deserialize(deserializer)?;
    tuples
        .into_iter()
        .map(|(hex_str, num)| {
            let stripped = hex_str.strip_prefix("0x").unwrap_or(&hex_str);
            hex::decode(stripped)
                .map(|bytes| (bytes, num))
                .map_err(serde::de::Error::custom)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::ComputeRequestPayload;

    #[test]
    fn test_deserialize_compute_request() {
        let json = r#"
        {
            "e3_id": 12345,
            "params": "0x12345ffa",
            "ciphertext_inputs": [
                ["0xffabc123", 100],
                ["0xaa6de432", 200]
            ],
            "callback_url": "https://example.com/callback"
        }
        "#;

        let payload: ComputeRequestPayload = serde_json::from_str(json).unwrap();

        assert_eq!(payload.e3_id, Some(12345));
        assert_eq!(payload.params, hex::decode("12345ffa").unwrap());
        assert_eq!(payload.ciphertext_inputs.len(), 2);
        assert_eq!(
            payload.ciphertext_inputs[0],
            (hex::decode("ffabc123").unwrap(), 100)
        );
        assert_eq!(
            payload.ciphertext_inputs[1],
            (hex::decode("aa6de432").unwrap(), 200)
        );
        assert_eq!(
            payload.callback_url,
            Some("https://example.com/callback".to_string())
        );
    }
}
