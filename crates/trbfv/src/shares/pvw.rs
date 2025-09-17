// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use derivative::Derivative;
use std::ops::Deref;

/// Privacy-preserving wrapper for Share data
#[derive(Derivative, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[derivative(Debug)]
pub struct PvwShare(#[derivative(Debug(format_with = "e3_utils::formatters::hexf"))] Vec<u8>);

impl Deref for PvwShare {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PvwShare {
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }
}

/// Privacy-preserving wrapper for ShareSet data
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PvwShareSet(Vec<PvwShare>);

impl PvwShareSet {
    pub fn new(shares: Vec<PvwShare>) -> Self {
        Self(shares)
    }

    pub fn into_vec(self) -> Vec<PvwShare> {
        self.0
    }

    pub fn as_slice(&self) -> &[PvwShare] {
        &self.0
    }
}

/// Privacy-preserving wrapper for ShareSetCollection data
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PvwShareSetCollection(Vec<PvwShareSet>);

impl PvwShareSetCollection {
    pub fn new(share_sets: Vec<PvwShareSet>) -> Self {
        Self(share_sets)
    }

    pub fn into_vec(self) -> Vec<PvwShareSet> {
        self.0
    }

    pub fn as_slice(&self) -> &[PvwShareSet] {
        &self.0
    }
}
