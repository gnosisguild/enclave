use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct E3Crisp {
    // pub emojis: [String; 2], // DONT NEED IN PROGRAM
    pub has_voted: Vec<String>,
    pub start_time: u64,
    pub status: String,
    pub vote_count: u64,
    pub votes_option_1: u64,
    pub votes_option_2: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CurrentRound {
    pub id: u64,
}
