// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use core::fmt;
use std::{ops::Deref, sync::Arc};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::formatters::hexf;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ArcBytes(Arc<Vec<u8>>);

impl ArcBytes {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(Arc::new(bytes))
    }

    pub fn extract_bytes(&self) -> Vec<u8> {
        (*self.0).clone()
    }

    pub fn size_bytes(&self) -> usize {
        (*self.0).clone().len()
    }

    pub fn size_bits(&self) -> usize {
        self.size_bytes() * 8
    }
}

impl Deref for ArcBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Debug for ArcBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        hexf(self, f)
    }
}

impl Serialize for ArcBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ArcBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<u8> = Vec::deserialize(deserializer)?;
        Ok(ArcBytes(Arc::new(vec)))
    }
}
