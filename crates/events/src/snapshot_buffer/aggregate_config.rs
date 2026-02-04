// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
use crate::AggregateId;
use std::collections::HashMap;

/// Central configuration for aggregates in the WriteBuffer
#[derive(Debug, Clone)]
pub struct AggregateConfig {
    pub delays: HashMap<AggregateId, u64>,
}

impl AggregateConfig {
    pub fn get_delay(&self, id: &AggregateId) -> u64 {
        self.delays.get(id).cloned().unwrap_or(0)
    }
}

impl AggregateConfig {
    /// Create a new AggregateConfig with the specified delays
    pub fn new(mut delays: HashMap<AggregateId, u64>) -> Self {
        // Always handle AggregatId of 0 with a delay of 0
        if let None = delays.get(&AggregateId::new(0)) {
            delays.insert(AggregateId::new(0), 0);
        }
        Self { delays }
    }

    /// Get the indexed aggregate IDs, defaulting to [0] if no delays are configured
    pub fn indexed_ids(&self) -> Vec<usize> {
        self.delays.keys().map(|id| id.to_usize()).collect()
    }

    pub fn aggregates(&self) -> Vec<AggregateId> {
        self.delays.keys().cloned().collect()
    }
}
