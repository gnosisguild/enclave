// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
use std::sync::Arc;

/// Reference count bytes so event can be cloned and shared between threads
pub type Bytes = Arc<Vec<u8>>;

/// Extension trait for Bytes to provide `from_bytes` method that accepts `&[u8]`
pub trait ArcBytes {
    /// Create Bytes from a byte slice
    fn from_bytes(bytes: &[u8]) -> Self;
}

impl ArcBytes for Bytes {
    fn from_bytes(bytes: &[u8]) -> Self {
        Arc::new(bytes.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::{ArcBytes, Bytes};

    #[test]
    fn test_from_bytes_with_slice() {
        let input: &[u8] = &[1, 2, 3, 4, 5];
        let bytes = Bytes::from_bytes(input);
        
        assert_eq!(bytes.as_ref(), &vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_from_bytes_with_empty_slice() {
        let input: &[u8] = &[];
        let bytes = Bytes::from_bytes(input);
        
        assert_eq!(bytes.as_ref(), &Vec::<u8>::new());
    }
}
