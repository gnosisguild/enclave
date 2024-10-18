mod plaintext_aggregator;
mod publickey_aggregator;
pub use plaintext_aggregator::{
    PlaintextAggregator, PlaintextAggregatorParams, PlaintextAggregatorState,
};
pub use publickey_aggregator::{
    PublicKeyAggregator, PublicKeyAggregatorParams, PublicKeyAggregatorState,
};
