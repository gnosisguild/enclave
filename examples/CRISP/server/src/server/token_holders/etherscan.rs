use reqwest;
use serde::{Deserialize};
use std::error::Error;
use tokio::time::{sleep, Duration};
use alloy::primitives::{Address, U256};
use std::collections::{HashMap, HashSet};

// Config
pub const ETHERSCAN_API_URL: &str = "https://api.etherscan.io/v2/api";
const ZERO_ADDRESS: Address = Address::ZERO;

// Response types
#[derive(Debug, Deserialize)]
struct EtherscanResponse<T> {
    status: String,
    message: String,
    result: Option<T>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContractCreation {
    block_number: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransferLog {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
    pub block_number: String,
    pub transaction_hash: String,
    pub transaction_index: String,
    pub block_hash: String,
    pub log_index: String,
}

/// Get the deployment block number for a contract
pub async fn get_deployment_block(
    token: &str,
    chain_id: u64,
    api_key: &str,
) -> Result<u64, Box<dyn Error>> {
    let client = reqwest::Client::new();
    
    let url = format!(
        "{}?module=contract&action=getcontractcreation&contractaddresses={}&chainid={}&apikey={}",
        ETHERSCAN_API_URL, token, chain_id, api_key
    );

    let response = client.get(&url).send().await?;
    let data: EtherscanResponse<Vec<ContractCreation>> = response.json().await?;

    if data.status != "1" {
        return Err(format!("Deployment block not found: {}", data.message).into());
    }

    let result = data.result
        .and_then(|r| r.into_iter().next())
        .ok_or("No deployment data found")?;

    // Parse block number (could be hex or decimal)
    let block_number = if result.block_number.starts_with("0x") {
        u64::from_str_radix(&result.block_number[2..], 16)?
    } else {
        result.block_number.parse::<u64>()?
    };

    Ok(block_number)
}

/// Get transfer logs for a token
pub async fn get_transfer_logs(
    token: &str,
    from_block: u64,
    to_block: u64,
    chain_id: u64,
    api_key: &str,
) -> Result<Vec<TransferLog>, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let mut all_logs = Vec::new();
    let mut page = 1;

    // ERC20 Transfer event signature
    let transfer_topic = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

    loop {
        let url = format!(
            "{}?module=logs&action=getLogs&address={}&fromBlock={}&toBlock={}&topic0={}&page={}&offset=10000&chainid={}&apikey={}",
            ETHERSCAN_API_URL, token, from_block, to_block, transfer_topic, page, chain_id, api_key
        );

        let response = client.get(&url).send().await?;
        let data: EtherscanResponse<Vec<TransferLog>> = response.json().await?;

        // Break if request failed
        if data.status != "1" {
            break;
        }

        // Break if no results
        let logs = match data.result {
            Some(logs) if !logs.is_empty() => logs,
            _ => break,
        };

        let log_count = logs.len();
        all_logs.extend(logs);

        // Break if we got less than the max page size
        if log_count < 10000 {
            break;
        }

        page += 1;

        // Rate limiting - wait 100ms between requests
        sleep(Duration::from_millis(100)).await;
    }

    Ok(all_logs)
}

/// Extract unique addresses from transfer logs
pub fn extract_addresses(logs: &[TransferLog]) -> Vec<Address> {
    let mut addresses = HashSet::new();

    for log in logs {
        if log.topics.len() >= 3 {
            // Extract addresses from topics (topics are 32 bytes, address is last 20 bytes)
            if let Ok(from) = parse_address_from_topic(&log.topics[1]) {
                if from != ZERO_ADDRESS {
                    addresses.insert(from);
                }
            }
            
            if let Ok(to) = parse_address_from_topic(&log.topics[2]) {
                if to != ZERO_ADDRESS {
                    addresses.insert(to);
                }
            }
        }
    }

    addresses.into_iter().collect()
}

/// Compute token balances from transfer logs
pub fn compute_balances_from_logs(logs: &[TransferLog]) -> HashMap<Address, U256> {
    let mut balances: HashMap<Address, U256> = HashMap::new();

    // Sort logs by block number to ensure chronological order
    let mut sorted_logs = logs.to_vec();
    sorted_logs.sort_by(|a, b| {
        let block_a = parse_block_number(&a.block_number);
        let block_b = parse_block_number(&b.block_number);
        block_a.cmp(&block_b)
    });

    for log in sorted_logs {
        if log.topics.len() < 3 {
            continue;
        }

        // Extract from and to addresses from Transfer event topics
        let from = match parse_address_from_topic(&log.topics[1]) {
            Ok(addr) => addr,
            Err(_) => continue,
        };
        
        let to = match parse_address_from_topic(&log.topics[2]) {
            Ok(addr) => addr,
            Err(_) => continue,
        };

        // Parse the transfer value (ERC-20 Transfer has value as uint256 ABI-encoded)
        let value = parse_transfer_value(&log.data);

        // Update balances
        if from != ZERO_ADDRESS {
            let balance = balances.entry(from).or_insert(U256::ZERO);
            *balance = balance.saturating_sub(value);
        }

        if to != ZERO_ADDRESS {
            let balance = balances.entry(to).or_insert(U256::ZERO);
            *balance = balance.saturating_add(value);
        }
    }

    // Check for negative balances (would underflow with U256)
    for (addr, bal) in &balances {
        if *bal == U256::ZERO {
            // This could indicate underflow was prevented by saturating_sub
            log::warn!("Potential underflow detected for address: {}", addr);
        }
    }

    balances
}

/// Parse address from 32-byte topic (last 20 bytes)
fn parse_address_from_topic(topic: &str) -> Result<Address, String> {
    // Remove "0x" prefix if present
    let hex = topic.strip_prefix("0x").unwrap_or(topic);
    
    // Topics are 32 bytes (64 hex chars), addresses are last 20 bytes (40 hex chars)
    if hex.len() >= 40 {
        let addr_hex = &hex[hex.len() - 40..];
        addr_hex.parse::<Address>()
            .map_err(|e| format!("Failed to parse address: {}", e))
    } else {
        Err("Topic too short".to_string())
    }
}

/// Parse block number from hex or decimal string
fn parse_block_number(block_number: &str) -> u64 {
    if block_number.starts_with("0x") {
        u64::from_str_radix(&block_number[2..], 16).unwrap_or(0)
    } else {
        block_number.parse::<u64>().unwrap_or(0)
    }
}

/// Parse transfer value from hex data string
fn parse_transfer_value(data: &str) -> U256 {
    // Remove "0x" prefix if present
    let hex_data = data.strip_prefix("0x").unwrap_or(data);
    
    // Parse as U256
    U256::from_str_radix(hex_data, 16).unwrap_or(U256::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_addresses() {
        let logs = vec![
            TransferLog {
                address: "0xtoken".to_string(),
                topics: vec![
                    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef".to_string(),
                    "0x000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".to_string(),
                    "0x000000000000000000000000dac17f958d2ee523a2206206994597c13d831ec7".to_string(),
                ],
                data: "0x0000000000000000000000000000000000000000000000000000000000000064".to_string(),
                block_number: "0x1".to_string(),
                transaction_hash: "0xhash".to_string(),
                transaction_index: "0x0".to_string(),
                block_hash: "0xblockhash".to_string(),
                log_index: "0x0".to_string(),
            },
        ];

        let addresses = extract_addresses(&logs);
        assert_eq!(addresses.len(), 2);
        
        let addr1: Address = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".parse().unwrap();
        let addr2: Address = "0xdac17f958d2ee523a2206206994597c13d831ec7".parse().unwrap();
        
        assert!(addresses.contains(&addr1));
        assert!(addresses.contains(&addr2));
    }

    #[test]
    fn test_compute_balances() {
        let logs = vec![
            TransferLog {
                address: "0xtoken".to_string(),
                topics: vec![
                    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef".to_string(),
                    "0x0000000000000000000000000000000000000000000000000000000000000000".to_string(), // from: zero address (mint)
                    "0x000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".to_string(), // to: address A
                ],
                data: "0x0000000000000000000000000000000000000000000000000000000000000064".to_string(), // 100 tokens
                block_number: "0x1".to_string(),
                transaction_hash: "0xhash1".to_string(),
                transaction_index: "0x0".to_string(),
                block_hash: "0xblock1".to_string(),
                log_index: "0x0".to_string(),
            },
            TransferLog {
                address: "0xtoken".to_string(),
                topics: vec![
                    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef".to_string(),
                    "0x000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".to_string(), // from: address A
                    "0x000000000000000000000000dac17f958d2ee523a2206206994597c13d831ec7".to_string(), // to: address B
                ],
                data: "0x0000000000000000000000000000000000000000000000000000000000000032".to_string(), // 50 tokens
                block_number: "0x2".to_string(),
                transaction_hash: "0xhash2".to_string(),
                transaction_index: "0x0".to_string(),
                block_hash: "0xblock2".to_string(),
                log_index: "0x0".to_string(),
            },
        ];

        let balances = compute_balances_from_logs(&logs);
        
        let addr_a: Address = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".parse().unwrap();
        let addr_b: Address = "0xdac17f958d2ee523a2206206994597c13d831ec7".parse().unwrap();
        
        // Address A: received 100, sent 50 = 50
        assert_eq!(balances.get(&addr_a), Some(&U256::from(50)));
        
        // Address B: received 50
        assert_eq!(balances.get(&addr_b), Some(&U256::from(50)));
    }

    #[test]
    fn test_parse_address_from_topic() {
        let topic = "0x000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
        let addr = parse_address_from_topic(topic).unwrap();
        let expected: Address = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".parse().unwrap();
        assert_eq!(addr, expected);
    }

    #[test]
    fn test_parse_transfer_value() {
        assert_eq!(parse_transfer_value("0x64"), U256::from(100));
        assert_eq!(parse_transfer_value("0x0"), U256::ZERO);
        assert_eq!(
            parse_transfer_value("0x0000000000000000000000000000000000000000000000000000000000000064"),
            U256::from(100)
        );
    }

    /// Integration tests (requires valid API key)

    #[tokio::test]
    #[ignore]
    async fn test_get_deployment_block() {
        let token = "0xb0BE360719f84c5351621590B7FfBD8EB0B46B5d"; // Your token address
        let chain_id = 11155111;
        let api_key = "xxx"; // Your Etherscan API key

        let result = get_deployment_block(token, chain_id, api_key).await;
        println!("Deployment block: {:?}", result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_transfer_logs() {
        let token = "0xb0BE360719f84c5351621590B7FfBD8EB0B46B5d";
        let from_block = 9501710;
        let to_block = 9501723;
        let chain_id = 11155111;
        let api_key = "xxx"; // Your Etherscan API key

        let result = get_transfer_logs(token, from_block, to_block, chain_id, api_key).await;
        println!("Transfer logs: {:?}", result);
        assert!(result.is_ok());
    }
}
