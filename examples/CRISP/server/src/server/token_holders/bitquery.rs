// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use serde::{Deserialize, Serialize};

/// Represents a token holder with their address and balance.
/// Balance is stored as a string to preserve precision for large numbers.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TokenHolder {
    pub address: String,
    pub balance: String,
}

/// Internal structure for deserializing Bitquery API response.
/// Uses serde rename to match the API's field naming convention.
#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceUpdate {
    #[serde(rename = "Address")]
    pub address: String,
}

/// Internal structure for deserializing Bitquery API response.
/// Contains both the balance and address information from the API.
#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceUpdateResponse {
    #[serde(rename = "Balance")]
    pub balance: String,
    #[serde(rename = "BalanceUpdate")]
    pub balance_update: BalanceUpdate,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphQLResponse {
    pub data: EvmData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvmData {
    #[serde(rename = "EVM")]
    pub evm: EvmDataInner,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvmDataInner {
    #[serde(rename = "BalanceUpdates")]
    pub balance_updates: Vec<BalanceUpdateResponse>,
}

#[derive(Debug, Serialize)]
struct GraphQLRequest {
    query: String,
    variables: serde_json::Value,
}

/// Client for querying token holder data from Bitquery GraphQL API.
pub struct BitqueryClient {
    client: reqwest::Client,
    api_key: String,
}

impl BitqueryClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }

    /// Maps chain IDs to Bitquery network names.
    /// Returns an error for unsupported chains.
    fn get_network_name(chain_id: u64) -> Result<&'static str, String> {
        match chain_id {
            1 => Ok("eth"),
            11155111 => Ok("sepolia"),
            56 => Ok("bsc"),
            137 => Ok("matic"),
            250 => Ok("fantom"),
            43114 => Ok("avalanche"),
            42161 => Ok("arbitrum"),
            10 => Ok("optimism"),
            _ => Err(format!("unsupported chain id: {}", chain_id)),
        }
    }

    /// Retrieves token holders for a specific contract at a given block.
    ///
    /// # Arguments
    /// * `token_contract` - The token contract address
    /// * `block_number` - The block number to query
    /// * `chain_id` - The blockchain network ID
    /// * `limit` - Maximum number of holders to return
    ///
    /// # Returns
    /// A vector of `TokenHolder` structs, or an error if the request fails.
    pub async fn get_token_holders(
        &self,
        token_contract: &str,
        block_number: u64,
        chain_id: u64,
        limit: u32,
    ) -> Result<Vec<TokenHolder>, String> {
        let network = Self::get_network_name(chain_id)?;

        // Build GraphQL query to fetch token holders.
        let query = format!(
            r#"
            {{
                EVM(dataset: archive, network: {}) {{
                    BalanceUpdates(
                        where: {{
                            Block: {{ Number: {{ le: "{}" }} }}
                            Currency: {{ SmartContract: {{ is: "{}" }} }}
                        }}
                        orderBy: [
                            {{ descendingByField: "Balance" }},
                            {{ ascending: BalanceUpdate_Address }}
                        ]
                        limit: {{ count: {} }}
                    ) {{
                        BalanceUpdate {{
                            Address
                        }}
                        Balance: sum(of: BalanceUpdate_Amount)
                    }}
                }}
            }}
            "#,
            network, block_number, token_contract, limit
        );

        let request = GraphQLRequest {
            query,
            variables: serde_json::Value::Object(serde_json::Map::new()),
        };

        // Send authenticated request to Bitquery API.
        let response = self
            .client
            .post("https://streaming.bitquery.io/graphql")
            .header("Authorization", format!("Bearer {}", &self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to send request to Bitquery: {}", e))?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response from Bitquery: {}", e))?;
        if !status.is_success() {
            return Err(format!("Bitquery HTTP {}: {}", status, response_text));
        }

        let graphql_response: GraphQLResponse = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse Bitquery response: {}", e))?;
        let token_holders: Vec<TokenHolder> = graphql_response
            .data
            .evm
            .balance_updates
            .iter()
            .map(|update| TokenHolder {
                address: update.balance_update.address.clone(),
                balance: update.balance.clone(),
            })
            .filter(|h| {
                h.balance
                    .parse::<bigdecimal::BigDecimal>()
                    .map_or(false, |b| b > 0.into())
            })
            .collect();

        Ok(token_holders)
    }
}

/// Convenience function to get mocked token holder data for testing.
/// This is useful when you don't need a BitqueryClient instance.
///
/// # Returns
/// A vector of 10 `TokenHolder` structs with realistic test data.
pub fn get_mock_token_holders() -> Vec<TokenHolder> {
    vec![
        TokenHolder {
            address: "0x1234567890123456789012345678901234567890".to_string(),
            balance: "1000".to_string(),
        },
        TokenHolder {
            address: "0x2345678901234567890123456789012345678901".to_string(),
            balance: "500".to_string(),
        },
        TokenHolder {
            address: "0x3456789012345678901234567890123456789012".to_string(),
            balance: "250".to_string(),
        },
        TokenHolder {
            address: "0x4567890123456789012345678901234567890123".to_string(),
            balance: "100".to_string(),
        },
        TokenHolder {
            address: "0x5678901234567890123456789012345678901234".to_string(),
            balance: "75".to_string(),
        },
        TokenHolder {
            address: "0x6789012345678901234567890123456789012345".to_string(),
            balance: "50".to_string(),
        },
        TokenHolder {
            address: "0x7890123456789012345678901234567890123456".to_string(),
            balance: "25".to_string(),
        },
        TokenHolder {
            address: "0x8901234567890123456789012345678901234567".to_string(),
            balance: "10".to_string(),
        },
        TokenHolder {
            address: "0x9012345678901234567890123456789012345678".to_string(),
            balance: "5".to_string(),
        },
        TokenHolder {
            address: "0x0123456789012345678901234567890123456789".to_string(),
            balance: "1".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    //! Minimal tests for the Bitquery client.
    //!
    //! These include:
    //! - A **live integration test** (ignored by default) that requires a valid `BITQUERY_API_KEY`
    //!   and exercises the real Bitquery API end‑to‑end. Run it manually with:
    //!   `cargo test --package crisp -- --ignored`
    //! - A **negative test** that verifies proper erroring with an invalid API key,
    //!   without depending on any third‑party mocking framework.
    //!
    //! Rationale:
    //! - Keep unit tests hermetic when possible; for external HTTP, run live tests only on demand.
    //! - Avoid "always‑green" tests; failures should surface incorrect credentials or error handling.

    use super::*;
    use std::env;

    /// Returns a known‑good tuple commonly used in examples:
    /// - USDT contract on Ethereum mainnet.
    /// - A historical block chosen to be well after deployment.
    fn example_params() -> (&'static str, u64, u64, u32) {
        (
            // Token contract
            "0xdAC17F958D2ee523a2206206994597C13D831ec7",
            // Historical block height (Ethereum)
            18_500_000,
            // Chain id (Ethereum mainnet)
            1,
            // Limit
            5,
        )
    }

    /// Live end‑to‑end test hitting the real Bitquery GraphQL endpoint.
    ///
    /// Requirements:
    /// - Set a valid environment variable `BITQUERY_API_KEY`.
    /// - Network connectivity.
    ///
    /// Execution:
    /// ```text
    /// cargo test --package crisp -- --ignored
    /// ```
    ///
    /// Expectations:
    /// - The request succeeds (no error).
    /// - The response parses into a non‑empty vector OR an empty vector (both are valid states),
    ///   but the shape must be correct (i.e., no deserialization error).
    #[tokio::test]
    #[ignore]
    async fn live_get_token_holders_succeeds_with_valid_key() {
        let api_key =
            env::var("BITQUERY_API_KEY").expect("Set BITQUERY_API_KEY to run this live test");

        let client = BitqueryClient::new(api_key);
        let (token, block, chain_id, limit) = example_params();

        let res = client
            .get_token_holders(token, block, chain_id, limit)
            .await;
        assert!(res.is_ok(), "Live call failed: {res:?}");

        // Check shape: accessing the vector ensures deserialization happened.
        let holders = res.unwrap();

        // Verify the number of holders matches the expected limit.
        assert_eq!(
            holders.len(),
            5,
            "Expected exactly 5 holders, got {}",
            holders.len()
        );

        // Verify that all holders have valid addresses and balances.
        for holder in &holders {
            assert!(
                !holder.address.is_empty(),
                "Holder address should not be empty"
            );
            assert!(
                !holder.balance.is_empty(),
                "Holder balance should not be empty"
            );
            // Verify address format (should start with 0x and be 42 characters)
            assert!(
                holder.address.starts_with("0x"),
                "Address should start with 0x"
            );
            assert_eq!(
                holder.address.len(),
                42,
                "Address should be 42 characters long"
            );
            // Verify balance is a valid number string.
            assert!(
                holder.balance.parse::<u64>().is_ok() || holder.balance.parse::<f64>().is_ok(),
                "Balance should be a valid number: {}",
                holder.balance
            );
        }
    }

    /// Negative test to ensure invalid credentials are handled as an error.
    ///
    /// This does **not** call any private or unstable API. It simply uses an obviously invalid key
    /// and expects the client to return an error (HTTP 401/403 or provider error mapped by the client).
    ///
    /// Why this test matters:
    /// - Verifies that authentication failures are surfaced as errors instead of being silently swallowed.
    /// - Does not depend on network flakiness; Bitquery consistently rejects invalid tokens.
    #[tokio::test]
    async fn get_token_holders_fails_with_invalid_key() {
        // Use a clearly invalid key; do not rely on any env configuration.
        let client = BitqueryClient::new("invalid_key_for_test_purposes".to_string());
        let (token, block, chain_id, limit) = example_params();

        let res = client
            .get_token_holders(token, block, chain_id, limit)
            .await;

        assert!(
            res.is_err(),
            "Expected an authentication error with invalid key, but got success"
        );
    }
}
