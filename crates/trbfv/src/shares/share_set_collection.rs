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

use super::Share;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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
        let mut matrix = vec![];
        for col in collections {
            // Check Invariant
            if col.len() != len {
                bail!("Invalid external modulus length!")
            }

            let set = col.extract_set(party_id, total)?;

            matrix.push(set);
        }

        let share_sets: Vec<ShareSet> = transpose_matrix(matrix)
            .into_iter()
            .map(|s| ShareSet::from(s))
            .collect();

        Ok(ShareSetCollection::from(share_sets))
    }

    // We need to extract the specific share for our party_id from the external set
    pub fn extract_set(&self, party_id: usize, total: usize) -> Result<Vec<Share>> {
        let mut set = vec![];
        for current in self.0.iter() {
            if current.len() != total {
                bail!(
                    "External shamir set length {} does not corresppond with the total length {}",
                    current.len(),
                    total
                )
            }
            let share = current.get_cloned(party_id)?;
            set.push(share);
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

fn transpose_matrix<T: Clone>(matrix: Vec<Vec<T>>) -> Vec<Vec<T>> {
    if matrix.is_empty() || matrix[0].is_empty() {
        return Vec::new();
    }

    let cols = matrix[0].len();

    (0..cols)
        .map(|col_idx| matrix.iter().map(|row| row[col_idx].clone()).collect())
        .collect()
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

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::shares::{Share, ShareSet};

    use super::ShareSetCollection;

    #[test]
    fn test_swap_shares() -> Result<()> {
        let collections = vec![
            // party 0
            ShareSetCollection::from(vec![
                // mod 0
                ShareSet::from(vec![
                    // [from, mod, to]
                    Share::new(vec![0, 0, 0]),
                    Share::new(vec![0, 0, 1]),
                    Share::new(vec![0, 0, 2]),
                    Share::new(vec![0, 0, 3]),
                ]),
                // mod 1
                ShareSet::from(vec![
                    // [from, mod, to]
                    Share::new(vec![0, 1, 0]),
                    Share::new(vec![0, 1, 1]),
                    Share::new(vec![0, 1, 2]),
                    Share::new(vec![0, 1, 3]),
                ]),
                // mod 2
                ShareSet::from(vec![
                    // [from, mod, to]
                    Share::new(vec![0, 2, 0]),
                    Share::new(vec![0, 2, 1]),
                    Share::new(vec![0, 2, 2]),
                    Share::new(vec![0, 2, 3]),
                ]),
            ]),
            // party 1
            ShareSetCollection::from(vec![
                // mod 0
                ShareSet::from(vec![
                    // [from, mod, to]
                    Share::new(vec![1, 0, 0]),
                    Share::new(vec![1, 0, 1]),
                    Share::new(vec![1, 0, 2]),
                    Share::new(vec![1, 0, 3]),
                ]),
                // mod 1
                ShareSet::from(vec![
                    // [from, mod, to]
                    Share::new(vec![1, 1, 0]),
                    Share::new(vec![1, 1, 1]),
                    Share::new(vec![1, 1, 2]),
                    Share::new(vec![1, 1, 3]),
                ]),
                // mod 2
                ShareSet::from(vec![
                    // [from, mod, to]
                    Share::new(vec![1, 2, 0]),
                    Share::new(vec![1, 2, 1]),
                    Share::new(vec![1, 2, 2]),
                    Share::new(vec![1, 2, 3]),
                ]),
            ]),
            // party 2
            ShareSetCollection::from(vec![
                // mod 0
                ShareSet::from(vec![
                    // [from, mod, to]
                    Share::new(vec![2, 0, 0]),
                    Share::new(vec![2, 0, 1]),
                    Share::new(vec![2, 0, 2]),
                    Share::new(vec![2, 0, 3]),
                ]),
                // mod 1
                ShareSet::from(vec![
                    // [from, mod, to]
                    Share::new(vec![2, 1, 0]),
                    Share::new(vec![2, 1, 1]),
                    Share::new(vec![2, 1, 2]),
                    Share::new(vec![2, 1, 3]),
                ]),
                // mod 2
                ShareSet::from(vec![
                    // [from, mod, to]
                    Share::new(vec![2, 2, 0]),
                    Share::new(vec![2, 2, 1]),
                    Share::new(vec![2, 2, 2]),
                    Share::new(vec![2, 2, 3]),
                ]),
            ]),
            // party 3
            ShareSetCollection::from(vec![
                // mod 0
                ShareSet::from(vec![
                    // [from, mod, to]
                    Share::new(vec![3, 0, 0]),
                    Share::new(vec![3, 0, 1]),
                    Share::new(vec![3, 0, 2]),
                    Share::new(vec![3, 0, 3]),
                ]),
                // mod 1
                ShareSet::from(vec![
                    // [from, mod, to]
                    Share::new(vec![3, 1, 0]),
                    Share::new(vec![3, 1, 1]),
                    Share::new(vec![3, 1, 2]),
                    Share::new(vec![3, 1, 3]),
                ]),
                // mod 2
                ShareSet::from(vec![
                    // [from, mod, to]
                    Share::new(vec![3, 2, 0]),
                    Share::new(vec![3, 2, 1]),
                    Share::new(vec![3, 2, 2]),
                    Share::new(vec![3, 2, 3]),
                ]),
            ]),
        ];

        let party_3 = ShareSetCollection::from_received(collections, 3)?;
        assert_eq!(
            party_3,
            // party 3
            ShareSetCollection::from(vec![
                // mod 0
                ShareSet::from(vec![
                    // [from, mod, to]
                    Share::new(vec![0, 0, 3]),
                    Share::new(vec![1, 0, 3]),
                    Share::new(vec![2, 0, 3]),
                    Share::new(vec![3, 0, 3]),
                ]),
                // mod 1
                ShareSet::from(vec![
                    // [from, mod, to]
                    Share::new(vec![0, 1, 3]),
                    Share::new(vec![1, 1, 3]),
                    Share::new(vec![2, 1, 3]),
                    Share::new(vec![3, 1, 3]),
                ]),
                // mod 2
                ShareSet::from(vec![
                    // [from, mod, to]
                    Share::new(vec![0, 2, 3]),
                    Share::new(vec![1, 2, 3]),
                    Share::new(vec![2, 2, 3]),
                    Share::new(vec![3, 2, 3]),
                ]),
            ]),
        );

        Ok(())
    }
}
