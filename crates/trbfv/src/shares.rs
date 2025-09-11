use anyhow::{bail, Result};
use e3_crypto::{Cipher, SensitiveBytes, ToSensitiveBytes};
use ndarray::Array2;
use std::ops::Deref;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Share(Vec<u64>);

impl Deref for Share {
    type Target = Vec<u64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Share {
    pub fn into_vec(self) -> Vec<u64> {
        self.0
    }

    pub fn new(v: Vec<u64>) -> Self {
        Self(v)
    }
}

impl ToSensitiveBytes for Share {
    fn encrypt(&self, cipher: &Cipher) -> Result<SensitiveBytes> {
        Ok(SensitiveBytes::new(bincode::serialize(&self.0)?, cipher)?)
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ShareSet(Vec<Share>);

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

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ShareSetCollection(Vec<ShareSet>);

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
                    .collect::<Result<Vec<SensitiveBytes>>>()
            })
            .collect::<Result<Vec<Vec<_>>>>()?;

        Ok(EncryptedShareSetCollection(out))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct EncryptedShareSetCollection(Vec<Vec<SensitiveBytes>>);

impl EncryptedShareSetCollection {
    pub fn decrypt(&self, cipher: &Cipher) -> Result<ShareSetCollection> {
        let out = self
            .0
            .iter()
            .map(|v| {
                Ok(ShareSet(
                    v.iter()
                        .map(|s| Ok(Share::new(bincode::deserialize(&s.access_raw(cipher)?)?)))
                        .collect::<Result<Vec<Share>>>()?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(ShareSetCollection(out))
    }
}
