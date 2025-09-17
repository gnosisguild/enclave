// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::{bail, Result};
use ndarray::Array2;

use crate::shares::pvw::PvwShareSet;
use crate::shares::share::Share;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ShareSet(pub Vec<Share>);

impl ShareSet {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn get(&self, row: usize) -> Option<&Share> {
        self.0.get(row)
    }

    pub fn get_cloned(&self, row: usize) -> Result<Share> {
        let Some(row) = self.0.get(row) else {
            bail!("Index out of bounds")
        };
        Ok(row.clone())
    }

    pub fn add(&mut self, share: Share) {
        self.0.push(share);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

// This currently serializes but will eventually encrypt to pvw
// Expect to have keys passed in here (via using a tuple)
impl TryInto<ShareSet> for PvwShareSet {
    type Error = anyhow::Error;
    fn try_into(self) -> std::result::Result<ShareSet, Self::Error> {
        Ok(ShareSet(
            self.into_vec()
                .into_iter()
                .map(|s| Share::try_from_pvw(s))
                .collect::<Result<_>>()?,
        ))
    }
}

// This currently serializes but will eventually encrypt to pvw
// Expect to have keys passed in here (via using a tuple)
impl TryInto<PvwShareSet> for ShareSet {
    type Error = anyhow::Error;
    fn try_into(self) -> std::result::Result<PvwShareSet, Self::Error> {
        Ok(PvwShareSet::new(
            self.0
                .into_iter()
                .map(|s| s.try_into_pvw())
                .collect::<Result<_>>()?,
        ))
    }
}

impl From<Array2<u64>> for ShareSet {
    fn from(value: Array2<u64>) -> Self {
        // This consumes the ndarray
        let shares = value
            .rows()
            .into_iter()
            .map(|row| Share::new(row.to_vec()))
            .collect();
        ShareSet(shares)
    }
}

impl TryFrom<ShareSet> for Array2<u64> {
    type Error = anyhow::Error;

    fn try_from(share_set: ShareSet) -> Result<Self, Self::Error> {
        if share_set.0.is_empty() {
            bail!("Cannot convert empty ShareSet to Array2");
        }

        let num_rows = share_set.0.len();
        let num_cols = share_set.0[0].len();

        let data: Vec<u64> = share_set
            .0
            .into_iter()
            .flat_map(|share| (*share).clone())
            .collect();

        Array2::from_shape_vec((num_rows, num_cols), data)
            .map_err(|e| anyhow::anyhow!("Shape mismatch: {}", e))
    }
}
