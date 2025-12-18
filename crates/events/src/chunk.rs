// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Unique identifier for a set of chunks, derived from content hash
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkSetId([u8; 32]);

impl ChunkSetId {
    /// Create a ChunkSetId from the original document content
    pub fn from_content(content: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let result = hasher.finalize();
        Self(result.into())
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for ChunkSetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "chunk:{}", hex::encode(&self.0[..4]))
    }
}

/// A single chunk of a larger document
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkedDocument {
    /// Identifier for the set of chunks this belongs to
    pub chunk_id: ChunkSetId,
    /// Index of this chunk (0-based)
    pub chunk_index: u32,
    /// Total number of chunks in the set
    pub total_chunks: u32,
    /// The actual data
    pub data: Vec<u8>,
}

impl ChunkedDocument {
    /// Create a new chunk
    pub fn new(chunk_id: ChunkSetId, chunk_index: u32, total_chunks: u32, data: Vec<u8>) -> Self {
        Self {
            chunk_id,
            chunk_index,
            total_chunks,
            data,
        }
    }

    /// Create a single-chunk document (no splitting needed)
    pub fn single(data: Vec<u8>) -> Self {
        let chunk_id = ChunkSetId::from_content(&data);
        Self {
            chunk_id,
            chunk_index: 0,
            total_chunks: 1,
            data,
        }
    }

    /// Check if this represents a complete (non-chunked) document
    pub fn is_complete_set(&self) -> bool {
        self.total_chunks == 1
    }

    /// Get the size of this chunk in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_creation() {
        let chunk = ChunkedDocument::single(vec![1, 2, 3, 4]);
        assert_eq!(chunk.chunk_index, 0);
        assert_eq!(chunk.total_chunks, 1);
        assert!(chunk.is_complete_set());
        assert_eq!(chunk.size(), 4);
    }

    #[test]
    fn test_chunk_set_id() {
        let content1 = b"test content 1";
        let content2 = b"test content 2";

        let id1 = ChunkSetId::from_content(content1);
        let id2 = ChunkSetId::from_content(content2);
        assert_ne!(id1, id2);

        // Same content should produce same ID
        let id3 = ChunkSetId::from_content(content1);
        assert_eq!(id1, id3);

        // Test from_bytes round-trip
        let bytes = *id1.as_bytes();
        let id4 = ChunkSetId::from_bytes(bytes);
        assert_eq!(id1, id4);
    }
}
