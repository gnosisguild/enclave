use core::fmt;
use std::{ops::Deref, sync::Arc};

use crate::formatters::hexf;

#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ArcBytes(Arc<Vec<u8>>);

impl ArcBytes {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(Arc::new(bytes))
    }

    pub fn extract_bytes(&self) -> Vec<u8> {
        (*self.0).clone()
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
