// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::{bail, Result};
use e3_crypto::Cipher;
use ndarray::Array2;

use crate::shares::encrypted::EncryptedShareSetCollection;
use crate::shares::pvw::PvwShareSetCollection;
use crate::shares::share_set::ShareSet;
use e3_crypto::ToSensitiveBytes;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ShareSetCollection(pub Vec<ShareSet>);

impl From<Vec<Array2<u64>>> for ShareSetCollection {
    fn from(value: Vec<Array2<u64>>) -> Self {
        ShareSetCollection(value.into_iter().map(|v| v.into()).collect())
    }
}

impl From<Vec<ShareSet>> for ShareSetCollection {
    fn from(value: Vec<ShareSet>) -> Self {
        ShareSetCollection(value.into_iter().map(|v| v.into()).collect())
    }
}

impl IntoIterator for ShareSetCollection {
    type Item = ShareSet;
    type IntoIter = std::vec::IntoIter<ShareSet>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl ShareSetCollection {
    /// Get the modudlus length for this shamir set
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// get the number of total parties in this shamir set
    pub fn get_total_parties(&self) -> Result<usize> {
        let Some(first) = self.0.first() else {
            bail!("No sets share sets");
        };
        Ok(first.len())
    }

    /// Received one ShareSetCollection from all other parties assume order based on party_id
    pub fn from_received(collections: Vec<ShareSetCollection>, party_id: usize) -> Result<Self> {
        let Some(first) = collections.first() else {
            bail!("No external sets found")
        };

        // Keep track of the modulus and total parties
        let len = first.len();
        let total = first.get_total_parties()?;

        // Each external set is in order based on party_id
        // Here we start a sets vector for this collection
        let mut sets = vec![];
        for col in collections {
            // Check Invariant
            if col.len() != len {
                bail!("Invalid external modulus length!")
            }
            let set = col.extract_set(party_id, total)?;
            sets.push(set)
        }
        Ok(Self(sets))
    }

    // We need to extract the specific share for our party_id from the external set
    pub fn extract_set(&self, party_id: usize, total: usize) -> Result<ShareSet> {
        let mut set = ShareSet::new();
        for current in self.0.iter() {
            if current.len() != total {
                bail!(
                    "External shamir set length {} does not corresppond with the total length {}",
                    current.len(),
                    total
                )
            }
            let share = current.get_cloned(party_id)?;
            set.add(share);
        }
        Ok(set)
    }

    pub fn try_to_ndarray_vec(&self) -> Result<Vec<Array2<u64>>> {
        self.0.iter().cloned().map(|s| s.try_into()).collect()
    }

    pub fn encrypt(&self, cipher: &Cipher) -> Result<EncryptedShareSetCollection> {
        let out = self
            .0
            .iter()
            .map(|v| {
                v.0.iter()
                    .map(|sh| sh.encrypt(cipher))
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<Vec<_>>>>()?;
        Ok(EncryptedShareSetCollection::new(out))
    }
}

// This currently serializes but will eventually encrypt to pvw
// Expect to have keys passed in here as a tuple
impl TryFrom<ShareSetCollection> for PvwShareSetCollection {
    type Error = anyhow::Error;

    fn try_from(value: ShareSetCollection) -> std::result::Result<Self, Self::Error> {
        Ok(PvwShareSetCollection::new(
            value
                .into_iter()
                .map(|s| s.try_into())
                .collect::<Result<_>>()?,
        ))
    }
}

// This currently serializes but will eventually encrypt to pvw
// Expect to have keys passed in here as a tuple
impl TryFrom<PvwShareSetCollection> for ShareSetCollection {
    type Error = anyhow::Error;
    fn try_from(value: PvwShareSetCollection) -> std::result::Result<Self, Self::Error> {
        Ok(ShareSetCollection(
            value
                .into_vec()
                .into_iter()
                .map(|s| s.try_into())
                .collect::<Result<_>>()?,
        ))
    }
}
