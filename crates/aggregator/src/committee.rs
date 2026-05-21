// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::primitives::Address;
use anyhow::{anyhow, Result};
use e3_events::OrderedSet;
use std::str::FromStr;

/// Parse ordered committee node strings (`topNodes` / `PublicKeyAggregated.nodes`) once at ingress.
pub fn committee_addresses_from_nodes(nodes: &OrderedSet<String>) -> Result<Vec<Address>> {
    nodes
        .iter()
        .map(|s| {
            Address::from_str(s).map_err(|e| anyhow!("invalid committee node address {s}: {e}"))
        })
        .collect()
}
