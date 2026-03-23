use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AuctionState {
    Open,
    Computing,
    Complete,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuctionResult {
    pub winner_address: String,
    pub winning_bid: u64,
}

#[derive(Debug)]
pub struct Bid {
    pub address: String,
    pub ciphertext_bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct Auction {
    pub id: u64,
    pub state: AuctionState,
    pub bids: Vec<Bid>,
    pub result: Option<AuctionResult>,
}

impl Auction {
    pub fn new(id: u64) -> Self {
        Auction {
            id,
            state: AuctionState::Open,
            bids: Vec::new(),
            result: None,
        }
    }
}
