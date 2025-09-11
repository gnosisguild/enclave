use anyhow::{bail, Result};
use ndarray::Array2;
use std::ops::Deref;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Share {
    inner: Vec<u64>,
}

impl Deref for Share {
    type Target = Vec<u64>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Share {
    pub fn into_vec(self) -> Vec<u64> {
        self.inner
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ShareSet {
    shares: Vec<Share>,
}

impl ShareSet {
    pub fn new() -> Self {
        Self { shares: vec![] }
    }

    pub fn from(array: Array2<u64>) -> Self {
        // This consumes the ndarray
        let shares = array
            .rows()
            .into_iter()
            .map(|row| Share {
                inner: row.to_vec(),
            })
            .collect();

        ShareSet { shares }
    }

    pub fn get(&self, row: usize) -> Option<&Share> {
        self.shares.get(row)
    }

    pub fn get_cloned(&self, row: usize) -> Result<Share> {
        let Some(row) = self.shares.get(row) else {
            bail!("Index out of bounds")
        };

        Ok(row.clone())
    }

    pub fn add(&mut self, share: Share) {
        self.shares.push(share);
    }

    pub fn len(&self) -> usize {
        self.shares.len()
    }
}

// XXX: Implement From trait so we can apply From trait to collection easily
// impl From<Array2<u64>

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ShareSetCollection {
    sets: Vec<ShareSet>,
}

impl IntoIterator for ShareSetCollection {
    type Item = ShareSet;
    type IntoIter = std::vec::IntoIter<ShareSet>;

    fn into_iter(self) -> Self::IntoIter {
        self.sets.into_iter()
    }
}

impl ShareSetCollection {
    /// Get the modudlus length for this shamir set
    pub fn len(&self) -> usize {
        self.sets.len()
    }

    /// get the number of total parties in this shamir set
    pub fn get_total_parties(&self) -> Result<usize> {
        let Some(first) = self.sets.first() else {
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

            // Create a set for this collection
            let mut set = ShareSet::new();
            for ext_set in col {
                // Check Invariant
                if ext_set.len() != total {
                    bail!("External shamir set length {} does not corresppond with the total length {}", ext_set.len(), total)
                }

                // We need to extract the specific share for our party_id from the external set
                let share = ext_set.get_cloned(party_id)?;
                set.add(share);
            }
            sets.push(set)
        }

        Ok(Self { sets })
    }
}
